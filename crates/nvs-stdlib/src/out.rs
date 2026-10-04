//! `Core\Out` — docs/spec/01-core-library.md § 12's one member, and the whole
//! of PHP's `ob_*` family in it.
//!
//! `capture(callable $fn, {through?: callable}): Core\Html\Markup|Core\Cli\Text` runs `$fn` with
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
//!   next `echo` escape them twice. The carrier is the sink's: a
//!   `Core\Html\Markup` under a request and a `Core\Cli\Text` everywhere else,
//!   built by [`carried`]. The checker cannot know which sink will be in force,
//!   so the declared type is the union of the two, and a program that wants a
//!   `string` narrows with `is` and calls the carrier's `text()`.
//!
//! # What it spends
//!
//! One buffer per open capture, holding what that level has captured so far,
//! plus the one carrier instance the member answers with — both charged
//! to the request and both freed with it. A request that never captures pays
//! one not-taken branch per `echo`, which `nvs_runtime::Ctx`'s own
//! `captures` field states.

use nvs_runtime::{Fault, NvsObj, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
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
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: "capture",
        names: &["fn"],
        params: &[
            CoreTy::CallableSig(&[], &CoreTy::Mixed),
            CoreTy::Options(CAPTURE_OPTIONS),
        ],
        defaults: &[],
        return_ty: CoreTy::Union(&[
            CoreTy::Instance(crate::html::MARKUP_NAME),
            CoreTy::Instance(crate::cli::NAME),
        ]),
        symbol: "nvs_core_out_capture",
        doc: Some(&CAPTURE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Out`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Collects what a piece of code prints, so your program can use it as a value. \
            `capture` runs a function and returns everything it printed. This replaces PHP's \
            `ob_start` and `ob_get_clean`.",
};

/// `Core\Out::capture`'s reference card — `rule:core-api/reference-card`.
const CAPTURE_DOC: MethodDoc = MethodDoc {
    short: "Runs `$fn` and returns everything it printed with `echo`. This replaces PHP's \
            `ob_start` and `ob_get_clean`. The printed text does not reach the output. You can \
            call `capture` inside another `capture`, and each call collects only what its own \
            function printed.",
    params: &[
        ParamDoc {
            name: "fn",
            desc: "The function to run. Its return value is not used.",
            shape: &[],
        },
        ParamDoc {
            name: "through",
            desc: "A function that takes the collected output and returns new output of the \
                   same class. `capture` returns that new value. Without it, `capture` \
                   returns the output as it was printed.",
            shape: &[],
        },
    ],
    ret: "What `$fn` printed. In a web request it is a `Core\\Html\\Markup`. In every other \
          program it is a `Core\\Cli\\Text`. It is empty when `$fn` printed nothing. Use \
          `echo` to print it again without escaping it twice. Use `is` to check the class, \
          then its `text` method to get a `string`. Output from `Core\\Debug::dump` is not \
          collected.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The `through` function returned a value of a different class than it was given.",
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
    /// `Core\Out::capture(callable $fn, {through?: callable}): Core\Html\Markup|Core\Cli\Text`
    /// — see the module docs.
    ///
    /// The capture level is opened before `$fn` runs and closed on **both**
    /// edges, the throwing one included: a `Fault` propagating out of a
    /// capture that stayed open would silently swallow the rest of the
    /// request's output, which is the worst failure this member could have.
    /// That is why the closure's result is bound rather than `?`-ed.
    fn nvs_core_out_capture(ctx, args: [2]) {
        ctx.begin_capture();
        let outcome = nvs_runtime::call_callable(ctx, args[0], &[]);
        let captured = ctx.end_capture().unwrap_or_default();
        // The body's own return value is discarded — `capture` answers what was
        // written, not what was computed — so its reference ends here.
        match outcome {
            Ok(result) => {
                #[expect(
                    unsafe_code,
                    reason = "`call_callable` hands back a value the caller owns, \
                              and this one is never handed on"
                )]
                unsafe {
                    result.release();
                }
            }
            Err(fault) => return Err(fault),
        }
        let text = carried(ctx, &captured);
        // `rule:security/capture-answers-the-carrier`'s `{through:}` — the transform takes and answers the
        // same carrier, so the reference in `text` is transferred into the
        // call and whatever comes back is what this member answers.
        if matches!(args[1].tag(), Some(Tag::Null) | None) {
            return Ok(text);
        }
        let transformed = nvs_runtime::call_callable(ctx, args[1], &[text]);
        #[expect(
            unsafe_code,
            reason = "`call_callable` takes its own reference to each argument, \
                      so the one this frame built is still ours to drop"
        )]
        unsafe {
            text.release();
        }
        let transformed = transformed?;
        let carrier = ctx.carrier();
        if let Some(answered) = not_the_carrier(transformed, carrier) {
            #[expect(
                unsafe_code,
                reason = "the closure's result is this frame's to drop before it \
                          reports the type error"
            )]
            unsafe {
                transformed.release();
            }
            return Err(Fault::thrown(format!(
                "`Core\\Out::capture`'s `through` must answer a `{carrier}`, and this one \
                 answered {answered}"
            )));
        }
        Ok(transformed)
    }
}

/// `captured` — bytes that have already been through this request's sink — as
/// that sink's carrier: a `Core\Html\Markup` under the HTML sink and a
/// `Core\Cli\Text` under every other (`Ctx::carrier`).
///
/// `rule:security/capture-answers-the-carrier`, for both of its members:
/// `Core\Out::capture` here and a `spawn script` result's `output`
/// (`crate::script`'s `result_of`), whose child writes into a sink of its
/// parent's carrier. Both are typed `Core\Html\Markup|Core\Cli\Text`, because
/// which sink is in force is not known when the program is checked; whichever
/// one this builds is the one `echo` writes unchanged. Like
/// [`crate::cli::built`] this transfers bytes and neutralizes nothing, which is
/// sound only because the sink neutralized them on their way in.
pub(crate) fn carried(ctx: &nvs_runtime::Ctx, captured: &[u8]) -> Value {
    let text = Value::str(NvsStr::new(captured));
    if ctx.carrier() == crate::html::MARKUP_NAME {
        crate::instance::build(&crate::html::MARKUP, [text])
    } else {
        crate::cli::built(text)
    }
}

/// How to name what a `through` closure answered, or `None` where it answered
/// `carrier`, the class [`carried`] built for this sink.
///
/// A `callable` is opaque as to signature (`rule:types/anonymous-function`),
/// so nothing static stands between `{through:}` and this check — which is why
/// it asks about the **class** and not merely about objecthood. A foreign
/// object, or the other sink's carrier, would make the member's registered
/// return type a claim about the value that is not true, and `echo` would
/// escape the other carrier a second time. The comparison is by rendered class
/// name, which is what a descriptor carries.
fn not_the_carrier(value: Value, carrier: &str) -> Option<String> {
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
    (object.class_name() != carrier).then(|| {
        let name = object.class_name();
        format!("an instance of `{name}`")
    })
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{
        CALLABLE_ARITY_SLOT, CALLABLE_INVOKE, CALLABLE_PARAM_TAGS_SLOT, CARRIER_TEXT_SLOT,
        ClassTable, Ctx, MethodRow, NvsFn, OK, THROWN, call,
    };

    use super::*;

    /// A closure value of no parameters whose `invoke` is a plain Rust
    /// function — `crates/nvs-stdlib/tests/allocation_policy.rs`'s
    /// `callable_of`, whose doc comment says why this is a whole closure.
    ///
    /// The table is leaked because a descriptor's *address* is its identity and
    /// it must outlive every instance made from it.
    fn callable_of(invoke: NvsFn) -> Value {
        let mut table = ClassTable::new();
        let id = table.define("{callable}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: CALLABLE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_callable(id);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(CALLABLE_ARITY_SLOT, Value::int(0));
        object.set_field(CALLABLE_PARAM_TAGS_SLOT, Value::int(0));
        Value::object(object)
    }

    /// `fn (): void => { echo "inside"; }`: write through the context the way
    /// a compiled `echo` does, release the receiver `call_callable` retained,
    /// and answer `null`.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes a live context and exactly one retained \
                  value, the receiver, and `abi::call` passes the address of a \
                  live `Value` for the result"
    )]
    unsafe extern "C" fn echoes(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let ctx = unsafe { &mut *ctx };
        ctx.write_output(b"inside").expect("a buffered sink");
        unsafe {
            (*args).release();
            *out = Value::null();
        }
        OK
    }

    /// `fn (): void => { echo "lost"; throw … }`: the same write, then a throw.
    #[expect(
        unsafe_code,
        reason = "the same contract as `echoes`, answering a throw instead"
    )]
    unsafe extern "C" fn echoes_then_throws(
        ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        let ctx = unsafe { &mut *ctx };
        ctx.write_output(b"lost").expect("a buffered sink");
        ctx.set_pending("the body threw");
        unsafe {
            (*args).release();
            *out = Value::null();
        }
        THROWN
    }

    /// `rule:security/capture-answers-the-carrier` driven through the member
    /// itself: what the closure wrote comes back as a `Core\Cli\Text`, the sink
    /// below sees none of it, and a closure that throws still leaves the
    /// capture closed, so the rest of the request's output is not swallowed.
    // covers: Core\Out::capture
    #[test]
    fn capture_answers_the_carrier_swallows_and_closes_on_a_throw() {
        let mut ctx = Ctx::buffered();
        ctx.write_output(b"before<").expect("a buffered sink");

        let body = callable_of(echoes);
        let captured = call(nvs_core_out_capture, &mut ctx, &[body, Value::null()])
            .expect("a body that returns is captured");
        assert!(
            not_the_carrier(captured, crate::cli::NAME).is_none(),
            "the carrier, not a string"
        );
        let ptr = captured.obj_ptr().expect("an object");
        assert_eq!(
            crate::instance::slot(ptr, CARRIER_TEXT_SLOT).as_text(),
            Some("inside")
        );
        assert_eq!(ctx.capture_depth(), 0);

        let thrower = callable_of(echoes_then_throws);
        assert!(call(nvs_core_out_capture, &mut ctx, &[thrower, Value::null()]).is_err());
        assert_eq!(
            ctx.capture_depth(),
            0,
            "the throwing edge closes the level too"
        );
        assert_eq!(ctx.take_pending().as_deref(), Some("the body threw"));

        ctx.write_output(b">after").expect("a buffered sink");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"before<>after"[..]),
            "nothing either body wrote reached the sink below"
        );
        #[expect(
            unsafe_code,
            reason = "each value is one this test built or was handed, and owns \
                      exactly one reference to"
        )]
        unsafe {
            captured.release();
            body.release();
            thrower.release();
        }
    }

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
            CoreTy::Union([CoreTy::Instance(markup), CoreTy::Instance(text)])
                if *markup == crate::html::MARKUP_NAME && *text == crate::cli::NAME
        ));
    }

    /// Under the HTML sink the same capture answers a `Core\Html\Markup`, the
    /// one class that sink writes unchanged, so re-echoing a captured page does
    /// not escape it a second time.
    // covers: Core\Out::capture
    #[test]
    fn capture_under_the_html_sink_answers_markup() {
        let mut ctx = Ctx::new(nvs_runtime::OutputSink::Body(Vec::new()));
        let body = callable_of(echoes);
        let captured = call(nvs_core_out_capture, &mut ctx, &[body, Value::null()])
            .expect("a body that returns is captured");
        assert!(
            not_the_carrier(captured, crate::html::MARKUP_NAME).is_none(),
            "the HTML sink's carrier"
        );
        let ptr = captured.obj_ptr().expect("an object");
        assert_eq!(
            crate::instance::slot(ptr, CARRIER_TEXT_SLOT).as_text(),
            Some("inside")
        );
        #[expect(
            unsafe_code,
            reason = "each value is one this test built or was handed, and owns \
                      exactly one reference to"
        )]
        unsafe {
            captured.release();
            body.release();
        }
    }
}
