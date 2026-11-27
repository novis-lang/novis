//! `rule:types/type-test`'s `$x is T`: what it answers, what it refuses, and
//! why the list of refusals is as short as it is.
//!
//! The operator is **total**. Every value has a representation, so every
//! subject has an answer, and this module refuses nothing about the left-hand
//! side: not a subject whose declared type settles the question, and not one
//! that can hold no object at all under the value arm. `rule:types/type-test`
//! is the argument, and it names the second reason too. Once `is` narrows, a
//! guard written inside an already-narrowed branch is statically true by
//! construction, and refusing that would let a flow analysis turn working code
//! into a compile error.
//!
//! The right side has **two arms** and one of them is a value
//! (`rule:types/type-test` § *The value arm*): a `class<T>` names the class to
//! test against, and the subject narrows to `T` on the true edge. The operand
//! is the only thing refused there. Anything that is not a class reference is
//! [`code::E_DYNAMIC_CLASS_NAME`], the one report `new $v(...)` and `$v::f(...)`
//! already share (`rule:types/class-reference-sites`).
//!
//! **One written type answers by the frame: `static`.** In a class that is not
//! `final`, `$x is static` and `$x is ?static` test the class the call named
//! (`rule:statements/static-is-a-member-modifier`), which only the running
//! frame knows. So [`always_holds`] never folds them: a subject of the
//! declaring class may still be a sibling of the called class. [`never_holds`]
//! still folds, because a subject disjoint from the declaring class is disjoint
//! from every subclass. In a `final` class `static` is the declaring class, and
//! the test is an ordinary one. [`called_class_test`] reads the written type,
//! since the lowered one cannot tell `static` from the declaring class.
//!
//! A settled answer **folds** instead: the expression's type is `true` or
//! `false` rather than `bool`, with no diagnostic and no warning
//! (`rule:types/single-value-types` is what makes those types sayable). Both
//! folds are deliberately one-sided, in [`always_holds`]'s and
//! [`never_holds`]'s directions: what neither proves types as plain `bool` and
//! is answered at run time, which is never wrong, only at worst a run-time test
//! where a constant would have done.
//!
//! The refusals here are about the **right**-hand side. Two are cases where
//! there is no question rather than a knowable answer:
//! [`code::E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE`] for `void` and `never`,
//! which no value inhabits, and [`code::E_TYPE_TEST_AGAINST_A_QUALIFIER`] for
//! a `tainted` or `secret` qualifier, which is erased before codegen and leaves
//! no bit to read. Two are about `static`, where no answer can be read:
//! [`code::E_STATIC_IN_ANON_FN`] inside an anonymous function, whose frame
//! holds no called class, and [`code::E_STATIC_INSIDE_A_TESTED_TYPE`] for a
//! `static` deeper than the whole type, as in `array<static>`.
//!
//! **Over a `mixed` subject, `tainted` is the answer rather than the
//! question** (`rule:security/taint-propagation`): text out of `mixed` is
//! tainted, so `$m is string` narrows `$m` to `tainted string`, a written
//! `$m is tainted {…}` is admitted and tests the shape with the qualifier
//! erased, and a shape with a text field written without it is
//! [`code::E_UNCHECKED_TEXT_NOT_TAINTED`]. The same holds for a plain
//! `object` subject wherever a shape is tested. `secret` is refused over any
//! subject.
//!
//! A test with a run-time answer **records what `nvs-ir` and narrowing read**:
//! [`crate::expr_table::ExprInfo::TypeTest`] and the type it lowered for the
//! type arm, [`crate::expr_table::ExprInfo::ClassRefTest`] and its base for the
//! value arm, and [`crate::expr_table::ExprInfo::CalledClassTest`] for
//! `is static`. Narrowing reads the type on the true edge, and `nvs-ir` reads
//! the record to emit the test. A folded one records the constant instead
//! ([`crate::expr_table::ExprInfo::SettledTypeTest`]). Which variant is on the
//! span tells the four apart, and the constant is carried rather than
//! re-derived because the fold is a question about types that no longer exist
//! below this crate. A refused test records none of them: it has a diagnostic,
//! so no lowering ever sees it.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{Expr, TestOperand, Type, TypeAtom, TypeKind};

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
use super::quals::{
    ErasingPosition, carries_secret, carries_tainted, carries_unchecked, erased_qualifiers,
    fields_unchecked, has_fields, reject_untainted_text_from_unchecked, secret_result,
    tainted_result,
};

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
    if reject_unanswerable_target(tested, subject, expr.span, env)
        || reject_untainted_text_from_unchecked(subject, tested, expr.span, env)
    {
        return env.interner.bool_ty();
    }
    let called = match called_class_test(ty, ctx, env) {
        CalledClass::No => None,
        CalledClass::Refused => return env.interner.bool_ty(),
        CalledClass::Test { nullable } => Some(nullable),
    };
    let tested = keep_subject_qualifiers(subject, tested, ErasingPosition::TypeTest, env.interner);
    if let Some(nullable) = called
        && !never_holds(subject, tested, env)
    {
        env.exprs
            .record(expr.span, ExprInfo::CalledClassTest { tested, nullable });
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

/// What [`called_class_test`] found in a tested type.
enum CalledClass {
    /// No `static` that names the called class: the test is an ordinary one.
    No,
    /// `static` or `?static`, tested against the called class at run time.
    Test { nullable: bool },
    /// A `static` this test cannot answer, already reported.
    Refused,
}

/// `rule:statements/static-is-a-member-modifier`'s `$x is static`: whether
/// the tested type names the called class.
///
/// In a class that is not `final`, `static` and `?static` are tested against
/// the frame's called class, so a call on a subclass answers for that
/// subclass. [`crate::lower::lower_type`] interns `static` as the declaring
/// class, which is what the true edge proves. In a `final` class `static` is
/// exactly the declaring class, so the ordinary test and its folds stay.
///
/// Two spellings are refused. Inside an anonymous function no frame holds the
/// called class, so any `static` there is [`code::E_STATIC_IN_ANON_FN`], as
/// `static::` is. A `static` deeper in the tested type, as in
/// `array<static>` or `static|int`, is
/// [`code::E_STATIC_INSIDE_A_TESTED_TYPE`]: the lowered type cannot tell it
/// from the declaring class, so the test would answer for the wrong class.
fn called_class_test(ty: &Type, ctx: &Ctx<'_>, env: &mut Env<'_>) -> CalledClass {
    let Some(class) = ctx.current_class else {
        return CalledClass::No;
    };
    let Some(at) = static_written_in(ty) else {
        return CalledClass::No;
    };
    if ctx.in_anon_fn {
        env.diags.report(
            Diagnostic::error(
                code::E_STATIC_IN_ANON_FN,
                "`is static` names no class inside an anonymous function",
            )
            .with_primary(at, "inside an anonymous function")
            .with_help(
                "an anonymous function does not know which class the method was called on. \
                 Test the value before the anonymous function, or test it against a class \
                 that you name",
            ),
        );
        return CalledClass::Refused;
    }
    if crate::signatures::class_is_final(class, env.signatures) {
        return CalledClass::No;
    }
    if let Some(nullable) = top_level_static(ty) {
        return CalledClass::Test { nullable };
    }
    env.diags.report(
        Diagnostic::error(
            code::E_STATIC_INSIDE_A_TESTED_TYPE,
            "`static` can only be the whole tested type, as in `is static` or `is ?static`",
        )
        .with_primary(at, "`static` inside a tested type")
        .with_help(
            "test the value with `is static` alone. To test the elements of an array, test \
             each element in a loop",
        ),
    );
    CalledClass::Refused
}

/// `Some(nullable)` when `ty` is `static`, `?static`, `static|null` or
/// `null|static`, parentheses aside.
fn top_level_static(ty: &Type) -> Option<bool> {
    match &ty.kind {
        TypeKind::Paren(inner) => top_level_static(inner),
        TypeKind::Atom(TypeAtom::StaticTy) => Some(false),
        TypeKind::Nullable(inner) => top_level_static(inner).map(|_| true),
        TypeKind::Union(items) if items.len() == 2 => {
            let is_null = |t: &Type| matches!(t.kind, TypeKind::Atom(TypeAtom::Null));
            match (&items[0], &items[1]) {
                (a, b) if is_null(a) => top_level_static(b).map(|_| true),
                (a, b) if is_null(b) => top_level_static(a).map(|_| true),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The span of the first `static` written anywhere in `ty`.
fn static_written_in(ty: &Type) -> Option<Span> {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => static_written_in(inner),
        TypeKind::Union(items) | TypeKind::Intersection(items) => {
            items.iter().find_map(static_written_in)
        }
        TypeKind::Atom(atom) => match atom {
            TypeAtom::StaticTy => Some(ty.span),
            TypeAtom::Array(Some(inner), key) => key
                .as_deref()
                .and_then(static_written_in)
                .or_else(|| static_written_in(inner)),
            TypeAtom::ClassRef(inner)
            | TypeAtom::PropertyKey(inner)
            | TypeAtom::PropertyPath(inner)
            | TypeAtom::StringHoles { key: inner, .. } => static_written_in(inner),
            TypeAtom::CallableSig { params, ret } => params
                .iter()
                .find_map(static_written_in)
                .or_else(|| static_written_in(ret)),
            TypeAtom::Name(_, args) => args.iter().find_map(static_written_in),
            TypeAtom::Shape(fields) => fields.iter().find_map(|f| static_written_in(&f.ty)),
            _ => None,
        },
        _ => None,
    }
}

/// `rule:types/conversion`'s shape row: `$x as Shape` is this module's test
/// with a throw where it answers `false`, so it is accepted from exactly the
/// operands `is` gives a run-time answer for. Answers whether the conversion
/// was accepted; `false` leaves the target to the caller's refusal.
///
/// An operand that [`always_holds`] the shape is the free row and records
/// nothing. One the shape [`never_holds`] is [`code::E_NO_CONVERSION`], the
/// refusal a class target gets for a pair sharing no value. A shape with a
/// `secret` field has no bit to test ([`qualified_atom`]), so it is not
/// accepted; a `tainted` field is tested as its plain text, since adding the
/// qualifier can never leak. Text out of `mixed` landing in a field written
/// without `tainted` is [`code::E_UNCHECKED_TEXT_NOT_TAINTED`]. Every other
/// operand records
/// [`ExprInfo::ShapeConversion`], and `nvs-ir` lowers the field walk an `is`
/// lowers, once per conversion: O(fields), nothing allocated.
pub(crate) fn accept_shape_conversion(
    from: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> bool {
    if !matches!(env.interner.get(to), Ty::Shape(_)) || qualified_atom(to, true, env).is_some() {
        return false;
    }
    if reject_untainted_text_from_unchecked(from, to, span, env)
        || reject_secret_property_in_shape(from, to, span, ErasingPosition::ErasedRead, env)
    {
        return true;
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
                "a conversion to a shape checks the fields of an object. A value of this type \
                 is never an object",
            ),
        );
        return true;
    }
    env.exprs
        .record(span, ExprInfo::ShapeConversion { shape: to });
    true
}

/// `rule:security/secret-qualifier`: a shape conversion over a known class
/// does not compile when a field of the shape names a `secret` property. A
/// shape cannot have a `secret` field, so the converted value would hold the
/// property as plain text. Reports [`code::E_SECRET_CONVERSION_LOSES_SECRET`]
/// at the first such field and answers whether it reported.
///
/// `from` is a class or a union with class members. A property the class does
/// not declare is not secret here, since nothing marks it so.
fn reject_secret_property_in_shape(
    from: TypeId,
    to: TypeId,
    span: Span,
    position: ErasingPosition,
    env: &mut Env<'_>,
) -> bool {
    debug_assert_eq!(position, ErasingPosition::ErasedRead);
    let Ty::Shape(fields) = env.interner.get(to).clone() else {
        return false;
    };
    let classes: Vec<(nvs_hir::QName, Vec<TypeId>)> = env
        .interner
        .union_members(from)
        .into_iter()
        .filter_map(|member| match env.interner.get(member) {
            Ty::Class(qname, args) => Some((qname.clone(), args.clone())),
            _ => None,
        })
        .collect();
    for (qname, args) in classes {
        for field in &fields {
            let Some(declared) =
                crate::signatures::resolve_property(&qname, &field.name, env.signatures, env.graph)
            else {
                continue;
            };
            let declared = crate::generics::with_class_args(&qname, &args, declared, env.interner);
            if !super::quals::carries_secret(declared, env.interner) {
                continue;
            }
            let name = &field.name;
            env.diags.report(
                Diagnostic::error(
                    code::E_SECRET_CONVERSION_LOSES_SECRET,
                    format!(
                        "the property `{qname}::${name}` is `secret`, so a shape cannot read it"
                    ),
                )
                .with_primary(span, "converted here")
                .with_help(format!(
                    "a shape cannot have a `secret` field. Remove `{name}` from the shape, and \
                     read the property from the object as `$value->{name}`"
                )),
            );
            return true;
        }
    }
    false
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
/// (`rule:types/type-test`), and the run-time walk would answer `false` at every
/// execution anyway. That is the one place this arm reads differently from the
/// PHP spelling it replaces (`rule:types/one-type-test`), which
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
///
/// A `tainted` atom over a `subject` whose text is unchecked is not one of
/// them: the qualifier there is the answer `rule:security/taint-propagation`
/// gives anyway, so it is tested as its plain text. The same holds for a
/// `tainted {…}` shape over a subject with fields, whose class may fill a text
/// field from a `mixed` or `tainted` one.
fn reject_unanswerable_target(
    tested: TypeId,
    subject: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> bool {
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
    // A class reference carries the class it names, not the `T` it was
    // declared with, so `is class<T>` has no run-time row. `as ?class<T>` is
    // the conversion that walks the named class instead.
    let class_ref = env
        .interner
        .union_members(tested)
        .into_iter()
        .find_map(|m| match env.interner.get(m) {
            Ty::ClassRef(inner) => Some(*inner),
            _ => None,
        });
    if let Some(inner) = class_ref {
        let base = env.interner.describe(inner);
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_TEST_AGAINST_A_CONVERSION_TYPE,
                format!("`is` cannot test for `class<{base}>`"),
            )
            .with_primary(span, "tested here")
            .with_help(format!(
                "write `$c as ?class<{base}>`. The result is `null` when the class is not \
                 `{base}` or a class that extends it"
            )),
        );
        return true;
    }
    // A key, a path and a string with holes are a plain string at run time,
    // so `is` has no row for them either. `as ?` checks the text instead.
    let checked_text = env.interner.union_members(tested).into_iter().find(|&m| {
        matches!(
            env.interner.get(m),
            Ty::PropertyKey(_) | Ty::PropertyPath(_) | Ty::StringHoles { .. }
        )
    });
    if let Some(member) = checked_text {
        let described = env.interner.describe(member);
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_TEST_AGAINST_A_CONVERSION_TYPE,
                format!("`is` cannot test for `{described}`"),
            )
            .with_primary(span, "tested here")
            .with_help(format!(
                "a `{described}` is a string while the program runs. Test for the other type \
                 of the union instead, such as `$x is SomeClass`, or check a string with \
                 `$s as ?{described}`, which gives `null` when the string is wrong"
            )),
        );
        return true;
    }
    let secret_only = fields_unchecked(subject, env.interner)
        || (has_fields(subject, env.interner) && matches!(env.interner.get(tested), Ty::Shape(_)));
    if let Some(qualified) = qualified_atom(tested, secret_only, env) {
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
/// Not just the top level: `tainted { a: string }` is rewritten while parsing
/// into the shape whose text fields are their tainted forms
/// (`rule:security/tainted-qualifier`), so a qualifier written over a shape
/// arrives here inside one, and `array<tainted string>` puts it inside an
/// element. Every one of those is the same missing bit. With `secret_only`
/// set, a `tainted` atom is passed over: that is the question asked where
/// adding `tainted` can only over-qualify the answer.
fn qualified_atom(tested: TypeId, secret_only: bool, env: &Env<'_>) -> Option<TypeId> {
    match env.interner.get(tested) {
        Ty::TaintedString | Ty::TaintedBytes if !secret_only => Some(tested),
        Ty::SecretString | Ty::SecretBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes => {
            Some(tested)
        }
        Ty::Array(element, _) => qualified_atom(*element, secret_only, env),
        Ty::Union(members) | Ty::Intersection(members) => members
            .iter()
            .find_map(|m| qualified_atom(*m, secret_only, env)),
        Ty::Shape(fields) => fields
            .iter()
            .find_map(|f| qualified_atom(f.ty, secret_only, env)),
        _ => None,
    }
}

/// Whether every value of `subject` holds a `tested` — the `true` fold.
///
/// Assignability answers this for every row of `rule:types/type-test`'s table
/// bar one, since a type that is a subtype of the target *is* the target at run
/// time: a `Dog` holds an `Animal`, a `"a"` holds a `string`, and a shape with
/// more fields holds the shape with fewer.
///
/// The question is about run time, where no qualifier exists, so both sides are
/// compared with `tainted` and `secret` erased ([`erased_qualifiers`]). A
/// `tainted string` and a `secret string` each hold a `string`. Assignability
/// alone would answer `false` there, since it never drops a qualifier. The fold
/// keeps the subject's type, so the qualifier stays on the variable.
///
/// The exception is the one implicit conversion the language has
/// (`rule:types/conversion`): an `int` *occupies* a `float` position by
/// converting, and `is` asks for the tag it does not carry. Unions are
/// decomposed here rather than left to [`is_assignable`] so that the exception
/// is reached at every atom, not only at the top.
pub(crate) fn always_holds(subject: TypeId, tested: TypeId, env: &mut Env<'_>) -> bool {
    if matches!(
        env.interner.get(tested),
        Ty::Mixed | Ty::SecretMixed | Ty::Error
    ) {
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
    let subject = erased_qualifiers(subject, env.interner);
    let tested = erased_qualifiers(tested, env.interner);
    subject == tested || is_assignable(subject, tested, env.interner, env.graph, env.signatures)
}

/// `tested` with the qualifiers of `subject` put back on it, so that narrowing
/// to it never drops one (`rule:types/type-test`, `rule:types/narrowing`).
///
/// `tainted` is set wherever [`tainted_result`] can set it when the subject
/// carries `tainted` or text out of `mixed`, and `secret` wherever
/// [`secret_result`] can set it when the subject carries `secret`. So
/// `?tainted string is string` narrows to `tainted string`, and
/// `array<secret string> is array<string>` to `array<secret string>`. A tested
/// type with no text in it, such as `int`, is returned unchanged.
pub(crate) fn keep_subject_qualifiers(
    subject: TypeId,
    tested: TypeId,
    position: ErasingPosition,
    interner: &mut crate::ty::TypeInterner,
) -> TypeId {
    debug_assert_eq!(position, ErasingPosition::TypeTest);
    let tested = if carries_unchecked(subject, interner) || carries_tainted(subject, interner) {
        tainted_result(tested, interner)
    } else {
        tested
    };
    if carries_secret(subject, interner) {
        secret_result(tested, interner)
    } else {
        tested
    }
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
