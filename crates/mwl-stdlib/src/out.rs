//! `Core\Out` — docs/spec/01-core-library.md § 12's one member, and the whole
//! of PHP's `ob_*` family in it.
//!
//! `capture(callable $fn, {through?: callable}): Core\Cli\Text` runs `$fn` with
//! this request's sink redirected into a buffer, and answers what it wrote.
//! Three properties of that sentence are the design, and each is
//! [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 5's or the spec's rather than this file's:
//!
//! * **Scoped to a closure, so it nests by call nesting.** `ob_start` and
//!   `ob_get_clean` are two members of a *global* stack that can be started in
//!   one function and ended in another, which is why inspecting it takes
//!   `ob_get_level` and a loop. Here the two ends are the two ends of one call,
//!   so the stack is an implementation detail — it lives on
//!   `mwl_runtime::Ctx` ([`Ctx::begin_capture`](mwl_runtime::Ctx::begin_capture))
//!   and no program can observe an unbalanced one.
//! * **It always swallows.** Nothing `$fn` echoes reaches the sink below.
//!   `ob_start($callback)`'s invisible pass-through has no equivalent: re-
//!   emitting is a visible `echo Core\Out::capture(…)`, and `{through:}`
//!   transforms the captured value rather than deciding whether it escapes.
//! * **It answers the carrier, not a `string`.** ADR 0088 § 5: those bytes have
//!   already been through the sink, so handing them back as text would let the
//!   next `echo` escape them twice. [`crate::cli`] is the carrier and owns what
//!   one is.
//!
//! # What it spends
//!
//! One buffer per open capture, holding what that level has captured so far,
//! plus the one `Core\Cli\Text` instance the member answers with — both charged
//! to the request and both freed with it. A request that never captures pays
//! one not-taken branch per `echo`, which `mwl_runtime::Ctx`'s own
//! `captures` field states.
//!
//! # Known gap
//!
//! 1. **`{through:}` has nothing useful to do yet.** Its closure takes and
//!    answers a `Core\Cli\Text`, and that class carries no member until M8
//!    builds the rest of `Core\Cli` — so today a `through` can only be the
//!    identity, or drop what it was given and fail to answer a carrier. The
//!    plumbing is complete and tested; what is missing is on the other side of
//!    [`crate::cli`]'s own gap 2.

use mwl_runtime::{Fault, MwlStr, Tag, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

/// The class's fully-qualified name.
pub(crate) const NAME: &str = r"Core\Out";

/// `capture`'s `{through?: callable}` — a transform applied to the captured
/// carrier before it is answered.
///
/// [`Const::Null`] rather than a do-nothing closure, for the reason that
/// variant's own docs give: a `callable` has no "absent" value, and inventing
/// an identity one would allocate a closure per omitting call site to express
/// "nothing to do".
const CAPTURE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "through",
    ty: CoreTy::Callable,
    default: Const::Null,
}];

/// Spec § 12's `Core\Out`, which has exactly this one member.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "capture",
        params: &[CoreTy::Callable, CoreTy::Options(CAPTURE_OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Instance(crate::cli::NAME),
        symbol: "mwl_core_out_capture",
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_out_capture" => (mwl_core_out_capture as *const ()).cast(),
        _ => return None,
    })
}

mwl_runtime::mwl_helper! {
    /// `Core\Out::capture(callable $fn, {through?: callable}): Core\Cli\Text`
    /// — see the module docs.
    ///
    /// The capture level is opened before `$fn` runs and closed on **both**
    /// edges, the throwing one included: a `Fault` propagating out of a
    /// capture that stayed open would silently swallow the rest of the
    /// request's output, which is the worst failure this member could have.
    /// That is why the closure's result is bound rather than `?`-ed.
    fn mwl_core_out_capture(ctx, args: [2]) {
        ctx.begin_capture();
        let outcome = mwl_runtime::call_closure(ctx, args[0], &[]);
        let captured = ctx.end_capture().unwrap_or_default();
        // The body's own return value is discarded — `capture` answers what was
        // written, not what was computed — so its reference ends here.
        match outcome {
            Ok(result) => {
                #[expect(
                    unsafe_code,
                    reason = "`call_closure` hands back a value the caller owns, \
                              and this one is never handed on"
                )]
                unsafe {
                    result.release();
                }
            }
            Err(fault) => return Err(fault),
        }
        let text = crate::cli::built(Value::str(MwlStr::new(&captured)));
        // ADR 0088 § 5's `{through:}` — the transform takes and answers the
        // same carrier, so the reference in `text` is transferred into the
        // call and whatever comes back is what this member answers.
        if matches!(args[1].tag(), Some(Tag::Null) | None) {
            return Ok(text);
        }
        let transformed = mwl_runtime::call_closure(ctx, args[1], &[text]);
        #[expect(
            unsafe_code,
            reason = "`call_closure` takes its own reference to each argument, \
                      so the one this frame built is still ours to drop"
        )]
        unsafe {
            text.release();
        }
        let transformed = transformed?;
        if transformed.obj_ptr().is_none() {
            #[expect(
                unsafe_code,
                reason = "the closure's result is this frame's to drop before it \
                          reports the type error"
            )]
            unsafe {
                transformed.release();
            }
            return Err(Fault::thrown(format!(
                "`Core\\Out::capture`'s `through` must answer a `{}`, and this one answered tag {}",
                crate::cli::NAME,
                transformed.tag_byte()
            )));
        }
        Ok(transformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registered row is what `mwl-types` seeds and what
    /// `mwl_ir::lower_call_args` flattens against, so its shape is worth
    /// pinning beside the helper's own `args: [2]`.
    #[test]
    fn capture_takes_a_callable_and_one_option_and_answers_the_carrier() {
        let capture = CLASS.methods[0];
        assert_eq!(capture.name, "capture");
        assert!(matches!(capture.params[0], CoreTy::Callable));
        assert!(matches!(capture.params[1], CoreTy::Options(options) if options.len() == 1));
        assert!(matches!(
            capture.return_ty,
            CoreTy::Instance(name) if name == crate::cli::NAME
        ));
    }
}
