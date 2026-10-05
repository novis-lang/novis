//! `rule:types/type-test`'s `$x is T`: what it answers, what it refuses, and
//! why the list of refusals is as short as it is.
//!
//! The operator is **total**. Every value has a representation, so every
//! subject has an answer, and this module refuses nothing about the left-hand
//! side — not even a subject whose declared type settles the question, and not
//! even one that can hold no object at all under the value arm. ADR 0150 § 6 is
//! the argument, and it also names the second reason — once `is` narrows, a
//! guard written inside an already-narrowed branch is statically true by
//! construction, and refusing that would let a flow analysis turn working code
//! into a compile error.
//!
//! The right side has **two arms** and one of them is a value
//! (`rule:types/type-test` § *The value arm*): a `class<T>` names the class to
//! test against, and the subject narrows to `T` on the true edge. The operand
//! is the only thing refused there — anything that is not a class reference is
//! [`code::E_DYNAMIC_CLASS_NAME`], the one report `new $v(...)` and `$v::f(...)`
//! already share (`rule:types/class-reference-sites`).
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
//! qualifier, which is erased before codegen and leaves no bit to read.
//!
//! A test that survives both folds and both refusals **records the type it
//! lowered** ([`crate::expr_table::ExprInfo::TypeTest`]), because that is the
//! one case with a run-time answer: narrowing reads it on the true edge, and
//! `nvs-ir` reads it to emit the test. The value arm records
//! [`crate::expr_table::ExprInfo::ClassRefTest`] and its base for the same two
//! readers. A folded one records the constant instead
//! ([`crate::expr_table::ExprInfo::SettledTypeTest`]) — which variant is on the
//! span is what tells the three apart, and the constant is carried rather than
//! re-derived because the fold is a question about types that no longer exist
//! below this crate. A refused test records none of them: it has a diagnostic,
//! so no lowering ever sees it.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{Expr, TestOperand, Type};

use crate::expr_table::ExprInfo;
use crate::locals::{Live, LocalScope};
use crate::lower::lower_type;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env};

use super::assign::is_assignable;
use super::check_expr;
use super::members::{
    can_hold_an_object, class_ref_argument, names_no_instance, reject_dynamic_class_name,
};
use super::operators::types_are_disjoint;

/// Checks `inner is against`, answering `bool` — or the literal `true`/`false`
/// the two sides settle between them.
///
/// The subject is checked for its own sake as much as for the fold: it is an
/// ordinary expression, and nothing else in this walk would visit it.
///
/// A test that reaches `bool` records what it lowered — [`ExprInfo::TypeTest`]
/// for the type arm, [`ExprInfo::ClassRefTest`] for the value one — which is
/// what [`crate::locals::narrow`] reads to narrow the subject on the true edge
/// and what `nvs-ir` reads to emit the test. A folded one records
/// [`ExprInfo::SettledTypeTest`] and its constant instead: there is no run-time
/// test left to narrow inside, and lowering answers with the constant while
/// still running the subject for its effects. A refused one records nothing at
/// all.
pub(crate) fn infer_type_test(
    expr: &Expr,
    inner: &Expr,
    against: &TestOperand,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match against {
        TestOperand::Type(ty) => infer_against_type(expr, inner, ty, live, scope, ctx, env),
        TestOperand::Value(value) => {
            infer_against_class_ref(expr, inner, value, live, scope, ctx, env)
        }
    }
}

/// `$x is T` — the type arm, and every row of `rule:types/type-test`'s table
/// but the last.
fn infer_against_type(
    expr: &Expr,
    inner: &Expr,
    ty: &Type,
    live: &mut Live,
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

/// `rule:types/conversion`'s shape row: `$x as Shape` is this module's test
/// with a throw where it answers `false`, so it is accepted from exactly the
/// operands `is` gives a run-time answer for. Answers whether the conversion
/// was accepted; `false` leaves the target to the caller's refusal.
///
/// An operand that [`always_holds`] the shape is the free row and records
/// nothing. One the shape [`never_holds`] is [`code::E_NO_CONVERSION`], the
/// refusal a class target gets for a pair sharing no value. A shape whose
/// fields carry a qualifier has no bit to test ([`qualified_atom`]), so it is
/// not accepted. Every other operand records
/// [`ExprInfo::ShapeConversion`], and `nvs-ir` lowers the field walk an `is`
/// lowers, once per conversion: O(fields), nothing allocated.
pub(crate) fn accept_shape_conversion(
    from: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> bool {
    if !matches!(env.interner.get(to), Ty::Shape(_)) || qualified_atom(to, env).is_some() {
        return false;
    }
    if always_holds(from, to, env) {
        return true;
    }
    if never_holds(from, to, env) {
        let described_from = env.interner.describe(from);
        let described_to = env.interner.describe(to);
        env.diags.report(
            Diagnostic::error(
                code::E_NO_CONVERSION,
                format!("`{described_from}` cannot be converted to `{described_to}`"),
            )
            .with_primary(span, "converted here")
            .with_help(
                "`rule:types/conversion` converts into a shape by testing the value's fields, and \
                 no value of this type is an object that could have them",
            ),
        );
        return true;
    }
    env.exprs
        .record(span, ExprInfo::ShapeConversion { shape: to });
    true
}

/// `$x is $cls` — the value arm, `rule:types/class-reference-sites`' third
/// site, and the only one of the three that consults nothing about `T`: the
/// operand carries the descriptor the test walks, so a class reference over any
/// base asks the same question and answers `bool` either way.
///
/// **The operand is the only thing refused here.** A value that is not a
/// `class<T>` is [`code::E_DYNAMIC_CLASS_NAME`] — the report `new $v(...)` and
/// `$v::f(...)` already share, whose help names the `as class<Base>` an author
/// can write — because nothing below the checker could resolve such a name
/// either.
///
/// A subject that can hold no object **folds to `false`**, the way every other
/// settled test folds, and costs no diagnostic: `is` refuses no left-hand side
/// (ADR 0150 § 6), and the run-time walk would answer `false` at every
/// execution anyway. That is the one place this arm reads differently from the
/// PHP spelling it replaces (`rule:php-migration/one-type-test`), which
/// refused the same subject outright.
///
/// The base is recorded rather than the operand's whole type because `T` is
/// what the true edge proves: a `class<T>` holds a `T` or an implementor of
/// one, so narrowing the subject to `T` is sound (`rule:types/narrowing`).
fn infer_against_class_ref(
    expr: &Expr,
    inner: &Expr,
    value: &Expr,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let subject = check_expr(inner, None, live, scope, ctx, env);
    let operand = check_expr(value, None, live, scope, ctx, env);
    let Some(base) = class_ref_argument(operand, env.interner) else {
        reject_dynamic_class_name(
            "the right-hand side of `is` is a type or a class reference, and this is neither",
            value.span,
            env,
        );
        return env.interner.bool_ty();
    };
    if !can_hold_an_object(subject, env.interner) {
        env.exprs
            .record(expr.span, ExprInfo::SettledTypeTest { answer: false });
        return env.interner.false_ty();
    }
    env.exprs.record(expr.span, ExprInfo::ClassRefTest { base });
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
///
/// The second half is the target that holds *nothing*, whatever the subject
/// is: a `Core` namespace class is a name for static members and no value is
/// ever one ([`names_no_instance`]), so `$m is Core\Str` is `false` at every
/// execution. It folds here rather than being refused, because a knowable
/// answer is not a meaningless question (`rule:types/type-test`), and it folds
/// at all because the walk `nvs-codegen` would otherwise emit has no
/// descriptor to walk against.
fn never_holds(subject: TypeId, tested: TypeId, env: &Env<'_>) -> bool {
    types_are_disjoint(subject, tested, env) || names_no_instance(tested, env.interner)
}
