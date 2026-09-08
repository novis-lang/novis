//! `Core\Cache` — `rule:concurrency/cross-request-state-is-explicit`'s
//! sanctioned exception to `rule:security/no-cross-request-state`'s closed door on cross-request state,
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
//! `rule:classes/two-copy-depths`
//! already defines, not a third mechanism — and leaves which of its two
//! carriers open. This tier uses the **byte** carrier
//! ([`nvs_runtime::encode`]/[`nvs_runtime::decode`]), which is the same walk
//! the live one is, for two reasons the live carrier cannot answer:
//!
//! 1. **A copied object holds a raw `ClassDesc` pointer**, and this store
//!    outlives the request that filled it — so under
//!    `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
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
//! `rule:security/capability-roster-is-closed`
//! listed `Core\Cache::local()` as the one capability-bearing member with no
//! grant named for it, and left the naming to `rule:concurrency/cross-request-state-is-explicit`. The answer written into
//! that ADR's § 1 is that there is **no grant**, because
//! `rule:security/capability-question-is-grant-and-scope`
//! checks a capability at the door to an *effect* and this tier has no
//! door: nothing leaves the process, no name is resolved and no file is opened.
//! What is left to bound is footprint, and § 3's `nvs.toml` cap is the
//! instrument for a bound — a boolean grant would not be one. The shared tier
//! has the `cache.shared` row instead, in [`crate::registry::CAPABILITIES`]
//! beside it, and that asymmetry is the whole of what the two rows say.
//!
//! # Decision: `shared()` is the door, and the two operations are behind it
//!
//! [`nvs_core_cache_shared`] is where the grant is asked for and where the
//! store is dialled; `put` and `get` on the store it answers ask nothing, and
//! so [`STORE`] carries no capability row of its own — exactly as
//! `Core\Http\Client` carries none behind `Core\Http::allowUrl`. A check at the
//! operation instead would ask the same unscoped question once per command and
//! answer it the same way every time, which is a cost with no boundary in it.
//!
//! The grant is `cache.shared` and it is asked at `Scope::Unscoped`, which is
//! `rule:config/cache-shared-is-the-grant-over-the-configured-store`: the store
//! is the one an operator wrote into root-owned configuration, and that writing
//! is the authorization, so nothing here asks `net.connect` about the host or
//! `rule:security/net-address-policy`'s table about the address. The name is
//! resolved once and the connection is made to what it answered.
//!
//! The door is [`open_configured`] rather than the member, because it has a
//! second caller: `Core\RateLimit::consume` limits over the same store — a
//! deployment has one — and `rule:core-classes/ratelimit-two-members` and `rule:core-classes/ratelimit-unreachable-store-throws` both write that member standing
//! alone, so it opens the store itself instead of requiring `shared()` to have
//! been called first. Both callers ask for the same grant and reuse the same
//! per-core socket; what differs is the sentence each appends to the refusal,
//! since what to do instead is the caller's own contract.
//!
//! Which store, and how long a command may take, are `[cache.shared] url` and
//! `[cache.shared] timeout`; what the tier behind `local()` may hold is
//! `[cache.local] max_size`. All three are `System`-class, because neither
//! where a fleet's coherent state lives nor how much of a core's memory it
//! keeps is a decision one request may make for the rest of them.
//!
//! # Decision: the cap is bytes on this core, and forgetting is how it is kept
//!
//! § 3 caps the local tier by an `nvs.toml` directive and says exceeding it
//! **evicts rather than failing an allocation**, so [`Local::put`] never
//! refuses: it forgets entries until the arrival fits, and an arrival too large
//! for the whole tier is itself forgotten as it lands. Nothing about `put`'s
//! contract changes, because § 1 already says an entry may be absent at any
//! time — the cap is a second reason for the `null` a caller had to handle
//! anyway, and a `put` that threw would be a third failure mode for a store
//! whose whole contract is that it has none.
//!
//! **The victim is the key written longest ago**, not the least recently read:
//! a read that reordered the queue would make `get` a write, taking a `&mut` on
//! the request path and turning a `RefCell` shared with `Core\RateLimit` into
//! something two live borrows could reach. What that costs is that a hot key
//! written once and read forever ages out under a cold key rewritten often;
//! what it buys is that `get` is a lookup and `put` is O(1) amortised. An
//! eviction policy that measured *use* would need a clock or a counter per
//! entry, which is footprint spent to protect footprint.
//!
//! # What is not here yet
//!
//! **A TTL and a `forget`.** A lifetime is a second reason an entry goes, and
//! the cap above is the one that had to exist first; `put` grows the trailing
//! options shape `rule:core-api/shape-rules` R2 puts last when a TTL lands, which is an addition
//! to the row rather than a change to it.
//!
//! **A shared store behind a password, a database index or TLS.** The URL this
//! reads is `redis://host[:port]` and nothing else, and each of the three is
//! refused with a sentence rather than half-served: `AUTH` needs `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s
//! secret plumbing to carry the credential, a database index is a second
//! namespace nothing yet names, and a `rediss://` client needs the trust-anchor
//! decision `crate::http::transport` is also waiting on.
//!
//! **What it spends:** on the local tier, per core, one map entry and one queue
//! slot per live key — the key's bytes, its payload's and [`ENTRY_OVERHEAD`] —
//! held until it is overwritten or forgotten, charged to the core rather than
//! to any request, and bounded by `[cache.local] max_size`, which ships at
//! [`DEFAULT_MAX_SIZE`]. That is § 3's O(cores × working set) with both factors
//! named, and it is deliberately not O(requests served): a key rewritten on
//! every request costs what it cost the first time. On the shared tier, one
//! socket per core and nothing per entry: the bytes are the store's.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use nvs_config::capability::{Cap, Scope};
use nvs_config::{Quantity, Setting, Unit};
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

/// `Core\Cache::local`'s reference card — `rule:core-api/reference-card`.
const LOCAL_DOC: MethodDoc = MethodDoc {
    short: "The per-core, in-process tier: one store per core, with no coherence between cores and \
            no network behind it.",
    params: &[],
    ret: "A `Core\\Cache\\Store` over this core's own entries. Any entry may be absent at any time, \
          for any reason, and a write on one core is not visible on another — a program that would \
          be incorrect if a `get` answered `null` wants `shared` instead.",
    errors: &[],
};

/// `Core\Cache::shared`'s reference card — `rule:core-api/reference-card`.
const SHARED_DOC: MethodDoc = MethodDoc {
    short: "The coherent tier: a real store over the network, shared by every core and every \
            machine that names it.",
    params: &[],
    ret: "A `Core\\Cache\\Store` over the configured shared store, whose entries every core sees.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The capability `cache.shared` is not granted; or no `[cache.shared] url` is \
                   configured, or it is not a URL this client reads. Each is a deployment that \
                   was not configured rather than a store that failed.",
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
            // difference is `rule:security/unclassified-parameter-refuses-tainted`'s. Not a byte of the key reaches any
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
            // `mixed` rather than `rule:core-api/shape-rules` R7's `?T`: what went in is any value
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
/// `rule:http-server/no-spelling-for-an-unbounded-wait`
/// refuses to give any outbound wait a spelling for.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// `Core\Cache\Store::put`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Cache\Store::get`'s reference card — `rule:core-api/reference-card`.
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
/// `[cache.local] max_size` — what this core's entries may hold together.
const MAX_SIZE: &str = "cache.local.max_size";

/// The cap a deployment that configured none inherits: the 32 MiB APCu ships
/// for the extension `rule:core-api/tier-placement` records this class as answering, read per core
/// rather than per host. Eight cores hold up to eight of it, which is § 3's
/// O(cores × working set) with a number in front of it.
const DEFAULT_MAX_SIZE: usize = 32 * 1024 * 1024;

/// What one entry costs beyond its own bytes: a map bucket, an `Rc` header, a
/// queue slot and the fat pointers over them, rounded to something a reader can
/// hold in their head. Charged so the cap bounds the *allocation* — a tier full
/// of one-byte entries under a cap that counted payloads alone would be a cap
/// measuring almost none of what it holds.
const ENTRY_OVERHEAD: usize = 64;

/// This core's entries and what they cost, together, because a size is only
/// meaningful against the map it measures: a second `thread_local` holding the
/// number beside the map would be two writers of one fact.
#[derive(Default)]
struct Local {
    /// The entries themselves, each an `rule:classes/serialize-is-a-closed-format` payload. Keys are `Rc` so
    /// that `order` below names one without a second copy of the bytes.
    entries: HashMap<Rc<[u8]>, Box<[u8]>>,
    /// Every live key, in the order it was **first** written, which is the
    /// order [`Local::forget_oldest`] gives them up in.
    ///
    /// First written rather than last: an overwrite keeps its place, so this
    /// holds exactly one slot per live key and a hot key rewritten a million
    /// times leaves nothing behind it. Ordering by last write instead would
    /// need a slot per *write*, which is the O(requests served) growth
    /// `AGENTS.md` calls a leak rather than a policy.
    order: VecDeque<Rc<[u8]>>,
    /// What `entries` costs by [`charged`], maintained on every write so that
    /// the cap is a comparison rather than a walk of the map.
    held: usize,
}

impl Local {
    /// Writes `payload` under `key`, forgetting whatever it has to.
    ///
    /// `cap` is `None` for the `false` an operator writes for no ceiling.
    fn put(&mut self, key: &[u8], payload: Vec<u8>, cap: Option<usize>) {
        let incoming = charged(key.len(), payload.len());

        if cap.is_some_and(|cap| incoming > cap) {
            // An entry the tier could not hold even empty is forgotten as it
            // arrives rather than failing the write — § 3 evicts, and § 1 has
            // already told the caller a `get` may answer nothing. What was
            // under the key goes with it, because `put` replaced it. Its slot
            // in `order` is the one that can outlive its entry, and
            // `forget_oldest` skips it when it reaches the front.
            if let Some((held, previous)) = self.entries.remove_entry(key) {
                self.held -= charged(held.len(), previous.len());
            }
            return;
        }

        // Before the old entry is taken out, so that a key evicted here is
        // evicted from both halves at once. The cost is that an overwrite is
        // priced as an arrival: a tier at its cap forgets one more entry than
        // the net change needs. Netting the two would have to name an entry
        // this loop may have just forgotten, which is the more expensive bug.
        if let Some(cap) = cap {
            while self.held + incoming > cap && self.forget_oldest() {}
        }

        let key = match self.entries.remove_entry(key) {
            Some((held, previous)) => {
                self.held -= charged(held.len(), previous.len());
                held
            }
            None => {
                let fresh: Rc<[u8]> = Rc::from(key);
                self.order.push_back(Rc::clone(&fresh));
                fresh
            }
        };
        self.held += incoming;
        self.entries.insert(key, payload.into_boxed_slice());
    }

    /// Forgets the entry whose key was written longest ago, and answers whether
    /// there was one to forget — which is what bounds [`Local::put`]'s loop.
    ///
    /// A slot naming nothing is skipped rather than counted: it is the oversized
    /// write above, and skipping it here is what keeps that case from paying for
    /// a scan of the queue at the time it happens.
    fn forget_oldest(&mut self) -> bool {
        while let Some(key) = self.order.pop_front() {
            if let Some(previous) = self.entries.remove(&key) {
                self.held -= charged(key.len(), previous.len());
                return true;
            }
        }
        false
    }
}

/// What an entry of these two lengths costs the cap.
fn charged(key: usize, payload: usize) -> usize {
    key + payload + ENTRY_OVERHEAD
}

thread_local! {
    /// The local tier itself: this core's entries and nothing shared with any
    /// other core.
    ///
    /// A `thread_local` rather than anything reachable from another thread is
    /// § 1's "no coherence between cores" as a *representation* rather than as
    /// a rule to remember — the runtime is thread-per-core, so a store another
    /// core could reach would need a lock this tier is defined not to have.
    static ENTRIES: RefCell<Local> = RefCell::new(Local::default());
}

/// `[cache.local] max_size` as a byte count, or [`DEFAULT_MAX_SIZE`], and `None`
/// for the `false` that removes the ceiling.
///
/// Read per write rather than once per core, because the directive is `Reload`
/// and a cap cached at the first `put` would outlive the configuration that set
/// it. What that costs is one small string and one parse on a path that is
/// already an O(graph) encode, and [`nvs_config::Quantity`] is the parse every
/// other size directive uses — a second reading of what `32M` means is exactly
/// the divergence `rule:config/ini-set-is-core-config-set` keeps one parser to prevent.
///
/// A value that will not parse is the shipped cap, which is [`timeout_of`]'s
/// reasoning: `nvs.toml` is refused where it is loaded, by the boundary that can
/// name the file and the line.
pub(crate) fn local_cap(ctx: &Ctx) -> Option<usize> {
    let Some(written) = configured(ctx, MAX_SIZE) else {
        return Some(DEFAULT_MAX_SIZE);
    };
    match Quantity::parse(MAX_SIZE, Unit::Bytes, &Setting::Text(written)) {
        Ok(Quantity::Unbounded) => None,
        Ok(Quantity::Bytes(bytes)) => Some(usize::try_from(bytes).unwrap_or(usize::MAX)),
        _ => Some(DEFAULT_MAX_SIZE),
    }
}

/// Writes `payload` under `key` on this core, replacing any entry there and
/// forgetting whatever `cap` does not leave room for.
///
/// Crate-visible because this tier is **the** per-core store rather than this
/// class's: [`crate::ratelimit`]'s `shed` keeps its arrival times here, under
/// its own key prefix, for the reason `consume` reaches [`on_shared`] — one
/// store per tier means one thing to bound and one thing to configure, and a
/// limiter that kept a second map would be a second footprint for no second
/// guarantee. § 3's cap bounds both by bounding this, which is why `cap` is a
/// parameter here rather than read inside: both callers read it from the same
/// [`local_cap`], and a store that read it for itself would be reachable from a
/// test with no configuration at all.
pub(crate) fn store_put(key: &[u8], payload: Vec<u8>, cap: Option<usize>) {
    ENTRIES.with_borrow_mut(|local| local.put(key, payload, cap));
}

/// This core's payload for `key`, or `None` — which is an ordinary answer and
/// not a failure, per § 1, and now also the answer for an entry the cap made
/// room by forgetting. Crate-visible for [`store_put`]'s reason.
pub(crate) fn store_get(key: &[u8]) -> Option<Vec<u8>> {
    ENTRIES.with_borrow(|local| local.entries.get(key).map(|payload| payload.to_vec()))
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
///
/// `pub(crate)` because [`crate::session`] reads its own three directives the
/// same way and a second copy of "blank is absent" would be a second rule.
pub(crate) fn configured(ctx: &Ctx, key: &str) -> Option<String> {
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
/// A thrown `RuntimeError` when `cache.shared` is not granted, when no
/// `[cache.shared] url` is set, or when the URL is not one this client reads —
/// each a deployment that has not been configured rather than the world saying
/// no. A thrown `IOError` for a store that is configured and cannot be reached,
/// which is the class `rule:core-classes/ratelimit-unreachable-store-throws`'s fail-open `catch` holds.
pub(crate) fn open_configured(ctx: &Ctx, member: &str, remedy: &str) -> Result<(), Fault> {
    let Some(url) = configured(ctx, URL) else {
        return Err(Fault::thrown(format!(
            "{member}: no shared store is configured, so there is nothing coherent to answer \
             with — set `[cache.shared] url`{remedy}"
        )));
    };

    // After the directive and before anything is dialled. A deployment that configured no store
    // hears that first, because there is no tier for a grant to be about yet — and asking here
    // rather than at the top is `rule:security/capability-costs-nothing-unasked`'s shape: the
    // question is put beside the effect it authorizes.
    nvs_runtime::capability::require(ctx, Cap::CacheShared, Scope::Unscoped, member)?;

    let (host, port) = endpoint(&url, member)?;
    // Resolved and not pinned: `rule:config/cache-shared-is-the-grant-over-the-configured-store` is
    // that the endpoint an operator wrote carries the authority that granted the capability, so
    // there is no attacker-influenced name here for `rule:security/net-address-policy`'s table to
    // hold at arm's length — and holding it there is what made a loopback store need `net.internal`.
    let address = nvs_runtime::capability::resolve_host(&host, member)?;
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
/// like absence: `rule:core-classes/ratelimit-unreachable-store-throws`'s standing rule is that an unreachable store
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
    /// `Core\Cache::local(): Core\Cache\Store` — `rule:core-api/two-cache-tiers`'s per-core tier.
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
    /// `Core\Cache::shared(): Core\Cache\Store` — `rule:core-api/two-cache-tiers`'s coherent tier,
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
    /// A thrown `RuntimeError` when `cache.shared` is not granted, when no
    /// `[cache.shared] url` is set, or when the URL is not one this client
    /// reads; a thrown `IOError` when the store cannot be reached.
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
    /// `Core\Cache\Store::put(string $key, mixed $value): void` — `rule:concurrency/a-cached-value-is-copied-across-the-boundary`'s
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
    fn nvs_core_cache_put(ctx, args: [3]) {
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
            Tier::Local => store_put(&key, payload, local_cap(ctx)),
            Tier::Shared => on_shared(STORE_NAME, "put", |open| open.set(&key, &payload))?,
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache\Store::get(string $key): mixed` — `rule:concurrency/a-cached-value-is-copied-across-the-boundary`'s copy back
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
    use std::net::TcpListener;

    use nvs_runtime::budget;
    use nvs_runtime::{Fault, ThrownClass};

    use crate::tests::granting;

    use super::{
        CLASS, Ctx, DEFAULT_MAX_SIZE, ENTRIES, ENTRY_OVERHEAD, GET_DOC, LOCAL_DOC, MAX_SIZE,
        SHARED_DOC, Value, local_cap, open_configured, store_get, store_put,
    };

    /// The member a store's door names in a refusal, and what a case here is
    /// standing in for.
    const MEMBER: &str = "Core\\Cache::shared()";

    /// A store that is listening and nothing more: a socket on loopback that
    /// accepts the connection the door dials and never answers a command.
    ///
    /// Enough for every case below, because [`open_configured`] finishes at
    /// `ensure` — what a store *says* is [`super::redis`]'s subject, and the
    /// question here is only which questions were asked on the way to dialling
    /// it. The listener is returned with the URL so it outlives the call.
    fn listening() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("the bound port").port();
        (listener, format!("redis://127.0.0.1:{port}"))
    }

    /// A context configured with `written`, which is a `[capabilities]` block
    /// and a `[cache.shared]` one.
    fn deployed(written: &str) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(written));
        ctx
    }

    /// `rule:concurrency/a-cached-value-is-copied-across-the-boundary`: the copy across this boundary is the graph copy `rule:classes/two-copy-depths`
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
        store_put(b"the-same-walk", payload.clone(), None);
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

    /// `rule:core-api/two-cache-tiers`: two members, not one API with a flag — and the check that
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

    /// `rule:core-api/two-cache-tiers`: any entry may be absent at any time, for any reason. The
    /// store answers `None` for a key nothing wrote, and — the half a program
    /// depends on — the card *says* so, since a contract nobody can read is one
    /// every caller will assume away.
    #[test]
    fn a_local_entry_may_be_absent_at_any_time_and_the_contract_says_so() {
        assert_eq!(store_get(b"absent-by-construction"), None);

        store_put(b"present", b"payload".to_vec(), None);
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

    /// `rule:core-api/two-cache-tiers`: a write on one core is not visible on another. The store is
    /// a `thread_local`, and the runtime is thread-per-core, so a second thread
    /// *is* a second core for this question.
    #[test]
    fn a_local_write_on_one_core_is_not_visible_on_another() {
        store_put(b"one-core-only", b"written here".to_vec(), None);
        assert_eq!(store_get(b"one-core-only"), Some(b"written here".to_vec()));

        let elsewhere = std::thread::spawn(|| {
            let before = store_get(b"one-core-only");
            // And the other direction: what the second core writes stays there.
            store_put(b"one-core-only", b"written there".to_vec(), None);
            (before, store_get(b"one-core-only"))
        })
        .join()
        .expect("the second core's thread runs to completion");

        assert_eq!(elsewhere, (None, Some(b"written there".to_vec())));
        assert_eq!(store_get(b"one-core-only"), Some(b"written here".to_vec()));
    }

    /// `rule:concurrency/cache-memory-is-charged-to-the-core`: exceeding the cap **evicts** rather than failing an
    /// allocation. The write that crosses it succeeds, and what goes is the key
    /// written longest ago — which § 1 has already told every caller to expect,
    /// since an entry may be absent at any time for any reason.
    #[test]
    fn the_cap_forgets_an_older_entry_rather_than_refusing_the_write() {
        // Room for two of these three entries and no more.
        let room = Some(2 * (2 + 6 + ENTRY_OVERHEAD));
        store_put(b"k1", b"first!".to_vec(), room);
        store_put(b"k2", b"second".to_vec(), room);
        assert_eq!(store_get(b"k1"), Some(b"first!".to_vec()));

        store_put(b"k3", b"third!".to_vec(), room);
        assert_eq!(
            store_get(b"k1"),
            None,
            "the oldest key is the one that goes"
        );
        assert_eq!(store_get(b"k2"), Some(b"second".to_vec()));
        assert_eq!(store_get(b"k3"), Some(b"third!".to_vec()));
    }

    /// An entry the tier could not hold even empty is forgotten as it arrives,
    /// and takes what was under its key with it — `put` replaced that entry.
    /// The tier is unharmed afterwards, which is the half that would break if
    /// the arrival's charge were left behind.
    #[test]
    fn an_entry_larger_than_the_whole_tier_is_forgotten_as_it_arrives() {
        let room = Some(64 + ENTRY_OVERHEAD);
        store_put(b"kept", vec![0; 8], room);
        store_put(b"kept", vec![0; 4096], room);
        assert_eq!(store_get(b"kept"), None);

        store_put(b"kept", vec![7; 8], room);
        assert_eq!(store_get(b"kept"), Some(vec![7; 8]));
    }

    /// `AGENTS.md`'s O(in-flight) rule, which is what makes the cap a bound at
    /// all: one key rewritten forever costs what it cost the first time. The
    /// eviction order holds one slot per live key rather than one per write —
    /// `Core\RateLimit::shed` rewrites a single key on every request, so the
    /// other spelling would grow with the traffic served.
    #[test]
    fn a_key_rewritten_forever_costs_what_it_cost_the_first_time() {
        let room = Some(4 * (4 + 8 + ENTRY_OVERHEAD));
        for step in 0..1_000u32 {
            store_put(b"hot!", step.to_string().into_bytes(), room);
        }
        assert_eq!(store_get(b"hot!"), Some(b"999".to_vec()));

        ENTRIES.with_borrow(|local| {
            assert_eq!(local.entries.len(), 1);
            assert_eq!(
                local.order.len(),
                1,
                "an overwrite keeps its place rather than taking a second one"
            );
        });
    }

    /// `rule:concurrency/cache-memory-is-charged-to-the-core`: the tier's memory is charged to the **core** that holds it
    /// — never to a request — and bounded by an `nvs.toml` directive.
    ///
    /// The name's two halves are one fact, and each is checked where the other
    /// cannot see it. The cap is the *directive's*, whose row is `Reload`, which
    /// is why [`local_cap`] is read per write rather than once per core. The
    /// cap then bounds what the core actually holds and not merely the
    /// bookkeeping beside it: a sweep writing thirty-two times the cap leaves
    /// this thread's allocator balance up by about the cap, so the entries
    /// `forget_oldest` dropped gave their bytes back. The eviction cases above
    /// assert what a `put` forgets; this one asserts that forgetting it was a
    /// deallocation.
    ///
    /// And the charge lands on the core rather than on a request:
    /// `nvs_runtime::budget::live_bytes` is *this thread's* balance —
    /// `Ctx::memory_used` is the per-request reading, and that module doc says
    /// how the two relate — so a second core writing the same key raises its own
    /// by the same amount. That is § 3's O(cores × working set) measured rather
    /// than restated, and it is the price `rule:security/no-cross-request-state`'s isolation is bought
    /// with.
    #[test]
    fn the_local_tiers_memory_is_charged_to_the_core_and_capped() {
        /// Small enough that the sweep below writes many times over it.
        const CAP: usize = 64 * 1024;
        /// One sweep entry's payload.
        const CHUNK: usize = 4 * 1024;
        /// How many of them, so that 32 × `CAP` is written in all.
        const WRITES: usize = 512;
        /// The charge one entry has to be able to show through the noise.
        const LARGE: usize = 256 * 1024;

        // The ceiling is `[cache.local] max_size` under `rule:config/three-changeability-classes`'s ordinary
        // rules: the shipped one where an operator wrote nothing, a size where
        // they wrote one, and none at all for the `false` that removes it.
        let mut ctx = Ctx::buffered();
        assert_eq!(local_cap(&ctx), Some(DEFAULT_MAX_SIZE));
        ctx.set_config(granting("[cache.local]\nmax_size = \"128K\"\n"));
        assert_eq!(local_cap(&ctx), Some(128 * 1024));
        ctx.set_config(granting("[cache.local]\nmax_size = false\n"));
        assert_eq!(local_cap(&ctx), None, "`false` is no ceiling at all");
        assert_eq!(
            nvs_config::directive::lookup(MAX_SIZE).map(|row| row.apply),
            Some(nvs_config::Apply::Reload),
            "a cap read once per core would outlive the snapshot that set it"
        );

        // Thirty-two times the cap, written under it.
        let before = budget::live_bytes();
        let mut over = 0;
        for step in 0..WRITES {
            store_put(
                format!("sweep-{step:04}").as_bytes(),
                vec![b'c'; CHUNK],
                Some(CAP),
            );
            over += usize::from(ENTRIES.with_borrow(|local| local.held) > CAP);
        }
        assert_eq!(
            over, 0,
            "{over} of {WRITES} writes left the tier over its cap"
        );

        let held = budget::live_bytes() - before;
        assert!(
            held < (4 * CAP).cast_signed(),
            "the core holds {held} bytes after writing {}, so the cap bounds the \
             bookkeeping and not the memory",
            WRITES * CHUNK
        );

        // Charged to *this* thread, which is what § 3 means by charged to the
        // core: the balance rises with the entry and stays risen with it.
        let alone = budget::live_bytes();
        store_put(b"charged-per-core", vec![b'x'; LARGE], None);
        assert!(
            budget::live_bytes() - alone >= LARGE.cast_signed(),
            "the entry's bytes are live on the core that wrote it"
        );

        // And a second core pays for its own copy of the same key rather than
        // sharing this one's — § 3's multiplication, as a measurement.
        let elsewhere = std::thread::spawn(|| {
            let fresh = budget::live_bytes();
            store_put(b"charged-per-core", vec![b'y'; LARGE], None);
            budget::live_bytes() - fresh
        })
        .join()
        .expect("the second core's thread runs to completion");
        assert!(
            elsewhere >= LARGE.cast_signed(),
            "eight cores hold eight copies, and each one is charged for its own"
        );
    }

    /// `rule:config/cache-shared-is-the-grant-over-the-configured-store`: the
    /// grant over a store an operator configured names the store, so the door
    /// opens one for a deployment that grants `cache.shared` and **nothing
    /// else** — no `net.connect` for the URL's host, and no host in any grant
    /// list at all.
    ///
    /// The absence is what the case is for. A door that had kept the address
    /// question would refuse this configuration, and it would refuse it with
    /// the same class of error the unconfigured case produces, so a case that
    /// only asserted "it throws for the wrong deployment" would pass against
    /// the rule this one replaces.
    #[test]
    fn a_configured_store_needs_no_address_grant() {
        let (_listener, url) = listening();
        let ctx = deployed(&format!(
            "[capabilities]\ncache.shared = true\n\n[cache.shared]\nurl = \"{url}\"\n"
        ));

        open_configured(&ctx, MEMBER, "").expect("a granted store, configured, is dialled");
    }

    /// The same rule from the other side: `cache.shared` is a **grant**, so a
    /// deployment carrying the old one is refused rather than quietly
    /// migrated.
    ///
    /// `net.connect = true` is written here — the widest the old question had
    /// a spelling for — because that is exactly the tree this change turns from
    /// working into refused, and the refusal names the key an operator has to
    /// write. Catchable, per
    /// `rule:security/denial-is-a-runtime-error`, and not the `IOError` an
    /// unreachable store answers with: nothing was dialled.
    #[test]
    fn a_configured_store_is_refused_without_its_own_grant() {
        let (_listener, url) = listening();
        let ctx = deployed(&format!(
            "[capabilities]\nnet.connect = true\n\n[cache.shared]\nurl = \"{url}\"\n"
        ));

        let refused = open_configured(&ctx, MEMBER, ", or use `Core\\Cache::local()`")
            .expect_err("`net.connect` is not the grant over a configured store");
        let Fault::Thrown(class, message) = refused else {
            panic!("a denied capability is catchable");
        };
        assert_eq!(class, ThrownClass::Runtime);
        assert!(
            message.contains("cache.shared") && message.contains(MEMBER),
            "the refusal names the capability in the spelling `nvs.toml` grants it under, and the \
             member that wanted it: {message}"
        );
    }

    /// The cost the old question had: a store on loopback — where an ordinary
    /// single-machine deployment puts one — sat inside
    /// `rule:security/net-address-policy`'s denied ranges, so reaching it took
    /// a `net.internal` exception beside the grant.
    ///
    /// Both halves are asserted, because the door skipping the table and the
    /// table no longer denying loopback would look identical from the first
    /// half alone: the address the case connects to is still one
    /// `denied_by_default` refuses, and the connection is still made. What
    /// changed is which doors ask.
    #[test]
    fn a_loopback_store_needs_no_net_internal_exception() {
        let (_listener, url) = listening();
        let ctx = deployed(&format!(
            "[capabilities]\ncache.shared = true\n\n[cache.shared]\nurl = \"{url}\"\n"
        ));

        open_configured(&ctx, MEMBER, "").expect("a loopback store is reached with no exception");
        assert!(
            nvs_config::capability::denied_by_default("127.0.0.1".parse().expect("a literal"))
                .is_some(),
            "the table still denies loopback for every door that asks it — one fewer does"
        );
    }

    /// `rule:core-classes/ratelimit-two-members`' coherent half and
    /// `rule:core-api/two-cache-tiers`' coherent tier reach the same store
    /// through [`open_configured`], so they declare one grant between them and
    /// their per-core siblings declare none.
    ///
    /// Asserted as agreement rather than as two values: a limiter that grew a
    /// grant of its own would still read plausibly on its own row, and what
    /// makes the pair correct is that a deployment granting the tier has
    /// granted the limiter and cannot do one without the other.
    #[test]
    fn the_limiter_and_the_tier_ask_one_grant_at_one_door() {
        let asked = |class: &str, member: &str| {
            crate::registry::CAPABILITIES
                .iter()
                .find(|(owner, name, _)| *owner == class && *name == member)
                .map(|(_, _, cap)| *cap)
                .expect("every capability-bearing member declares a row")
        };

        assert_eq!(
            asked(super::NAME, "shared"),
            asked(crate::ratelimit::NAME, "consume"),
            "one door, one grant"
        );
        assert_eq!(
            asked(super::NAME, "shared"),
            Some(nvs_config::Cap::CacheShared)
        );
        assert_eq!(
            asked(super::NAME, "local"),
            asked(crate::ratelimit::NAME, "shed"),
            "and the two per-core halves ask nothing, for the reason the rows state"
        );
        assert_eq!(asked(super::NAME, "local"), None);
    }
}
