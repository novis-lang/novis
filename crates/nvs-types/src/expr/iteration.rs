//! ADR 0053: what a `foreach` subject may be, what its bindings type as, and
//! which `yield` forms exist.
//!
//! [`ForeachSource`] is the classification the rest of the crate asks for —
//! `crate::locals` drives a `foreach`'s bindings from it, and `nvs-ir` lowers
//! from the [`ForeachDrive`](crate::expr_table::ForeachDrive) it records. Its
//! three accepted shapes are `rule:iteration/foreach-subjects`'s, and its `Unchecked` case is the
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
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `yield` / `yield $v` / `yield $k => $v` — [`super::infer`]'s
/// `ExprKind::Yield` arm.
///
/// `rule:iteration/generators`. Whether this is legal here at all, and what the operand has to
/// satisfy, are the same question — see `Ctx::generator_elem`, which
/// `crate::check::check_method` set from the enclosing body's own shape.
pub(crate) fn infer_yield(
    expr: &Expr,
    key: Option<&Expr>,
    value: Option<&Expr>,
    live: &mut Live,
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
                "`rule:iteration/two-interfaces` gives `Iterator<T>` exactly `advance()` and \
                 `current()`; drop the `key =>`",
            ),
        );
    }
    match (ctx.generator_elem, value) {
        (Some(elem), Some(v)) => {
            check_expr(v, Some(elem), live, scope, ctx, env);
        }
        (Some(_), None) => {
            // `rule:types/declaration` leaves no position untyped, and a bare `yield` would
            // have to produce a `T` out of nothing.
            env.diags.report(
                Diagnostic::error(code::E_YIELD_FORM_UNSUPPORTED, "a `yield` needs a value")
                    .with_primary(expr.span, "nothing is yielded here")
                    .with_help(
                        "`rule:iteration/two-interfaces`: `current()` returns a `T`, never nothing",
                    ),
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
/// exists only to refuse it (`rule:iteration/one-way-only`).
pub(crate) fn infer_yield_from(
    expr: &Expr,
    inner: &Expr,
    live: &mut Live,
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
            "`rule:iteration/one-way-only`: write `foreach ($inner as T $v) { yield $v; }`, which is \
             what it is a second spelling of",
        ),
    );
    env.interner.void()
}

/// What one `foreach` subject turns out to be — `rule:iteration/foreach-subjects`'s three
/// accepted shapes, plus the two that are neither accepted nor worth a second
/// diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForeachSource {
    /// An `array<T>`, iterated directly by the IR with no interface call at
    /// all. The only shape with keys.
    Array { value: TypeId },
    /// An `Iterable<T>` or `Iterator<T>`, written as such or reached through
    /// a class that implements one. A cursor has no key: `rule:iteration/two-interfaces`'s
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
    /// The element type the subject gives its value binding, or `None` for a
    /// subject that carries none.
    pub(crate) fn value_ty(self) -> Option<TypeId> {
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

/// Classifies a `foreach` subject, diagnosing one that is none of `rule:iteration/foreach-subjects`'s three shapes.
pub(crate) fn foreach_source(subject_ty: TypeId, span: Span, env: &mut Env<'_>) -> ForeachSource {
    let source = classify_foreach_source(subject_ty, span, env);
    if let Some(drive) = source.drive() {
        env.exprs.record_foreach(span, drive);
    }
    source
}

pub(crate) fn classify_foreach_source(
    subject_ty: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> ForeachSource {
    match env.interner.get(subject_ty).clone() {
        Ty::Array(elem) => ForeachSource::Array { value: elem },
        // `mixed` is the one unchecked position (`rule:types/declaration`) and `iterable`
        // is a keyword `rule:iteration/two-interfaces` leaves untouched — neither is a mistake, and
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
/// [`crate::generics::with_class_args`] owns, at the one site that is not a
/// call. That function's own docs are why it exists and what it answers for a
/// class a program declared.
fn with_subject_args(qname: &QName, args: &[TypeId], element: TypeId, env: &mut Env<'_>) -> TypeId {
    crate::generics::with_class_args(qname, args, element, env.interner)
}

pub(crate) fn report_not_iterable(subject_ty: TypeId, span: Span, env: &mut Env<'_>) {
    let got = env.interner.describe(subject_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_FOREACH_SUBJECT_NOT_ITERABLE,
            format!("`foreach` cannot iterate a `{got}`"),
        )
        .with_primary(span, format!("this is `{got}`"))
        .with_help(
            "`rule:iteration/foreach-subjects`: `foreach` accepts an `array<T>`, an `Iterable<T>` or an \
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
    if is_assignable(value, declared, env.interner, env.graph, env.signatures) {
        note_float_widening_at(binding.span, value, declared, env);
    } else {
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
///    a cursor has no element storage at all (`rule:iteration/two-interfaces` gives
///    `Iterator<T>` `advance()` and `current()`, neither of which is a place).
///    PHP refuses both, the cursor by name.
/// 2. **The binding's type must be the element type exactly.** A by-value
///    binding may widen — reading an `array<Dog>` as an `Animal` is the same
///    element covariance any array read has — but a by-reference one writes
///    too, and writing an `Animal` into an `array<Dog>` is unsound. `rule:types/declaration`'s "no type ever changes by itself" leaves the two directions
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
                    "`rule:iteration/two-interfaces` gives a cursor exactly `advance()` and `current()`, so there \
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

/// The type a `foreach` over `subject_ty` gives its key binding: `tainted
/// string` where [`super::quals::keys_from_outside`] says the keys can come
/// from outside the program, and `string` everywhere else. A `var` key takes
/// it, and a written key is checked against it by [`check_foreach_key`].
pub(crate) fn foreach_key_ty(subject_ty: TypeId, env: &mut Env<'_>) -> TypeId {
    if super::quals::keys_from_outside(subject_ty, env.interner) {
        env.interner.tainted_string()
    } else {
        env.interner.string()
    }
}

/// Checks a `foreach` key binding: a cursor has no key at all, an
/// `array<T>`'s key is a `string` and nothing else, and a key that can come
/// from outside the program is written `tainted string`.
///
/// # Why an array's key binding is exact
///
/// `rule:types/arrays` gives the container **one** stored key type — "every key is a
/// `string`. There is no integer key" — so the type parameter is the value's
/// and the key needs none. That makes `foreach ($a as int $k => …)` neither a
/// narrowing nor a widening but simply wrong: no array can produce an `int`
/// key for the binding to hold. The same goes for a `mixed $k`, which would
/// be a widening if there were a second key type to widen over and is instead
/// a binding at a representation the loop never produces — `rule:types/arrays`
/// normalises `$a[8]` to `$a["8"]` at the *subscript*, and there is no
/// conversion on the way back out. `tainted string` is the same
/// representation, so it is admitted over any array.
///
/// # Why an outside key is `tainted`
///
/// `given` is [`foreach_key_ty`]'s answer. Where it is `tainted string`, a key
/// written as a plain `string` is [`code::E_UNCHECKED_TEXT_NOT_TAINTED`], the
/// refusal a value binding over `mixed` gets: the binding is a type the
/// program wrote, so the qualifier is not added behind it
/// (`rule:security/taint-propagation`). A `mixed` or `iterable` subject is
/// asked the same question, though it checks nothing else about the key.
///
/// A cursor is the sharper case: `rule:iteration/two-interfaces` gives `Iterator<T>` exactly
/// `advance()` and `current()`, so there is provably no key at all, and it
/// keeps its own code.
///
/// A binding that declared no type is left alone — `nvs_syntax`'s parser
/// already reported the omission, and the `mixed` its absence lowers to is an
/// error-recovery placeholder rather than something the author wrote (see
/// [`nvs_syntax::ast::ForeachBindingTy::Omitted`]). A `var` key is `given` by
/// construction, so it passes.
pub(crate) fn check_foreach_key(
    source: &ForeachSource,
    given: TypeId,
    declared: TypeId,
    binding: &ForeachBinding,
    env: &mut Env<'_>,
) {
    let string = env.interner.string();
    if declared == string && given != string {
        report_untainted_outside_key(declared, binding.span, env);
        return;
    }
    match *source {
        ForeachSource::Cursor { .. } => {
            env.diags.report(
                Diagnostic::error(
                    code::E_FOREACH_KEY_ON_CURSOR,
                    "an `Iterable`/`Iterator` subject has no key to bind",
                )
                .with_primary(binding.span, "no key exists here")
                .with_help(
                    "`rule:iteration/two-interfaces` gives `Iterator<T>` exactly `advance()` and `current()`; \
                     drop the `$k =>` or iterate an `array<T>` instead",
                ),
            );
        }
        ForeachSource::Array { .. } => {
            if matches!(binding.ty, ForeachBindingTy::Omitted) {
                return;
            }
            let tainted = env.interner.tainted_string();
            if declared != string && declared != tainted {
                let got = env.interner.describe(declared);
                let want = env.interner.describe(given);
                env.diags.report(
                    Diagnostic::error(
                        code::E_FOREACH_KEY_TY,
                        format!("an array's key binding is a `{want}`, not `{got}`"),
                    )
                    .with_primary(binding.span, format!("this binds as `{got}`"))
                    .with_help(format!(
                        "`rule:types/arrays` gives an `array<T>` one stored key type — every key is a \
                         `string`, and `$a[8]` is normalised to `$a[\"8\"]` at the subscript \
                         rather than converted — so write `{want} $k`, and convert inside the \
                         body if the loop wants another type"
                    )),
                );
            }
        }
        // `mixed`, or a subject already diagnosed as something else — one
        // mistake, one diagnostic.
        ForeachSource::Unchecked => {}
    }
}

/// [`check_foreach_key`]'s refusal of a plain `string` key whose keys can come
/// from outside the program.
fn report_untainted_outside_key(declared: TypeId, span: Span, env: &mut Env<'_>) {
    let written = env.interner.describe(declared);
    env.diags.report(
        Diagnostic::error(
            code::E_UNCHECKED_TEXT_NOT_TAINTED,
            format!(
                "this key can come from outside the program, so it must be `tainted {written}`"
            ),
        )
        .with_primary(span, format!("this is `{written}`"))
        .with_help(
            "The keys of a `mixed` value can come from outside the program. So can the keys of an \
             array whose values are tainted or `mixed`. Write `tainted string $key`. Then check or \
             escape the key before you use it in a query, a page or a command.",
        ),
    );
}

/// `rule:types/callable-values`/§ 3: a bare string or `[$obj, 'method']`-shaped array
/// literal reaching a `callable`-typed position gets a targeted diagnostic
/// naming the method-reference replacement, rather than the
/// generic `E_TYPE_MISMATCH` [`is_assignable`] would otherwise report for
/// the same expression. Returns whether it reported one, so the caller can
/// skip its own generic check for this expression.
/// `rule:iteration/generators`'s lexical confinement, reported once per stray `yield`.
pub(crate) fn report_yield_outside_generator(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_YIELD_OUTSIDE_GENERATOR,
            "`yield` is only allowed in a generator's own body",
        )
        .with_primary(span, "this is not inside a generator")
        .with_help(
            "`yield` works only in the body of the generator itself. An anonymous function \
             inside the generator, or a function the generator calls, cannot yield for it",
        ),
    );
}
