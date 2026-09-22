//! `Core\Out` — docs/spec/01-core-library.md § 12's one member, and the whole
//! of PHP's `ob_*` family in it.
//!
//! `capture(callable $fn, {through?: callable}): Core\Cli\Text` runs `$fn` with
//! this request's sink redirected into a buffer, and answers what it wrote.
//! Three properties of that sentence are the design, and each is
//! `rule:security/capture-answers-the-carrier`
//! 's or the spec's rather than this file's:
//!
//! * **Scoped to a closure, so it nests by call nesting.** `ob_start` and
//!   `ob_get_clean` are two members of a *global* stack that can be started in
//!   one function and ended in another, which is why inspecting it takes
//!   `ob_get_level` and a loop. Here the two ends are the two ends of one call,
//!   so the stack is an implementation detail — it lives on
//!   `nvs_runtime::Ctx` ([`Ctx::begin_capture`](nvs_runtime::Ctx::begin_capture))
//!   and no program can observe an unbalanced one.
//! * **It always swallows.** Nothing `$fn` echoes reaches the sink below.
//!   `ob_start($callback)`'s invisible pass-through has no equivalent: re-
//!   emitting is a visible `echo Core\Out::capture(…)`, and `{through:}`
//!   transforms the captured value rather than deciding whether it escapes.
//! * **It answers the carrier, not a `string`.** `rule:security/capture-answers-the-carrier`: those bytes have
//!   already been through the sink, so handing them back as text would let the
//!   next `echo` escape them twice. [`crate::cli`] is the carrier and owns what
//!   one is — `Core\Cli\Text::text` included, which is how a `{through:}` reads
//!   what it was handed and why that read is this sink's alone.
//!
//! # What it spends
//!
//! One buffer per open capture, holding what that level has captured so far,
//! plus the one `Core\Cli\Text` instance the member answers with — both charged
//! to the request and both freed with it. A request that never captures pays
//! one not-taken branch per `echo`, which `nvs_runtime::Ctx`'s own
//! `captures` field states.

use nvs_runtime::{Fault, NvsObj, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
};

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
    doc: None,
    methods: &[CoreMethod {
        name: "capture",
        names: &["fn"],
        params: &[
            CoreTy::CallableSig(&[], &CoreTy::Mixed),
            CoreTy::Options(CAPTURE_OPTIONS),
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(crate::cli::NAME),
        symbol: "nvs_core_out_capture",
        doc: Some(&CAPTURE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Out::capture`'s reference card — `rule:core-api/reference-card`.
const CAPTURE_DOC: MethodDoc = MethodDoc {
    short: "Runs `$fn` with this request's output sink redirected into a buffer and answers \
            what it wrote, as the carrier of the sink in force — `ob_start`/`ob_get_clean` and \
            `ob_start($callback)`, scoped to one closure so it nests by call nesting and \
            always swallows.",
    params: &[
        ParamDoc {
            name: "fn",
            desc: "The closure to run; its own return value is discarded, since the capture \
                   answers what was written rather than what was computed.",
            shape: &[],
        },
        ParamDoc {
            name: "through",
            desc: "A `callable(Core\\Cli\\Text): Core\\Cli\\Text` applied to the captured \
                   carrier before it is answered; the default answers it as captured.",
            shape: &[],
        },
    ],
    ret: "The captured output as a `Core\\Cli\\Text` — never a plain `string`, since those \
          bytes have already been through the sink — and an empty carrier when `$fn` wrote \
          nothing. Nothing `$fn` echoed reaches the sink below; re-emitting is a \
          visible `echo Core\\Out::capture(…)`, and a `Core\\Debug::dump` inside `$fn` is not \
          captured.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`through` answered something other than a `Core\\Cli\\Text`.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_out_capture" => (nvs_core_out_capture as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Out::capture(callable $fn, {through?: callable}): Core\Cli\Text`
    /// — see the module docs.
    ///
    /// The capture level is opened before `$fn` runs and closed on **both**
    /// edges, the throwing one included: a `Fault` propagating out of a
    /// capture that stayed open would silently swallow the rest of the
    /// request's output, which is the worst failure this member could have.
    /// That is why the closure's result is bound rather than `?`-ed.
    fn nvs_core_out_capture(ctx, args: [2]) {
        ctx.begin_capture();
        let outcome = nvs_runtime::call_closure(ctx, args[0], &[]);
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
        let text = crate::cli::built(Value::str(NvsStr::new(&captured)));
        // `rule:security/capture-answers-the-carrier`'s `{through:}` — the transform takes and answers the
        // same carrier, so the reference in `text` is transferred into the
        // call and whatever comes back is what this member answers.
        if matches!(args[1].tag(), Some(Tag::Null) | None) {
            return Ok(text);
        }
        let transformed = nvs_runtime::call_closure(ctx, args[1], &[text]);
        #[expect(
            unsafe_code,
            reason = "`call_closure` takes its own reference to each argument, \
                      so the one this frame built is still ours to drop"
        )]
        unsafe {
            text.release();
        }
        let transformed = transformed?;
        if let Some(answered) = not_the_carrier(transformed) {
            #[expect(
                unsafe_code,
                reason = "the closure's result is this frame's to drop before it \
                          reports the type error"
            )]
            unsafe {
                transformed.release();
            }
            return Err(Fault::thrown(format!(
                "`Core\\Out::capture`'s `through` must answer a `{}`, and this one answered \
                 {answered}",
                crate::cli::NAME
            )));
        }
        Ok(transformed)
    }
}

/// How to name what a `through` closure answered, or `None` where it answered
/// the carrier this member is declared to hand back.
///
/// A `callable` is opaque as to signature (`rule:types/closure-literal`),
/// so nothing static stands between `{through:}` and this check — which is why
/// it asks about the **class** and not merely about objecthood. Answering a
/// foreign object used to be accepted here, and the member's registered
/// `Core\Cli\Text` return type was then a claim about the value that was not
/// true; the failure surfaced much later, wherever the carrier was next read.
/// The comparison is by rendered class name, which is what a descriptor
/// carries and what [`crate::cli::built`] is the one producer of.
fn not_the_carrier(value: Value) -> Option<String> {
    let Some(ptr) = value.obj_ptr() else {
        return Some(value.tag().map_or_else(
            || format!("a value carrying tag {}", value.tag_byte()),
            |tag| format!("a value of type `{}`", tag.describe()),
        ));
    };
    #[expect(
        unsafe_code,
        reason = "the value owns a reference to a live allocation, so it is live \
                  for this borrow; the handle is never dropped, so the reference \
                  is not released twice"
    )]
    let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
    (object.class_name() != crate::cli::NAME).then(|| {
        let name = object.class_name();
        format!("an instance of `{name}`")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registered row is what `nvs-types` seeds and what
    /// `nvs_ir::lower_call_args` flattens against, so its shape is worth
    /// pinning beside the helper's own `args: [2]`.
    #[test]
    fn capture_takes_a_callable_and_one_option_and_answers_the_carrier() {
        let capture = CLASS.methods[0];
        assert_eq!(capture.name, "capture");
        assert!(matches!(
            capture.params[0],
            CoreTy::CallableSig(params, ret)
                if params.is_empty() && matches!(ret, CoreTy::Mixed)
        ));
        assert!(matches!(capture.params[1], CoreTy::Options(options) if options.len() == 1));
        assert!(matches!(
            capture.return_ty,
            CoreTy::Instance(name) if name == crate::cli::NAME
        ));
    }
}
