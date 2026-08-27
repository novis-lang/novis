//! ADR 0053: what a `foreach` subject may be, what its bindings type as, and
//! which `yield` forms exist.
//!
//! [`ForeachSource`] is the classification the rest of the crate asks for —
//! `crate::locals` drives a `foreach`'s bindings from it, and `nvs-ir` lowers
//! from the [`ForeachDrive`](crate::expr_table::ForeachDrive) it records. Its
//! three accepted shapes are ADR 0053 § 3's, and its `Unchecked` case is the
//! deliberate one-mistake-one-diagnostic path: a subject already diagnosed as
//! something else lets the written binding types stand rather than reporting
//! twice.
//!
//! `yield`'s rules are the same ADR's § 1 and § 5: there is no key half
//! (`Iterator<T>` has exactly `advance()` and `current()`), a bare `yield`
//! with no value has no `T` to produce, and `yield from` does not exist —
//! `foreach ($inner as T $v) { yield $v; }` is what it was a second spelling
//! of. Whether a `yield` is legal here at all is `Ctx::generator_elem`, which
//! `crate::check::check_method` set from the enclosing body's own shape.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `yield` / `yield $v` / `yield $k => $v` — [`super::infer`]'s
/// `ExprKind::Yield` arm.
///
/// ADR 0053 § 4. Whether this is legal here at all, and what the operand has to
/// satisfy, are the same question — see `Ctx::generator_elem`, which
/// `crate::check::check_method` set from the enclosing body's own shape.
pub(super) fn infer_yield(
    expr: &Expr,
    key: Option<&Expr>,
    value: Option<&Expr>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if let Some(k) = key {
        check_expr(k, None, live, scope, ctx, env);
        env.diags.report(
            Diagnostic::error(
                code::E_YIELD_FORM_UNSUPPORTED,
                "a `yield` has no key half in Novis",
            )
            .with_primary(k.span, "no key exists here")
            .with_help(
                "ADR 0053 § 1 gives `Iterator<T>` exactly `advance()` and \
                 `current()`; drop the `key =>`",
            ),
        );
    }
    match (ctx.generator_elem, value) {
        (Some(elem), Some(v)) => {
            check_expr(v, Some(elem), live, scope, ctx, env);
        }
        (Some(_), None) => {
            // ADR 0007 leaves no position untyped, and a bare `yield` would
            // have to produce a `T` out of nothing.
            env.diags.report(
                Diagnostic::error(code::E_YIELD_FORM_UNSUPPORTED, "a `yield` needs a value")
                    .with_primary(expr.span, "nothing is yielded here")
                    .with_help("ADR 0053 § 1: `current()` returns a `T`, never nothing"),
            );
        }
        (None, _) => {
            if let Some(v) = value {
                check_expr(v, None, live, scope, ctx, env);
            }
            report_yield_outside_generator(expr.span, env);
        }
    }
    env.interner.void()
}

/// `yield from $inner` — [`super::infer`]'s `ExprKind::YieldFrom` arm, which
/// exists only to refuse it (ADR 0053 § 5).
pub(super) fn infer_yield_from(
    expr: &Expr,
    inner: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_expr(inner, None, live, scope, ctx, env);
    env.diags.report(
        Diagnostic::error(
            code::E_YIELD_FORM_UNSUPPORTED,
            "`yield from` does not exist in Novis",
        )
        .with_primary(expr.span, "this delegation form")
        .with_help(
            "ADR 0053 § 5: write `foreach ($inner as T $v) { yield $v; }`, which is \
             what it is a second spelling of",
        ),
    );
    env.interner.void()
}

/// What one `foreach` subject turns out to be — ADR 0053 § 3's three
/// accepted shapes, plus the two that are neither accepted nor worth a second
/// diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForeachSource {
    /// An `array<T>`, iterated directly by the IR with no interface call at
    /// all. The only shape with keys.
    Array { value: TypeId },
    /// An `Iterable<T>` or `Iterator<T>`, written as such or reached through
    /// a class that implements one. A cursor has no key: ADR 0053 § 1's
    /// member set is `advance`/`current` and nothing else.
    Cursor {
        /// The element type `current()` yields.
        value: TypeId,
        /// Whether the subject reaches `Iterable<T>` — so `foreach` calls
        /// `iterate()` once before driving — rather than being a cursor
        /// already.
        via_iterable: bool,
    },
    /// `mixed`, or a subject already diagnosed as something else — check
    /// nothing further and let the written binding types stand, so one
    /// mistake produces one diagnostic.
    Unchecked,
}

impl ForeachSource {
    fn value_ty(self) -> Option<TypeId> {
        match self {
            Self::Array { value } | Self::Cursor { value, .. } => Some(value),
            Self::Unchecked => None,
        }
    }

    /// The [`ForeachDrive`] `nvs-ir` reads back off the subject's span, or
    /// `None` for the shape that records nothing at all.
    fn drive(self) -> Option<ForeachDrive> {
        match self {
            Self::Array { .. } => Some(ForeachDrive::Array),
            Self::Cursor { via_iterable, .. } => Some(if via_iterable {
                ForeachDrive::Iterable
            } else {
                ForeachDrive::Cursor
            }),
            Self::Unchecked => None,
        }
    }
}

/// Classifies a `foreach` subject, diagnosing one that is none of ADR 0053
/// § 3's three shapes.
pub(crate) fn foreach_source(subject_ty: TypeId, span: Span, env: &mut Env<'_>) -> ForeachSource {
    let source = classify_foreach_source(subject_ty, span, env);
    if let Some(drive) = source.drive() {
        env.exprs.record_foreach(span, drive);
    }
    source
}

pub(super) fn classify_foreach_source(
    subject_ty: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> ForeachSource {
    match env.interner.get(subject_ty).clone() {
        Ty::Array(elem) => ForeachSource::Array { value: elem },
        // `mixed` is the one unchecked position (ADR 0007 § 1) and `iterable`
        // is a keyword ADR 0053 leaves untouched — neither is a mistake, and
        // neither carries an element type to check a binding against.
        Ty::Mixed | Ty::Iterable => ForeachSource::Unchecked,
        Ty::Class(qname, args) => {
            if qname.is_reserved_global_interface()
                && let Some(&value) = args.first()
            {
                return ForeachSource::Cursor {
                    value,
                    via_iterable: qname.short_name() == nvs_hir::interfaces::ITERABLE,
                };
            }
            match crate::signatures::resolve_iteration_element(&qname, env.signatures, env.graph) {
                Some((interface, value)) => ForeachSource::Cursor {
                    value: with_subject_args(&qname, &args, value, env),
                    via_iterable: interface.short_name() == nvs_hir::interfaces::ITERABLE,
                },
                None => {
                    report_not_iterable(subject_ty, span, env);
                    ForeachSource::Unchecked
                }
            }
        }
        _ => {
            report_not_iterable(subject_ty, span, env);
            ForeachSource::Unchecked
        }
    }
}

/// The element type an `implements` clause named, with the *subject's* own
/// type arguments substituted in — the receiver-driven binding
/// [`crate::generics`] describes, at the one site that is not a call.
///
/// A user class fixes its interface at a concrete type (ADR 0053 § 2), so this
/// is the identity for every subject a program declares. `Core`'s § 9
/// collections are what need it: `Core\ObjectMap<K, V>` implements
/// `Iterable<K>` for whatever `K` the subject was constructed at, so the
/// element comes back as that variable and the receiver is the only thing that
/// can say what it is. Anything the receiver leaves unbound substitutes to
/// `mixed`, which is [`crate::generics`]' standing answer and keeps the "a type
/// variable never survives" property this crate relies on.
fn with_subject_args(qname: &QName, args: &[TypeId], element: TypeId, env: &mut Env<'_>) -> TypeId {
    if args.is_empty() {
        return element;
    }
    let Some(params) = nvs_stdlib::registry::class_type_params(&qname.to_string()) else {
        return element;
    };
    let bindings: crate::generics::Bindings = params
        .iter()
        .map(|name| (*name).to_owned())
        .zip(args.iter().copied())
        .collect();
    crate::generics::substitute(element, &bindings, env.interner)
}

pub(super) fn report_not_iterable(subject_ty: TypeId, span: Span, env: &mut Env<'_>) {
    let got = env.interner.describe(subject_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_FOREACH_SUBJECT_NOT_ITERABLE,
            format!("`foreach` cannot iterate a `{got}`"),
        )
        .with_primary(span, format!("this is `{got}`"))
        .with_help(
            "ADR 0053 § 3: `foreach` accepts an `array<T>`, an `Iterable<T>` or an \
             `Iterator<T>`, and nothing else",
        ),
    );
}

/// Checks a `foreach` value binding's declared type against what the subject
/// actually yields.
pub(crate) fn check_foreach_value(
    source: &ForeachSource,
    declared: TypeId,
    binding: &ForeachBinding,
    env: &mut Env<'_>,
) {
    let Some(value) = source.value_ty() else {
        return;
    };
    if !is_assignable(value, declared, env.interner, env.graph, env.signatures) {
        report_mismatch(binding.span, declared, value, env);
    }
}

/// The two extra obligations a `foreach (… as inout $v)` value binding carries,
/// beyond the ones [`check_foreach_value`] already checked for any binding.
/// They are [`super::args::check_inout_arg`]'s two, arrived at from the same
/// direction — `inout $v` writes each element back where it came from, exactly as
/// an `inout $x` parameter writes its argument back.
///
/// 1. **The subject must be a plain variable holding an `array<T>`.** The
///    write-back re-points the subject's own slot, so there has to be one:
///    `foreach (rows() as inout $v)` has nowhere to leave what the body wrote, and
///    a cursor has no element storage at all (ADR 0053 § 1 gives
///    `Iterator<T>` `advance()` and `current()`, neither of which is a place).
///    PHP refuses both, the cursor by name.
/// 2. **The binding's type must be the element type exactly.** A by-value
///    binding may widen — reading an `array<Dog>` as an `Animal` is the same
///    element covariance any array read has — but a by-reference one writes
///    too, and writing an `Animal` into an `array<Dog>` is unsound. ADR 0007
///    § 1's "no type ever changes by itself" leaves the two directions
///    meeting only at `T` itself.
///
/// Obligation 2 is only reported when the binding would otherwise have been
/// accepted, for [`super::args::check_inout_arg`]'s reason: a type that is
/// not assignable at all already produced its own diagnostic at this span.
pub(crate) fn check_foreach_inout(
    source: &ForeachSource,
    subject: &Expr,
    declared: TypeId,
    binding: &ForeachBinding,
    env: &mut Env<'_>,
) {
    let value = match *source {
        // Already diagnosed, or a `mixed` subject that carries no element
        // type to be exact about — one mistake, one diagnostic.
        ForeachSource::Unchecked => return,
        ForeachSource::Cursor { .. } => {
            env.diags.report(
                Diagnostic::error(
                    code::E_FOREACH_INOUT_SUBJECT,
                    "an `Iterable`/`Iterator` subject has no element to bind by reference",
                )
                .with_primary(binding.span, "bound by reference here")
                .with_help(
                    "ADR 0053 § 1 gives a cursor exactly `advance()` and `current()`, so there \
                     is no storage to write back to — drop the `inout`, or iterate an `array<T>`",
                ),
            );
            return;
        }
        ForeachSource::Array { value } => value,
    };
    if !matches!(subject.kind, ExprKind::Variable(_)) {
        env.diags.report(
            Diagnostic::error(
                code::E_FOREACH_INOUT_SUBJECT,
                "only a variable can be iterated by reference",
            )
            .with_primary(subject.span, "this is not a variable")
            .with_help(
                "`inout $v` writes each element back into the subject, so the subject has to name \
                 storage that outlives the loop — bind it to a local first, or drop the `inout`",
            ),
        );
        return;
    }
    if declared != value && is_assignable(value, declared, env.interner, env.graph, env.signatures)
    {
        let (want, got) = (
            env.interner.describe(value),
            env.interner.describe(declared),
        );
        env.diags.report(
            Diagnostic::error(
                code::E_FOREACH_INOUT_ELEMENT_TY,
                format!(
                    "a by-reference binding over an `array<{want}>` needs the type `{want}` \
                     exactly, not `{got}`"
                ),
            )
            .with_primary(binding.span, format!("this binds as `{got}`"))
            .with_help(
                "`inout $v` writes back at the declared type, so widening on the way in would mean \
                 storing that wider type into the array",
            ),
        );
    }
}

/// Checks a `foreach` key binding — which today means refusing one over a
/// cursor and nothing else.
///
/// # Why an array's key binding is unchecked
///
/// `array<T>` records the *value* type and no key type at all (ADR 0007 § 5
/// fixes the two legal key types, `int` and `string`, but not which one a
/// given array holds). So a declared `string $k` is neither provable nor
/// refutable here: the only sound statement about it is `int|string`, and
/// requiring every author to write that union — over a map whose keys are all
/// strings by construction — would be noise, not safety. Narrowing it
/// properly needs `array<K, V>`, which is its own decision. A cursor is the
/// opposite case: ADR 0053 § 1 gives `Iterator<T>` exactly `advance()` and
/// `current()`, so there is provably no key, and that *is* refused.
pub(crate) fn check_foreach_key(
    source: &ForeachSource,
    _declared: TypeId,
    binding: &ForeachBinding,
    env: &mut Env<'_>,
) {
    if matches!(source, ForeachSource::Cursor { .. }) {
        env.diags.report(
            Diagnostic::error(
                code::E_FOREACH_KEY_ON_CURSOR,
                "an `Iterable`/`Iterator` subject has no key to bind",
            )
            .with_primary(binding.span, "no key exists here")
            .with_help(
                "ADR 0053 § 1 gives `Iterator<T>` exactly `advance()` and `current()`; \
                 drop the `$k =>` or iterate an `array<T>` instead",
            ),
        );
    }
}

/// ADR 0027 § 1/§ 3: a bare string or `[$obj, 'method']`-shaped array
/// literal reaching a `callable`-typed position gets a targeted diagnostic
/// naming the first-class-callable-syntax replacement, rather than the
/// generic `E_TYPE_MISMATCH` [`is_assignable`] would otherwise report for
/// the same expression. Returns whether it reported one, so the caller can
/// skip its own generic check for this expression.
/// ADR 0053 § 4's lexical confinement, reported once per stray `yield`.
pub(super) fn report_yield_outside_generator(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_YIELD_OUTSIDE_GENERATOR,
            "`yield` is only allowed in a generator's own body",
        )
        .with_primary(span, "this is not inside a generator")
        .with_help(
            "ADR 0053 § 4 lowers a generator to a state machine rather than to a coroutine, \
             which is what confines `yield` to the body it is written in — a closure, or a \
             helper it calls, cannot yield into it",
        ),
    );
}
