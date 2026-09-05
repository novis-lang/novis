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
//! connection this process never opened. `Core\Sse` (§ 5) and `Core\Topic`
//! (§ 4) are unregistered for the same reason and are this module's other two
//! known gaps.
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
//! # Why the member answers `void` where the ADR writes `Http\Response`
//!
//! § 2's example ends `return Core\Socket::upgrade(...)`, on the reading that
//! returning the upgrade is what performs it. **Nothing in this tree can read
//! that return.** [ADR 0077](/docs/adr/0077-compile-time-routing.md) § 4 and
//! [ADR 0102](/docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
//! § 1 are one rule — the server matches and stops, and "never decides what a
//! return value means" — so a handler's return value reaches no interpreter,
//! and [`crate::response`] is a class of `void` members writing the response
//! the request already has rather than a value type to hand one back.
//!
//! So calling this **is** performing the upgrade, exactly as
//! `Core\Response::redirect` is, and ADR 0083 § 2's sentence is folded to say
//! so. The alternative — a response value type, interpreted by a dispatcher —
//! is a second response model and a crossing of 0077 § 4's refusal list, which
//! is a larger change than the one sentence it would buy back.
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

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// `Core\Socket`'s registry rows — ADR 0083 § 2's `upgrade`, and so far
/// nothing else. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Socket",
    methods: &[CoreMethod {
        name: "upgrade",
        names: &["entry", "args"],
        // The entry is a *sink* where `spawn script`'s operand is a plain
        // `string`: its content becomes the instruction "execute this file",
        // which is ADR 0088 § 1's definition, and
        // [ADR 0097](/docs/adr/0097-development-server-and-proxied-origin.md)
        // § 2 is the same rule written for the server — a filesystem path is
        // never derived from a URL at request time, and the upgrade is a
        // request-time call. `args` takes no expected type at all, for the
        // reason the sibling site takes none: ADR 0023 § 2's walk decides what
        // may cross, and that is a run-time question for everything a declared
        // type does not already settle.
        params: &[CoreTy::Text(Qual::Sink), CoreTy::Mixed],
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
