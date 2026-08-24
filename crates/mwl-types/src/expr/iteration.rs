//! ADR 0053: what a `foreach` subject may be, what its bindings type as, and
//! which `yield` forms exist.
//!
//! [`ForeachSource`] is the classification the rest of the crate asks for —
//! `crate::locals` drives a `foreach`'s bindings from it, and `mwl-ir` lowers
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

    /// The [`ForeachDrive`] `mwl-ir` reads back off the subject's span, or
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
                    via_iterable: qname.short_name() == mwl_hir::interfaces::ITERABLE,
                };
            }
            match crate::signatures::resolve_iteration_element(&qname, env.signatures, env.graph) {
                Some((interface, value)) => ForeachSource::Cursor {
                    value,
                    via_iterable: interface.short_name() == mwl_hir::interfaces::ITERABLE,
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
