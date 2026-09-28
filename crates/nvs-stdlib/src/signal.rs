//! `Core\Signal` — what a program gets to run when the process is asked to
//! stop, and nothing else that `pcntl_*` had.
//!
//! `rule:core-api/tier-roster` places the class
//! ([0051](/docs/decisions/0051.md) § 3) in one clause: `fork` is refused, and
//! what remains is "a narrow `Core\Signal` for graceful shutdown". The spec's
//! § 16 row says the same thing and lists no members, so the surface is the
//! narrowest one that answers the row — a single registration.
//!
//! # There is no `kill`, no `alarm` and no signal number
//!
//! Every one of those is job control, which is the half of `pcntl` the roster
//! refuses rather than the half it keeps. A number is refused with them: a
//! member taking `int $signo` would be a way to ask for `SIGKILL` as much as
//! for `SIGTERM`, and a constant like `SIGTERM = 15` would be that member's
//! argument list written down. So this class holds no constant, no integer
//! anywhere in a signature, and one verb — which is what
//! `there_is_no_kill_no_alarm_and_no_signal_number_as_an_integer` holds it to.
//!
//! **A terminating signal is therefore not distinguished from another.**
//! `SIGTERM`, `SIGINT` and the console's Ctrl-C all mean the same thing to a
//! program that may only shut down gracefully, and a handler that could tell
//! them apart could only act on the difference by doing something this class
//! does not offer.
//!
//! # Why there is no `isShuttingDown` here
//!
//! Because [`crate::server`] already answers it. `Core\Server::isDraining()`
//! reads [`nvs_runtime::drain`]'s process bit, which is the same bit a delivered
//! signal sets and the same one `[server] health_path` reports to a proxy — one
//! fact with one home, so a second spelling of it under this class would be a
//! second reading of the same atomic for a reader to choose between. A CLI
//! program with no accept loop reads it too, and is told `false` until a
//! shutdown begins.
//!
//! # How a delivery reaches the handler
//!
//! `rule:concurrency/a-drain-closes-a-connection-cleanly`'s drain is the state
//! machine, and this class adds none of its own. The path has three steps and
//! the middle one is the point:
//!
//! 1. The delivery — whatever installs the operating system's handler — begins
//!    the process drain, which is one relaxed store, and returns. Nothing else
//!    happens in a signal context.
//! 2. Each running request is asked for a safepoint
//!    ([`nvs_runtime::SafepointFlags::SHUTDOWN`]).
//! 3. `nvs_safepoint` runs the registered closure between two Novis statements,
//!    on the request's own stack, with the whole budget and every `Core` member
//!    reachable — `Ctx::run_shutdown_handler` is the home of the once-only rule
//!    and of why nothing is reserved for it.
//!
//! So the handler observes a process that is *already* draining: it is entered
//! inside the shutdown rather than being the thing that starts one, and
//! `Core\Server::isDraining()` answers `true` throughout it.
//!
//! # Registration and nothing else lives here
//!
//! [`crate::fatal`]'s division exactly. The member puts a closure on the
//! request's context and stops; what a delivery then does with it belongs to
//! `nvs_runtime`, because the safepoint is what has the request in hand. The
//! slot is request-local for that module's reason as well — a `static` here
//! would leave one request's handler armed while another ran on the same core,
//! which is what `rule:security/no-cross-request-state` refuses.

use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Signal";

/// `Core\Signal`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Runs a function of your program when the process is asked to stop, so the program can \
            finish its work cleanly. `Core\\Server::isDraining()` tells whether a shutdown has \
            started.",
};

/// `Core\Signal`'s registry rows — one member, and the module docs above own
/// why the rest of `pcntl` is not beside it.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: "onShutdown",
        names: &["handler"],
        params: &[CoreTy::CallableSig(&[], &CoreTy::Mixed)],
        defaults: &[],
        return_ty: CoreTy::Void,
        symbol: "nvs_core_signal_on_shutdown",
        doc: Some(&ON_SHUTDOWN_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Signal::onShutdown`'s reference card — `rule:core-api/reference-card`.
const ON_SHUTDOWN_DOC: MethodDoc = MethodDoc {
    short: "Registers a function that runs when the process is asked to stop, for example by \
            Ctrl-C, `SIGTERM` or an operator who stops the server. The function runs once, \
            between two statements. At that time `Core\\Server::isDraining()` already returns \
            `true`.",
    params: &[ParamDoc {
        name: "handler",
        desc: "The function to run. It gets no arguments and returns nothing. Every stop signal \
               has the same effect, so the function cannot tell which one arrived. If it throws an \
               error or reaches the request's memory limit, the rest of it does not run.",
        shape: &[],
    }],
    ret: "Nothing. Each request has its own function, and a second call replaces the first. The \
          function is deleted when the request ends, and other requests cannot see it. It cannot \
          stop the shutdown or delay it.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_signal_on_shutdown" => (nvs_core_signal_on_shutdown as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Signal::onShutdown(callable $handler): void` —
    /// `rule:core-api/tier-roster`'s graceful shutdown.
    ///
    /// [`crate::fatal`]'s registration, over the third handler slot: the retain
    /// is the whole body's reason for existing, because a helper's arguments are
    /// borrowed from the caller's frame and this one outlives the call by the
    /// whole of the request.
    fn nvs_core_signal_on_shutdown(ctx, args: [1]) {
        // The row's one parameter is a callable, so `E0401` refuses a `null` at
        // the call and this guard is unreachable from source: what it catches is
        // a lowering bug, and it is here rather than absent because the slot it
        // would otherwise write is the one the safepoint reads to decide whether
        // a handler exists at all.
        if args[0].tag() == Some(Tag::Null) {
            return Err(Fault::fatal(
                "Core\\Signal::onShutdown expected a closure for its handler, got null".to_string(),
            ));
        }
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame and the \
                      context keeps it past this call, so it needs a reference \
                      of its own — which `Ctx::set_shutdown_handler` then owns"
        )]
        // SAFETY: the value is owned by the caller's argument slot, which
        // outlives this call; the reference taken here is the one the context
        // releases when the handler runs, when the request ends, or when a
        // second registration replaces it.
        unsafe {
            args[0].retain();
        }
        ctx.set_shutdown_handler(args[0]);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use nvs_runtime::drain::Drain;
    use nvs_runtime::{Ctx, OutputSink, SafepointFlags, Value};

    use super::{CLASS, nvs_core_signal_on_shutdown};
    use crate::registry::CoreTy;

    thread_local! {
        /// What the handler below saw when it ran, or `None` if it never ran —
        /// which is the distinction both of the first two cases rest on.
        static SEEN: Cell<Option<Seen>> = const { Cell::new(None) };
    }

    /// One entry of the handler, as the two facts a signal handler could not
    /// have observed: it was handed a live request context, and the process was
    /// already draining around it.
    #[derive(Clone, Copy)]
    struct Seen {
        /// Whether the closure was entered with this request's context — the
        /// thing that makes it ordinary Novis code rather than a callback.
        held_a_context: bool,
        /// What `nvs_runtime::drain::is_draining()` answered inside the call.
        drain_had_begun: bool,
    }

    /// The closure the cases register: it records what it could see and answers
    /// nothing, exactly as a `void` handler compiled from Novis would.
    ///
    /// `call_closure` retains the receiver for this callee to release, and a
    /// handler declaring no parameter gets that one slot and no other.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver live and retained for this \
                  callee to release, and the address of a live `Value` for the \
                  result — neither is expressible in the signature compiled code \
                  calls through"
    )]
    unsafe extern "C" fn records_what_it_saw(
        ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        SEEN.with(|seen| {
            seen.set(Some(Seen {
                held_a_context: !ctx.is_null(),
                drain_had_begun: nvs_runtime::drain::is_draining(),
            }));
        });
        // SAFETY: the caller passed one live retained slot and a writable
        // result; releasing the receiver is this callee's half of that contract.
        unsafe {
            args.read().release();
            *out = Value::null();
        }
        nvs_runtime::OK
    }

    /// A closure value whose `invoke` is the callback above, declaring no
    /// parameter.
    ///
    /// `nvs_runtime::call_closure` reads a closure's arity and invoke address
    /// and nothing else, so this is a whole `callable` with no compiler in front
    /// of it. The table is leaked because a descriptor's address is its identity
    /// and it must outlive every instance made from it; the test process exiting
    /// is what reclaims it.
    fn a_handler() -> Value {
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                code: records_what_it_saw as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives every \
                      instance made from it — `NvsObj::new`'s whole obligation"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(0));
        object.set_field(nvs_runtime::CLOSURE_PARAM_TAGS_SLOT, Value::int(0));
        Value::object(object)
    }

    /// Registers a handler on `ctx` through the member itself, and hands back
    /// the reference this frame still owns.
    fn register_on(ctx: &mut Ctx) -> Value {
        SEEN.with(|seen| seen.set(None));
        let handler = a_handler();
        nvs_runtime::call(nvs_core_signal_on_shutdown, ctx, &[handler])
            .expect("registering a shutdown handler answers");
        handler
    }

    /// Drops the reference `register_on` handed back.
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `a_handler` made; the \
                  context's own is released by the run or by the request ending"
    )]
    fn release(handler: Value) {
        // SAFETY: one reference, held by this frame and by nothing else.
        unsafe { handler.release() };
    }

    /// The handler is not entered by the delivery: it runs at a safepoint, as
    /// ordinary code with the request's context in hand.
    ///
    /// Both halves are asserted, because only the pair says what the rule does.
    /// Registration on its own runs nothing — a delivery that called the closure
    /// where it arrived would show up here — and the run that does happen is a
    /// `nvs_safepoint` frame the request continues out of, with a live `Ctx` the
    /// callee was handed. No signal handler could have either.
    #[expect(
        unsafe_code,
        reason = "`nvs_safepoint`'s pointer contract is discharged by the \
                  borrow, which is live for the whole call"
    )]
    // covers: Core\Signal::onShutdown
    #[test]
    fn a_handler_runs_as_ordinary_novis_code_at_a_safepoint_and_never_in_a_signal_context() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let handler = register_on(&mut ctx);
        assert!(
            SEEN.with(Cell::get).is_none(),
            "registering a handler ran it"
        );
        ctx.request_safepoint(SafepointFlags::SHUTDOWN);
        // SAFETY: the pointer is a reborrow of a live local.
        let status = unsafe { nvs_runtime::nvs_safepoint(&raw mut ctx) };
        assert_eq!(
            status,
            nvs_runtime::OK,
            "a shutdown stopped the request instead of asking it to finish"
        );
        let seen = SEEN.with(Cell::get).expect("the safepoint ran the handler");
        assert!(
            seen.held_a_context,
            "the handler ran without the request's context"
        );
        assert!(
            !ctx.safepoint_flags().contains(SafepointFlags::SHUTDOWN),
            "the request is still being asked to shut down after its handler ran"
        );
        release(handler);
    }

    /// The handler runs *inside* the drain `nvs_runtime::drain` already owns,
    /// and this class keeps no second bit to run inside instead.
    ///
    /// The delivery's whole effect on state is `Drain::process().begin()`, so
    /// that is what this performs; what it then asserts is that the closure sees
    /// a process already draining, which is only true if the two are one state
    /// machine. The roster half is asserted beside it: a member here answering
    /// the drain would be the second reading `Core\Server::isDraining()` already
    /// has.
    #[expect(
        unsafe_code,
        reason = "`nvs_safepoint`'s pointer contract is discharged by the \
                  borrow, which is live for the whole call"
    )]
    // covers: Core\Signal::onShutdown
    #[test]
    fn a_handler_enters_the_existing_drain_rather_than_a_second_state_machine() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let handler = register_on(&mut ctx);
        Drain::process().begin();
        ctx.request_safepoint(SafepointFlags::SHUTDOWN);
        // SAFETY: the pointer is a reborrow of a live local.
        let status = unsafe { nvs_runtime::nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, nvs_runtime::OK, "the drain stopped the request");
        let seen = SEEN.with(Cell::get).expect("the safepoint ran the handler");
        assert!(
            seen.drain_had_begun,
            "the handler ran outside the drain the server crate had already begun"
        );
        assert!(
            !CLASS
                .methods
                .iter()
                .any(|method| method.name.to_ascii_lowercase().contains("drain")),
            "`Core\\Signal` grew a second reading of `Core\\Server::isDraining()`"
        );
        release(handler);
    }

    /// The roster is one verb, and nothing in it is a signal number.
    ///
    /// Job control is the half of `pcntl` the tier roster refuses, so the
    /// failure this guards against is a later member arriving with it: `kill`,
    /// `alarm` or `raise` by name, or the integer any of them would have to
    /// take. A constant would be the same addition written as data, which is why
    /// the class's constant list is asserted empty rather than left to the
    /// member check.
    // covers: Core\Signal::onShutdown
    #[test]
    fn there_is_no_kill_no_alarm_and_no_signal_number_as_an_integer() {
        let names: Vec<&str> = CLASS.methods.iter().map(|method| method.name).collect();
        assert_eq!(
            names,
            vec!["onShutdown"],
            "`Core\\Signal` is graceful shutdown and nothing else"
        );
        assert!(
            CLASS.instance.is_empty() && CLASS.slots.is_empty(),
            "`Core\\Signal` grew an instance to hold a signal on"
        );
        assert!(
            CLASS.constants.is_empty(),
            "`Core\\Signal` grew a constant, which is a signal number as data"
        );
        for method in CLASS.methods {
            for ty in method
                .params
                .iter()
                .chain(std::iter::once(&method.return_ty))
            {
                assert!(
                    !matches!(ty, CoreTy::Int | CoreTy::Uint),
                    "`Core\\Signal::{}` names a signal as an integer",
                    method.name
                );
            }
        }
    }
}
