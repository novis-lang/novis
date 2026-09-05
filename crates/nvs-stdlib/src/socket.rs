//! `Core\Socket` — [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md)
//! §§ 1-2's WebSocket upgrade, as the signature a program writes.
//!
//! § 1 makes a connection a **root isolate** rather than a suspended request:
//! its own arena, its own `[limits]` budget, its own grants, and none of the
//! upgrading request's heap. § 2 makes opening one `spawn script`-shaped, so
//! the operand here is that construct's operand under exactly
//! [ADR 0006](/docs/adr/0006-isolated-script-execution.md)'s rule — a path, or
//! a static method written `Chat::run(...)`, and never a closure.
//!
//! # What is here, and what is not
//!
//! The row and its signature. **The body throws**: § 1's hand-off — creating
//! the root isolate, moving the socket into it, and framing RFC 6455 over it —
//! is not built, and a member that answered plausibly without it would report a
//! connection this process never opened. What that body becomes instead is
//! decided below, under *this member spawns nothing*. `Core\Sse` (§ 5) and
//! `Core\Topic` (§ 4) are unregistered for the same reason and are this
//! module's other two known gaps.
//!
//! There is no [`crate::registry::CAPABILITIES`] row, and that is a statement
//! about the body rather than about the member: what throws reaches no
//! spelling that performs an effect, which is what
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` holds mechanically. The
//! slice that opens the connection is the one that owes the declaration — for
//! reading the entry file, at least — and a class becomes a door the moment
//! any one of its members is declared, so the row cannot be written ahead of
//! what it would be describing.
//!
//! The `entry` parameter is a [`CoreTy::Entry`](crate::registry::CoreTy::Entry)
//! and that is the whole of how § 2's operand rule reaches a call site: the
//! mark is what `nvs_types::expr::isolate` finds, so `Chat::run(...)` is
//! accepted here and a `callable` in a variable is refused with the same
//! `E0802` a `spawn script` reports. Its variant doc is the home of why the
//! rule cannot be a parameter type.
//!
//! # Decision: this member spawns nothing, and the connection starts it
//!
//! § 1's root isolate is not started here, and it cannot be. A `Core` member
//! runs on the request isolate's own task holding the request's
//! `nvs_runtime::Ctx`, so every route out of this body reaches
//! `nvs_runtime::host::Host::start_isolate` with *that* context — which is
//! `Ctx::isolate`'s tree: the child's memory is charged to the request, its
//! deadline is the same word the request's `wall_time` expires, and it is a
//! task under the request's, so the request cannot return while it runs
//! ([ADR 0072](/docs/adr/0072-core-task-structured-concurrency.md) § 4). Each
//! of those is the opposite of what § 1 states, and none of them is a builder
//! away.
//!
//! So the member **prepares** an isolate and records it; the **connection**
//! starts it. What is prepared is what `nvs_host::Isolate::new` takes — a
//! `nvs_runtime::script::Program` and the argument value that has already
//! crossed — and it is prepared here, inside the request, because all three
//! things that can refuse belong to the request:
//!
//! - **The capability.** ADR 0006's `script.spawn` grant, and the root check
//!   under it, are asked against the request's own configuration overlay, so a
//!   request that narrowed its grants cannot upgrade into a connection holding
//!   the ones it gave up — § 1's "narrowed from the request's, never widened".
//! - **The argument.** ADR 0023 § 2's refusal is the *parent's* fault, and
//!   `nvs_host`'s `isolate` module doc owns that asymmetry; here is the one
//!   point at which a `secret` passed to a socket is still a throw the program
//!   can catch rather than a connection that closes after its `101`.
//! - **The code.** A path is resolved through `nvs_runtime::script`; a static
//!   method is code the request's *own* unit already holds, and the request's
//!   context is the only place that unit's statics recipes and class table can
//!   be taken from.
//!
//! Both of § 2's entry forms therefore collapse to one prepared `Program`, and
//! that is what makes them one isolate at the far end rather than two shapes to
//! keep in step. A path is the resolver's program unchanged. A method arrives
//! as a **first-class callable value** — the entry interns as `mixed`, so
//! `Chat::run(...)` reaches this body as the closure `nvs_ir`'s `lower_callable`
//! built — so its program is a closure over that value, retained here, beside
//! the statics recipes and the class table the request's context is holding;
//! it arms the child itself, exactly as a path entry's `install_in` does.
//! `nvs_host::Isolate`'s own method entry is *not* what a connection uses, and
//! that is the same fact from the other end: that builder re-materializes the
//! recipes off the **spawning** context, which for a connection is a context
//! that never ran the unit.
//!
//! **The preparation rides on `nvs_runtime::Inbound`**, the request carrier,
//! and that answers two questions at once. Both halves of that are on disk —
//! `nvs_runtime::Upgrade` is the prepared pair and `nvs_runtime::UpgradeSlot`
//! the cell it is left in, offered by `nvs_server::serve_connection` to a
//! request `hyper` framed an upgrade for and to no other — so what the body
//! below still owes is the filling: resolving the entry into one `Program` and
//! crossing `args`. A connection is the only thing an
//! upgrade can happen to, so a request that did not arrive on one — a CLI
//! program, a `spawn script` child, a request the server could offer no
//! upgrade for — has no slot to write into and this member throws, for the
//! reason `Core\Request::method()` throws there
//! ([ADR 0012](/docs/adr/0012-no-superglobals.md) § 7). And the server holds
//! the other half of that slot, so § 1's ordering is what the code can express
//! rather than what it must remember: the request is joined, its context is
//! dropped and its arena with it, and only then is there a caller left holding
//! the program.
//!
//! What the connection starts it over is its **own** context — the one
//! `nvs-server`'s `serve` module already starts the request isolate from. The
//! connection isolate is that request's *sibling* rather than its child, which
//! is where its own budget, its own deadline and its own `spawn script` depth
//! come from.
//!
//! **What it spends:** the argument graph is copied twice per upgrade — once
//! here, and once by the spawn at the far end — because it crosses two
//! boundaries and the refusal has to land on this side of the first one. That
//! is one extra copy of a value an application chose to hand a connection, once
//! per connection opened, and it buys a throw the program can still catch. The
//! first copy is allocated before the connection's context takes its zero point
//! and released after, so that context reads its own share a little low — the
//! balance is a signed `Ctx::memory_base` for exactly this reason, and the
//! error is bounded by the size of one `args` graph.
//!
//! Two alternatives were refused. **The request isolate becoming the
//! connection** keeps the arena § 1 says is released and hands the connection
//! the request's session, headers and statics. **Riding out on the isolate's
//! completion** instead of on the carrier makes a value boundary carry a
//! connection, and gives a `spawn script` child a completion its parent would
//! then have to forward.
//!
//! # Why the member answers `void`
//!
//! Because calling it performs the upgrade — ADR 0083 § 2's own bullet, which
//! is the home of the reasoning and of what the alternative would cost. The
//! short of it: nothing in this language reads a handler's return, so
//! [`crate::response`] is a class of `void` members writing the response the
//! request already has, and this is one more of them.
//!
//! # Why `limits:`, `grants:` and `on:` are not parameters
//!
//! § 2 lists four options and calls them "0006's, spelled as ordinary named
//! arguments". Three of the four are **refused** at the sibling site:
//! `nvs_types::expr::isolate` reports `E_SPAWN_OPTION_UNSUPPORTED` for
//! `spawn script`'s `limits:`, `grants:` and `on:`, because a `grants:`
//! narrowing that were silently dropped would hand the child the parent's
//! authority. Declaring them here would be that same drop with a different
//! spelling — the row would accept them and the body would ignore them — so
//! they are simply not declared, and a program writing `grants: {}` gets the
//! unknown-argument refusal rather than a silent widening. They arrive here
//! when they are enforced there; that refusal is the rule's one home and this
//! module does not restate it.
//!
//! `output:` is the fifth and does not apply: an isolate's `output:` chooses
//! between its own buffer and the parent's stream
//! ([ADR 0088](/docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 3), and a connection's output is the socket.

use nvs_runtime::Fault;

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc};

/// `Core\Socket`'s registry rows — ADR 0083 § 2's `upgrade`, and so far
/// nothing else. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Socket",
    methods: &[CoreMethod {
        name: "upgrade",
        names: &["entry", "args"],
        // [`CoreTy::Entry`] is § 2's "0006's operand", as the one mark that
        // carries that ADR's whole rule to a call site: a path or a static
        // method written `Chat::run(...)`, and never a `callable` in a
        // variable. It classifies as a sink for the reason a path always does
        // — its content becomes the instruction "execute this file", ADR 0088
        // § 1's definition, and
        // [ADR 0097](/docs/adr/0097-development-server-and-proxied-origin.md)
        // § 2 is the same rule written for the server, a filesystem path never
        // derived from a URL at request time. `args` takes no expected type at
        // all, for the reason the sibling site takes none: ADR 0023 § 2's walk
        // decides what may cross, and that is a run-time question for
        // everything a declared type does not already settle.
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
const UPGRADE_SYMBOL: &str = "nvs_core_socket_upgrade";

/// `Core\Socket::upgrade`'s reference card — ADR 0117.
const UPGRADE_DOC: MethodDoc = MethodDoc {
    short: "Turns this request into a WebSocket connection running `$entry` as a root isolate — \
            its own arena, its own budget and its own grants, sharing nothing with the request \
            that opened it but the values `$args` copied in.",
    params: &[
        ParamDoc {
            name: "entry",
            desc: "What the connection runs: a file path, resolved and root-checked exactly as \
                   `spawn script`'s operand is, or a static method written `Chat::run(...)`. \
                   Never a closure — an isolate shares nothing but compiled code, so a capture \
                   would cross the boundary the isolate exists to be.",
            shape: &[],
        },
        ParamDoc {
            name: "args",
            desc: "The values the connection starts with, bound to the entry's parameters by \
                   name. They cross by the graph copy an isolate boundary already uses, so what \
                   arrives is a value and never a shared reference; a `secret` may not cross and \
                   a `tainted` value stays `tainted` on the other side.",
            shape: &[],
        },
    ],
    ret: "Nothing. Calling it performs the upgrade — this is not a response value a handler \
          hands back, because nothing interprets a handler's return.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "Always, so far: the root isolate a connection runs in is not built, and this \
               member reports that rather than answering as though a peer were attached.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        UPGRADE_SYMBOL => (nvs_core_socket_upgrade as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket::upgrade(string $entry, mixed $args = null): void` — ADR
    /// 0083 § 2's upgrade, as far as it is built.
    ///
    /// A throw and not an abort, unlike [`crate::program`]'s unreachable body:
    /// this member is genuinely callable and a program reaching it has written
    /// something the compiler was right to accept, so the ladder's ordinary
    /// rung is the honest report. The module doc owns what is missing behind
    /// it.
    fn nvs_core_socket_upgrade(_ctx, _args: [2]) {
        Err(Fault::thrown(
            "`Core\\Socket::upgrade` cannot open a connection yet: the root isolate a \
             WebSocket connection runs in is not built, so there is nothing to hand this \
             connection to",
        ))
    }
}
