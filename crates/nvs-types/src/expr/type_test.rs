//! `rule:types/type-test`'s `$x is T`: what it answers, what it refuses, and
//! why the list of refusals is as short as it is.
//!
//! The operator is **total**. Every value has a representation, so every
//! subject has an answer, and this module refuses nothing about the left-hand
//! side — not even a subject whose declared type settles the question. That is
//! the one thing most likely to be written here by analogy with
//! [`super::members::infer_instanceof`], which *does* refuse a subject that can
//! hold no object: `instanceof` needs a class to test against and a scalar has
//! none, so the operator is inapplicable there rather than merely predictable.
//! ADR 0150 § 6 is the argument, and it also names the second reason — once
//! `is` narrows, a guard written inside an already-narrowed branch is
//! statically true by construction, and refusing that would let a flow
//! analysis turn working code into a compile error.
//!
//! A settled answer **folds** instead: the expression's type is `true` or
//! `false` rather than `bool`, with no diagnostic and no warning
//! (`rule:types/literal-types` is what makes those types sayable). Both folds
//! are deliberately one-sided, in [`always_holds`]'s and
//! [`never_holds`]'s directions: what neither proves types as plain `bool` and
//! is answered at run time, which is never wrong — only, at worst, a run-time
//! test where a constant would have done.
//!
//! The two refusals here are about the **right**-hand side, and both are cases
//! where there is no question rather than a knowable answer:
//! [`code::E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE`] for `void` and `never`,
//! which no value inhabits, and
//! [`code::E_TYPE_TEST_AGAINST_A_QUALIFIER`] for a `tainted` or `secret`
//! qualifier, which is erased before codegen and leaves no bit to read. The
//! third refusal in `rule:types/type-test`'s table is the parser's, where the
//! `$` of `$x is $cls` is still in hand.
//!
//! A test that survives both folds and both refusals **records the type it
//! lowered** ([`crate::expr_table::ExprInfo::TypeTest`]), because that is the
//! one case with a run-time answer: narrowing reads it on the true edge, and
//! `nvs-ir` reads it to emit the test. A folded one records the constant
//! instead ([`crate::expr_table::ExprInfo::SettledTypeTest`]) — which variant
//! is on the span is what tells the two apart, and the constant is carried
//! rather than re-derived because the fold is a question about types that no
//! longer exist below this crate. A refused test records neither: it has a
//! diagnostic, so no lowering ever sees it.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{Expr, Type};
use rustc_hash::FxHashSet;

use crate::expr_table::ExprInfo;
use crate::locals::LocalScope;
use crate::lower::lower_type;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env};

use super::assign::is_assignable;
use super::check_expr;
use super::operators::types_are_disjoint;

/// Checks `inner is ty`, answering `bool` — or the literal `true`/`false` the
/// two types settle between them.
///
/// The subject is checked for its own sake as much as for the fold: it is an
/// ordinary expression, and nothing else in this walk would visit it.
///
/// A test that reaches `bool` records the type it lowered
/// ([`ExprInfo::TypeTest`]), which is what [`crate::locals::narrow`] reads to
/// narrow the subject on the true edge and what `nvs-ir` reads to emit the
/// test. A folded one records [`ExprInfo::SettledTypeTest`] and its constant
/// instead: there is no run-time test left to narrow inside, and lowering
/// answers with the constant while still running the subject for its effects.
/// A refused one records nothing at all.
pub(crate) fn infer_type_test(
    expr: &Expr,
    inner: &Expr,
    ty: &Type,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let tested = lower_type(ty, ctx, env);
    let subject = check_expr(inner, None, live, scope, ctx, env);
    if reject_unanswerable_target(tested, expr.span, env) {
        return env.interner.bool_ty();
    }
    if always_holds(subject, tested, env) {
        env.exprs
            .record(expr.span, ExprInfo::SettledTypeTest { answer: true });
        return env.interner.true_ty();
    }
    if never_holds(subject, tested, env) {
        env.exprs
            .record(expr.span, ExprInfo::SettledTypeTest { answer: false });
        return env.interner.false_ty();
    }
    env.exprs.record(expr.span, ExprInfo::TypeTest { tested });
    env.interner.bool_ty()
}

/// The two right-hand sides that have no answer rather than a knowable one.
/// Reports the first that applies and answers whether anything was reported.
fn reject_unanswerable_target(tested: TypeId, span: Span, env: &mut Env<'_>) -> bool {
    if matches!(env.interner.get(tested), Ty::Void | Ty::Never) {
        let described = env.interner.describe(tested);
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE,
                format!("no value is ever a `{described}`, so `is {described}` has no answer"),
            )
            .with_primary(span, "tested here")
            .with_help("test against a type a value can hold"),
        );
        return true;
    }
    if let Some(qualified) = qualified_atom(tested, env) {
        let described = env.interner.describe(qualified);
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_TEST_AGAINST_A_QUALIFIER,
                format!(
                    "`{described}` is a qualifier the compiler erases, so no value carries it at \
                     run time"
                ),
            )
            .with_primary(span, "tested here")
            .with_help(
                "test the unqualified type — a qualifier is decided where the value is declared, \
                 not where it is read",
            ),
        );
        return true;
    }
    false
}

/// The first `tainted`/`secret` atom inside `tested`, wherever it is written.
///
/// Not just the top level: `tainted {a: string}` is rewritten while parsing
/// into the shape whose text fields are their tainted forms
/// (`rule:security/tainted-qualifier`), so a qualifier written over a shape
/// arrives here inside one, and `array<tainted string>` puts it inside an
/// element. Every one of those is the same missing bit.
fn qualified_atom(tested: TypeId, env: &Env<'_>) -> Option<TypeId> {
    match env.interner.get(tested) {
        Ty::TaintedString
        | Ty::TaintedBytes
        | Ty::SecretString
        | Ty::SecretBytes
        | Ty::SecretTaintedString
        | Ty::SecretTaintedBytes => Some(tested),
        Ty::Array(element) => qualified_atom(*element, env),
        Ty::Union(members) | Ty::Intersection(members) => {
            members.iter().find_map(|m| qualified_atom(*m, env))
        }
        Ty::Shape(fields) => fields.iter().find_map(|f| qualified_atom(f.ty, env)),
        _ => None,
    }
}

/// Whether every value of `subject` holds a `tested` — the `true` fold.
///
/// Assignability answers this for every row of `rule:types/type-test`'s table
/// bar one, since a type that is a subtype of the target *is* the target at run
/// time: a `Dog` holds an `Animal`, a `"a"` holds a `string`, a `tainted
/// string` holds a `string` (the qualifier being erased), and a shape with more
/// fields holds the shape with fewer.
///
/// The exception is the one implicit conversion the language has
/// (`rule:types/conversion`): an `int` *occupies* a `float` position by
/// converting, and `is` asks for the tag it does not carry. Unions are
/// decomposed here rather than left to [`is_assignable`] so that the exception
/// is reached at every atom, not only at the top.
fn always_holds(subject: TypeId, tested: TypeId, env: &mut Env<'_>) -> bool {
    if matches!(env.interner.get(tested), Ty::Mixed) {
        return true;
    }
    if subject == tested {
        return true;
    }
    if let Ty::Union(members) = env.interner.get(subject) {
        let members = members.clone();
        return members.iter().all(|m| always_holds(*m, tested, env));
    }
    if let Ty::Union(members) = env.interner.get(tested) {
        let members = members.clone();
        return members.iter().any(|m| always_holds(subject, *m, env));
    }
    if matches!(env.interner.get(subject), Ty::Int | Ty::Uint)
        && matches!(env.interner.get(tested), Ty::Float)
    {
        return false;
    }
    is_assignable(subject, tested, env.interner, env.graph, env.signatures)
}

/// Whether no value of `subject` holds a `tested` — the `false` fold, and
/// exactly `rule:expressions/disjoint-comparison-refused`'s disjointness read
/// as a question about one value instead of two.
///
/// That check is already one-sided in the direction this needs: it answers
/// `true` only where disjointness is provable from the two types alone, and a
/// qualifier or a literal shares its base's domain, so neither `tainted string`
/// against `string` nor `"a"` against `"b"` folds away a test that has real
/// work to do at run time.
fn never_holds(subject: TypeId, tested: TypeId, env: &Env<'_>) -> bool {
    types_are_disjoint(subject, tested, env)
}
