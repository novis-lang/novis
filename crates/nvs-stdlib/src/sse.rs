//! `Core\Sse` — [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md)
//! § 5's event stream, as the signature a program writes.
//!
//! § 5 makes an SSE connection "the same model without `receive`": the isolate
//! is § 1's root one — its own arena, its own `[limits]` budget, its own grants
//! — and the operand is § 2's, under
//! `rule:security/isolate-shares-nothing`'s rule. So
//! everything behind the door is [`crate::socket`]'s, shared outright and
//! documented there once: why the member *prepares* an isolate the connection
//! starts, why all three refusals belong to the request, why it answers `void`,
//! and why `limits:`, `grants:` and `on:` are not parameters.
//!
//! # What is different is the door, and it is the whole module
//!
//! **The cell is [`nvs_runtime::SseSlot`] and not
//! [`nvs_runtime::UpgradeSlot`]**, and § 5 states why as a difference between
//! two hand-overs rather than as a preference. A WebSocket upgrade *takes the
//! socket*: the hand-over is the socket itself, it happens after the request's
//! own future has ended, and only a request the server framed an upgrade for
//! has one to give. An SSE connection takes nothing. Its response is an
//! ordinary `200 text/event-stream` that must still be **sent**, and what the
//! isolate writes into is that response's body while the connection future is
//! still running — so there is nothing for the server to frame in advance and
//! nothing to decide before the program has asked.
//!
//! Two consequences, and they are this module's whole surface area:
//!
//! - **Every request a server answers is offered the cell**, where its sibling
//!   is offered only to an upgradable one. That is § 1's fail-closed rule read
//!   against a different hand-over rather than a relaxation of it — what still
//!   has no cell, and so still throws, is everything that is not a served
//!   request: a command-line program, a `spawn script` child, a `#[Test]`
//!   method. The refusal names that, because "no server is answering this" is
//!   what a program has done wrong and "no upgrade was framed" is not.
//! - **The two cells cannot be confused**, because they are two types. A single
//!   slot carrying both would be a slot the connection has to ask the *kind* of
//!   before it could use it, which is a tag standing in for a distinction the
//!   types already make. `nvs_runtime::Inbound`'s module doc is the home of
//!   that reading from the carrier's side.
//!
//! A request that filled both cells is not refused here, and that is deliberate
//! rather than missing: one cell knows nothing of the other, and asking for a
//! socket and an event stream at once is a contradiction about the **response**
//! — refused where the response is written, which is the only place both
//! answers are in hand.
//!
//! # What is not here yet
//!
//! The stream itself. `Core\Sse::current()` and the `send` § 5 gives the
//! isolate are unregistered, and so is the `200 text/event-stream` the door
//! would have to answer with in place of the handler's own response — an event
//! stream prepared today opens a root isolate with nothing wired to a body.
//! The method entry form throws for [`crate::socket`]'s reason, unchanged: a
//! `callable` carries no parameter names and `rule:security/isolate-shares-nothing` binds `args:` by name.

use nvs_runtime::{Fault, ThrownClass, Upgrade, Value, copy_graph};

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc};
use crate::socket::{entry_program, release_crossed, retained};

/// `Core\Sse`'s registry rows — ADR 0083 § 5's `upgrade`, and so far nothing
/// else. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Sse",
    methods: &[CoreMethod {
        name: "upgrade",
        names: &["entry", "args"],
        // The same two marks the sibling row carries, and for the same reasons:
        // [`CoreTy::Entry`] is ADR 0083 § 2's operand rule — a path or a static
        // method written `Feed::run(...)`, never a `callable` in a variable —
        // and `args` takes no expected type because `rule:classes/graph-copy`'s walk decides
        // what may cross at run time. `crate::socket`'s row is where those two
        // are argued; § 5 gives this member "the identical clause", so a
        // difference between the two signatures would be one this ADR does not
        // license.
        params: &[CoreTy::Entry, CoreTy::Mixed],
        defaults: &[Const::Null],
        return_ty: CoreTy::Void,
        symbol: UPGRADE_SYMBOL,
        doc: Some(&UPGRADE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The symbol [`CLASS`]'s `upgrade` row is reached through.
const UPGRADE_SYMBOL: &str = "nvs_core_sse_upgrade";

/// `Core\Sse::upgrade`'s reference card — `rule:core-api/reference-card`.
const UPGRADE_DOC: MethodDoc = MethodDoc {
    short: "Answers this request with an event stream running `$entry` as a root isolate — its \
            own arena, its own budget and its own grants, sharing nothing with the request that \
            opened it but the values `$args` copied in.",
    params: &[
        ParamDoc {
            name: "entry",
            desc: "What the stream runs: a file path, resolved and root-checked exactly as \
                   `spawn script`'s operand is, or a static method written `Feed::run(...)`. \
                   Never a closure — an isolate shares nothing but compiled code, so a capture \
                   would cross the boundary the isolate exists to be.",
            shape: &[],
        },
        ParamDoc {
            name: "args",
            desc: "The values the stream starts with, bound to the entry's parameters by name. \
                   They cross by the graph copy an isolate boundary already uses, so what arrives \
                   is a value and never a shared reference; a `secret` may not cross and a \
                   `tainted` value stays `tainted` on the other side.",
            shape: &[],
        },
    ],
    ret: "Nothing. Calling it opens the stream — this is not a response value a handler hands \
          back, because nothing interprets a handler's return.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "A request no server is answering, which is every command-line program and \
                   every `spawn script` child; an `$entry` path `script.spawn` does not grant or \
                   that does not compile; a second call on one request.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "An `$args` value with no meaning on the other side of an isolate boundary — a \
                   resource, or a `secret` the call site could not see through; and, for a static \
                   method entry, an `$args` map that omits a parameter the method declares or \
                   names one it does not.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::registry::CLASSES`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        UPGRADE_SYMBOL => (nvs_core_sse_upgrade as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse::upgrade(string $entry, mixed $args = null): void` — ADR 0083
    /// § 5's event stream, prepared here and started by the connection.
    ///
    /// The sibling body, with one line different, and that line is the module
    /// doc's subject: the cell asked for is the one every served request has.
    /// The three refusals are [`crate::socket`]'s — no cell, the entry, then
    /// the argument — in the order they cost the least, and each is a fact
    /// about a different thing.
    fn nvs_core_sse_upgrade(ctx, args: [2]) {
        // Cloned rather than borrowed, for the sibling's reason: `entry_program`
        // takes the context, and both halves of the cell are shared handles by
        // construction, so a clone is one refcount and no borrow held across a
        // call.
        let cell = ctx
            .inbound()
            .and_then(nvs_runtime::Inbound::sse_slot)
            .cloned()
            .ok_or_else(|| {
                Fault::thrown(
                    "`Core\\Sse::upgrade` needs a request a server is answering and this is not \
                     one: every request the server runs is offered an event stream, and a \
                     program run from the command line, a `spawn script` child and a `#[Test]` \
                     method are not requests",
                )
            })?;
        let program = entry_program(ctx, args[0], args[1], "Core\\Sse::upgrade")?;
        // No case can reach this, for the sibling's reason: it stands after the
        // cell, and a `.nvst` case runs a script no server is answering. The
        // crossing itself is `crate::socket`'s, and
        // `an_args_value_that_cannot_cross_is_refused_before_the_slot_is_filled`
        // there is the `#[test]` that asserts this arm of it.
        let crossed = copy_graph(retained(args[1])).map_err(|refused| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("`Core\\Sse::upgrade`'s `args:` cannot cross into a connection: {refused}"),
            )
        })?;
        if let Err(returned) = cell.fill(Upgrade::new(program, crossed)) {
            let (_program, crossed) = returned.into_parts();
            release_crossed(crossed);
            // No case can reach this: a second stream needs a first, and a first
            // needs a cell no `.nvst` case is offered.
            // `a_second_event_stream_on_one_request_is_refused_and_the_first_still_stands`
            // is the `#[test]` that asserts it instead.
            return Err(Fault::thrown(
                "`Core\\Sse::upgrade` was called twice on one request, and a request opens at \
                 most one event stream",
            ));
        }
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::script::{Installed, Program, Resolver, install};
    use nvs_runtime::{Ctx, Inbound, NvsArray, NvsStr, SseSlot, Value};

    use super::nvs_core_sse_upgrade;
    use crate::socket::release_crossed;

    /// A resolver answering with the length of the path it was asked for —
    /// `crate::socket`'s test fixture, and its doc is the home of why a length
    /// is what proves a program's identity on this side of the seam.
    #[derive(Debug)]
    struct Fixed;

    impl Resolver for Fixed {
        fn resolve(&self, path: &str) -> Result<Program, String> {
            let len = i64::try_from(path.len()).unwrap_or(-1);
            Ok(Box::new(move |ctx, args| {
                ctx.set_isolate_argument(args);
                Value::int(len)
            }))
        }
    }

    // `install` takes a `&'static dyn Resolver`, so a `static` is the only way
    // to reach it directly.
    static FIXED: Fixed = Fixed;

    fn resolving() -> Installed {
        install(&FIXED)
    }

    /// A context granting `script.spawn` for everything, because `rule:security/capability-check-at-the-door`'s
    /// door is inside `resolve` and a bare context grants nothing — every case
    /// below is about the cell rather than about the grant.
    fn granting() -> Ctx {
        let mut snapshot = nvs_config::Snapshot::default();
        snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
            script: Some(nvs_config::tree::CapScript {
                spawn: Some(nvs_config::tree::Setting::Bool(true)),
            }),
            ..nvs_config::tree::Capabilities::default()
        });
        let mut ctx = Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(snapshot));
        ctx
    }

    /// A carrier for an ordinary request a server is answering — no upgrade
    /// offered, which is the difference § 5 turns on.
    fn served(ctx: &mut Ctx, cell: &SseSlot) {
        let mut inbound = Inbound::new("GET", "/events", "");
        inbound.offer_sse(cell.clone());
        ctx.set_inbound(inbound);
    }

    /// `{room: 7}`, as the one refcounted argument a crossing can be observed
    /// on: an `int` would cross as itself and prove nothing.
    fn a_room() -> Value {
        let mut map = NvsArray::new();
        map.set(NvsStr::new(b"room"), Value::int(7));
        Value::array(map)
    }

    /// § 5's hand-over, asserted as the sibling's is: after the call the cell
    /// holds the resolver's program for the path that was *written*, and the
    /// argument beside it is a **copy** rather than the request's own graph.
    ///
    /// What this adds over the sibling's assertion is the carrier it runs on —
    /// a plain `GET` no upgrade was framed for, which is the request § 5 says
    /// is offered a cell and § 1 says is not offered a slot.
    #[test]
    fn an_event_stream_leaves_the_resolvers_program_and_a_copy_of_its_argument_in_the_cell() {
        let _resolver = resolving();
        let mut ctx = granting();
        let cell = SseSlot::new();
        served(&mut ctx, &cell);

        let path = Value::str(NvsStr::new(b"streams/feed.nvs"));
        let mine = a_room();
        nvs_runtime::call(nvs_core_sse_upgrade, &mut ctx, &[path, mine])
            .expect("an offered cell takes the stream");

        let (program, crossed) = cell
            .take()
            .expect("the member filled the cell the server left")
            .into_parts();
        assert_ne!(
            crossed.array_ptr(),
            mine.array_ptr(),
            "the connection was handed the request's own array rather than a copy of it"
        );

        // Run it the way the connection does — on a context of its own, with
        // the crossed argument transferred into it — so the program is proved
        // to be the resolver's answer for the written path and the crossed
        // reference lands in an ownership root that will release it.
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(16),
            "the cell holds a program for some other path"
        );
        release_crossed(mine);
    }

    /// `nvs_runtime::SseSlot::fill`'s refusal reaching a program: the second
    /// call is told so and the first stream is what the connection still
    /// starts. Unreachable from a `.nvst` case — a second stream needs a first,
    /// and a first needs a cell nothing offers a command-line script.
    #[test]
    fn a_second_event_stream_on_one_request_is_refused_and_the_first_still_stands() {
        let _resolver = resolving();
        let mut ctx = granting();
        let cell = SseSlot::new();
        served(&mut ctx, &cell);

        let first = Value::str(NvsStr::new(b"streams/feed.nvs"));
        nvs_runtime::call(nvs_core_sse_upgrade, &mut ctx, &[first, Value::null()])
            .expect("an offered cell takes the first stream");

        let second = Value::str(NvsStr::new(b"streams/other-feed.nvs"));
        nvs_runtime::call(nvs_core_sse_upgrade, &mut ctx, &[second, Value::null()])
            .expect_err("one request opens at most one event stream");

        // The first one is intact rather than displaced, which is the half a
        // refusal that overwrote would still pass without.
        let (program, crossed) = cell.take().expect("the first fill stands").into_parts();
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(16),
            "the second call displaced the first stream's program"
        );
    }
}
