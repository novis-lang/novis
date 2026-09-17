//! `Core\Fatal` — `rule:errors/on-limit` and `rule:errors/on-uncaught-throw`'s tiers 1 and 2: the two places a program gets to say anything at all
//! after a resource limit has stopped it, or after a throw reached the root of
//! the request with nothing left to catch it.
//!
//! **Registration and nothing else lives here.** Each member takes a closure
//! and puts it on the request's context; what a breach or a root throw then
//! does with it — the reserved slice tier 1 runs under and tier 2 has none of,
//! the zero-retry rule, the fall to tier 3 — belongs to the ladder in
//! `nvs_runtime`, because the ladder is what *has* the failure in hand.
//! `Ctx::set_limit_handler` and `Ctx::set_uncaught_handler` are the boundary,
//! and their own doc comments are the home for the ownership rule.
//!
//! **Two tiers, two slots, one shape of registration.** The members differ in
//! what fires them and in what the handler is handed — § 1's `LimitReport`
//! array, § 2's real `Throwable` object — and in nothing else, so a program
//! registering both gets two independent handlers and neither registration
//! disturbs the other.
//!
//! **Why the closure is held by the context and not by this module.** A handler
//! is request-local by `rule:errors/on-limit` — it dies with the request like every other
//! per-request slot (`rule:statements/static-is-a-member-modifier`,
//! `rule:statements/no-host-populated-variables`) — so a `static`
//! here would be the exact thing that section refuses: one request's safety net
//! still armed while another request runs on the same core.
//!
//! **The row is `callable` whatever the report is.** § 1 spells the parameter
//! `closure(LimitReport): void`, but `rule:types/callable-absorbs-closure` makes `callable` the only
//! closure type there is and it says nothing about what a closure takes, so
//! what the handler is *handed* is decided at the call rather than here. It is
//! an array with a `limit` key naming the limit that stopped the request, and
//! `nvs_runtime::Limit` is that decision's home — including why an array and
//! not a `Core` class. A handler declaring no parameter at all still runs.

use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Fatal";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "onLimit",
            names: &["handler"],
            params: &[CoreTy::CallableSig(
                &[CoreTy::Array(&CoreTy::Str)],
                &CoreTy::Mixed,
            )],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_fatal_on_limit",
            doc: Some(&ON_LIMIT_DOC),
        },
        CoreMethod {
            name: "onUncaughtThrow",
            names: &["handler"],
            params: &[CoreTy::CallableSig(
                &[CoreTy::Instance("Throwable")],
                &CoreTy::Mixed,
            )],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_fatal_on_uncaught_throw",
            doc: Some(&ON_UNCAUGHT_THROW_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Fatal::onLimit`'s reference card — `rule:core-api/reference-card`.
const ON_LIMIT_DOC: MethodDoc = MethodDoc {
    short: "Registers the closure this request runs when a resource limit stops it — memory, CPU \
            time, output, wall time, script depth or call-stack depth. It runs out of a slice of the \
            request's budget reserved for it, once and never twice, and it is the only thing that \
            observes a `FATAL` a `catch` never sees.",
    params: &[ParamDoc {
        name: "handler",
        desc: "What to run. It is handed one array whose `limit` key names the limit that stopped \
               the request — `memory` or `cpu_time`, the directive's own spelling — and answers \
               nothing; declaring no parameter is allowed. A handler that throws, or that \
               exhausts the reserved slice itself, is abandoned where it stands.",
        shape: &[],
    }],
    ret: "Nothing. Registering is request-local and a second call replaces the first: the handler \
          is gone when the request ends, and no other request on this core can see it.",
    errors: &[],
};

/// `Core\Fatal::onUncaughtThrow`'s reference card — `rule:core-api/reference-card`.
const ON_UNCAUGHT_THROW_DOC: MethodDoc = MethodDoc {
    short: "Registers the closure this request runs when a throw reaches the top of it with nothing \
            left to catch it. It runs out of the request's ordinary remaining budget, once and \
            never twice, and it is handed the exception itself.",
    params: &[ParamDoc {
        name: "handler",
        desc: "What to run. It is handed the `Throwable` that went uncaught — the object the \
               program threw, with its own class, message and backtrace — and answers nothing; \
               declaring no parameter is allowed. A handler that throws, or that exhausts what \
               the request has left, is abandoned where it stands, and the failure reported is \
               still the one that reached the root.",
        shape: &[],
    }],
    ret: "Nothing. Registering is request-local and a second call replaces the first: the handler \
          is gone when the request ends, and no other request on this core can see it. It does \
          not stop the failure being reported — the engine still writes its own record — and it \
          does not change the exit status.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_fatal_on_limit" => (nvs_core_fatal_on_limit as *const ()).cast(),
        "nvs_core_fatal_on_uncaught_throw" => {
            (nvs_core_fatal_on_uncaught_throw as *const ()).cast()
        }
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Fatal::onLimit(callable $handler): void` — `rule:errors/on-limit`.
    ///
    /// The retain is the whole body's reason for existing: a helper's arguments
    /// are borrowed from the caller's frame, and this one outlives the call by
    /// the whole of the request.
    fn nvs_core_fatal_on_limit(ctx, args: [1]) {
        // The row's one parameter is `CoreTy::Callable`, so `E0401` refuses a
        // `null` at the call and the guard below is unreachable from source:
        // what it catches is a lowering bug, and it is here rather than absent
        // because the slot it would otherwise write is the one the ladder reads
        // to decide whether a handler exists at all.
        if args[0].tag() == Some(Tag::Null) {
            return Err(Fault::fatal(
                "Core\\Fatal::onLimit expected a closure for its handler, got null".to_string(),
            ));
        }
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame and the \
                      context keeps it past this call, so it needs a reference \
                      of its own — which `Ctx::set_limit_handler` then owns"
        )]
        // SAFETY: the value is owned by the caller's argument slot, which
        // outlives this call; the reference taken here is the one the context
        // releases when the request ends or a second registration replaces it.
        unsafe {
            args[0].retain();
        }
        ctx.set_limit_handler(args[0]);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Fatal::onUncaughtThrow(callable $handler): void` — `rule:errors/on-uncaught-throw`.
    ///
    /// Its sibling above, over the second slot: the retain is again the whole
    /// body's reason for existing, because a helper's arguments are borrowed
    /// from the caller's frame and this one outlives the call by the whole of
    /// the request. What the two members do *not* share is on the ladder's side
    /// of the boundary — `Ctx::run_uncaught_handler` owns the real `Throwable`,
    /// the absent reserve and the tiers below that still run.
    fn nvs_core_fatal_on_uncaught_throw(ctx, args: [1]) {
        // `nvs_core_fatal_on_limit`'s guard, for its reason: the row's one
        // parameter is `CoreTy::Callable`, so this is unreachable from source
        // and what it catches is a lowering bug writing the slot the ladder
        // reads to decide whether a handler exists at all.
        if args[0].tag() == Some(Tag::Null) {
            return Err(Fault::fatal(
                "Core\\Fatal::onUncaughtThrow expected a closure for its handler, got null"
                    .to_string(),
            ));
        }
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame and the \
                      context keeps it past this call, so it needs a reference \
                      of its own — which `Ctx::set_uncaught_handler` then owns"
        )]
        // SAFETY: the value is owned by the caller's argument slot, which
        // outlives this call; the reference taken here is the one the context
        // releases when the request ends or a second registration replaces it.
        unsafe {
            args[0].retain();
        }
        ctx.set_uncaught_handler(args[0]);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use nvs_runtime::{
        CLOSURE_ARITY_SLOT, CLOSURE_INVOKE, CLOSURE_PARAM_TAG_ANY, CLOSURE_PARAM_TAGS_SLOT,
        ClassTable, Ctx, ErrorClass, MethodRow, NvsFn, NvsObj, OK, Tag, Value, call,
    };

    use super::nvs_core_fatal_on_uncaught_throw;

    thread_local! {
        /// The payload bits of whatever [`records`] was handed, which is how a
        /// plain `extern "C"` callback reports back to the test that installed
        /// it: a closure with no captured state has nowhere else to put it.
        static SEEN: Cell<u64> = const { Cell::new(0) };
    }

    /// `rule:errors/on-uncaught-throw`'s headline claim, asked as an **identity** rather than as a
    /// resemblance: the tier-2 handler is handed "the **real `Throwable`
    /// object**, not copied data", so the value that arrives is the very
    /// allocation the request threw and not a report rebuilt from it.
    ///
    /// Comparing the payload bits is what makes that a check on the *ladder*
    /// rather than on a serialiser: a `Ctx::run_uncaught_handler` that built an
    /// array like § 1's `LimitReport`, or that copied the exception across the
    /// way an `rule:security/isolate-shares-nothing` boundary has to, would still carry the right class and
    /// message and would differ here in the one respect § 2 names.
    ///
    /// Both halves are driven for real — the registration through
    /// [`super::nvs_core_fatal_on_uncaught_throw`] itself, the firing through
    /// the ladder — so this also pins the boundary between them: the member
    /// takes a reference of its own, and the ladder takes the registration out
    /// of the slot on the way in, which is the whole of § 2's zero-retry rule.
    #[test]
    fn on_uncaught_throw_receives_the_real_throwable() {
        // Spec § 10's root shape, installed the one way a context takes one —
        // an installed class is what promotes a bare failure to an object at
        // all, exactly as `crate::log`'s own agreement test needs.
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), root));
        ctx.set_pending("the store said no");
        ctx.push_frame("Main::main");
        let thrown = ctx.take_thrown();
        assert!(
            !thrown.is_none(),
            "an installed class is what promotes a bare failure to an object"
        );

        let handler = closure_of(1, records);
        call(nvs_core_fatal_on_uncaught_throw, &mut ctx, &[handler])
            .expect("registering answers `void` and cannot fail");
        assert!(
            ctx.has_uncaught_handler(),
            "the member's whole job is to fill the slot the ladder reads"
        );

        SEEN.with(|seen| seen.set(0));
        ctx.run_uncaught_handler(&thrown);

        assert_eq!(
            SEEN.with(Cell::get),
            thrown.as_value().bits(),
            "`rule:errors/on-uncaught-throw`: the handler is handed the object the program threw, \
             not a copy of what it said"
        );
        assert_eq!(
            thrown.as_value().tag(),
            Some(Tag::Object),
            "and it arrives as the exception object rather than as a report"
        );
        assert!(
            !ctx.has_uncaught_handler(),
            "§ 2's zero retries: the registration leaves the slot on the way in"
        );
        // The handler borrowed the exception rather than consuming it: this
        // frame still owns the only reference, and the object is still readable
        // after the call that was handed it.
        assert_eq!(thrown.message(), "the store said no");
        assert_eq!(thrown.class_name(), "RuntimeError");
        release(handler);
    }

    /// The callback the test registers: record what arrived, sweep the
    /// references `call_closure` retained for this callee, and answer `null` —
    /// which is what a `void` closure answers.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes exactly two live values, each retained \
                  for this callee to release, and `abi::call` passes the \
                  address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn records(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        // Slot 0 is the closure itself and slot 1 its one parameter, which is
        // the exception — `nvs_runtime::call_closure` builds the frame that
        // way for a compiled callee and for this one alike.
        let thrown = unsafe { *args.add(1) };
        SEEN.with(|seen| seen.set(thrown.bits()));
        for index in 0..2 {
            release(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::null();
        }
        OK
    }

    /// A closure value whose `invoke` is a plain Rust function —
    /// `crates/nvs-stdlib/tests/allocation_policy.rs`'s `closure_of`, and its
    /// doc comment is the home for why this is a whole closure: `call_closure`
    /// reads the arity slot, the tags slot and the invoke address, and nothing
    /// else in a compiled closure's representation is anything but captured
    /// state a native callback does not have.
    ///
    /// The table is leaked because a descriptor's *address* is its identity and
    /// it must outlive every instance made from it.
    fn closure_of(arity: usize, invoke: NvsFn) -> Value {
        let mut table = ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(
            CLOSURE_ARITY_SLOT,
            Value::int(i64::try_from(arity).expect("a small arity")),
        );
        let mut tags: u64 = 0;
        for parameter in 0..arity {
            tags |= u64::from(CLOSURE_PARAM_TAG_ANY) << (parameter * 4);
        }
        object.set_field(
            CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from_ne_bytes(tags.to_ne_bytes())),
        );
        Value::object(object)
    }

    /// One reference, given back through the ABI's own entry point.
    #[expect(
        unsafe_code,
        reason = "every value released here is one this test or its callee \
                  owns a reference to"
    )]
    fn release(value: Value) {
        unsafe {
            nvs_runtime::nvs_value_release(u64::from(value.tag_byte()), value.bits());
        }
    }
}
