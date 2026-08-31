//! `Core\Cache` — [ADR 0059](../../../../docs/adr/0059-cross-request-state-is-explicit.md)'s
//! sanctioned exception to ADR 0052 § 3's closed door on cross-request state,
//! as two members that hand back a store and the two operations on one.
//!
//! § 1's two tiers are two members rather than one API with a flag, so the
//! choice a program made is visible in review rather than in an argument list.
//! Neither takes a parameter at all, which is what makes that structural: there
//! is no spelling of `local()` that can be turned into `shared()` by a value
//! computed at run time.
//!
//! # Decision: an entry is § 3's byte payload, not a live graph
//!
//! § 2 fixes the *operation* — the recursive graph copy
//! [ADR 0023](../../../../docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)
//! already defines, not a third mechanism — and leaves which of its two
//! carriers open. This tier uses the **byte** carrier
//! ([`nvs_runtime::encode`]/[`nvs_runtime::decode`]), which is the same walk
//! the live one is, for two reasons the live carrier cannot answer:
//!
//! 1. **A copied object holds a raw `ClassDesc` pointer**, and this store
//!    outlives the request that filled it — so under
//!    [ADR 0017](../../../../docs/adr/0017-hot-reload-without-restart.md)'s
//!    unit swap a live entry would name a descriptor the old unit owned. A
//!    payload names its classes by *name*, resolved against the receiving
//!    program's own table on the way out, exactly as
//!    [`crate::serialize`]'s `decode` does; a class the new unit no longer has
//!    is refused by name rather than dereferenced.
//! 2. **The shared tier needs bytes anyway.** A Redis entry is bytes over a
//!    socket, so a live-graph local tier would be a second representation to
//!    keep in agreement with the first — the two-writers shape this project
//!    treats as the bug rather than the redundancy.
//!
//! What it costs is one encode per `put` and one decode per `get`, both
//! O(graph) exactly as the live copy is, on a path that is already a copy by
//! § 2's own decision. That is priority 3 spent on priority 1, which is the
//! direction AGENTS.md's ordering allows.
//!
//! # Decision: the local tier needs no grant, and § 1's open question is closed
//!
//! [ADR 0112](../../../../docs/adr/0112-authority-is-keyed-on-the-enclosing-namespace.md)
//! § 8 listed `Core\Cache::local()` as the one capability-bearing member with no
//! grant named for it, and left the naming to ADR 0059. The answer written into
//! that ADR's § 1 is that there is **no grant**, because
//! [ADR 0118](../../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md)
//! § 1 checks a capability at the door to an *effect* and this tier has no
//! door: nothing leaves the process, no name is resolved and no file is opened.
//! What is left to bound is footprint, and § 3's `nvs.toml` cap is the
//! instrument for a bound — a boolean grant would not be one. The shared tier
//! has the `net.connect` row instead, in [`crate::registry::CAPABILITIES`]
//! beside it, and that asymmetry is the whole of what the two rows say.
//!
//! # Decision: `shared()` is the door, and the two operations are behind it
//!
//! [`nvs_core_cache_shared`] is where the grant is asked for and where the
//! address is pinned; `put` and `get` on the store it answers ask nothing. That
//! is [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md) § 4's own
//! shape — `Core\Http::allowUrl` is the launderer and `Core\Http\Client` the
//! thing that talks — and it is what makes the address a *pin*: a check at the
//! operation instead would leave a window in which a second resolution answers
//! differently, and re-resolving per command would be that window per command.
//! So [`STORE`] carries no capability row of its own, exactly as
//! `Core\Http\Client` carries none.
//!
//! The door is [`open_configured`] rather than the member, because it has a
//! second caller: `Core\RateLimit::consume` limits over the same store — a
//! deployment has one — and ADR 0075 §§ 1 and 5 both write that member standing
//! alone, so it opens the store itself instead of requiring `shared()` to have
//! been called first. Both callers ask for the same grant at the same host and
//! reuse the same per-core socket; what differs is the sentence each appends to
//! the refusal, since what to do instead is the caller's own contract.
//!
//! Which store, and how long a command may take, are `[cache.shared] url` and
//! `[cache.shared] timeout` — both `System`-class, because where a fleet's
//! coherent state lives is not a decision a request may make for itself.
//!
//! # What is not here yet
//!
//! **A TTL, an eviction and a `forget`.** § 3's cap is what evicts, and a
//! lifetime is meaningless before something enforces one. `put` grows the
//! trailing options shape ADR 0063 R2 puts last when that lands, which is an
//! addition to the row rather than a change to it.
//!
//! **A shared store behind a password, a database index or TLS.** The URL this
//! reads is `redis://host[:port]` and nothing else, and each of the three is
//! refused with a sentence rather than half-served: `AUTH` needs ADR 0103 § 7's
//! secret plumbing to carry the credential, a database index is a second
//! namespace nothing yet names, and a `rediss://` client needs the trust-anchor
//! decision `crate::http::transport` is also waiting on.
//!
//! **What it spends:** on the local tier, per core, one map entry per live key —
//! the key's bytes plus its payload's — held until it is overwritten and charged
//! to the core rather than to any request, which is § 3's O(cores × working set)
//! and deliberately not O(requests served). On the shared tier, one socket per
//! core and nothing per entry: the bytes are the store's.

use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use nvs_runtime::{Ctx, Fault, NvsStr, ThrownClass, Value};
use nvs_syntax::duration;

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

pub(crate) mod redis;

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Cache";

/// [`STORE`]'s name — see [`NAME`].
pub(crate) const STORE_NAME: &str = r"Core\Cache\Store";

/// § 1's two tiers, as the two members that hand one back.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "local",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(STORE_NAME),
            symbol: "nvs_core_cache_local",
            doc: Some(&LOCAL_DOC),
        },
        CoreMethod {
            name: "shared",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(STORE_NAME),
            symbol: "nvs_core_cache_shared",
            doc: Some(&SHARED_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Cache::local`'s reference card — ADR 0117.
const LOCAL_DOC: MethodDoc = MethodDoc {
    short: "The per-core, in-process tier: one store per core, with no coherence between cores and \
            no network behind it.",
    params: &[],
    ret: "A `Core\\Cache\\Store` over this core's own entries. Any entry may be absent at any time, \
          for any reason, and a write on one core is not visible on another — a program that would \
          be incorrect if a `get` answered `null` wants `shared` instead.",
    errors: &[],
};

/// `Core\Cache::shared`'s reference card — ADR 0117.
const SHARED_DOC: MethodDoc = MethodDoc {
    short: "The coherent tier: a real store over the network, shared by every core and every \
            machine that names it.",
    params: &[],
    ret: "A `Core\\Cache\\Store` over the configured shared store, whose entries every core sees.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "No `[cache.shared] url` is configured; or the capability `net.connect` is not \
                   granted for that host, or the address it resolves to is one the outbound \
                   policy denies. Each is a deployment that was not configured rather than a \
                   store that failed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The configured store cannot be reached — it throws rather than answering as \
                   though the entry were absent, since the two mean opposite things to whatever \
                   asked.",
        },
    ],
};

/// § 1's store, as the two operations § 2 defines over it.
///
/// One class for both tiers rather than two, because a tier is a *destination*
/// and not a different operation: `put` copies out and `get` copies in
/// whichever end the copy is going to. Which tier this one is, is the `tier`
/// slot, written by the member that produced it and readable by no program —
/// asking a store what it is would be the flag § 1 refuses, arriving one call
/// later.
pub(crate) const STORE: CoreClass = CoreClass {
    name: STORE_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "put",
            names: &["key", "value"],
            // The key is `Qual::Neutral` and the value is unclassified, and the
            // difference is ADR 0088 § 2's. Not a byte of the key reaches any
            // answer, so a `tainted` one is admitted — a cache key derived from
            // a request is ordinary, and the key is data rather than an
            // instruction (§ 7). The *value* is refused, because `mixed` has
            // nowhere to carry the qualifier back out of `get` and admitting one
            // would launder it (`nvs_types`' `admits_tainted_argument`).
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_cache_put",
            doc: Some(&PUT_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["key"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // `mixed` rather than ADR 0063 R7's `?T`: what went in is any value
            // the copy admits, so there is no `T` to make nullable, and `mixed`
            // already spells the absent answer.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_cache_get",
            doc: Some(&GET_DOC),
        },
    ],
    slots: &["tier"],
    constants: &[],
};

/// [`STORE`]'s tier slot, by index — the layout its `slots` names.
const TIER_SLOT: usize = 0;

/// What [`nvs_core_cache_local`] writes into [`TIER_SLOT`].
const LOCAL_TIER: &str = "local";

/// What [`nvs_core_cache_shared`] writes into [`TIER_SLOT`].
const SHARED_TIER: &str = "shared";

/// `[cache.shared] url` — which store the coherent tier is, as
/// `redis://host[:port]`. Absent, there is no shared tier and `shared()` says so.
const URL: &str = "cache.shared.url";

/// `[cache.shared] timeout` — the bound on a handshake and on a command, each.
const TIMEOUT: &str = "cache.shared.timeout";

/// The bound a deployment that configured none inherits.
///
/// A shared `get` is on the request path, so this is a latency question and not
/// a patience one: a store that has not answered in five seconds is a store the
/// request should be told about rather than one it should keep waiting for. It
/// is deliberately not "unbounded unless configured", which
/// [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 5
/// refuses to give any outbound wait a spelling for.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// `Core\Cache\Store::put`'s reference card — ADR 0117.
const PUT_DOC: MethodDoc = MethodDoc {
    short: "Copies `$value` into the store under `$key`, replacing whatever was there — a recursive \
            graph copy, so the entry shares nothing with the request that wrote it.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The name to store under; `tainted` is admitted, since no byte of it reaches any \
                   answer.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value to copy in. A closure and a `secret` may not cross, exactly as at the \
                   isolate boundary.",
            shape: &[],
        },
    ],
    ret: "Nothing. A successful `put` is still no promise that a later `get` answers — see \
          `Core\\Cache::local`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The value cannot cross: it is or holds a closure, or an object with a `secret` \
                   property that was not revealed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "On the shared tier only: the store cannot be reached or refused the write. \
                   The local tier has nothing to be unreachable.",
        },
    ],
};

/// `Core\Cache\Store::get`'s reference card — ADR 0117.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Copies the entry stored under `$key` back into this request, or answers `null` when \
            there is none.",
    params: &[ParamDoc {
        name: "key",
        desc: "The name to read; `tainted` is admitted, as it is on `put`.",
        shape: &[],
    }],
    ret: "The value as it was copied in, or `null` — an entry may be absent at any time, for any \
          reason, and on the local tier that is the contract rather than a failure.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "The entry names a class this program cannot resolve — the same refusal \
                   `Core\\Serialize::decode` makes, and the ordinary consequence of a deployment \
                   whose classes changed under a store that outlives them.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "On the shared tier only: the store cannot be reached. An entry that is simply \
                   not there is `null` on either tier, which is the difference between a miss and \
                   a failure.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_cache_local" => (nvs_core_cache_local as *const ()).cast(),
        "nvs_core_cache_shared" => (nvs_core_cache_shared as *const ()).cast(),
        "nvs_core_cache_put" => (nvs_core_cache_put as *const ()).cast(),
        "nvs_core_cache_get" => (nvs_core_cache_get as *const ()).cast(),
        _ => return None,
    })
}

/// This core's entries, by key — a name of its own because the nesting is what
/// `clippy::type_complexity` counts, and the two runs of bytes are a key and
/// the payload stored under it.
type Entries = HashMap<Box<[u8]>, Box<[u8]>>;

thread_local! {
    /// The local tier itself: this core's entries, each an ADR 0023 § 3
    /// payload, and nothing shared with any other core.
    ///
    /// A `thread_local` rather than anything reachable from another thread is
    /// § 1's "no coherence between cores" as a *representation* rather than as
    /// a rule to remember — the runtime is thread-per-core, so a store another
    /// core could reach would need a lock this tier is defined not to have.
    static ENTRIES: RefCell<Entries> = RefCell::new(HashMap::new());
}

/// Writes `payload` under `key` on this core, replacing any entry there.
fn store_put(key: &[u8], payload: Vec<u8>) {
    ENTRIES.with_borrow_mut(|entries| {
        entries.insert(key.into(), payload.into_boxed_slice());
    });
}

/// This core's payload for `key`, or `None` — which is an ordinary answer and
/// not a failure, per § 1.
fn store_get(key: &[u8]) -> Option<Vec<u8>> {
    ENTRIES.with_borrow(|entries| entries.get(key).map(|payload| payload.to_vec()))
}

/// The `string` in argument slot `at`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: the row declares a `string` there, so
/// another tag is compiled-code's bug rather than anything a program can write
/// — `E0401` refuses the call first.
fn key_of<'a>(args: &'a [Value], at: usize, member: &str) -> Result<&'a str, Fault> {
    args[at].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{STORE_NAME}::{member} expected a `string` key, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// Which tier a store is — [`TIER_SLOT`] read back, as the one choice every
/// operation on it makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    /// This core's own entries, in process.
    Local,
    /// The configured store, over the connection [`nvs_core_cache_shared`]
    /// opened for this core.
    Shared,
}

/// The tier of the store the call was made on.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding neither word: the two members that
/// build a store write one of the two and nothing else can build one, so a third
/// value is compiled code's bug rather than anything a program can write.
fn tier_of(args: &[Value], member: &str) -> Result<Tier, Fault> {
    let receiver = crate::instance::receiver(args[0], &STORE, member)?;
    let tier = crate::instance::slot(receiver, TIER_SLOT);
    match tier.as_text() {
        Some(LOCAL_TIER) => Ok(Tier::Local),
        Some(SHARED_TIER) => Ok(Tier::Shared),
        _ => Err(Fault::fatal(format!(
            "{STORE_NAME}::{member} found a `tier` slot that is neither `{LOCAL_TIER}` nor \
             `{SHARED_TIER}`"
        ))),
    }
}

thread_local! {
    /// This core's connection to the shared store, opened by
    /// [`nvs_core_cache_shared`] and reused by every request that runs here.
    ///
    /// Per core rather than per request for the reason a socket is expensive and
    /// a round trip is not: a request that had to hand-shake before its first
    /// `get` would pay the handshake on the request path, every request. What it
    /// spends is one socket per core — O(cores), released when the core ends —
    /// and it is the same shape [`ENTRIES`] above already has, so the tier
    /// question is answered the same way on both sides.
    static SHARED: RefCell<Option<redis::Connection>> = const { RefCell::new(None) };
}

/// The directive `key`'s value, with an empty one read as absent.
///
/// Absent and blank are the same answer on purpose: `url = ""` is an operator
/// clearing a setting, and reading it as a host would produce a refusal about a
/// name rather than about the configuration.
fn configured(ctx: &Ctx, key: &str) -> Option<String> {
    ctx.config()
        .and_then(|config| config.get(key))
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

/// `[cache.shared] timeout`, or [`DEFAULT_TIMEOUT`].
///
/// A directive that will not parse, or that parses to zero, is the shipped bound
/// rather than a refusal — `crate::http`'s `bound_of` reasoning, that `nvs.toml`
/// is validated where it is loaded and failing a request over a key the operator
/// can no longer see is the wrong direction.
fn timeout_of(ctx: &Ctx) -> Duration {
    configured(ctx, TIMEOUT)
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()))
        .filter(|bound| !bound.is_zero())
        .unwrap_or(DEFAULT_TIMEOUT)
}

/// `[cache.shared] url` split into the host the grant is asked about and the
/// port the connection is made to.
///
/// # Errors
///
/// A thrown `RuntimeError` naming what this client reads, for a URL carrying a
/// scheme it does not speak, a path, or a port that is not one. Each of the
/// three is refused rather than ignored: a `rediss://` treated as `redis://`
/// would be a plaintext connection wearing a TLS spelling, and a database index
/// dropped on the floor would put the entries somewhere the operator did not
/// ask for.
fn endpoint(url: &str, member: &str) -> Result<(String, u16), Fault> {
    let refuse = |why: &str| {
        Fault::thrown(format!(
            "{member}: `{URL}` is `{url}`, and {why} — this client reads `redis://host[:port]`"
        ))
    };
    let Some(authority) = url.strip_prefix("redis://") else {
        return Err(refuse(
            "that is not a scheme it speaks; a `rediss://` store is refused rather than \
             half-served, for the reason `Core\\Http\\Client` refuses `https`, that which \
             certificates this binary trusts has no decision yet",
        ));
    };
    let authority = authority.trim_end_matches('/');
    if authority.contains('/') {
        return Err(refuse(
            "a database index is a namespace nothing here names yet",
        ));
    }
    // An IPv6 literal is written `[::1]` and carries colons of its own, so the
    // last one is a port separator only when nothing after it belongs to the
    // address. `pin_host` takes the brackets off itself.
    let (host, port) = match authority.rsplit_once(':') {
        Some((head, tail)) if !tail.contains(']') => {
            let port = tail
                .parse::<u16>()
                .map_err(|_| refuse(&format!("`{tail}` is not a port")))?;
            (head, port)
        }
        _ => (authority, redis::DEFAULT_PORT),
    };
    if host.is_empty() {
        return Err(refuse("it names no host"));
    }
    Ok((host.to_owned(), port))
}

/// Makes this core's connection the one to `address`, and dials it.
///
/// A connection already open to that same address is kept — the ordinary case,
/// since every request on this core asks for the same configured store. One to a
/// *different* address is replaced, which is what a reloaded configuration looks
/// like from here.
///
/// # Errors
///
/// A thrown `IOError` for a store that cannot be reached, which is the second
/// half of [`SHARED_DOC`]'s card: an unreachable store throws at the door rather
/// than answering a handle whose every operation would fail.
fn open_shared(address: SocketAddr, timeout: Duration, member: &str) -> Result<(), Fault> {
    SHARED.with_borrow_mut(|held| {
        if held.as_ref().is_none_or(|open| open.address() != address) {
            *held = Some(redis::Connection::new(address, timeout));
        }
        held.as_mut()
            .expect("the connection was just written")
            .ensure()
            .map_err(|why| Fault::thrown_as(ThrownClass::Io, format!("{member}: {why}")))
    })
}

/// Opens this core's connection to the configured shared store — the **door**,
/// as the module doc's third decision defines one: the directive is read here,
/// the grant is asked for here, the host is pinned here, and the connection is
/// made to the address the grant approved.
///
/// `remedy` is the clause a caller appends to the unconfigured refusal, because
/// what to do instead is the caller's own contract: `Core\Cache::shared()` can
/// name the local tier, and a limiter enforced across every core cannot.
///
/// # Errors
///
/// A thrown `RuntimeError` when no `[cache.shared] url` is set, when the URL is
/// not one this client reads, or when `net.connect` does not cover its host —
/// each a deployment that has not been configured rather than the world saying
/// no. A thrown `IOError` for a store that is configured and cannot be reached,
/// which is the class ADR 0075 § 5's fail-open `catch` holds.
pub(crate) fn open_configured(ctx: &Ctx, member: &str, remedy: &str) -> Result<(), Fault> {
    let Some(url) = configured(ctx, URL) else {
        return Err(Fault::thrown(format!(
            "{member}: no shared store is configured, so there is nothing coherent to answer \
             with — set `[cache.shared] url`{remedy}"
        )));
    };

    let (host, port) = endpoint(&url, member)?;
    // ADR 0058 § 2: the door answers with the address, and the connection is
    // made to *that* — the whole of why a name is not resolved again below.
    let address = nvs_runtime::capability::pin_host(ctx, &host, member)?;
    open_shared(SocketAddr::new(address, port), timeout_of(ctx), member)
}

/// One command on this core's connection to the shared store, for `owner`'s
/// `member`.
///
/// Two names rather than one because the connection now has two owners: the
/// store's own operations, and `Core\RateLimit::consume`, which reaches the
/// same socket because a deployment has one shared store and a limiter that
/// named a second would be a second thing to configure.
///
/// # Errors
///
/// A thrown `IOError` for a store this core never opened, for one that cannot
/// be reached, and for one that refuses the command. Never an answer that looks
/// like absence: ADR 0075 § 5's standing rule is that an unreachable store
/// throws, because the failure mode belongs to the application that knows
/// whether the entry was a cache or a lock.
pub(crate) fn on_shared<T>(
    owner: &str,
    member: &str,
    command: impl FnOnce(&mut redis::Connection) -> Result<T, String>,
) -> Result<T, Fault> {
    SHARED.with_borrow_mut(|held| {
        let open = held.as_mut().ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{owner}::{member}: this core has no connection to the shared store, and \
                     `{NAME}::shared()` is the only thing that opens one"
                ),
            )
        })?;
        command(open)
            .map_err(|why| Fault::thrown_as(ThrownClass::Io, format!("{owner}::{member}: {why}")))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache::local(): Core\Cache\Store` — ADR 0059 § 1's per-core tier.
    ///
    /// Nothing is allocated for the tier itself: the entries live in this
    /// core's [`ENTRIES`] whether a program has asked for a store or not, and
    /// what this answers is a two-word handle naming which tier it is.
    fn nvs_core_cache_local(_ctx, _args: [0]) {
        Ok(crate::instance::build(
            &STORE,
            [Value::str(NvsStr::new(LOCAL_TIER.as_bytes()))],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache::shared(): Core\Cache\Store` — ADR 0059 § 1's coherent tier,
    /// which no deployment can configure yet.
    ///
    /// This is the **door**, and the module doc's third decision is why: the
    /// grant is asked for here, the address is pinned here, and the connection
    /// is dialled here, so the two operations on the store it answers ask
    /// nothing and re-resolve nothing.
    ///
    /// A deployment that configured no store gets a refusal rather than a store
    /// that would silently behave like the local one: the two tiers make
    /// different promises, and a `shared` that quietly served per-core entries
    /// would be the accident § 1 splits the members to prevent.
    ///
    /// # Errors
    ///
    /// A thrown `RuntimeError` when no `[cache.shared] url` is set, when the URL
    /// is not one this client reads, or when `net.connect` does not cover its
    /// host or the outbound policy denies its address; a thrown `IOError` when
    /// the store cannot be reached.
    fn nvs_core_cache_shared(ctx, _args: [0]) {
        let member = format!("{NAME}::shared()");
        open_configured(
            ctx,
            &member,
            ", or use `Core\\Cache::local()` and accept its contract",
        )?;

        Ok(crate::instance::build(
            &STORE,
            [Value::str(NvsStr::new(SHARED_TIER.as_bytes()))],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache\Store::put(string $key, mixed $value): void` — ADR 0059 § 2's
    /// copy out of the request heap.
    ///
    /// # Errors
    ///
    /// A thrown `LogicError` for a value that may not cross — the graph copy's
    /// own refusal, classified as [`crate::serialize`]'s `encode` classifies it,
    /// because a value the program itself built is the program's bug — and a
    /// thrown `IOError` on the shared tier for a store that cannot be reached
    /// or that refuses the write.
    ///
    /// **The copy happens before the tier is consulted**, which is § 2's "not a
    /// third mechanism" written as control flow: there is one call to the walk
    /// in this module and both tiers are downstream of it, so a tier cannot grow
    /// a representation of its own without deleting that structure first.
    fn nvs_core_cache_put(_ctx, args: [3]) {
        let tier = tier_of(args, "put")?;
        let key = key_of(args, 1, "put")?.as_bytes().to_vec();

        // The walk consumes one reference and the argument slot keeps its own,
        // so this is the reference the walk gives up.
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[2].retain();
        }
        let payload = nvs_runtime::encode(args[2]).map_err(|why| {
            Fault::thrown_as(ThrownClass::Logic, format!("{STORE_NAME}::put(): {why}"))
        })?;

        match tier {
            Tier::Local => store_put(&key, payload),
            Tier::Shared => on_shared(STORE_NAME, "put", |open| open.set(&key, &payload))?,
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache\Store::get(string $key): mixed` — ADR 0059 § 2's copy back
    /// in, and § 1's "may be absent at any time" as the `null` it answers.
    ///
    /// # Errors
    ///
    /// A thrown `ParseError` for an entry naming a class this program cannot
    /// resolve, which is [`crate::serialize`]'s `decode` refusal reached through
    /// the same resolver — the program's own class table — and, on the shared
    /// tier, a thrown `IOError` for a store that cannot be reached. An
    /// entry that is simply not there is `null` on either tier, which is the
    /// difference § 1 draws between a miss and a failure.
    ///
    /// **The tier is consulted before the copy, and only for the bytes**: it
    /// answers where the payload comes from and nothing about how it is read
    /// back, which is [`nvs_core_cache_put`]'s structure in the other direction.
    fn nvs_core_cache_get(ctx, args: [2]) {
        let tier = tier_of(args, "get")?;
        let key = key_of(args, 1, "get")?.as_bytes().to_vec();

        let held = match tier {
            Tier::Local => store_get(&key),
            Tier::Shared => on_shared(STORE_NAME, "get", |open| open.get(&key))?,
        };
        let Some(payload) = held else {
            return Ok(Value::null());
        };
        let resolve = |name: &str| ctx.class_desc(name);
        nvs_runtime::decode(&payload, &resolve).map_err(|why| {
            Fault::thrown_as(ThrownClass::Parse, format!("{STORE_NAME}::get(): {why}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CLASS, GET_DOC, LOCAL_DOC, SHARED_DOC, Value, store_get, store_put};

    /// ADR 0059 § 2: the copy across this boundary is the graph copy ADR 0023
    /// already defines and the isolate boundary already shares — not a third
    /// mechanism, and still not a second one now that there are two tiers.
    ///
    /// Two claims, because either alone would pass a module that had grown a
    /// carrier of its own. The **bytes** an entry holds are exactly what
    /// `nvs_runtime::encode` answers for the value that went in, so an entry is
    /// that walk's output rather than a rendering of it. And each half of the
    /// carrier is reached from exactly **one** place in this module's shipped
    /// code, so the tier a `put` is bound for cannot select a representation:
    /// the copy happens first and the tier only decides where the bytes go.
    #[test]
    fn a_cache_put_and_get_use_the_same_graph_copy_as_the_isolate_boundary() {
        let payload = nvs_runtime::encode(Value::int(7)).expect("an `int` crosses any boundary");
        store_put(b"the-same-walk", payload.clone());
        assert_eq!(
            store_get(b"the-same-walk"),
            Some(payload),
            "an entry is the carrier's own bytes, byte for byte"
        );

        // The scan stops where `crate::registry`'s own do, at the test module:
        // what a member can reach at run time is the shipped half.
        let shipped: Vec<&str> = include_str!("cache.rs")
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]")
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        for half in ["nvs_runtime::encode(", "nvs_runtime::decode("] {
            let sites = shipped.iter().filter(|line| line.contains(half)).count();
            assert_eq!(
                sites, 1,
                "`{half}` is reached from {sites} places in this module; both tiers share one \
                 carrier, so there is exactly one"
            );
        }
    }

    /// ADR 0059 § 1: two members, not one API with a flag — and the check that
    /// makes that structural is that neither takes an argument at all, so no
    /// value computed at run time can choose the tier.
    #[test]
    fn local_and_shared_are_separate_members_with_separate_contracts() {
        let names: Vec<&str> = CLASS.methods.iter().map(|method| method.name).collect();
        assert_eq!(names, ["local", "shared"], "§ 1's roster is these two");

        for method in CLASS.methods {
            assert!(
                method.params.is_empty(),
                "{}::{} takes an argument, which is the flag § 1 refuses",
                CLASS.name,
                method.name
            );
        }

        // Separate *contracts*, which is the half a shared implementation would
        // quietly lose: one answers a store that may forget, the other one that
        // is coherent and can be unreachable.
        assert_ne!(LOCAL_DOC.short, SHARED_DOC.short);
        assert!(
            LOCAL_DOC.ret.contains("not visible on another"),
            "the local card has to state § 1's per-core contract"
        );
        assert!(
            LOCAL_DOC.errors.is_empty() && !SHARED_DOC.errors.is_empty(),
            "an unreachable shared store throws; a local one has nothing to be unreachable"
        );
    }

    /// ADR 0059 § 1: any entry may be absent at any time, for any reason. The
    /// store answers `None` for a key nothing wrote, and — the half a program
    /// depends on — the card *says* so, since a contract nobody can read is one
    /// every caller will assume away.
    #[test]
    fn a_local_entry_may_be_absent_at_any_time_and_the_contract_says_so() {
        assert_eq!(store_get(b"absent-by-construction"), None);

        store_put(b"present", b"payload".to_vec());
        assert_eq!(store_get(b"present"), Some(b"payload".to_vec()));

        assert!(
            GET_DOC.ret.contains("null") && GET_DOC.ret.contains("absent at any time"),
            "`get`'s card has to state the absence its caller must handle"
        );
        assert!(
            GET_DOC
                .errors
                .iter()
                .all(|error| error.error != "RuntimeError"),
            "absence is an answer, never a throw"
        );
    }

    /// ADR 0059 § 1: a write on one core is not visible on another. The store is
    /// a `thread_local`, and the runtime is thread-per-core, so a second thread
    /// *is* a second core for this question.
    #[test]
    fn a_local_write_on_one_core_is_not_visible_on_another() {
        store_put(b"one-core-only", b"written here".to_vec());
        assert_eq!(store_get(b"one-core-only"), Some(b"written here".to_vec()));

        let elsewhere = std::thread::spawn(|| {
            let before = store_get(b"one-core-only");
            // And the other direction: what the second core writes stays there.
            store_put(b"one-core-only", b"written there".to_vec());
            (before, store_get(b"one-core-only"))
        })
        .join()
        .expect("the second core's thread runs to completion");

        assert_eq!(elsewhere, (None, Some(b"written there".to_vec())));
        assert_eq!(store_get(b"one-core-only"), Some(b"written here".to_vec()));
    }
}
