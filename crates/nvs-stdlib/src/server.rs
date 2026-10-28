//! `Core\Server` — the class an application asks about the server it is running
//! under. `$_SERVER` is not populated
//! (`rule:statements/no-host-populated-variables`), and what it carried lives on
//! `Core\Request` and `Core\Env`, so this class has no request environment.
//!
//! # What is here, and what is not
//!
//! Two members. `isDraining()` is
//! `rule:http-server/the-server-block-is-boot-class`
//! 's last sentence — the same fact `[server] health_path` answers a proxy
//! with, given to an application for an endpoint of its own. `traceId()` is
//! `rule:observability/a-trace-id-exists-for-every-request`'s id, rendered by
//! [`nvs_runtime::TraceContext::trace_id_hex`] so it is the same thirty-two
//! characters a `[log]` record and a `traceparent` carry. The context already
//! holds the trace for the logger, so the member spends no per-request memory;
//! each call allocates its string. The HTTP version and the server's and the
//! client's ports have no member anywhere: no rule
//! or example needs them, and one is added when a program does.
//!
//! # Why the bit is not read from `nvs-server`
//!
//! It could not be: this crate does not depend on that one, and should not —
//! a `Core` member that needed the HTTP server linked in to answer a question
//! about the process would put `hyper` in the graph of every CLI program.
//! [`nvs_runtime::drain`] is the crate both rest on, and it owns the atomic,
//! its ordering, and why a real server takes the process's bit. `nvs_server`'s
//! accept loop is the only writer; everything here is a reader.
//!
//! **A program that is not being served reads `false` from `isDraining()`**,
//! and that is the answer rather than an error: a CLI program, a scheduled
//! script and a test all run in a process no accept loop is draining, and
//! asking whether a shutdown has begun is a question they may ask and be told
//! no. **`traceId()` throws there instead**, because the id belongs to a
//! request (`rule:security/request-state-throws-in-an-isolate`).

use nvs_runtime::{NvsStr, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc};

/// `Core\Server`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Tells a program about the server it runs under. `isDraining()` returns `true` when the \
            server has started to shut down. `traceId()` returns the id of the current request.",
};

/// `Core\Server`'s registry rows: `isDraining` and `traceId`.
///
/// The third request-facing class that needs no request — so far. What is registered is
/// `rule:http-server/the-server-block-is-boot-class`'s `isDraining`, which asks the same kind of question those
/// two do about the *process* rather than about a request; the rest of
/// `Core\Server` is the request's own environment and waits on a served
/// request carrying it. [`crate::server`]'s module doc owns that gap, and
/// why the bit it reads lives in `nvs-runtime`.
// rank: 670
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Server",
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "isDraining",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_server_is_draining",
            doc: Some(&IS_DRAINING_DOC),
        },
        CoreMethod {
            name: "traceId",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_server_trace_id",
            doc: Some(&TRACE_ID_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Server::isDraining`'s reference card — `rule:core-api/reference-card`.
const IS_DRAINING_DOC: MethodDoc = MethodDoc {
    short: "Returns `true` when the server has started to shut down. From then on, the server \
            accepts no new connections and finishes the requests it already has.",
    params: &[],
    ret: "`true` after the server stopped accepting new connections. `false` while it still \
          accepts them. A program that no server runs, such as a command-line program, always \
          gets `false`.",
    errors: &[],
};

/// `Core\Server::traceId`'s reference card — `rule:core-api/reference-card`.
const TRACE_ID_DOC: MethodDoc = MethodDoc {
    short: "Returns the id of the current request. Every request has one. The server writes the \
            same id into its log lines and into the `traceresponse` header of the response.",
    params: &[],
    ret: "32 lower-case hexadecimal characters. If the request came with a `traceparent` header, \
          the id is the one in that header.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The program is not handling a request. This is the case in a command-line program \
               or a test.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_server_is_draining" => (nvs_core_server_is_draining as *const ()).cast(),
        "nvs_core_server_trace_id" => (nvs_core_server_trace_id as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Server::isDraining(): bool` — `rule:http-server/the-server-block-is-boot-class`'s drain, read by the
    /// application rather than by the built-in probe.
    ///
    /// One relaxed load of the process's bit, and deliberately not a snapshot
    /// taken at the start of the request: a shutdown that begins mid-request is
    /// exactly what this exists to report, so an answer cached for the length of
    /// a request would be stale in the only window anyone asks in.
    fn nvs_core_server_is_draining(_ctx, _args: [0]) {
        Ok(Value::bool(nvs_runtime::drain::is_draining()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Server::traceId(): string` — the request's trace id, and this
    /// class's only request identifier.
    ///
    /// Read off the context rather than the carrier: [`nvs_runtime::Ctx::set_inbound`]
    /// copies a continued trace onto the context, and the context's copy is
    /// the one the logger stamps, so the two cannot disagree. Asked of the
    /// carrier only to throw where no request arrived, because the context
    /// draws a root trace for every run and would otherwise answer one.
    ///
    /// The hex is written on the stack, so the returned string is the call's
    /// one allocation. The digits are [`nvs_runtime::TraceContext::trace_id_hex`]'s,
    /// which a unit test below holds it to.
    fn nvs_core_server_trace_id(ctx, _args: [0]) {
        crate::request::served(ctx, r"Core\Server::traceId")?;
        #[expect(clippy::disallowed_methods, reason = "hex digits are ASCII")]
        let id = Value::str(NvsStr::new(&hex_id(ctx.trace_context().trace_id())));
        Ok(id)
    }
}

/// `id` as 32 lower-case hex digits.
fn hex_id(id: [u8; 16]) -> [u8; 32] {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = [0u8; 32];
    for (pair, byte) in out.as_chunks_mut::<2>().0.iter_mut().zip(id) {
        pair[0] = DIGITS[usize::from(byte >> 4)];
        pair[1] = DIGITS[usize::from(byte & 0x0f)];
    }
    out
}

#[cfg(test)]
mod tests {
    use nvs_runtime::Ctx;
    use nvs_runtime::drain::Drain;

    /// Calls the member the way a program does and answers its `bool`.
    fn is_draining(ctx: &mut Ctx) -> bool {
        nvs_runtime::call(super::nvs_core_server_is_draining, ctx, &[])
            .expect("`isDraining` never throws")
            .as_bool()
            .expect("`isDraining` returns a `bool`")
    }

    /// The member agrees with the process's bit before a drain, and answers
    /// `true` from the moment the process drain begins.
    ///
    /// Beginning the process drain cannot be undone, and `crate::signal`'s
    /// tests begin it on another thread of this test binary at any moment. So
    /// the first reading is taken between two readings of the process's bit,
    /// and it lies between them: the bit only ever goes from `false` to `true`.
    // covers: Core\Server::isDraining
    #[test]
    fn the_member_answers_true_once_the_process_drain_begins() {
        let mut ctx = Ctx::buffered();
        let before = nvs_runtime::drain::is_draining();
        let member = is_draining(&mut ctx);
        let after = nvs_runtime::drain::is_draining();
        assert!(
            before <= member && member <= after,
            "the member read {member} between the process's bit reading {before} and then {after}"
        );
        Drain::process().begin();
        assert!(
            is_draining(&mut ctx),
            "the member answered `false` inside a process drain"
        );
        assert!(
            is_draining(&mut ctx),
            "a second reading inside the drain answered `false`"
        );
    }

    /// The member reads the *process's* bit and not one of its own.
    ///
    /// Asserted through [`Drain::process`] rather than by calling the helper
    /// with a `Ctx`: what could go wrong here is the member reading a detached
    /// handle, and that is a question about which bit, not about the ABI.
    ///
    /// The test begins the process drain itself and reads `true`. Comparing
    /// two readings races the test above, which may begin the drain between
    /// them, and the bit only ever goes from `false` to `true`.
    #[test]
    fn the_member_reads_the_processs_own_bit() {
        Drain::process().begin();
        assert!(
            nvs_runtime::drain::is_draining(),
            "`isDraining` answered from a bit no server writes"
        );
    }

    /// `traceId` over `ctx`: the string it returns, or `None` where it threw.
    fn trace_id(ctx: &mut Ctx) -> Option<String> {
        let value = nvs_runtime::call(super::nvs_core_server_trace_id, ctx, &[]).ok()?;
        let text = String::from_utf8(
            value
                .as_str_bytes()
                .expect("`traceId` returns a string")
                .to_vec(),
        )
        .expect("a trace id is ASCII");
        #[expect(
            unsafe_code,
            reason = "the test owns the returned reference and is done with it"
        )]
        unsafe {
            value.release();
        }
        Some(text)
    }

    /// A served request reads the id its `traceparent` carried, as the
    /// lower-case hex the header wrote it in.
    // covers: Core\Server::traceId
    #[test]
    fn the_member_returns_the_trace_id_the_request_carried() {
        let header = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
        inbound.set_trace_context(nvs_runtime::TraceContext::continuing(Some(header), 1.0));
        let mut ctx = Ctx::buffered();
        ctx.set_inbound(inbound);
        assert_eq!(
            trace_id(&mut ctx).as_deref(),
            Some("4bf92f3577b34da6a3ce929d0e0e4736")
        );
        assert_eq!(
            trace_id(&mut ctx),
            Some(ctx.trace_context().trace_id_hex()),
            "a second reading parted from the context's own id"
        );
    }

    /// The stack rendering is the same text the logger and a `traceparent` write.
    #[test]
    fn the_stack_rendering_matches_the_trace_contexts_own() {
        let trace = nvs_runtime::TraceContext::continuing(None, 1.0);
        assert_eq!(
            super::hex_id(trace.trace_id()).as_slice(),
            trace.trace_id_hex().as_bytes()
        );
    }

    /// A request that carried no trace still has an id: the root the context
    /// drew, 32 hex characters wide.
    #[test]
    fn a_request_with_no_traceparent_still_has_an_id() {
        let mut ctx = Ctx::buffered();
        ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        let id = trace_id(&mut ctx).expect("a served request has an id");
        assert_eq!(id.len(), 32);
        assert!(
            id.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
    }

    /// With no request the member throws rather than answering the run's root.
    #[test]
    fn the_member_throws_where_no_request_arrived() {
        let mut ctx = Ctx::buffered();
        assert_eq!(trace_id(&mut ctx), None);
        assert!(ctx.take_pending().is_some(), "the refusal left no message");
    }
}
