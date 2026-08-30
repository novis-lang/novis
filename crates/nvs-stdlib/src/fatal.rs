//! `Core\Fatal` — [ADR 0020](../../../../docs/adr/0020-error-escalation-ladder.md)
//! § 1's tier 1: the one place a program gets to say anything at all after a
//! resource limit has stopped it.
//!
//! **Registration and nothing else lives here.** The member takes a closure and
//! puts it on the request's context; what a breach then does with it — the
//! reserved slice it runs under, the zero-retry rule, the fall to tier 2 —
//! belongs to the ladder in `nvs_runtime`, because the ladder is what *has* a
//! breach in hand. `Ctx::set_limit_handler` is the boundary, and its own doc
//! comment is the home for the ownership rule.
//!
//! **Why the closure is held by the context and not by this module.** A handler
//! is request-local by ADR 0020 § 1 — it dies with the request like every other
//! per-request slot ([ADR 0008](../../../../docs/adr/0008-static-and-global.md),
//! [ADR 0012](../../../../docs/adr/0012-no-superglobals.md)) — so a `static`
//! here would be the exact thing that section refuses: one request's safety net
//! still armed while another request runs on the same core.
//!
//! **The row is `callable` whatever the report is.** § 1 spells the parameter
//! `closure(LimitReport): void`, but ADR 0031 § 4 makes `callable` the only
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
    methods: &[CoreMethod {
        name: "onLimit",
        names: &["handler"],
        params: &[CoreTy::Callable],
        defaults: &[],
        return_ty: CoreTy::Void,
        symbol: "nvs_core_fatal_on_limit",
        doc: Some(&ON_LIMIT_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Fatal::onLimit`'s reference card — ADR 0117.
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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_fatal_on_limit" => (nvs_core_fatal_on_limit as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Fatal::onLimit(callable $handler): void` — ADR 0020 § 1.
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
