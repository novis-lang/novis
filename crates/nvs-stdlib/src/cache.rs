//! `Core\Cache` — `rule:concurrency/cross-request-state-is-explicit`'s
//! sanctioned exception to `rule:security/no-cross-request-state`'s closed door on cross-request state,
//! as a member per tier that hands back a store and the two operations on one.
//!
//! § 1's tiers are a member each rather than one API with a flag, so the choice
//! a program made is visible in review rather than in an argument list. Not one
//! of them takes a parameter at all, which is what makes that structural: there
//! is no spelling of `local()` that can be turned into `process()` or
//! `shared()` by a value computed at run time.
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
//! **evicts rather than failing an allocation**, so [`Entries::put`] never
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
//! # Decision: a lifetime is a deadline the write records and the read judges
//!
//! `put`'s trailing `{ttl?: Duration}` is `rule:core-api/shape-rules` R2's
//! options shape carrying R12's unit-as-a-type, and it is *optional* where
//! `rule:core-api/a-lifetime-is-written` makes a signature's lifetime a
//! required key. That rule is about a bearer credential that leaves the
//! process and never stops being one; an entry left here without a lifetime
//! is forgotten by the cap above and reaches nobody. So an omitted `ttl` is
//! § 1's contract unchanged, and nothing a program already wrote reads
//! differently.
//!
//! **The two in-process tiers hold a deadline read from [`Instant`]**, never
//! from the wall clock, because a lifetime is an elapsed length of time
//! rather than a date: a clock step would otherwise lengthen or shorten every
//! lifetime in the process at once. The shared tier holds no deadline at all
//! — it is told how long to keep the entry and keeps its own clock — which is
//! why [`Lifetime`] carries the duration the call *wrote* and each tier turns
//! that into what it can hold.
//!
//! **The read judges, and never writes.** An entry past its deadline answers
//! `null` and stays where it is until the cap forgets it or a `put` replaces
//! it, because taking it out would make `get` a write:
//! `rule:concurrency/the-process-tier-is-one-store-per-process` fixes that a
//! lookup takes a shard's read lock and nothing more, and the local tier's
//! `RefCell` is shared with `Core\RateLimit` for the reason § 3's eviction
//! policy is by write age. What that spends is the bytes of an expired entry
//! nobody has asked for again, still counted against the cap; what it buys is
//! that every core reading one hot key reads it at the same moment.
//!
//! **A lifetime already over is not a write.** A `ttl` of zero — and a
//! negative one, which lands in the same place — forgets whatever was under
//! the key and stores nothing, exactly as an arrival too large for the tier
//! is forgotten as it lands. It is not a refusal, because § 1's store has no
//! failure mode for one to be.
//!
//! # What it spends
//!
//! On the local tier, per core, one map entry and one queue
//! slot per live key — the key's bytes, its payload's and [`ENTRY_OVERHEAD`] —
//! held until it is overwritten or forgotten, charged to the core rather than
//! to any request, and bounded by `[cache.local] max_size`, which ships at
//! [`DEFAULT_MAX_SIZE`]. That is § 3's O(cores × working set) with both factors
//! named, and it is deliberately not O(requests served): a key rewritten on
//! every request costs what it cost the first time. On the process tier, the
//! same entry and the same overhead, held once for the whole process instead of
//! once per core and bounded by `[cache.process] max_size` — O(working set)
//! where the local tier is O(cores × working set) — plus [`SHARDS`] locks and
//! their empty queues, which is a fixed cost paid once and not a per-entry one.
//! On the shared tier, one socket per core and nothing per entry: the bytes are
//! the store's.
//!
//! # Known gaps
//!
//! 1. **A shared store behind a password, a database index or TLS.** The URL
//!    this reads is `redis://host[:port]` and nothing else, and each of the
//!    three is refused with a sentence rather than half-served: `AUTH` needs
//!    `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secret
//!    plumbing to carry the credential, a database index is a second namespace
//!    nothing yet names, and a `rediss://` store is a second caller of the TLS
//!    client `crate::http::transport` dials through. Each is a key beside
//!    `[cache.shared] url` before it is a connection, which is what makes the
//!    three one decision rather than three pieces of plumbing.
//!    — owner: unowned

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

use nvs_config::capability::{Cap, Scope};
use nvs_config::{Quantity, Setting, Unit};
use nvs_runtime::{Ctx, Fault, NvsStr, Tag, ThrownClass, Value, budget};
use nvs_syntax::duration;

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

pub(crate) mod redis;

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Cache";

/// [`STORE`]'s name — see [`NAME`].
pub(crate) const STORE_NAME: &str = r"Core\Cache\Store";

/// [`SECRET_ENTRY`]'s name — see [`NAME`].
pub(crate) const SECRET_ENTRY_NAME: &str = r"Core\Cache\SecretEntry";

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
            name: "process",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(STORE_NAME),
            symbol: "nvs_core_cache_process",
            doc: Some(&PROCESS_DOC),
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
    short: "The per-core, in-process tier: a map in this core's own memory, with no lock, no \
            network and no coherence between cores. A value one request writes may or may not be \
            there when the next request asks, because the next request usually runs on another \
            core with an empty map of its own.",
    params: &[],
    ret: "A `Core\\Cache\\Store` over this core's own entries. Any entry may be absent at any time, \
          for any reason, and a write on one core is not visible on another. The only read this \
          tier promises is your own write back in the same request; a value every core should find \
          wants `process`, and a program that would be incorrect on a `null` wants `shared`.",
    errors: &[],
};

/// `Core\Cache::process`'s reference card — `rule:core-api/reference-card`.
const PROCESS_DOC: MethodDoc = MethodDoc {
    short: "The per-process tier: one store every core of this serving process shares, in memory \
            only, and gone when the process ends. A follow-up request on the same machine finds \
            what an earlier one wrote, whichever core it lands on, which is what `local` cannot \
            promise; this is the tier for ordinary application caching.",
    params: &[],
    ret: "A `Core\\Cache\\Store` over this process's own entries, which every core of it reads and \
          writes behind a lock. Any entry may be absent at any time — for the cap, or because this \
          is a different process than the one that wrote it — so a program that would be incorrect \
          on a `null` wants `shared` instead.",
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

/// `{ttl?: Duration}` — [`STORE`]'s one trailing options shape, and the whole
/// of what a program writes about how long an entry lives.
///
/// A `Core\Time\Duration` rather than a count of seconds, which is
/// `rule:core-api/shape-rules` R12: the unit is the type, so no call site has
/// to remember which scale this member chose. Optional, where
/// `rule:core-api/a-lifetime-is-written` makes a signature's lifetime a
/// required key — the module doc's own decision says why the two differ.
const PUT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "ttl",
    ty: CoreTy::Instance(crate::time::DURATION_NAME),
    default: Const::Null,
}];

/// `{fill?: callable(): Core\Cache\SecretEntry, wait?: Duration}` — [`STORE`]'s
/// other trailing options shape, and the whole of what a program writes about
/// supplying a miss.
///
/// The fill answers an object rather than the value alone, because the fetch is
/// the only party that knows how long what it fetched stays good —
/// [`SECRET_ENTRY`] is where that is argued. Its signature is written out
/// rather than left a bare `callable`, so a closure of the wrong shape is
/// refused where the call is written (`rule:types/callable-signature`).
///
/// `wait` is a `Core\Time\Duration` for [`PUT_OPTIONS`]' R12 reason, and there
/// is no spelling here for waiting forever: an omitted one inherits
/// `[cache.process] fill_wait` rather than removing the bound, which is
/// `rule:http-server/no-spelling-for-an-unbounded-wait`'s shape one class over.
/// Both defaults are [`Const::Null`] — the absent callable that variant's own
/// docs describe, and the absent bound the directive then supplies.
const GET_SECRET_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "fill",
        ty: CoreTy::CallableSig(&[], &CoreTy::Instance(SECRET_ENTRY_NAME)),
        default: Const::Null,
    },
    CoreOption {
        name: "wait",
        ty: CoreTy::Instance(crate::time::DURATION_NAME),
        default: Const::Null,
    },
];

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
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Mixed,
                CoreTy::Options(PUT_OPTIONS),
            ],
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
        CoreMethod {
            name: "forget",
            names: &["key"],
            // `Qual::Neutral` for `put`'s reason, and the more plainly: a key
            // that is only ever compared and then dropped reaches no answer at
            // all, so a `tainted` one is as ordinary here as it is there.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_cache_forget",
            doc: Some(&FORGET_DOC),
        },
        CoreMethod {
            name: "putSecret",
            names: &["key", "value", "ttl", "keys"],
            // The value is a **demand** and not an admission, which is
            // `rule:security/secret-qualifier` read through `nvs_types`'
            // assignment relation: it widens onto a qualifier bit and narrows
            // through none, so a plain `string` reaches this parameter and a
            // `secret` one does too, with nothing laundered either way. What
            // the row says is that this member is written for a confidential
            // value — `put`'s `mixed` refuses one, and this is where it goes
            // instead.
            //
            // The `ttl` is required where `put`'s is an option, because a
            // secret never outlives a lifetime someone stated.
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::SecretText(Qual::Neutral),
                CoreTy::Instance(crate::time::DURATION_NAME),
                CoreTy::Array(&crate::keyring::KEY),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_cache_put_secret",
            doc: Some(&PUT_SECRET_DOC),
        },
        CoreMethod {
            name: "getSecret",
            names: &["key", "keys"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Array(&crate::keyring::KEY),
                CoreTy::Options(GET_SECRET_OPTIONS),
            ],
            defaults: &[],
            // `rule:core-api/shape-rules` R7's `?T` where `get` had no `T` to
            // make nullable: a sealed entry holds the one type `putSecret`
            // admitted. The `secret` is a promise rather than a conditional —
            // what comes back out is confidential whatever the `string` that
            // went in was typed as — which is why the return spells
            // `CoreTy::SecretStr` and not the parameter form beside it.
            return_ty: CoreTy::Nullable(&CoreTy::SecretStr),
            symbol: "nvs_core_cache_get_secret",
            doc: Some(&GET_SECRET_DOC),
        },
    ],
    slots: &["tier"],
    constants: &[],
};

/// [`STORE`]'s tier slot, by index — the layout its `slots` names.
const TIER_SLOT: usize = 0;

/// What [`nvs_core_cache_local`] writes into [`TIER_SLOT`].
const LOCAL_TIER: &str = "local";

/// What [`nvs_core_cache_process`] writes into [`TIER_SLOT`].
const PROCESS_TIER: &str = "process";

/// What [`nvs_core_cache_shared`] writes into [`TIER_SLOT`].
const SHARED_TIER: &str = "shared";

/// `[cache.shared] url` — which store the coherent tier is, as
/// `redis://host[:port]` or `unix:/path/to.sock`. Absent, there is no shared tier
/// and `shared()` says so.
const URL: &str = "cache.shared.url";

/// The scheme [`socket`]'s spelling of [`URL`] carries, from the crate that owns
/// the key — `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host` names
/// the transport and no protocol, because the block speaks RESP and nothing else.
///
/// An alias and not a second copy of the string: `[cache.shared]`'s vocabulary is
/// `nvs_config`'s, and this module is what reads a value written in it.
const UNIX: &str = nvs_config::store::UNIX_SCHEME;

/// The spellings of [`URL`] this client reads, named by every refusal.
///
/// The `unix:` half only where there is a transport for it: naming a spelling
/// this build refuses would be advice the operator taking it lands back here
/// with.
#[cfg(unix)]
const READS: &str = "`redis://host[:port]` or `unix:/path/to.sock`";

/// [`READS`] on a build with no Unix-domain transport.
#[cfg(not(unix))]
const READS: &str = "`redis://host[:port]`";

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
        ParamDoc {
            name: "ttl",
            desc: "How long the entry stays readable, counted from this call. Omitted, it stays \
                   until the tier's cap forgets it or another `put` replaces it. A lifetime that \
                   has already run out forgets whatever was under the key and stores nothing.",
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

/// `Core\Cache\Store::forget`'s reference card — `rule:core-api/reference-card`.
const FORGET_DOC: MethodDoc = MethodDoc {
    short: "Takes the entry under `$key` out of this store, whether or not there was one there.",
    params: &[ParamDoc {
        name: "key",
        desc: "The name to forget; `tainted` is admitted, as it is on `put`.",
        shape: &[],
    }],
    ret: "Nothing. A key nothing was stored under is already forgotten, so there is no second \
          answer here for whether there had been an entry — the same reading `get` gives a miss, \
          and for the same reason: on these tiers an entry may be absent at any time.",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "On the shared tier only: the store cannot be reached or refused the command. The \
               in-process tiers have nothing to be unreachable.",
    }],
};

/// `Core\Cache\Store::putSecret`'s reference card — `rule:core-api/reference-card`.
const PUT_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Seals `$value` under the newest key of `$keys` and stores the ciphertext under `$key` — \
            the only way a secret reaches a cache, and no `secret` value ever enters a tier.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The name to store under; `tainted` is admitted, as it is on `put`. It is bound \
                   into the ciphertext, so the same entry moved to another name does not open.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The secret to store. A plain `string` reaches this parameter too, and nothing is \
                   laundered either way — what makes the value confidential is its own type.",
            shape: &[],
        },
        ParamDoc {
            name: "ttl",
            desc: "How long the secret stays readable, counted from this call. Required, where \
                   `put`'s is optional: the lifetime is sealed into the entry as well as given to \
                   the tier, so an entry written back under a longer store lifetime is still past \
                   its own expiry.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The key ring, newest first, as `Core\\SignedCookie` takes one. The newest key \
                   seals; every key opens, so a rotated ring reads what the retired one wrote.",
            shape: &[],
        },
    ],
    ret: "Nothing. What sealing buys is that code which knows an entry's name but not the ring \
          cannot read it, and that a tampered, moved or replayed entry is a miss — **it does not \
          protect a secret from a compromised process**, which holds the ring in the same memory.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or an entry of it is not a key of the construction's length. \
                   A ring that cannot key anything is the program's bug rather than a miss.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "On the shared tier only: the store cannot be reached or refused the write, as \
                   on `put`.",
        },
    ],
};

/// `Core\Cache\Store::getSecret`'s reference card — `rule:core-api/reference-card`.
const GET_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Opens the secret stored under `$key` against every key of `$keys`, or answers `null` \
            when there is none that opens.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The name to read; the one `putSecret` wrote under, since the name is sealed in.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The key ring, newest first. Every key is tried, so an entry sealed under a key \
                   still in the ring opens after a rotation.",
            shape: &[],
        },
        ParamDoc {
            name: "fill",
            desc: "What supplies a miss: a callable answering a \
                   `Core\\Cache\\SecretEntry`, which carries both the secret it fetched and how \
                   long that secret stays good. Exactly one caller in this process runs it, in \
                   that caller's own request and under its own capabilities, while every other \
                   waits; a `fill` that throws releases the waiters with nothing and the next \
                   caller runs it again, so no failure crosses from one request into another.",
            shape: &[],
        },
        ParamDoc {
            name: "wait",
            desc: "How long this caller waits for another's `fill`, defaulting to \
                   `[cache.process] fill_wait`. There is no spelling for waiting forever.",
            shape: &[],
        },
    ],
    ret: "The secret, or `null`. A sealed entry that opens under no key of this ring, or whose own \
          expiry has passed, is a miss like any other — never an error, so a rotated ring re-fetches \
          rather than failing. A plain `get` on the same name is a miss too: the sealed door is the \
          only door.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or an entry of it is not a key of the construction's length — \
                   `putSecret`'s refusal, unchanged. Also a `fill` that asks for the key it is \
                   filling, which would be a wait on itself.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "Another caller in this process was still running `fill` when this call's \
                   `wait` was up, which defaults to `[cache.process] fill_wait`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "On the shared tier only: the store cannot be reached. An entry that is simply \
                   not there is `null`, as it is on `get`.",
        },
    ],
};

/// What a `getSecret` fill answers with: a secret, and how long it stays good.
///
/// An object rather than a pair, because `rule:core-api/shape-rules` R14 makes
/// anything with a lifetime one — and a fill that answered the value alone
/// would leave this store guessing at a lifetime only the fetch knows. A token
/// endpoint says how long its token lasts, and that answer is what belongs in
/// the entry beside the token.
///
/// It answers nothing. A program builds one for a fill to hand back and never
/// reads one, so a member that read a secret back out of it would be a second
/// door onto the value that `getSecret` is the only door onto — which is why
/// the class is a singular noun with one static and no instance member at all
/// (R16, R18).
pub(crate) const SECRET_ENTRY: CoreClass = CoreClass {
    name: SECRET_ENTRY_NAME,
    methods: &[CoreMethod {
        name: "of",
        names: &["value", "ttl"],
        // `Qual::Neutral` on a `secret string`: the value is confidential, and
        // the answer is an object, which carries no qualifier on either axis.
        // That admits a `tainted` secret, which is the ordinary case rather
        // than the exception — a token fetched from an endpoint came off the
        // wire — and the mark says only that this object is not what carries
        // the mark onward.
        params: &[
            CoreTy::SecretText(Qual::Neutral),
            CoreTy::Instance(crate::time::DURATION_NAME),
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(SECRET_ENTRY_NAME),
        symbol: "nvs_core_cache_secret_entry_of",
        doc: Some(&SECRET_ENTRY_OF_DOC),
    }],
    instance: &[],
    // The lifetime is held as the count of nanoseconds the `Duration` carries,
    // not as the `Duration` itself, because [`Lifetime::of`] is what every
    // sealing path takes it through and that is the number it reads. A slot
    // holding the instance would be the same count one dereference further
    // away, and readable by no program either way.
    slots: &["value", "nanos"],
    constants: &[],
};

/// `Core\Cache\SecretEntry::of`'s reference card — `rule:core-api/reference-card`.
const SECRET_ENTRY_OF_DOC: MethodDoc = MethodDoc {
    short: "Builds the entry a `getSecret` fill answers with: the secret the fill fetched, and how \
            long that secret stays good.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The secret to store. A plain `string` reaches this parameter too, and nothing \
                   is laundered either way — what makes the value confidential is its own type.",
            shape: &[],
        },
        ParamDoc {
            name: "ttl",
            desc: "How long the entry stays readable, counted from the write that stores it — \
                   what the fetch itself said, rather than a lifetime the caller guessed at.",
            shape: &[],
        },
    ],
    ret: "The entry. It answers nothing about either half: a program builds one for a fill to hand \
          back, and reads a secret out of `getSecret` instead.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_cache_local" => (nvs_core_cache_local as *const ()).cast(),
        "nvs_core_cache_process" => (nvs_core_cache_process as *const ()).cast(),
        "nvs_core_cache_shared" => (nvs_core_cache_shared as *const ()).cast(),
        "nvs_core_cache_put" => (nvs_core_cache_put as *const ()).cast(),
        "nvs_core_cache_get" => (nvs_core_cache_get as *const ()).cast(),
        "nvs_core_cache_forget" => (nvs_core_cache_forget as *const ()).cast(),
        "nvs_core_cache_put_secret" => (nvs_core_cache_put_secret as *const ()).cast(),
        "nvs_core_cache_get_secret" => (nvs_core_cache_get_secret as *const ()).cast(),
        "nvs_core_cache_secret_entry_of" => (nvs_core_cache_secret_entry_of as *const ()).cast(),
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
/// queue slot, the deadline held beside the payload, and the fat pointers over
/// them, rounded to something a reader can hold in their head. Charged so the
/// cap bounds the *allocation* — a tier full of one-byte entries under a cap
/// that counted payloads alone would be a cap measuring almost none of what it
/// holds.
const ENTRY_OVERHEAD: usize = 64;

/// How long an entry stays readable, as the call wrote it.
///
/// The duration rather than the deadline it becomes, because the tiers keep it
/// two ways: the in-process ones turn it into an [`Instant`] at the moment of
/// the write, and the shared one is told a length of time and keeps its own
/// clock. A deadline here would have to be turned back into a remaining length
/// against a clock that tier does not share.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lifetime {
    /// No `ttl` was written: the entry stays until the cap forgets it or a
    /// `put` replaces it, which is § 1's contract for every entry.
    Forever,
    /// The `ttl` that was written, counted from the moment of the write. Zero
    /// is a lifetime that is already over, which is where a negative one lands
    /// too — [`Lifetime::of`] is where that happens.
    For(Duration),
}

impl Lifetime {
    /// The lifetime `nanos` names, with everything at or below zero reading as
    /// one that is already over.
    ///
    /// A negative `ttl` is not refused, because `put` has none to make: § 3
    /// turns a tier at its cap into an eviction rather than a throw precisely
    /// so that this store keeps the one contract it has, and a second answer
    /// for a duration a program computed to be negative would be the failure
    /// mode it says it has none of.
    fn of(nanos: i64) -> Self {
        Self::For(Duration::from_nanos(nanos.max(0).unsigned_abs()))
    }

    /// Whether this lifetime is over at the moment it is written.
    fn elapsed(self) -> bool {
        matches!(self, Self::For(ttl) if ttl.is_zero())
    }

    /// The instant an entry written now stops being readable, or `None` for
    /// one that never does — which a lifetime the monotonic clock cannot name
    /// an end for also is, there being no instant left to compare against.
    fn until(self) -> Option<Instant> {
        match self {
            Self::Forever => None,
            Self::For(ttl) => Instant::now().checked_add(ttl),
        }
    }
}

/// One entry of an in-process tier: the payload and when it stops being
/// readable.
///
/// The deadline is beside the payload rather than in a queue of its own,
/// because the only thing that ever asks is the read that just found the entry
/// — a second structure ordered by deadline would buy a sweeper this tier does
/// not have and cannot afford, `get` being a read lock and nothing more.
struct Entry {
    /// The entry's `rule:classes/serialize-is-a-closed-format` payload.
    payload: Box<[u8]>,
    /// The instant this stops being readable, or `None` for an entry only the
    /// cap or an overwrite takes out.
    until: Option<Instant>,
}

/// A tier's entries and what they cost, together, because a size is only
/// meaningful against the map it measures: a second cell holding the number
/// beside the map would be two writers of one fact.
///
/// Generic over the key handle rather than written once per tier: the local
/// tier's keys are `Rc` because a `thread_local` belongs to one thread, the
/// process tier's are `Arc` because every core of the process reaches the same
/// map, and the eviction policy
/// `rule:concurrency/the-process-tier-is-one-store-per-process` states the two
/// tiers share — by write age, oldest first, an arrival too large for the tier
/// forgotten as it lands — is one implementation that cannot come to disagree
/// with itself.
struct Entries<K> {
    /// The entries themselves, each a payload and the deadline it is readable
    /// until. Keys are handles so that `order` below names one without a
    /// second copy of the bytes.
    entries: HashMap<K, Entry>,
    /// Every live key, in the order it was **first** written, which is the
    /// order [`Entries::forget_oldest`] gives them up in.
    ///
    /// First written rather than last: an overwrite keeps its place, so this
    /// holds exactly one slot per live key and a hot key rewritten a million
    /// times leaves nothing behind it. Ordering by last write instead would
    /// need a slot per *write*, which is the O(requests served) growth
    /// `AGENTS.md` calls a leak rather than a policy.
    order: VecDeque<K>,
    /// What `entries` costs by [`charged`], maintained on every write so that
    /// the cap is a comparison rather than a walk of the map.
    held: usize,
}

/// The empty tier, written out rather than derived: `#[derive(Default)]` would
/// ask the key handle for a default it has no meaning for, and no tier starts
/// with a key in it.
impl<K> Default for Entries<K> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            held: 0,
        }
    }
}

impl<K: Borrow<[u8]> + Clone + Eq + Hash + for<'a> From<&'a [u8]>> Entries<K> {
    /// Writes `payload` under `key` for as long as `lifetime` names, forgetting
    /// whatever it has to.
    ///
    /// `cap` is `None` for the `false` an operator writes for no ceiling. The
    /// bytes are copied into an allocation of the store's own rather than taken
    /// from the caller's `Vec`, so that everything this tier holds was
    /// allocated where [`store_put`]'s bracket will free it again.
    fn put(&mut self, key: &[u8], payload: &[u8], lifetime: Lifetime, cap: Option<usize>) {
        let incoming = charged(key.len(), payload.len());

        if lifetime.elapsed() || cap.is_some_and(|cap| incoming > cap) {
            // An arrival with nothing left to live, and one the tier could not
            // hold even empty, take the same exit: it is forgotten as it lands
            // rather than failing the write — § 3 evicts, and § 1 has already
            // told the caller a `get` may answer nothing. What was under the
            // key goes with it, because `put` replaced it.
            self.forget(key);
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

        let handle = match self.entries.remove_entry(key) {
            Some((held, previous)) => {
                self.held -= charged(key.len(), previous.payload.len());
                held
            }
            None => {
                let fresh = K::from(key);
                self.order.push_back(fresh.clone());
                fresh
            }
        };
        self.held += incoming;
        self.entries.insert(
            handle,
            Entry {
                payload: Box::from(payload),
                until: lifetime.until(),
            },
        );
    }

    /// Takes the entry under `key` out, and answers nothing about whether
    /// there was one.
    ///
    /// The slot `order` holds for it is left behind rather than searched for:
    /// it is the one slot that can outlive its entry, [`Entries::forget_oldest`]
    /// skips it when it reaches the front, and a scan of the queue to remove it
    /// would price every forget at the length of the tier.
    fn forget(&mut self, key: &[u8]) {
        if let Some(previous) = self.entries.remove(key) {
            self.held -= charged(key.len(), previous.payload.len());
        }
    }

    /// The payload under `key` while it is still readable, and `None` once it
    /// is not.
    ///
    /// An entry past its deadline is **left where it is** rather than taken
    /// out: this is `&self` so that a lookup on the process tier holds a read
    /// lock and nothing more, which is the property
    /// `rule:concurrency/the-process-tier-is-one-store-per-process` fixes about
    /// a `get`. Its bytes stay against the cap until the eviction order reaches
    /// it or a `put` replaces it, which is footprint spent on every core
    /// reading one hot key at the same moment.
    ///
    /// The clock is read only once an entry with a deadline is in hand, so a
    /// tier nothing wrote a `ttl` to pays nothing for the question.
    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        let held = self.entries.get(key)?;
        match held.until {
            Some(until) if Instant::now() >= until => None,
            _ => Some(&held.payload),
        }
    }

    /// Forgets the entry whose key was written longest ago, and answers whether
    /// there was one to forget — which is what bounds [`Entries::put`]'s loop.
    ///
    /// A slot naming nothing is skipped rather than counted: it is the oversized
    /// write above, and skipping it here is what keeps that case from paying for
    /// a scan of the queue at the time it happens.
    fn forget_oldest(&mut self) -> bool {
        while let Some(key) = self.order.pop_front() {
            let raw = bytes(&key);
            if let Some(previous) = self.entries.remove(raw) {
                self.held -= charged(raw.len(), previous.payload.len());
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

/// A key handle's own bytes.
///
/// Named as a function because a handle implements `Borrow` twice — once for the
/// bytes and once for itself, through the blanket impl every type has — so
/// `key.borrow()` at a call site does not say which one it meant, while the
/// return type here does.
fn bytes<K: Borrow<[u8]>>(key: &K) -> &[u8] {
    key.borrow()
}

thread_local! {
    /// The local tier itself: this core's entries and nothing shared with any
    /// other core.
    ///
    /// A `thread_local` rather than anything reachable from another thread is
    /// § 1's "no coherence between cores" as a *representation* rather than as
    /// a rule to remember — the runtime is thread-per-core, so a store another
    /// core could reach would need a lock this tier is defined not to have.
    static ENTRIES: RefCell<Entries<Rc<[u8]>>> = RefCell::new(Entries::default());
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
///
/// **What the store allocates is detached from the request that wrote it.** The
/// copy [`Entries::put`] keeps, the map and queue that name it, and the frees an
/// eviction makes are all inside `nvs_runtime::budget::Detached`, so they move
/// the process's balance and not the reading any request is measured by —
/// `rule:concurrency/cache-memory-is-charged-to-the-core`'s *not attributable to
/// a request* as arithmetic rather than as a sentence. Without it, the request
/// that evicts an entry is credited for bytes an earlier request allocated,
/// which is a ceiling any program widens at will: fill the tier under one
/// request and evict it under the next.
///
/// `payload` is the caller's own temporary, allocated under the request and
/// released here outside the bracket, under the same request. That symmetry is
/// what a bracket owes — a block allocated on one balance and freed on the other
/// corrupts both — and it is why the entry is copied rather than this `Vec`
/// being kept. What that spends is one copy of the payload per local write, on a
/// path that has already encoded the value it is storing.
pub(crate) fn store_put(key: &[u8], payload: Vec<u8>, lifetime: Lifetime, cap: Option<usize>) {
    {
        let _bracket = budget::Detached::begin();
        ENTRIES.with_borrow_mut(|local| local.put(key, &payload, lifetime, cap));
    }
    // Where it was allocated: the caller's `Vec` is the request's, and what the
    // tier holds is the copy made above.
    drop(payload);
}

/// This core's payload for `key`, or `None` — which is an ordinary answer and
/// not a failure, per § 1, and also the answer for an entry the cap made room
/// by forgetting and for one whose lifetime has run out. Crate-visible for
/// [`store_put`]'s reason.
pub(crate) fn store_get(key: &[u8]) -> Option<Vec<u8>> {
    ENTRIES.with_borrow(|local| local.get(key).map(<[u8]>::to_vec))
}

/// Takes `key`'s entry out of this core's tier, whether or not there was one.
///
/// Under the same [`budget::Detached`] bracket [`store_put`] holds, because the
/// release has to land on the balance the allocation did — a free credited to
/// the request that happened to call `forget` is the asymmetry
/// `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` says a
/// bracket owes against, arriving from the other direction.
pub(crate) fn store_forget(key: &[u8]) {
    let _bracket = budget::Detached::begin();
    ENTRIES.with_borrow_mut(|local| local.forget(key));
}

/// `[cache.process] max_size` — what this process's entries may hold together.
const PROCESS_MAX_SIZE: &str = "cache.process.max_size";

/// `[cache.process] fill_wait` — how long a caller waits for the one filler in
/// this process before it gives up on that fill.
const PROCESS_FILL_WAIT: &str = "cache.process.fill_wait";

/// The wait a call inherits when neither its own `wait` nor `nvs.toml` names
/// one, which is the figure `nvs_config`'s `default.toml` states beside that
/// key.
///
/// Long enough for a token endpoint on the other side of the internet to answer
/// once, and short enough that a request behind a fetch that will never answer
/// ends as a request rather than holding a core for as long as the fetch takes.
const DEFAULT_FILL_WAIT: Duration = Duration::from_secs(5);

/// How many pieces the process map is cut into, so that two cores writing keys
/// that hash apart do not wait on each other.
///
/// A fixed constant rather than a directive or a function of `[server] workers`,
/// which `rule:concurrency/the-process-tier-is-one-store-per-process` fixes for
/// a reason about *when* the map exists: it is created before a worker does
/// under `nvs serve`, and there is one worker under `nvs run`, so the number
/// cannot be read from either.
const SHARDS: usize = 64;

/// The process tier itself: the one map every core of this process reads and
/// writes, in [`SHARDS`] pieces behind a read/write lock each.
///
/// A `static` and not something a core is handed, because there is exactly one
/// of it per process and a map reachable only through whatever had a reference
/// would be a second answer to "which store is this" — the tier *is* the
/// process's. Creating it allocates nothing at all, since an empty [`HashMap`]
/// and an empty [`VecDeque`] each hold no heap, so [`arm_process_tier`] costs
/// the fleet nothing and buys that no worker is the one that built the map.
static PROCESS: LazyLock<[Shard; SHARDS]> =
    LazyLock::new(|| std::array::from_fn(|_| Shard::new(Entries::default())));

/// One piece of [`PROCESS`]: a share of this process's entries under one lock.
type Shard = RwLock<Entries<Arc<[u8]>>>;

/// Creates the process tier, for a caller that is about to start the cores that
/// will share it — `nvs serve` before its first worker accepts.
///
/// Calling it is not a precondition for using the tier: the first operation on a
/// process that never called it creates the map on the spot. What it buys is
/// that the creation is not on a request path at all, which is the claim
/// `rule:concurrency/the-process-tier-is-one-store-per-process` makes when it
/// says the map exists before the workers do.
pub fn arm_process_tier() {
    LazyLock::force(&PROCESS);
}

/// The shard `key` lives in.
///
/// A hash of the whole key and not a slice of it, because a program's keys share
/// prefixes — one namespace per feature is the ordinary shape, and every key
/// here already carries [`scoped`]'s prefix — so slicing would put a whole
/// namespace, or a whole application, on one lock.
fn shard_of(key: &[u8]) -> &'static Shard {
    let mut hash = DefaultHasher::new();
    key.hash(&mut hash);
    let index = usize::try_from(hash.finish() % SHARDS as u64)
        .expect("a remainder under `SHARDS` fits a `usize` on every host");
    &PROCESS[index]
}

/// What one shard may hold: the tier's cap, divided evenly.
///
/// Each shard evicts against its own share rather than against the tier's total,
/// because a total is one number every writer would have to agree on and that is
/// a lock over every shard on the write path — the lock sharding exists to
/// remove. It costs two things, both stated rather than discovered: a perfectly
/// skewed key set holds less than the cap, since a shard fills while its
/// neighbours are empty, which is the direction a footprint ceiling must err in;
/// and the largest entry the tier holds is a share rather than the whole cap,
/// past which an arrival is forgotten as it lands exactly as an oversized one is
/// on the local tier.
fn shard_cap(cap: Option<usize>) -> Option<usize> {
    cap.map(|cap| cap / SHARDS)
}

/// `[cache.process] max_size` as a byte count, or [`DEFAULT_MAX_SIZE`], and
/// `None` for the `false` that removes the ceiling.
///
/// The same figure the local tier ships, read per operation for [`local_cap`]'s
/// reason — and the same number means *less* memory here than there, because
/// this map is held once per process where that one is held once per core.
fn process_cap(ctx: &Ctx) -> Option<usize> {
    let Some(written) = configured(ctx, PROCESS_MAX_SIZE) else {
        return Some(DEFAULT_MAX_SIZE);
    };
    match Quantity::parse(PROCESS_MAX_SIZE, Unit::Bytes, &Setting::Text(written)) {
        Ok(Quantity::Unbounded) => None,
        Ok(Quantity::Bytes(bytes)) => Some(usize::try_from(bytes).unwrap_or(usize::MAX)),
        _ => Some(DEFAULT_MAX_SIZE),
    }
}

/// The `{wait?: Duration}` written in argument slot `at`, or
/// `[cache.process] fill_wait`, or [`DEFAULT_FILL_WAIT`].
///
/// An omitted option inherits the directive rather than removing the bound, and
/// a directive naming none inherits the figure above, so a caller waits on
/// another request's `fill` for a time it can name and never for as long as that
/// `fill` takes. Neither place has a spelling for waiting until the filler
/// answers, which is `rule:http-server/no-spelling-for-an-unbounded-wait`'s
/// shape one class over: the guarantee is the absence of the spelling rather
/// than a check.
///
/// A directive that will not parse, or that parses to nothing, leaves the
/// shipped bound standing — [`timeout_of`]'s reasoning, that `nvs.toml` is
/// refused where it is loaded, and `nvs_config::store` has already failed both
/// of those spellings with `E0642` where the operator can still see the file. A
/// `wait` a program writes is taken as written, and one written negative is no
/// wait at all rather than its magnitude.
///
/// # Errors
///
/// [`crate::time::nanos_of`]'s fatal for a slot holding neither a duration nor
/// the `null` [`GET_SECRET_OPTIONS`] defaults it to, which the option's own
/// declared type has already refused at the call site.
fn wait_of(ctx: &Ctx, args: &[Value], at: usize, member: &str) -> Result<Duration, Fault> {
    if let Some(held) = args.get(at)
        && !matches!(held.tag(), Some(Tag::Null))
    {
        let nanos = crate::time::nanos_of(args, at, member)?;
        return Ok(Duration::from_nanos(u64::try_from(nanos).unwrap_or(0)));
    }
    let written = configured(ctx, PROCESS_FILL_WAIT).and_then(|text| {
        Quantity::parse(PROCESS_FILL_WAIT, Unit::Duration, &Setting::Text(text)).ok()
    });
    Ok(match written {
        Some(Quantity::Nanos(nanos)) if nanos > 0 => Duration::from_nanos(nanos),
        _ => DEFAULT_FILL_WAIT,
    })
}

/// How often a caller waiting on somebody else's `fill` looks at the table
/// again.
///
/// A look on a tick rather than a wake, because the caller it is waiting for is
/// on another core and a `nvs_runtime::host::Waker` belongs to the one task that
/// made it — there is nothing a table every core shares could hold that would
/// reach across. `Core\Sse`'s cross-core tick is the same shape for the same
/// reason, and both end the day a wake can cross cores.
///
/// **What it spends:** one wakeup per waiting caller per tick, for as long as a
/// fill is in flight, and nothing at all on a hit. The number trades priority 3
/// against itself — a shorter tick buys the waiters' latency with wakeups —
/// against a fetch that is an outbound request and so is measured in tens of
/// milliseconds at best.
const FILL_TICK: Duration = Duration::from_millis(10);

/// The fills this process is running at this moment: one entry per key being
/// filled, holding the address of the [`Ctx`] whose call is filling it.
///
/// One table for every tier, keyed as [`scoped`] keys the process tier, because
/// what `rule:concurrency/a-secret-fill-runs-once-per-process` elects is one
/// caller per **process** per name: a second fetch of the same secret is the
/// cost the election exists to remove, whichever store the answer is written
/// to. It is not once per fleet, which is a lease over the shared tier and
/// nothing this key spells.
///
/// The owner is an address rather than a task or a thread identity because it
/// answers one question: whether the caller now asking is the very call that is
/// filling, which is a `fill` asking for its own key and so a wait on itself. A
/// context is alive for as long as the call holding the key is, so an entry
/// never names a freed one, and two requests sharing a core hold two contexts.
///
/// **What it spends:** one key and one machine word per fill in flight, on the
/// detached balance — O(concurrent fills), and nothing between them.
static FILLS: LazyLock<Mutex<HashMap<Vec<u8>, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// One caller's hold on a key for as long as it is filling it.
///
/// A guard rather than a pair of calls because **every** ending has to release:
/// a `fill` that throws, a request cancelled while it fetched and the ordinary
/// answer alike leave the key to the next caller rather than wedging it for the
/// life of the process.
struct Filling {
    /// The table key this hold was taken under, which is this frame's own
    /// allocation — the table's copy is [`elected`]'s, made and freed on the
    /// process's balance.
    key: Vec<u8>,
}

impl Drop for Filling {
    fn drop(&mut self) {
        // The entry the table holds is the process's, so its release lands on
        // the balance the claim moved —
        // `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`'s
        // symmetry, owed here exactly as [`process_put`] owes it. This frame's
        // own copy of the key is freed after the bracket, where it was
        // allocated.
        let _bracket = budget::Detached::begin();
        FILLS
            .lock()
            .expect("the fill table's lock is never poisoned")
            .remove(&self.key);
    }
}

/// Elects this call as the one caller in the process that fills `key`, or
/// answers `None` for a key another call is already filling.
///
/// # Errors
///
/// A `LogicError` when the call already holding `key` is this one, which is a
/// `fill` asking for the key it is filling. Waiting would be a wait on itself,
/// so it is said here rather than discovered when the bound runs out.
fn elected(ctx: &Ctx, key: &[u8], who: &str) -> Result<Option<Filling>, Fault> {
    let mine = std::ptr::from_ref(ctx).addr();
    let mut fills = FILLS
        .lock()
        .expect("the fill table's lock is never poisoned");
    match fills.get(key) {
        Some(&owner) if owner == mine => Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{who}: this call's `fill` asked for the key it is filling, which would be a \
                 wait on itself"
            ),
        )),
        Some(_) => Ok(None),
        None => {
            {
                let _bracket = budget::Detached::begin();
                fills.insert(key.to_vec(), mine);
            }
            Ok(Some(Filling { key: key.to_vec() }))
        }
    }
}

/// Waits for whichever call holds `key` to be done with it and then answers
/// what `look` finds, for at most `wait`.
///
/// The state this waits on is another core's, so it is looked at on a
/// [`FILL_TICK`] rather than woken, and the park is where the core goes back to
/// its neighbours. The clock is what ends the wait: a waiter is released with
/// whatever the filler left, which is the value on the ordinary path and
/// nothing at all where the `fill` threw —
/// `rule:concurrency/a-secret-fill-runs-once-per-process` makes that a miss for
/// this caller rather than another request's failure crossing into it.
///
/// # Errors
///
/// A thrown `TimeoutError` for a fill still running when `wait` was up, which
/// is `rule:http-server/no-spelling-for-an-unbounded-wait`'s ending one class
/// over: the caller gets its request back rather than the core standing still
/// for as long as somebody else's fetch takes. Whatever `look` answers with,
/// and [`Ctx::cancel`]'s status for a request cancelled while it waited, which
/// no `catch` sees.
fn waited<T>(
    ctx: &mut Ctx,
    key: &[u8],
    wait: Duration,
    who: &str,
    look: impl FnOnce(&mut Ctx) -> Result<T, Fault>,
) -> Result<T, Fault> {
    let until = Instant::now() + wait;
    loop {
        let filling = FILLS
            .lock()
            .expect("the fill table's lock is never poisoned")
            .contains_key(key);
        if !filling {
            return look(ctx);
        }
        let now = Instant::now();
        if now >= until {
            return Err(Fault::thrown_as(
                ThrownClass::Timeout,
                format!(
                    "{who}: another request in this process is still supplying this key, and \
                     this call's `wait` of {wait:?} is up"
                ),
            ));
        }
        let tick = until.min(now + FILL_TICK);
        match nvs_runtime::host::with_current(|host| host.park(Some(tick))) {
            Some(nvs_runtime::host::Woken::Cancelled) => return Err(ctx.cancel()),
            Some(_) => {}
            // No scheduler on this thread at all, which is a `nvs run` and a
            // `#[test]`: there is no core here to give back, so the wait is the
            // sleep `nvs_host::timer::park_until` performs off a core for the
            // same reason.
            None => std::thread::sleep(tick.saturating_duration_since(Instant::now())),
        }
    }
}

/// The key an entry is really held under: the `[[app]]` it was written for and
/// the configuration generation that was live when it was written, ahead of the
/// key the program wrote.
///
/// That scoping is `rule:concurrency/the-process-tier-is-one-store-per-process`
/// and it answers two questions at once — two applications on one server never
/// read each other's entries, and a reload never serves an entry written under
/// the configuration it replaced. The application is the most specific `[[app]]`
/// block that matched this entry file, which is the name an operator wrote; a
/// deployment with no block at all is one application and names none.
///
/// **The generation is [`nvs_config::Snapshot`]'s own number and not the address
/// of the `Arc` holding it**, which is the one difference from
/// `nvs_runtime::pool::Ticket`'s database-pool key. That ticket holds its
/// generation alive for as long as a connection names it, so the address cannot
/// be reused underneath it; an entry in this map cannot hold one, because
/// freeing a boot-time allocation inside a [`budget::Detached`] bracket is
/// exactly the asymmetry
/// `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` says a
/// bracket owes against. A number that is never reused needs nothing held.
///
/// Two `NUL` separators and not one: neither a path nor a decimal number can
/// contain one, so the three parts are unambiguous however a program spells its
/// own key — a key that spells another application's prefix is still the third
/// part and still inside the application that wrote it.
fn scoped(ctx: &Ctx, key: &[u8]) -> Vec<u8> {
    let snapshot = ctx.config().map(nvs_config::Request::snapshot);
    let app = application(ctx);
    let generation = snapshot.map_or(0, |snapshot| snapshot.generation);

    let mut real = Vec::with_capacity(app.len() + key.len() + 24);
    real.extend_from_slice(app);
    real.push(0);
    real.extend_from_slice(generation.to_string().as_bytes());
    real.push(0);
    real.extend_from_slice(key);
    real
}

/// The application this request belongs to, as the bytes that name it, or
/// nothing at all on a context nobody configured.
///
/// The innermost configured block, which is the application in the sense both
/// callers mean: [`scoped`] keeps one application's process-tier entries away
/// from another's, and [`bound`] keeps one application's sealed entries from
/// opening inside another. One reader so the two cannot come to disagree about
/// which block that is.
///
/// `pub(crate)` because [`crate::session`]'s sealed pair binds a record entry to
/// the same application through the same [`bound`], and a second reading of
/// which block that is would be the disagreement this one reader exists to
/// prevent.
pub(crate) fn application(ctx: &Ctx) -> &[u8] {
    ctx.config()
        .map(nvs_config::Request::snapshot)
        .and_then(|snapshot| snapshot.blocks.last())
        .map(|block| block.as_os_str().as_encoded_bytes())
        .unwrap_or_default()
}

/// The octet this store's sealed entries open their additional data with.
///
/// A number rather than a name because it is never read back and never shown:
/// the whole of what it does is differ from whatever the next construction over
/// a key ring picks for itself, so a `Core\SignedCookie` answer pasted into
/// this store opens under none of the ring it was sealed with.
///
/// [`bound`] takes it as an argument rather than reading it here, because
/// `crate::session`'s sealed pair writes the same additional data under an octet
/// of its own: one construction, and two numbers that differ, is the whole of
/// why a value sealed for a cache does not open as a session secret.
pub(crate) const SEAL_DOMAIN: u8 = 1;

/// The key space sealed entries live in.
///
/// **A space of its own is why `get` answers `null` for a sealed entry** rather
/// than throwing over a payload it cannot read: the sealed door is the only
/// door, and a miss is what every other name that was never written gives. The
/// separator is a `NUL` for [`scoped`]'s reason. A program that writes these
/// octets into its own key does reach the space — a cache key is arbitrary text
/// — and what a `get` there finds is a payload `Core\Serialize` refuses, which
/// is the `ParseError` that member already documents; no secret is readable
/// either way, the payload being ciphertext.
const SEALED_SPACE: &[u8] = b"\0secret\0";

/// The name `key`'s sealed entry is stored under.
///
/// `pub(crate)` because [`crate::session`]'s record holds its sealed values in
/// the same space — a record is an array rather than a tier, and what the two
/// share is the rule that one program-visible name reaches a sealed value only
/// through the sealed door.
pub(crate) fn sealed_key(key: &[u8]) -> Vec<u8> {
    let mut stored = Vec::with_capacity(SEALED_SPACE.len() + key.len());
    stored.extend_from_slice(SEALED_SPACE);
    stored.extend_from_slice(key);
    stored
}

/// What a sealed entry is bound to: this construction, the application that
/// wrote it, and the name it was written under.
///
/// The AEAD's additional data rather than a prefix of the plaintext, so the tag
/// covers it without the value needing a frame to be told apart from it. It
/// travels with neither the ciphertext nor the ring, so the reader states it
/// again from what it knows and an entry lifted into another application or
/// moved to another name opens under none of them —
/// [`crate::crypto::open_under`]'s `None` is where that becomes the miss this
/// module reads.
///
/// The configuration's generation is deliberately absent where [`scoped`] has
/// it: a reload renames every process-tier key on purpose, and doing the same
/// here would make an operator's reload silently discard every secret the
/// shared tier is holding for the whole fleet.
///
/// `app` is [`application`]'s answer at every call site; it is an argument
/// rather than read here so that the two halves a sealed entry is bound to can
/// be varied one at a time by whatever is asking. `domain` is the door's own
/// octet — [`SEAL_DOMAIN`] here, `crate::session`'s beside it — so that the two
/// doors share this construction instead of each writing one.
pub(crate) fn bound(domain: u8, app: &[u8], key: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(app.len() + key.len() + 2);
    aad.push(domain);
    aad.extend_from_slice(app);
    aad.push(0);
    aad.extend_from_slice(key);
    aad
}

/// How wide the expiry written ahead of a sealed value is.
const EXPIRY_LEN: usize = 8;

/// How wide the lifetime written after it is.
const LIFETIME_LEN: usize = 8;

/// What fraction of a lifetime the refresh-ahead window is — the last fifth,
/// fixed in [ADR 0181](/docs/decisions/0181.md).
///
/// A fraction rather than a duration because the tier does not know what a
/// lifetime means to its caller: a fifth of a minute and a fifth of a day are
/// both "nearly over" to whoever wrote them.
const REFRESH_WINDOW: i64 = 5;

/// What an opened seal holds: the secret, and whether it is inside the last
/// fifth of the lifetime it was written for.
///
/// One shape for both readings of that pair, where `T` is the secret's bytes
/// while the plaintext is still borrowed and the `string` value a member hands
/// back once it is not — so the flag cannot come to mean one thing in
/// [`sealed_value`] and another in [`opened`].
struct Held<T> {
    /// The secret itself.
    value: T,
    /// Whether a `fill` written beside this read replaces the entry now, which
    /// is `rule:concurrency/a-secret-fill-runs-once-per-process`'s refresh
    /// ahead of an expiry: the entry is still answered to every caller, and
    /// exactly one of them fetches its replacement.
    refreshing: bool,
}

/// The plaintext a sealed entry holds: when the secret stops being readable,
/// how long it was given, and the secret.
///
/// Milliseconds since the epoch, big-endian, because the reading has to survive
/// the shared tier — a monotonic instant means nothing in the process that
/// reads it back, and [`Entry::until`]'s deadline is the writing core's own
/// clock. The value is the rest of the buffer and carries no length of its own:
/// both numbers are fixed-width, and the tag covers the whole of it.
///
/// **The lifetime is sealed in beside the expiry** because [`REFRESH_WINDOW`]
/// is a fraction of it and there is nowhere else to read it from: the shared
/// tier keeps no deadline this process can see, and a tier's own lifetime is
/// what an operator may have copied the entry forward under.
fn sealed_plaintext(expiry: i64, lifetime: i64, value: &[u8]) -> Vec<u8> {
    let mut plain = Vec::with_capacity(EXPIRY_LEN + LIFETIME_LEN + value.len());
    plain.extend_from_slice(&expiry.to_be_bytes());
    plain.extend_from_slice(&lifetime.to_be_bytes());
    plain.extend_from_slice(value);
    plain
}

/// What an opened entry holds at `now`, or `None` for one whose sealed expiry
/// has passed.
///
/// **Whatever the store says.** A tier slow to forget an entry, and one an
/// operator copied forward under a longer store lifetime, both answer a miss
/// here: the expiry the secret was sealed with is the one that decides, and it
/// is under the tag rather than beside it.
///
/// A buffer too short to carry both numbers is that same miss, which is what
/// keeps a truncated payload from being read as a secret.
fn sealed_value(plain: &[u8], now: i64) -> Option<Held<&[u8]>> {
    let (expiry, rest) = plain.split_first_chunk::<EXPIRY_LEN>()?;
    let (lifetime, value) = rest.split_first_chunk::<LIFETIME_LEN>()?;

    let expiry = i64::from_be_bytes(*expiry);
    if now >= expiry {
        return None;
    }
    Some(Held {
        value,
        refreshing: expiry.saturating_sub(now) <= i64::from_be_bytes(*lifetime) / REFRESH_WINDOW,
    })
}

/// What every step of one sealed operation holds in common: which tier it is
/// over, the name the entry is under, the ring it seals or opens against, and
/// the two spellings of the member a refusal names.
///
/// One shape rather than the same five arguments threaded through each step, so
/// that a step cannot be handed one member's ring under another member's name.
struct Sealing<'a> {
    /// Which tier this operation is over.
    tier: Tier,
    /// The name the program wrote, which [`sealed_key`] turns into the one the
    /// entry is stored under and [`bound`] seals in.
    key: &'a [u8],
    /// The ring, newest first.
    ring: &'a nvs_runtime::NvsArray,
    /// The member as the registry names it, for [`on_shared`]'s refusals.
    member: &'a str,
    /// The member as a refusal spells it, which is `Class::member`.
    who: &'a str,
}

/// Seals `value` under the ring's newest key and writes it to the tier under
/// the sealed name, for the lifetime `nanos` names.
///
/// One writer for both doors — the `putSecret` a program writes and the `fill`
/// `getSecret` runs — so that what a fill leaves behind is the entry the other
/// door would have written: the same seal, the same [`bound`] it may not be
/// moved away from, and the same lifetime written twice for
/// [`nvs_core_cache_put_secret`]'s reason.
///
/// # Errors
///
/// [`crate::keyring`]'s `LogicError` for a ring whose newest entry is not a
/// key, and a thrown `IOError` on the shared tier for a store that cannot be
/// reached or that refuses the write.
fn seal_and_store(ctx: &mut Ctx, at: &Sealing<'_>, value: &[u8], nanos: i64) -> Result<(), Fault> {
    let &Sealing {
        tier,
        key,
        ring,
        member,
        who,
    } = at;
    let (slot, held) = crate::keyring::newest(ring);
    let cipher = crate::keyring::cipher_at(&held, slot, who)?;

    // Rounded up rather than down, so that a lifetime shorter than this
    // clock's resolution is still a lifetime: an entry whose sealed expiry
    // had passed before the write would be a miss no program could account
    // for. A zero stays zero, which is the lifetime that is already over.
    let ttl = nanos.max(0).saturating_add(999_999) / 1_000_000;
    let expiry = clock_ms(ctx, member)?.saturating_add(ttl);
    let aad = bound(SEAL_DOMAIN, application(ctx), key);
    let plain = sealed_plaintext(expiry, ttl, value);
    let sealed = crate::crypto::seal_under(ctx, &cipher, &aad, &plain, who)?;

    let stored = sealed_key(key);
    let lifetime = Lifetime::of(nanos);
    match tier {
        Tier::Local => store_put(&stored, sealed, lifetime, local_cap(ctx)),
        Tier::Process => {
            process_put(&scoped(ctx, &stored), sealed, lifetime, process_cap(ctx));
        }
        // [`nvs_core_cache_put`]'s three commands, unchanged: the store keeps
        // its own clock, so it is told a length of time, and a lifetime already
        // over leaves it in the state the in-process tiers are left in.
        Tier::Shared => on_shared(STORE_NAME, member, |open| {
            if lifetime.elapsed() {
                return open.del(&stored);
            }
            match lifetime {
                Lifetime::Forever => open.set(&stored, &sealed),
                Lifetime::For(ttl) => open.set_expiring(&stored, &sealed, ttl),
            }
        })?,
    }
    Ok(())
}

/// The secret `tier` holds under `key`, or `None` for every way of not opening
/// one.
///
/// **Every way of not opening is the same miss**, and never an error: a ring
/// that has rotated past the key this entry was sealed under, an entry moved to
/// another name or lifted into another application, a tampered payload, and one
/// past its sealed expiry all answer `None`. A caller that could tell them apart
/// would learn something about the ring from an entry it cannot read, and a
/// rotated ring is meant to re-fetch rather than fail. A ring that is wrong in
/// itself is a different thing and throws.
///
/// Every key of the ring is tried rather than the newest alone, which is
/// `Core\SignedCookie`'s walk and buys the same thing: an entry written before a
/// rotation stays readable until it expires on its own.
///
/// # Errors
///
/// [`crate::keyring`]'s `LogicError` for an entry of the ring that is not a key,
/// and a thrown `IOError` on the shared tier for a store that cannot be reached.
fn opened(ctx: &Ctx, at: &Sealing<'_>) -> Result<Option<Held<Value>>, Fault> {
    let &Sealing {
        tier,
        key,
        ring,
        member,
        who,
    } = at;
    let stored = sealed_key(key);
    let held = match tier {
        Tier::Local => store_get(&stored),
        Tier::Process => process_get(&scoped(ctx, &stored)),
        Tier::Shared => on_shared(STORE_NAME, member, |open| open.get(&stored))?,
    };
    let Some(sealed) = held else {
        return Ok(None);
    };

    let aad = bound(SEAL_DOMAIN, application(ctx), key);
    let now = clock_ms(ctx, member)?;
    for (slot, entry) in crate::keyring::entries(ring) {
        let cipher = crate::keyring::cipher_at(&entry, slot, who)?;
        if let Some(plain) = crate::crypto::open_under(&cipher, &aad, &sealed, who)?
            && let Some(held) = sealed_value(&plain, now)
        {
            return Ok(Some(Held {
                value: Value::str(NvsStr::new(held.value)),
                refreshing: held.refreshing,
            }));
        }
    }
    Ok(None)
}

/// Runs `fill` as the one caller in this process filling `key`, storing what it
/// answers and handing the secret back — or `None` for a key another call is
/// already filling.
///
/// The hold is released by its own `Drop` whichever way this ends, and it is
/// released **after** the entry is written: a waiter let go a step earlier
/// would look at a store the fill had not reached yet.
///
/// # Errors
///
/// [`elected`]'s `LogicError` for a `fill` that asked for the key it is
/// filling, whatever the `fill` itself threw — which reaches the request that
/// ran it and no other — and [`supplied`]'s.
fn filled(
    ctx: &mut Ctx,
    at: &Sealing<'_>,
    filling: &[u8],
    fill: Value,
) -> Result<Option<Value>, Fault> {
    let Some(_held) = elected(ctx, filling, at.who)? else {
        return Ok(None);
    };
    let entry = nvs_runtime::call_closure(ctx, fill, &[])?;
    let answered = supplied(ctx, at, entry);
    #[expect(
        unsafe_code,
        reason = "`call_closure` hands back a value this frame owns, and the \
                  entry is never handed on -- what leaves here is the secret it \
                  carried"
    )]
    unsafe {
        entry.release();
    }
    answered.map(Some)
}

/// Stores what a `fill` answered and hands its secret back to the caller that
/// ran it.
///
/// The entry is read rather than kept: what a program holds afterwards is the
/// secret, and the object it arrived in is this frame's to release. The
/// lifetime is the one the fetch learned, a token endpoint being the authority
/// on how long its own token stays good.
///
/// # Errors
///
/// [`seal_and_store`]'s, and a [`Fault::fatal`] for an entry whose value slot is
/// not text — the row declares the closure answers a
/// `Core\Cache\SecretEntry`, so that is compiled code's bug rather than
/// anything a program can write.
fn supplied(ctx: &mut Ctx, at: &Sealing<'_>, entry: Value) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(entry, &SECRET_ENTRY, at.member)?;
    let held = crate::instance::slot(receiver, 0);
    let nanos = crate::instance::slot(receiver, 1).as_int().unwrap_or(0);
    let Some(value) = held.as_text() else {
        return Err(Fault::fatal(format!(
            "{} expected a `string` from its `fill`, got tag {}",
            at.who,
            held.tag_byte()
        )));
    };

    seal_and_store(ctx, at, value.as_bytes(), nanos)?;
    Ok(Value::str(NvsStr::new(value.as_bytes())))
}

/// The wall clock this request reads, in milliseconds since the epoch.
///
/// [`crate::time::wall_clock`] and not a [`std::time::SystemTime`] of this
/// module's own, so that a `#[Test(at: …)]` moves a sealed expiry exactly as it
/// moves every other reading in `Core`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a fixed reading outside the representable range,
/// which `Core\Test::advance` refuses to store — so it is a state no program
/// can reach and no member has anything to say about.
fn clock_ms(ctx: &Ctx, member: &str) -> Result<i64, Fault> {
    crate::time::wall_clock(ctx)
        .map(|at| at.as_millisecond())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{STORE_NAME}::{member} read a fixed clock outside the representable range"
            ))
        })
}

/// The `secret string` in argument slot `at`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`key_of`]'s reason with one more
/// on top: `secret` is checked once and erased before codegen, so what arrives
/// here is a `string` or compiled code's bug.
fn secret_of<'a>(args: &'a [Value], at: usize, member: &str) -> Result<&'a str, Fault> {
    args[at].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{STORE_NAME}::{member} expected a `string` value, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// Writes `payload` under `key` for every core of this process, replacing any
/// entry there and forgetting whatever `cap` does not leave room for.
///
/// One shard's write lock and no other, held across the map write and the
/// evictions it makes: a core writing a key that hashes elsewhere waits on
/// nothing, and a `get` on another shard never waits at all.
///
/// **The bytes are the process's rather than the writing request's**, under the
/// same [`budget::Detached`] bracket [`store_put`] holds and for the reason that
/// tier holds one — which this tier needs the more sharply of the two: an entry
/// here is written by one request and forgotten by another, on another core, so
/// a balance that moved with it would credit whichever request happened to make
/// the room. `payload` is the caller's own temporary, copied in under the
/// bracket and released outside it under the request that allocated it, which is
/// the symmetry the bracket owes.
fn process_put(key: &[u8], payload: Vec<u8>, lifetime: Lifetime, cap: Option<usize>) {
    {
        let _bracket = budget::Detached::begin();
        let mut shard = shard_of(key)
            .write()
            .expect("a cache shard's lock is never poisoned");
        shard.put(key, &payload, lifetime, shard_cap(cap));
    }
    // Where it was allocated: [`store_put`]'s own closing note, unchanged.
    drop(payload);
}

/// This process's payload for `key`, or `None` — an ordinary answer here for
/// every reason it is one on the local tier, and for one more: this may be a
/// different process than the one that wrote the entry.
///
/// **A read lock and never a write one**, which is what
/// `rule:concurrency/the-process-tier-is-one-store-per-process` fixes about a
/// lookup: every core reading the same hot key reads it at the same moment, and
/// nothing on the request path takes a shard exclusively to answer a question.
/// That is also why eviction is by write age rather than by use — a policy that
/// reordered anything on a read would need the lock this one does not take.
fn process_get(key: &[u8]) -> Option<Vec<u8>> {
    shard_of(key)
        .read()
        .expect("a cache shard's lock is never poisoned")
        .get(key)
        .map(<[u8]>::to_vec)
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

/// Takes `key`'s entry out of the process's tier, whether or not there was one.
///
/// One shard's write lock and no other, exactly as [`process_put`] takes one,
/// and inside the same bracket [`store_forget`] holds for the same reason.
fn process_forget(key: &[u8]) {
    let _bracket = budget::Detached::begin();
    shard_of(key)
        .write()
        .expect("a cache shard's lock is never poisoned")
        .forget(key);
}

/// The `{ttl?: Duration}` in argument slot `at`, as the lifetime the call
/// wrote.
///
/// An omitted option arrives as the [`Const::Null`] [`PUT_OPTIONS`] defaults it
/// to, which is the one reading that cannot be a written duration: the option's
/// declared type admits no `null`, so there is no second spelling for
/// `rule:core-api/omission-is-not-a-written-null` to have to tell apart here.
///
/// # Errors
///
/// [`crate::time::nanos_of`]'s fatal for a slot holding neither a duration nor
/// that `null`, which the row's own type has already refused at the call site.
fn lifetime_of(args: &[Value], at: usize, member: &str) -> Result<Lifetime, Fault> {
    if args
        .get(at)
        .is_none_or(|held| matches!(held.tag(), Some(Tag::Null)))
    {
        return Ok(Lifetime::Forever);
    }
    Ok(Lifetime::of(crate::time::nanos_of(args, at, member)?))
}

/// Which tier a store is — [`TIER_SLOT`] read back, as the one choice every
/// operation on it makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    /// This core's own entries, in process.
    Local,
    /// Every core of this process's entries, in the one map [`PROCESS`] holds.
    Process,
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
        Some(PROCESS_TIER) => Ok(Tier::Process),
        Some(SHARED_TIER) => Ok(Tier::Shared),
        _ => Err(Fault::fatal(format!(
            "{STORE_NAME}::{member} found a `tier` slot naming none of `{LOCAL_TIER}`, \
             `{PROCESS_TIER}` and `{SHARED_TIER}`"
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

/// `[cache.shared] url` read into the place this core's connection goes.
///
/// # Errors
///
/// A thrown `RuntimeError` naming what this client reads, for a URL carrying a
/// scheme it does not speak, a path, or a port that is not one. Each of the
/// three is refused rather than ignored: a `rediss://` treated as `redis://`
/// would be a plaintext connection wearing a TLS spelling, and a database index
/// dropped on the floor would put the entries somewhere the operator did not
/// ask for. Plus everything [`socket`] refuses, for the second spelling.
fn endpoint(url: &str, member: &str) -> Result<Target, Fault> {
    let refuse = |why: &str| {
        Fault::thrown(format!(
            "{member}: `{URL}` is `{url}`, and {why} — this client reads {READS}"
        ))
    };
    if let Some(path) = url.strip_prefix(UNIX) {
        return socket(path).map_err(refuse);
    }
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
    // address. `resolve_host` takes the brackets off itself.
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
    // Resolved and not pinned: `rule:config/cache-shared-is-the-grant-over-the-configured-store` is
    // that the endpoint an operator wrote carries the authority that granted the capability, so
    // there is no attacker-influenced name here for `rule:security/net-address-policy`'s table to
    // hold at arm's length — and holding it there is what made a loopback store need `net.internal`.
    let address = nvs_runtime::capability::resolve_host(host, member)?;
    Ok(Target::Tcp(SocketAddr::new(address, port)))
}

/// The `unix:` spelling's target, on a build whose reactor carries `AF_UNIX`.
///
/// `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host` is the scheme,
/// and the path after it must be absolute for the reason a bare path in a key
/// called `url` is refused outright: a socket named against a working directory
/// this process promises nothing about is a store two readers cannot point at
/// twice.
///
/// # Errors
///
/// The clause [`endpoint`]'s refusal appends, for a `unix:` with nothing after
/// it or with a relative path.
#[cfg(unix)]
fn socket(path: &str) -> Result<Target, &'static str> {
    if path.is_empty() {
        return Err("it names no socket after the scheme");
    }
    if !path.starts_with('/') {
        return Err("a socket is named by an absolute path and this one is relative");
    }
    Ok(Target::Socket(std::path::PathBuf::from(path)))
}

/// The same arm on a build with no Unix-domain transport, where the answer is a
/// refusal.
///
/// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot` puts
/// this refusal at boot, where `E0627` names the key an operator wrote; this is
/// the same answer at the door, for a snapshot that reached one anyway.
///
/// # Errors
///
/// Always. There is nothing here a socket could be dialled with, and reading the
/// spelling as loopback TCP instead is the thing that rule refuses.
#[cfg(not(unix))]
fn socket(_path: &str) -> Result<Target, &'static str> {
    Err(
        "this build carries no Unix-domain transport, so there is nothing to dial a socket \
         with — and reading it as loopback TCP is refused for the reason `rediss://` is",
    )
}

/// Where this core's connection to the shared store goes.
///
/// The key [`open_shared`] compares to decide reuse-or-replace, which is what a
/// reloaded configuration looks like from there. It widened from a
/// [`SocketAddr`] because
/// `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`'s second
/// spelling has no address for one to hold, and it lives here rather than beside
/// the wire for the split [`redis`]'s module doc states: this module decides
/// *which* store is reached, and that one speaks to it.
///
/// The socket half is `#[cfg(unix)]` rather than a variant that refuses when it
/// is dialled, so a build with no `AF_UNIX` transport cannot hold a path to dial
/// at all — the platform half of
/// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`, made
/// impossible by the type instead of caught by a check.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Target {
    /// A `redis://host[:port]`, resolved to the one address the connection is
    /// made to, carrying the URL's port or [`redis::DEFAULT_PORT`].
    Tcp(SocketAddr),
    /// A `unix:/path`, exactly as an operator wrote it: there is nothing to
    /// resolve, and
    /// `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`
    /// is why no program can name one.
    #[cfg(unix)]
    Socket(std::path::PathBuf),
}

impl std::fmt::Display for Target {
    /// What a failure names the store as, which is the spelling an operator
    /// wrote rather than a description of it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp(address) => write!(f, "{address}"),
            #[cfg(unix)]
            Self::Socket(path) => write!(f, "{}", path.display()),
        }
    }
}

/// Makes this core's connection the one to `target`, and dials it.
///
/// A connection already open to that same target is kept — the ordinary case,
/// since every request on this core asks for the same configured store. One to a
/// *different* target is replaced, which is what a reloaded configuration looks
/// like from here, and it is the whole reason [`Target`] carries the address or
/// the path rather than only what one of the two transports can say.
///
/// # Errors
///
/// A thrown `IOError` for a store that cannot be reached, which is the second
/// half of [`SHARED_DOC`]'s card: an unreachable store throws at the door rather
/// than answering a handle whose every operation would fail.
fn open_shared(target: Target, timeout: Duration, member: &str) -> Result<(), Fault> {
    SHARED.with_borrow_mut(|held| {
        if held.as_ref().is_none_or(|open| open.target() != &target) {
            *held = Some(redis::Connection::new(target, timeout));
        }
        held.as_mut()
            .expect("the connection was just written")
            .ensure()
            .map_err(|why| Fault::thrown_as(ThrownClass::Io, format!("{member}: {why}")))
    })
}

/// Opens this core's connection to the configured shared store — the **door**,
/// as the module doc's third decision defines one: the directive is read here,
/// the grant is asked for here, the URL becomes a [`Target`] here, and the
/// connection is made to that and to nothing the store answers with later.
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

    open_shared(endpoint(&url, member)?, timeout_of(ctx), member)
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
    /// `Core\Cache::process(): Core\Cache\Store` — `rule:core-api/two-cache-tiers`'s per-process
    /// tier.
    ///
    /// Nothing is allocated for the tier itself and no grant is asked for, which
    /// is [`nvs_core_cache_local`]'s two sentences holding here for the same two
    /// reasons: the entries are in [`PROCESS`] whether a program has asked for a
    /// store or not, and a tier with no door onto an effect has nothing to check
    /// at one.
    fn nvs_core_cache_process(_ctx, _args: [0]) {
        Ok(crate::instance::build(
            &STORE,
            [Value::str(NvsStr::new(PROCESS_TIER.as_bytes()))],
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
    ///
    /// The lifetime is read before the copy and carried into whichever tier
    /// the store names, because each of the three keeps it differently and
    /// none of them may read the option for itself — the module doc's own
    /// decision on a lifetime says which keeps what.
    fn nvs_core_cache_put(ctx, args: [4]) {
        let tier = tier_of(args, "put")?;
        let key = key_of(args, 1, "put")?.as_bytes().to_vec();
        let lifetime = lifetime_of(args, 3, "put")?;

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
            Tier::Local => store_put(&key, payload, lifetime, local_cap(ctx)),
            Tier::Process => {
                process_put(&scoped(ctx, &key), payload, lifetime, process_cap(ctx));
            }
            Tier::Shared => on_shared(STORE_NAME, "put", |open| {
                // The store keeps its own clock, so it is told a length of
                // time rather than an instant — and a lifetime already over
                // is not a write here either, which is the one command that
                // leaves the same state the in-process tiers do.
                if lifetime.elapsed() {
                    return open.del(&key);
                }
                match lifetime {
                    Lifetime::Forever => open.set(&key, &payload),
                    Lifetime::For(ttl) => open.set_expiring(&key, &payload, ttl),
                }
            })?,
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
            Tier::Process => process_get(&scoped(ctx, &key)),
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

nvs_runtime::nvs_helper! {
    /// `Core\Cache\Store::forget(string $key): void` — the second reason an
    /// entry goes, after the cap and the lifetime that are the tier's own.
    ///
    /// **It answers nothing, not even whether there was an entry.** § 1 already
    /// says one may be absent at any time for any reason, so a program that
    /// could act on that answer would be acting on a coincidence of the cap and
    /// the clock; a store whose `get` reports a miss as an ordinary answer
    /// cannot report the same state as news here.
    ///
    /// The tier decides where the key is and nothing else, which is
    /// [`nvs_core_cache_get`]'s structure: the two in-process tiers take the
    /// entry out of their own map, and the shared tier sends the one command
    /// that leaves the store in the state the other two are left in.
    ///
    /// # Errors
    ///
    /// A thrown `IOError` on the shared tier for a store that cannot be
    /// reached or that refuses the command. The in-process tiers have no
    /// failure to report, so neither has an error at all.
    fn nvs_core_cache_forget(ctx, args: [2]) {
        let tier = tier_of(args, "forget")?;
        let key = key_of(args, 1, "forget")?.as_bytes().to_vec();

        match tier {
            Tier::Local => store_forget(&key),
            Tier::Process => process_forget(&scoped(ctx, &key)),
            Tier::Shared => on_shared(STORE_NAME, "forget", |open| open.del(&key))?,
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache\Store::putSecret(string $key, secret string $value, Duration
    /// $ttl, array<secret bytes> $keys): void` — the one door through which a
    /// secret meets a cache.
    ///
    /// **What reaches the tier is ciphertext**, by that tier's ordinary path
    /// and as bytes, so `rule:security/secret-crosses-no-boundary` is untouched
    /// — no `secret` value crosses the copy at all, and a sealed entry is as
    /// safe on the shared tier as on this core's own. The ring's newest key
    /// seals and [`bound`] is what the entry may not be moved away from.
    ///
    /// **The lifetime is written twice, on purpose.** The tier is told it, so
    /// that a secret nobody reads is forgotten on the ordinary schedule and
    /// costs the cap nothing after it is over; and it is sealed in as an
    /// absolute expiry, so that a tier which kept the entry longer than it was
    /// asked to still answers a miss. [`sealed_value`] is where the second
    /// reading wins.
    ///
    /// # Errors
    ///
    /// [`crate::keyring`]'s `LogicError` for a ring that is empty or holds
    /// something that is not a key, and a thrown `IOError` on the shared tier
    /// for a store that cannot be reached or that refuses the write.
    fn nvs_core_cache_put_secret(ctx, args: [5]) {
        let tier = tier_of(args, "putSecret")?;
        let key = key_of(args, 1, "putSecret")?.as_bytes().to_vec();
        let value = secret_of(args, 2, "putSecret")?;
        let nanos = crate::time::nanos_of(args, 3, "putSecret")?;

        let who = format!("{STORE_NAME}::putSecret");
        let ring = crate::keyring::borrow(args, 4, &who)?;

        let at = Sealing {
            tier,
            key: &key,
            ring: &ring,
            member: "putSecret",
            who: &who,
        };
        seal_and_store(ctx, &at, value.as_bytes(), nanos)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache\Store::getSecret(string $key, array<secret bytes> $keys,
    /// {fill?: callable(): Core\Cache\SecretEntry, wait?: Duration}): ?secret
    /// string` — [`nvs_core_cache_put_secret`]'s door in the other direction.
    ///
    /// [`opened`] is the read, and it is every caller's first step: a hit
    /// outside its refresh window is answered without the fill table being
    /// touched at all.
    ///
    /// **A miss with a `fill` written is supplied exactly once per process.**
    /// [`elected`] chooses the one caller that runs it, in that caller's own
    /// request and under its own capabilities; every other caller on every core
    /// [`waited`]s for it, for at most the bound [`wait_of`] answers. What the
    /// filler leaves behind is [`seal_and_store`]'s entry, which is the one
    /// `putSecret` writes — so a waiter's second look is an ordinary read. A
    /// miss with no `fill` is a miss, unchanged.
    ///
    /// **An entry inside the last fifth of its lifetime is answered to every
    /// caller** while exactly one of them runs the `fill` that replaces it, so
    /// the expiry of a hot key costs nobody a wait: a caller that already has
    /// the entry never waits here, and only the one that was elected pays the
    /// fetch. [`Held::refreshing`] is that window, and a plain `get` past a
    /// lifetime is simply absent — refresh-ahead is `getSecret`-with-a-`fill`
    /// and nothing else.
    ///
    /// # Errors
    ///
    /// [`crate::keyring`]'s `LogicError` for a ring that is empty or holds
    /// something that is not a key, and [`elected`]'s for a `fill` that asked
    /// for the key it is filling. [`waited`]'s `TimeoutError` for a fill still
    /// running when this call's `wait` was up, and a thrown `IOError` on the
    /// shared tier for a store that cannot be reached. What a `fill` itself
    /// throws reaches the request that ran it and no other.
    fn nvs_core_cache_get_secret(ctx, args: [5]) {
        let tier = tier_of(args, "getSecret")?;
        let key = key_of(args, 1, "getSecret")?.as_bytes().to_vec();

        let who = format!("{STORE_NAME}::getSecret");
        let ring = crate::keyring::borrow(args, 2, &who)?;

        let at = Sealing {
            tier,
            key: &key,
            ring: &ring,
            member: "getSecret",
            who: &who,
        };
        let held = opened(ctx, &at)?;
        let fill = args[3];
        if matches!(fill.tag(), Some(Tag::Null)) {
            return Ok(held.map_or_else(Value::null, |held| held.value));
        }
        let filling = scoped(ctx, &sealed_key(&key));

        let Some(held) = held else {
            // Read before the election so that what this call waits for does
            // not depend on which caller won it.
            let wait = wait_of(ctx, args, 4, "getSecret")?;
            if let Some(value) = filled(ctx, &at, &filling, fill)? {
                return Ok(value);
            }
            return waited(ctx, &filling, wait, &who, |ctx| {
                Ok(opened(ctx, &at)?.map_or_else(Value::null, |held| held.value))
            });
        };
        if !held.refreshing {
            return Ok(held.value);
        }

        let refreshed = filled(ctx, &at, &filling, fill);
        if matches!(refreshed, Ok(None)) {
            // Another caller is already replacing it, and this one is answered
            // what is there rather than waiting for a value it already holds.
            return Ok(held.value);
        }
        #[expect(
            unsafe_code,
            reason = "the entry this frame opened is replaced by what the fill \
                      answered, so the reference it built is never handed on"
        )]
        unsafe {
            held.value.release();
        }
        refreshed.map(Option::unwrap_or_default)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cache\SecretEntry::of(secret string $value, Duration $ttl):
    /// Core\Cache\SecretEntry` — the two halves a fill learned, as the one
    /// value it hands back.
    ///
    /// The lifetime is read here rather than where the entry is stored, so
    /// that the `Duration` instance is finished with at the call that wrote it
    /// and what the entry holds is the number every tier turns into its own
    /// deadline — [`Lifetime`]'s own doc is the home of that split.
    ///
    /// **What it spends:** one object allocation per entry, charged to the
    /// request like every other `Core` instance. The secret's bytes are not
    /// copied — the slot holds one more reference to the same [`NvsStr`], and
    /// the copy happens at the seal.
    fn nvs_core_cache_secret_entry_of(_ctx, args: [2]) {
        let nanos = crate::time::nanos_of(args, 1, "of")?;

        // Unreachable from source: the row declares a `secret string` there
        // and `E0401` refuses another type at the call site, so a non-text tag
        // here is compiled code's bug rather than anything a program can
        // write.
        if args[0].as_text().is_none() {
            return Err(Fault::fatal(format!(
                "{SECRET_ENTRY_NAME}::of expected a `string` value, got tag {}",
                args[0].tag_byte()
            )));
        }

        // A `CoreCall`'s arguments are borrowed and `instance::build` takes
        // over each slot's reference, so the reference the entry ends up
        // holding is taken here rather than handed over by the caller.
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(crate::instance::build(
            &SECRET_ENTRY,
            [args[0], Value::int(nanos)],
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier, Mutex, mpsc};
    use std::time::{Duration, Instant};

    use nvs_runtime::budget;
    use nvs_runtime::{Fault, ThrownClass};

    use crate::tests::granting;

    use super::{
        CLASS, Ctx, DEFAULT_FILL_WAIT, DEFAULT_MAX_SIZE, ENTRIES, ENTRY_OVERHEAD, GET_DOC,
        LOCAL_DOC, Lifetime, MAX_SIZE, PROCESS, PROCESS_DOC, PROCESS_FILL_WAIT, PROCESS_MAX_SIZE,
        SEAL_DOMAIN, SHARDS, SHARED_DOC, Value, bound, charged, elected, endpoint, local_cap,
        open_configured, process_cap, process_forget, process_get, process_put, scoped, sealed_key,
        sealed_plaintext, sealed_value, shard_cap, shard_of, store_forget, store_get, store_put,
        wait_of, waited,
    };

    /// Taken by every case that touches the process tier, first thing.
    ///
    /// That tier is the *process's*, and `cargo test` runs these cases on threads
    /// of one process — so a case filling it under a cap would forget another
    /// case's entry while that case was still reading it, and a case measuring a
    /// balance would be credited for bytes it never allocated. What this
    /// serializes is the cases; the tier itself needs no such thing, which is
    /// what its shards are.
    static TIER: Mutex<()> = Mutex::new(());

    /// A snapshot a process is serving: one `[[app]]` block and one generation
    /// number, which is both halves of what [`scoped`] reads.
    fn served(app: &str, generation: u64) -> Arc<nvs_config::Snapshot> {
        Arc::new(nvs_config::Snapshot {
            blocks: vec![std::path::PathBuf::from(app)],
            generation,
            ..Default::default()
        })
    }

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
        store_put(b"the-same-walk", payload.clone(), Lifetime::Forever, None);
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

    /// `rule:core-api/two-cache-tiers`: a member per tier, not one API with a flag — and the check
    /// that makes that structural is that not one of them takes an argument at
    /// all, so no value computed at run time can choose the tier.
    #[test]
    fn local_and_shared_are_separate_members_with_separate_contracts() {
        let names: Vec<&str> = CLASS.methods.iter().map(|method| method.name).collect();
        assert_eq!(
            names,
            ["local", "process", "shared"],
            "the roster is one member per tier and nothing else"
        );

        for method in CLASS.methods {
            assert!(
                method.params.is_empty(),
                "{}::{} takes an argument, which is the flag § 1 refuses",
                CLASS.name,
                method.name
            );
        }

        // Separate *contracts*, which is the half a shared implementation would
        // quietly lose: a store that may forget and is one core's, a store that
        // may forget and is the whole process's, and a store that is coherent and
        // can be unreachable.
        let shorts = [LOCAL_DOC.short, PROCESS_DOC.short, SHARED_DOC.short];
        for (at, short) in shorts.iter().enumerate() {
            assert!(
                !shorts[..at].contains(short),
                "two tiers describe themselves the same way, which is one contract under two names"
            );
        }
        assert!(
            LOCAL_DOC.ret.contains("not visible on another"),
            "the local card has to state § 1's per-core contract"
        );
        assert!(
            PROCESS_DOC.ret.contains("every core"),
            "the process card has to state what its tier is coherent across"
        );
        assert!(
            LOCAL_DOC.errors.is_empty()
                && PROCESS_DOC.errors.is_empty()
                && !SHARED_DOC.errors.is_empty(),
            "an unreachable shared store throws; neither in-process tier has anything to be \
             unreachable"
        );
    }

    /// `rule:core-api/two-cache-tiers`: any entry may be absent at any time, for any reason. The
    /// store answers `None` for a key nothing wrote, and — the half a program
    /// depends on — the card *says* so, since a contract nobody can read is one
    /// every caller will assume away.
    #[test]
    fn a_local_entry_may_be_absent_at_any_time_and_the_contract_says_so() {
        assert_eq!(store_get(b"absent-by-construction"), None);

        store_put(b"present", b"payload".to_vec(), Lifetime::Forever, None);
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
        store_put(
            b"one-core-only",
            b"written here".to_vec(),
            Lifetime::Forever,
            None,
        );
        assert_eq!(store_get(b"one-core-only"), Some(b"written here".to_vec()));

        let elsewhere = std::thread::spawn(|| {
            let before = store_get(b"one-core-only");
            // And the other direction: what the second core writes stays there.
            store_put(
                b"one-core-only",
                b"written there".to_vec(),
                Lifetime::Forever,
                None,
            );
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
        store_put(b"k1", b"first!".to_vec(), Lifetime::Forever, room);
        store_put(b"k2", b"second".to_vec(), Lifetime::Forever, room);
        assert_eq!(store_get(b"k1"), Some(b"first!".to_vec()));

        store_put(b"k3", b"third!".to_vec(), Lifetime::Forever, room);
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
        store_put(b"kept", vec![0; 8], Lifetime::Forever, room);
        store_put(b"kept", vec![0; 4096], Lifetime::Forever, room);
        assert_eq!(store_get(b"kept"), None);

        store_put(b"kept", vec![7; 8], Lifetime::Forever, room);
        assert_eq!(store_get(b"kept"), Some(vec![7; 8]));
    }

    /// `forget` takes an entry out of the tier it was named on and out of no
    /// other, and gives the bytes back to the balance they were taken from.
    ///
    /// Three claims, because each alone passes something broken: a `forget`
    /// that emptied the map would satisfy the first, one that forgot on every
    /// tier at once would satisfy the first two, and one that removed the entry
    /// without crediting [`Entries::held`] would leave a tier evicting against
    /// a cap it no longer measures.
    #[test]
    fn a_forget_takes_one_entry_out_of_one_tier_and_credits_the_cap() {
        const KEY: &[u8] = b"forgettable";

        let _held = TIER.lock().expect("the process tier, one case at a time");
        store_put(KEY, b"local".to_vec(), Lifetime::Forever, None);
        process_put(KEY, b"process".to_vec(), Lifetime::Forever, None);
        store_put(b"kept-here", b"local".to_vec(), Lifetime::Forever, None);

        store_forget(KEY);

        assert_eq!(store_get(KEY), None, "the entry named is gone");
        assert_eq!(
            store_get(b"kept-here"),
            Some(b"local".to_vec()),
            "and nothing else on the tier went with it"
        );
        assert_eq!(
            process_get(KEY),
            Some(b"process".to_vec()),
            "the other in-process tier keeps its own entry of that name"
        );
        ENTRIES.with_borrow(|local| {
            assert_eq!(
                local.held,
                charged(b"kept-here".len(), b"local".len()),
                "the forgotten entry is still charged to the cap"
            );
        });

        // A key nothing is under is already forgotten, so this is the state the
        // call above reached rather than a second one.
        store_forget(KEY);
        process_forget(KEY);
        assert_eq!((store_get(KEY), process_get(KEY)), (None, None));
    }

    /// The module doc's *the read judges, and never writes*: an entry past its
    /// deadline answers nothing and **stays where it is**, until the cap or an
    /// overwrite reaches it.
    ///
    /// Two claims, because the absence alone would pass a `get` that took the
    /// entry out — the shape
    /// `rule:concurrency/the-process-tier-is-one-store-per-process` refuses
    /// when it fixes a lookup as a read lock and nothing more. The bytes still
    /// standing against the cap afterwards are what that costs, and the
    /// overwrite is what ends it.
    #[test]
    fn an_entry_past_its_lifetime_is_absent_and_the_read_leaves_it_there() {
        const KEY: &[u8] = b"past-it";
        const PAYLOAD: &[u8] = b"gone";

        store_put(
            KEY,
            PAYLOAD.to_vec(),
            Lifetime::For(Duration::from_nanos(1)),
            None,
        );
        std::thread::sleep(Duration::from_millis(2));

        assert_eq!(
            store_get(KEY),
            None,
            "an entry past its lifetime is the `null` § 1 had already promised"
        );
        ENTRIES.with_borrow(|local| {
            assert!(
                local.entries.contains_key(KEY),
                "the read took the entry out, which is what makes `get` a write"
            );
            assert_eq!(
                local.held,
                charged(KEY.len(), PAYLOAD.len()),
                "and it is still charged, which is what leaving it there costs"
            );
        });

        // A lifetime already over is not a write at all, so the overwrite that
        // reclaims those bytes leaves nothing under the key either.
        store_put(KEY, PAYLOAD.to_vec(), Lifetime::For(Duration::ZERO), None);
        ENTRIES.with_borrow(|local| {
            assert!(
                local.entries.is_empty(),
                "a lifetime already over wrote an entry rather than forgetting one"
            );
            assert_eq!(local.held, 0, "and the tier is holding bytes for nothing");
        });
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
            store_put(
                b"hot!",
                step.to_string().into_bytes(),
                Lifetime::Forever,
                room,
            );
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

    /// `rule:concurrency/the-process-tier-is-one-store-per-process`: one map
    /// every core of this process reads, and a `get` that takes a read lock and
    /// never a write.
    ///
    /// Two claims, because either alone passes a tier that is not one. An entry
    /// crosses **cores**, in both directions: a second thread reads what this one
    /// wrote and this one reads what that thread wrote, which a `thread_local`
    /// like [`ENTRIES`] fails either way round. And a lookup is a *shared* borrow
    /// of the shard — a second core reads a key while this one is holding that
    /// shard's read guard, which a `get` taking the write lock could not answer
    /// at all. It is asked with a timeout rather than with a `join`, so a `get`
    /// that took the wrong lock fails this case instead of hanging the suite.
    #[test]
    fn process_tier_is_one_map_every_core_reads() {
        /// Long enough that a loaded machine is not why this fails, and short
        /// enough that a wrong lock is a failure rather than a wedged run.
        const WAIT: Duration = Duration::from_secs(5);

        let _serial = TIER.lock().expect("the tier's cases run one at a time");

        let mine = nvs_runtime::encode(Value::int(11)).expect("an `int` crosses any boundary");
        process_put(b"one-map-mine", mine.clone(), Lifetime::Forever, None);
        let (read_there, written_there) = std::thread::spawn(|| {
            let theirs =
                nvs_runtime::encode(Value::int(13)).expect("an `int` crosses any boundary");
            process_put(b"one-map-theirs", theirs.clone(), Lifetime::Forever, None);
            (process_get(b"one-map-mine"), theirs)
        })
        .join()
        .expect("the second core's thread runs to completion");
        assert_eq!(
            read_there,
            Some(mine),
            "a second core read nothing this one wrote, so the map is not the process's"
        );
        assert_eq!(
            process_get(b"one-map-theirs"),
            Some(written_there),
            "this core read nothing the second wrote, and one store answers both ways"
        );

        let held = shard_of(b"one-map-mine")
            .read()
            .expect("a cache shard's lock is never poisoned");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(process_get(b"one-map-mine").is_some());
        });
        assert_eq!(
            rx.recv_timeout(WAIT),
            Ok(true),
            "a `get` waited on a read another core was holding, so it took the write lock"
        );
        drop(held);
    }

    /// `rule:concurrency/the-process-tier-is-one-store-per-process`: an entry's
    /// real key carries the `[[app]]` it was written for and the configuration
    /// generation it was written under.
    ///
    /// Four claims about one name the program spells the same way every time.
    /// Two applications are two scopes and two generations are two scopes, which
    /// is what the rule asks for; the same application in the same generation is
    /// **one** scope, without which nothing a request wrote would outlive it and
    /// the tier would be per-request; and a program spelling another
    /// application's prefix into its own key stays inside its own, which is what
    /// the separators are there for. The entries are then read back through two
    /// of those scopes, because keys that differ prove nothing about a map that
    /// ignored them.
    #[test]
    fn process_tier_keys_are_scoped_per_app_and_per_generation() {
        let _serial = TIER.lock().expect("the tier's cases run one at a time");

        let mut ctx = Ctx::buffered();
        ctx.set_config(served("/srv/one", 1));
        let one = scoped(&ctx, b"the-same-name");

        ctx.set_config(served("/srv/two", 1));
        let two = scoped(&ctx, b"the-same-name");
        assert_ne!(one, two, "two applications on one server share a scope");

        ctx.set_config(served("/srv/one", 2));
        assert_ne!(
            one,
            scoped(&ctx, b"the-same-name"),
            "a reload serves an entry written under the configuration it replaced"
        );

        ctx.set_config(served("/srv/one", 1));
        assert_eq!(
            one,
            scoped(&ctx, b"the-same-name"),
            "one application in one generation is two scopes, so nothing outlives a request"
        );
        assert_ne!(
            scoped(&ctx, b"/srv/two\x001\x00the-same-name"),
            two,
            "a program wrote its own key into another application's namespace"
        );

        let first = nvs_runtime::encode(Value::int(1)).expect("an `int` crosses any boundary");
        let second = nvs_runtime::encode(Value::int(2)).expect("an `int` crosses any boundary");
        process_put(&one, first.clone(), Lifetime::Forever, None);
        process_put(&two, second.clone(), Lifetime::Forever, None);
        assert_eq!(process_get(&one), Some(first));
        assert_eq!(process_get(&two), Some(second));
    }

    /// `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`: an
    /// entry's bytes move the process's detached balance, and the request that
    /// made the write is measured as though it had not.
    ///
    /// The local tier's own case asks this of [`store_put`]; it is asked again
    /// because a bracket is held by the *store* and this is a second store. The
    /// half this tier adds is the release: the key is rewritten until many times
    /// one entry has been allocated, and the balance ends up holding one — so the
    /// frees landed on the balance the allocations did, which is the symmetry a
    /// bracket owes and the only way a cross-request store's credit stays
    /// bounded.
    #[test]
    fn process_tier_bytes_are_on_the_detached_balance() {
        /// The charge one entry has to show through the noise.
        const ENTRY: usize = 256 * 1024;
        /// How many times the one key is written.
        const REWRITES: usize = 16;

        let _serial = TIER.lock().expect("the tier's cases run one at a time");

        let ctx = Ctx::buffered();
        let used = ctx.memory_used();
        let live = budget::live_bytes();
        let held = budget::detached_bytes();

        process_put(
            b"process-charged-to-the-process",
            vec![b'p'; ENTRY],
            Lifetime::Forever,
            None,
        );

        assert!(
            budget::detached_bytes() - held >= ENTRY.cast_signed(),
            "the entry reached no balance at all, so nothing holds its bytes to the process"
        );
        assert_eq!(
            budget::live_bytes(),
            live,
            "the entry was charged to the balance this request's ceiling is armed against"
        );
        assert_eq!(
            ctx.memory_used(),
            used,
            "the request that happened to write the entry is measured as having written it"
        );

        let before = budget::detached_bytes();
        for _ in 0..REWRITES {
            process_put(
                b"process-rewritten",
                vec![b'r'; ENTRY],
                Lifetime::Forever,
                None,
            );
        }
        let after = budget::detached_bytes() - before;
        assert!(
            after < (2 * ENTRY).cast_signed(),
            "{REWRITES} writes of one key hold {after} bytes, so the entry each one replaced was \
             freed somewhere other than where it was allocated"
        );
    }

    /// `[cache.process] max_size` is read per write, and what bounds a shard is
    /// its own even share of it.
    ///
    /// The directive half is [`local_cap`]'s case over the other key: the shipped
    /// cap where nothing is written, a size where one is, no ceiling for `false`,
    /// and a `Reload` row, which is why it is read per operation rather than once
    /// per process. The tier half is the eviction: a sweep writing many times the
    /// cap never fails a write and leaves no shard holding more than its share,
    /// which is `rule:concurrency/cache-memory-is-charged-to-the-core`'s "evicts
    /// rather than failing an allocation" on a map that is cut into pieces. The
    /// survivors are counted rather than named, because which shard a key lands
    /// in is a hash's business and the bound is what the cap promises.
    #[test]
    fn the_process_tiers_cap_is_a_share_per_shard_and_evicts() {
        /// One sweep entry's payload.
        const CHUNK: usize = 4 * 1024;
        /// The whole tier's cap: three `CHUNK`s to a shard, which is room for two
        /// entries once each one's key and [`ENTRY_OVERHEAD`] are charged too.
        const CAP: usize = SHARDS * 3 * CHUNK;
        /// How many entries, so that the sweep writes many times the cap.
        const WRITES: usize = 2048;

        let _serial = TIER.lock().expect("the tier's cases run one at a time");

        let mut ctx = Ctx::buffered();
        assert_eq!(process_cap(&ctx), Some(DEFAULT_MAX_SIZE));
        ctx.set_config(granting("[cache.process]\nmax_size = \"128K\"\n"));
        assert_eq!(process_cap(&ctx), Some(128 * 1024));
        ctx.set_config(granting("[cache.process]\nmax_size = false\n"));
        assert_eq!(process_cap(&ctx), None, "`false` is no ceiling at all");
        assert_eq!(
            nvs_config::directive::lookup(PROCESS_MAX_SIZE).map(|row| row.apply),
            Some(nvs_config::Apply::Reload),
            "a cap read once per process would outlive the snapshot that set it"
        );
        assert_eq!(shard_cap(Some(CAP)), Some(3 * CHUNK));
        assert_eq!(shard_cap(None), None, "no ceiling divides into no ceiling");

        for step in 0..WRITES {
            process_put(
                format!("process-sweep-{step:04}").as_bytes(),
                vec![b's'; CHUNK],
                Lifetime::Forever,
                Some(CAP),
            );
        }
        let survived = (0..WRITES)
            .filter(|step| process_get(format!("process-sweep-{step:04}").as_bytes()).is_some())
            .count();
        assert!(
            survived > 0 && survived <= 2 * SHARDS,
            "{survived} of {WRITES} entries survived a sweep of {} times the cap, where each \
             shard's share holds two of them and the newest write is always one",
            WRITES * CHUNK / CAP
        );
        let over = PROCESS
            .iter()
            .filter(|shard| {
                shard
                    .read()
                    .expect("a cache shard's lock is never poisoned")
                    .held
                    > 3 * CHUNK
            })
            .count();
        assert_eq!(
            over, 0,
            "{over} shard(s) hold more than the share the cap leaves them"
        );
    }

    /// `rule:concurrency/a-secret-fill-runs-once-per-process`: a caller that
    /// writes no `wait` is bounded by `[cache.process] fill_wait`, and by the
    /// shipped figure where the file names none.
    ///
    /// The refusals are the boot check's — `nvs_config::store` fails a `0` and
    /// the `false` that removes a ceiling elsewhere with `E0642` — so what is
    /// asked here is the half that check cannot cover: a configuration that
    /// reaches a request having been cleared to nothing, or holding a word that
    /// is not a measurement, leaves the bound standing rather than removing it.
    /// A reader that answered "no wait" to either would make every concurrent
    /// caller but the filler throw. The directive's class is asserted beside
    /// them for [`process_cap`]'s reason: a bound read once per process would
    /// outlive the snapshot that set it.
    #[test]
    fn an_omitted_wait_inherits_the_process_directive_and_never_removes_the_bound() {
        let mut ctx = Ctx::buffered();
        let waited = |ctx: &Ctx| wait_of(ctx, &[], 4, "getSecret").expect("an absent slot is null");

        assert_eq!(waited(&ctx), DEFAULT_FILL_WAIT, "a file naming none");
        ctx.set_config(granting("[cache.process]\nfill_wait = \"250ms\"\n"));
        assert_eq!(waited(&ctx), Duration::from_millis(250));
        ctx.set_config(granting("[cache.process]\nfill_wait = \"\"\n"));
        assert_eq!(
            waited(&ctx),
            DEFAULT_FILL_WAIT,
            "a cleared key is an absent one"
        );
        ctx.set_config(granting("[cache.process]\nfill_wait = \"soon\"\n"));
        assert_eq!(
            waited(&ctx),
            DEFAULT_FILL_WAIT,
            "a word is not a measurement"
        );
        ctx.set_config(granting("[cache.process]\nfill_wait = \"0s\"\n"));
        assert_eq!(
            waited(&ctx),
            DEFAULT_FILL_WAIT,
            "a wait of nothing is not a wait, and boot refuses it"
        );
        assert_eq!(
            nvs_config::directive::lookup(PROCESS_FILL_WAIT).map(|row| row.apply),
            Some(nvs_config::Apply::Reload),
            "a wait read once per process would outlive the snapshot that set it"
        );
    }

    /// The member the fill table is reached through, which is what a case here
    /// stands in for.
    const FILLER: &str = "Core\\Cache\\Store::getSecret";

    /// `rule:concurrency/a-secret-fill-runs-once-per-process`: the election is
    /// the whole of what makes a fill single-flight, so it is asserted where
    /// every core reaches it at once rather than through one member's body.
    ///
    /// Two claims, because either alone passes a table that is not one. Exactly
    /// **one** caller of the race holds the key — the barrier after the attempt
    /// is what makes that a race rather than a queue, since no hold is released
    /// until every core has asked. And every other caller is released only
    /// *after* the holder was done with it, which the flag the filler sets
    /// before it lets go is what a waiter reads.
    #[test]
    fn fill_runs_once_while_every_other_core_waits() {
        const CORES: usize = 8;
        let key = b"\0fills-once".to_vec();
        let racing = Barrier::new(CORES);
        let asked = Barrier::new(CORES);
        let fills = AtomicUsize::new(0);
        let waits = AtomicUsize::new(0);
        let landed = AtomicBool::new(false);

        std::thread::scope(|cores| {
            for _ in 0..CORES {
                cores.spawn(|| {
                    let mut ctx = Ctx::buffered();
                    racing.wait();
                    let held = elected(&ctx, &key, FILLER).expect("no call of this one asks twice");
                    asked.wait();
                    match held {
                        Some(held) => {
                            fills.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(40));
                            landed.store(true, Ordering::SeqCst);
                            drop(held);
                        }
                        None => {
                            waited(&mut ctx, &key, Duration::from_secs(5), FILLER, |_| {
                                assert!(
                                    landed.load(Ordering::SeqCst),
                                    "a waiter looks once the filler is done with the key, never \
                                     before"
                                );
                                Ok(())
                            })
                            .expect("a waiter is released with what the filler left");
                            waits.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                });
            }
        });

        assert_eq!(
            fills.load(Ordering::SeqCst),
            1,
            "one caller in the process runs the fill"
        );
        assert_eq!(
            waits.load(Ordering::SeqCst),
            CORES - 1,
            "and every other one waits for it"
        );
    }

    /// `rule:concurrency/a-secret-fill-runs-once-per-process`: a wait is bounded
    /// and its end is a throw, which is
    /// `rule:http-server/no-spelling-for-an-unbounded-wait`'s shape one class
    /// over — the caller gets its request back rather than the core standing
    /// still for as long as somebody else's fetch takes.
    ///
    /// The bound is waited *out* rather than refused on sight, which is the half
    /// a class assertion alone would not catch: a reader that took every wait
    /// for an expired one would throw here just as promptly.
    #[test]
    fn a_wait_past_its_bound_throws_timeout() {
        let key = b"\0waits-past-its-bound".to_vec();
        let filler = Ctx::buffered();
        let _held = elected(&filler, &key, FILLER)
            .expect("an unheld key is nobody's")
            .expect("and the caller that asks for it holds it");

        let mut ctx = Ctx::buffered();
        let began = Instant::now();
        let outcome = waited(
            &mut ctx,
            &key,
            Duration::from_millis(60),
            FILLER,
            |_| -> Result<(), Fault> { panic!("a held key is never looked past") },
        );

        let Err(Fault::Thrown(class, message)) = outcome else {
            panic!("a wait that ran out is catchable");
        };
        assert_eq!(class, ThrownClass::Timeout);
        assert!(
            message.contains(FILLER) && message.contains("60ms"),
            "the refusal names the member and the bound that was up: {message}"
        );
        assert!(
            began.elapsed() >= Duration::from_millis(60),
            "the bound is waited out rather than refused on sight"
        );
    }

    /// `rule:concurrency/a-secret-fill-runs-once-per-process`: a failure is not
    /// shared. A `fill` that throws leaves no entry behind, so the waiters it
    /// releases are released with whatever is there — nothing — rather than
    /// with an exception that crossed from another request, and the key is free
    /// for the next caller to fill.
    #[test]
    fn a_failed_fill_is_not_shared_and_the_next_caller_fills() {
        let key = b"\0a-failed-fill".to_vec();
        let looked = AtomicUsize::new(0);

        let filler = Ctx::buffered();
        let held = elected(&filler, &key, FILLER)
            .expect("an unheld key is nobody's")
            .expect("and the caller that asks for it holds it");

        std::thread::scope(|cores| {
            let waiting = cores.spawn(|| {
                let mut ctx = Ctx::buffered();
                waited(&mut ctx, &key, Duration::from_secs(5), FILLER, |_| {
                    looked.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
                .expect("a waiter is released rather than handed what failed");
            });
            // Long enough that the waiter is waiting rather than racing the
            // hold it is waiting on.
            std::thread::sleep(Duration::from_millis(30));
            // The `fill` throws: the hold goes with the frame that took it,
            // and nothing was written under the key.
            drop(held);
            waiting.join().expect("the waiter ends with its own answer");
        });

        assert_eq!(
            looked.load(Ordering::SeqCst),
            1,
            "a waiter looks once, at what the filler left"
        );

        let next = Ctx::buffered();
        assert!(
            elected(&next, &key, FILLER)
                .expect("a released key is nobody's")
                .is_some(),
            "a failed fill leaves the key to the next caller rather than the failure"
        );
    }

    /// `rule:concurrency/a-secret-fill-runs-once-per-process`: a request that
    /// ends while it holds a fill releases the key, so a cancelled fetch never
    /// wedges one for the life of the process.
    ///
    /// Both halves, because the release alone would pass a table that never
    /// held anything: the key is held for as long as the frame that took it is,
    /// and free the moment that frame is gone — by the ending a cancelled
    /// request carries out rather than by any path that ending never took.
    #[test]
    fn a_request_ending_during_its_fill_releases_the_key() {
        let key = b"\0a-request-that-ends".to_vec();
        let other = Ctx::buffered();

        let mut ending = Ctx::buffered();
        {
            let _held = elected(&ending, &key, FILLER)
                .expect("an unheld key is nobody's")
                .expect("and the caller that asks for it holds it");
            let _cancelled = ending.cancel();
            assert!(
                elected(&other, &key, FILLER)
                    .expect("a key another call holds is not this one's")
                    .is_none(),
                "the key is held for as long as the frame filling it is"
            );
        }

        assert!(
            elected(&other, &key, FILLER)
                .expect("a released key is nobody's")
                .is_some(),
            "and is free the moment that frame is gone"
        );
    }

    /// `rule:concurrency/a-secret-fill-runs-once-per-process`: an entry inside
    /// the last fifth of its lifetime is still answered to every caller while
    /// exactly one of them replaces it, so the expiry of a hot key costs
    /// nobody a wait.
    ///
    /// The window is asserted on **both** sides of its edge, because a reader
    /// that called every entry due for refresh prints plausibly against the
    /// inside alone — and the edge is the whole of what
    /// [`super::REFRESH_WINDOW`] fixes. Past the expiry there is no entry to
    /// refresh at all, which is the third answer a fraction has to keep
    /// separate from the second.
    #[test]
    fn an_entry_near_its_expiry_is_answered_while_one_caller_refreshes_it() {
        let plain = sealed_plaintext(1_000, 1_000, b"hunter2");
        let at = |now: i64| sealed_value(&plain, now).map(|held| held.refreshing);

        assert_eq!(
            at(799),
            Some(false),
            "an entry with more than a fifth of its lifetime left is answered and left alone"
        );
        assert_eq!(
            at(800),
            Some(true),
            "the last fifth is where one caller replaces it ahead of its expiry"
        );
        assert_eq!(at(999), Some(true), "and every moment of that window is");
        assert_eq!(
            at(1_000),
            None,
            "past the expiry there is nothing to answer and nothing to refresh"
        );

        // And while one caller is replacing it, a second does not run a fill
        // of its own: the key is held for the length of that fetch, which is
        // what leaves every other caller the entry that is still there.
        let key = b"\0near-its-expiry".to_vec();
        let refresher = Ctx::buffered();
        let held = elected(&refresher, &key, FILLER)
            .expect("an unheld key is nobody's")
            .expect("and the caller that asks for it holds it");
        let other = Ctx::buffered();
        assert!(
            elected(&other, &key, FILLER)
                .expect("a key another call holds is not this one's")
                .is_none(),
            "one caller refreshes, and it is the one that was elected"
        );
        drop(held);
    }

    /// `rule:concurrency/cache-memory-is-charged-to-the-core`: the tier's memory is charged to the **core** that holds it
    /// — never to a request — and bounded by an `nvs.toml` directive.
    ///
    /// The name's two halves are one fact, and each is checked where the other
    /// cannot see it. The cap is the *directive's*, whose row is `Reload`, which
    /// is why [`local_cap`] is read per write rather than once per core. The
    /// cap then bounds what the core actually holds and not merely the
    /// bookkeeping beside it: a sweep writing thirty-two times the cap leaves
    /// the process's balance up by about the cap, so the entries
    /// `forget_oldest` dropped gave their bytes back. The eviction cases above
    /// assert what a `put` forgets; this one asserts that forgetting it was a
    /// deallocation.
    ///
    /// And the charge lands on the core rather than on a request:
    /// `nvs_runtime::budget::detached_bytes` is the process's share of *this
    /// thread's* balance, which is where [`store_put`]'s bracket puts an entry
    /// and what no request's ceiling is armed against — so a second core writing
    /// the same key raises its own by the same amount, and neither core's
    /// requests are measured by either. That is § 3's O(cores × working set)
    /// measured rather than restated, and it is the price
    /// `rule:security/no-cross-request-state`'s isolation is bought with.
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
        let before = budget::detached_bytes();
        let mut over = 0;
        for step in 0..WRITES {
            store_put(
                format!("sweep-{step:04}").as_bytes(),
                vec![b'c'; CHUNK],
                Lifetime::Forever,
                Some(CAP),
            );
            over += usize::from(ENTRIES.with_borrow(|local| local.held) > CAP);
        }
        assert_eq!(
            over, 0,
            "{over} of {WRITES} writes left the tier over its cap"
        );

        let held = budget::detached_bytes() - before;
        assert!(
            held < (4 * CAP).cast_signed(),
            "the core holds {held} bytes after writing {}, so the cap bounds the \
             bookkeeping and not the memory",
            WRITES * CHUNK
        );

        // Charged to *this* thread, which is what § 3 means by charged to the
        // core: the balance rises with the entry and stays risen with it.
        let alone = budget::detached_bytes();
        store_put(
            b"charged-per-core",
            vec![b'x'; LARGE],
            Lifetime::Forever,
            None,
        );
        assert!(
            budget::detached_bytes() - alone >= LARGE.cast_signed(),
            "the entry's bytes are live on the core that wrote it"
        );

        // And a second core pays for its own copy of the same key rather than
        // sharing this one's — § 3's multiplication, as a measurement.
        let elsewhere = std::thread::spawn(|| {
            let fresh = budget::detached_bytes();
            store_put(
                b"charged-per-core",
                vec![b'y'; LARGE],
                Lifetime::Forever,
                None,
            );
            budget::detached_bytes() - fresh
        })
        .join()
        .expect("the second core's thread runs to completion");
        assert!(
            elsewhere >= LARGE.cast_signed(),
            "eight cores hold eight copies, and each one is charged for its own"
        );
    }

    /// The entry's bytes are the process's, and the request that happened to
    /// make the write is measured as though it had not made it.
    ///
    /// `rule:concurrency/cache-memory-is-charged-to-the-core`'s *not
    /// attributable to a request*, asked of the accounting rather than of the
    /// cap: the balance every ceiling is armed against, and the reading a limit
    /// handler reports, both end where they started. The payload is built
    /// *after* those readings are taken and released by the store, so a bracket
    /// that kept the caller's `Vec` instead of copying it would leave the
    /// balance short by an entry and fail here.
    #[test]
    fn a_cache_write_is_charged_to_the_process_and_not_to_the_request() {
        /// The charge one entry has to show through the noise.
        const ENTRY: usize = 256 * 1024;

        let ctx = Ctx::buffered();
        let used = ctx.memory_used();
        let live = budget::live_bytes();
        let held = budget::detached_bytes();

        store_put(
            b"charged-to-the-process",
            vec![b'p'; ENTRY],
            Lifetime::Forever,
            None,
        );

        assert!(
            budget::detached_bytes() - held >= ENTRY.cast_signed(),
            "the entry reached no balance at all, so nothing holds its bytes to the process"
        );
        assert_eq!(
            budget::live_bytes(),
            live,
            "the entry was charged to the balance this request's ceiling is armed against"
        );
        assert_eq!(
            ctx.memory_used(),
            used,
            "a cache write raised the reading this request's own limit handler reports"
        );
    }

    /// The attack the cap does not stop on its own: an entry one request wrote
    /// is freed by whichever later request's write evicts it, and a release
    /// credited to *that* request is a ceiling any program widens at will.
    ///
    /// What the case asserts is the **gap** between the armed threshold and the
    /// balance, because the threshold is absolute — `budget::armed_ceiling` is
    /// a balance and not a size — so headroom is the only number a widening
    /// moves. The eviction is asserted as well, since a write that forgot
    /// nothing would pass the headroom half without ever freeing anything.
    #[test]
    fn an_eviction_from_a_later_request_lowers_no_ceiling() {
        /// One entry's payload.
        const ENTRY: usize = 128 * 1024;
        /// Room in the tier for one of them and no more.
        const KEY: &[u8] = b"inherited-from-an-earlier-request";

        let room = Some(charged(KEY.len(), ENTRY));
        store_put(KEY, vec![b'i'; ENTRY], Lifetime::Forever, room);

        // The request that comes next, under a ceiling of its own.
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting("[limits]\nmemory = \"8M\"\n"));
        let ceiling = budget::armed_ceiling();
        assert_ne!(
            ceiling, 0,
            "the request armed no ceiling, so there is no headroom here to widen"
        );
        let headroom = ceiling - budget::live_bytes();

        store_put(
            b"written-by-this-request",
            vec![b'w'; ENTRY],
            Lifetime::Forever,
            room,
        );
        assert_eq!(
            store_get(KEY),
            None,
            "the write forgot nothing, so no release happened to be credited"
        );

        assert_eq!(
            budget::armed_ceiling(),
            ceiling,
            "an eviction moved the threshold the request is measured against"
        );
        assert_eq!(
            budget::armed_ceiling() - budget::live_bytes(),
            headroom,
            "evicting what an earlier request stored bought this request headroom"
        );
    }

    /// The bytes moved off the request's balance; the tier's own ceiling still
    /// measures them.
    ///
    /// This is the half the bracket could quietly break, because what the cap
    /// compares is [`Local::held`] — maintained from the lengths the store was
    /// handed, never from an allocator counter — and a write that escaped it
    /// would now be bytes the process holds and *nothing* bounds, where before
    /// they were at least inside some request's ceiling. Both ends: the charge
    /// arrives with the entry, and a sweep many times the cap never leaves the
    /// tier over it.
    #[test]
    fn a_cache_entry_still_counts_against_the_local_tier_max_size() {
        /// One entry's payload, small enough that the sweep writes many.
        const ENTRY: usize = 4 * 1024;
        /// The ceiling those writes are made under.
        const CAP: usize = 64 * 1024;
        /// How many of them, so that 32 × `CAP` is written in all.
        const WRITES: usize = 512;
        /// The one entry the charge is read off.
        const KEY: &[u8] = b"counted-against-the-cap";

        store_put(KEY, vec![b'c'; ENTRY], Lifetime::Forever, Some(CAP));
        ENTRIES.with_borrow(|local| {
            assert_eq!(
                local.held,
                charged(KEY.len(), ENTRY),
                "the entry arrived without its charge, so the cap is measuring something else"
            );
        });

        let mut over = 0;
        for step in 0..WRITES {
            store_put(
                format!("swept-{step:04}").as_bytes(),
                vec![b's'; ENTRY],
                Lifetime::Forever,
                Some(CAP),
            );
            over += usize::from(ENTRIES.with_borrow(|local| local.held) > CAP);
        }
        assert_eq!(
            over, 0,
            "{over} of {WRITES} writes left the tier over its cap"
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

    /// `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`'s scheme at
    /// the door, and
    /// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`'s
    /// answer on the platform that has none: one `url` is a socket target where
    /// the reactor carries `AF_UNIX` and a refusal where it does not.
    ///
    /// Both sides are here for `nvs_config::store::validate`'s reason one layer
    /// up — a case written for one of them passes against a door that answers
    /// that way everywhere — and what the refusing side is really asserting is
    /// that the spelling never quietly becomes loopback TCP, which is the
    /// failure the rule exists to prevent and the only one review cannot see.
    #[test]
    fn a_unix_url_is_refused_where_the_platform_has_no_transport() {
        let read = endpoint("unix:/run/redis.sock", MEMBER);

        #[cfg(unix)]
        {
            assert_eq!(
                read.expect("a socket is an ordinary store where there is a transport for one"),
                super::Target::Socket(std::path::PathBuf::from("/run/redis.sock")),
                "the path is taken as written: there is nothing here to resolve"
            );
            // And it is a *path*, so the two spellings that are not one are
            // refused rather than read against a working directory this process
            // promises nothing about.
            assert!(endpoint("unix:redis.sock", MEMBER).is_err());
            assert!(endpoint("unix:", MEMBER).is_err());
        }

        #[cfg(not(unix))]
        {
            let Fault::Thrown(class, message) =
                read.expect_err("a socket with no transport under it is refused at the door")
            else {
                panic!("a store this build cannot reach is catchable, not fatal");
            };
            assert_eq!(class, ThrownClass::Runtime);
            assert!(
                message.contains("no Unix-domain transport") && message.contains("redis://"),
                "the refusal says what is missing and what to write instead: {message}"
            );
        }
    }

    /// `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`:
    /// one path is a store where an operator wrote it into `[cache.shared] url`
    /// and is never a target where a program supplied it, so what separates the
    /// two is the authority that wrote the endpoint and nothing about the path
    /// itself. That asymmetry is this tier's rather than
    /// `nvs_runtime::capability`'s, which is why the pair is asserted here and
    /// not beside the members that happen to call the door.
    ///
    /// The refusal is asked twice, of a path that exists and of one that does
    /// not, because reaching a socket is only half of what the rule prevents: a
    /// refusal that opened the path before deciding would answer the two
    /// differently, and that difference is a directory listing a program was
    /// never granted. `nvs_runtime::capability::open_read` orders its two
    /// questions for the same reason.
    #[test]
    fn a_program_supplied_socket_path_is_never_a_target() {
        // The other authority, standing in here for all of it: `Core\Net::connect`,
        // `Core\Http\Client` and `Core\Db::open`'s settings reach one door with a
        // program's endpoint, and differ in nothing this case reads.
        const SUPPLIED: &str = "Core\\Db::open()";

        let ctx = deployed("[capabilities]\ncache.shared = true\n");
        let present =
            std::env::temp_dir().join(format!("nvs-supplied-{}.sock", std::process::id()));
        let absent =
            std::env::temp_dir().join(format!("nvs-supplied-{}-gone.sock", std::process::id()));
        std::fs::write(&present, b"").expect("a writable temporary directory");
        let _ = std::fs::remove_file(&absent);

        let refused = |path: &std::path::Path| {
            let host = path.to_str().expect("a temporary path is UTF-8");
            let Fault::Thrown(class, message) =
                nvs_runtime::capability::pinned_address(&ctx, host, SUPPLIED)
                    .expect_err("a program-supplied socket path is not a target")
            else {
                panic!("a target this deployment cannot authorize is catchable, not fatal");
            };
            assert_eq!(class, ThrownClass::Runtime);
            assert!(
                message.contains("net.local"),
                "the refusal names the grant that would answer it: {message}"
            );
            message.replace(host, "<the path>")
        };

        assert_eq!(
            refused(&present),
            refused(&absent),
            "a socket that is there and one that is not are refused in the same words, because \
             the difference between them is what a probe would read"
        );

        // And the same path down the other door, where an operator wrote it: a
        // store. The refusal above is therefore about who supplied the endpoint,
        // which is the only thing the two calls do not share.
        #[cfg(unix)]
        {
            let written = format!("unix:{}", present.display());
            assert_eq!(
                endpoint(&written, MEMBER)
                    .expect("a socket an operator configured is an ordinary store"),
                super::Target::Socket(present.clone()),
            );
        }

        let _ = std::fs::remove_file(&present);
    }

    /// The cipher every sealed-entry case keys, from one key, so that what
    /// varies between the assertions below is the binding and never the key.
    fn sealing() -> chacha20poly1305::XChaCha20Poly1305 {
        crate::crypto::cipher(&[3_u8; crate::crypto::KEY_LEN])
            .expect("a key of the construction's own length")
    }

    /// A sealed entry opens where it was written and nowhere else: the
    /// application and the entry's name are both under the tag, so a ciphertext
    /// an operator copied to another name, or one an application read out of a
    /// store it shares with a neighbour, is a miss.
    ///
    /// Four refusals rather than one, because each alone passes something
    /// broken: a binding that held only the name would open across
    /// applications, one that held only the application would open across
    /// names, one that compared prefixes would open `token` against `tok`, and
    /// one with no domain octet would open a ciphertext another member of this
    /// crate produced under the same ring.
    #[test]
    fn sealed_entry_moved_to_another_key_or_app_is_a_miss() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_random_state(11);
        let cipher = sealing();
        let plain = sealed_plaintext(i64::MAX, 0, b"hunter2");
        let sealed = crate::crypto::seal_under(
            &mut ctx,
            &cipher,
            &bound(SEAL_DOMAIN, b"shop", b"token"),
            &plain,
            "test",
        )
        .expect("a short value seals");

        let opens = |app: &[u8], key: &[u8]| {
            crate::crypto::open_under(&cipher, &bound(SEAL_DOMAIN, app, key), &sealed, "test")
                .expect("nothing is unaffordable here")
                .is_some()
        };

        assert!(opens(b"shop", b"token"), "where it was written, it opens");
        assert!(
            !opens(b"shop", b"other"),
            "the same ring under another name does not"
        );
        assert!(
            !opens(b"admin", b"token"),
            "and another application under the same name does not"
        );
        assert!(
            !opens(b"shop", b"tok"),
            "nor a name the written one begins with, which a comparison by prefix would admit"
        );
        assert!(
            crate::crypto::open_under(&cipher, &[], &sealed, "test")
                .expect("nothing is unaffordable here")
                .is_none(),
            "and a construction that bound nothing at all does not open it either"
        );
    }

    /// The expiry sealed into an entry is what decides whether it is readable,
    /// and the tier's own answer does not enter into it.
    ///
    /// The bound is asserted on both sides — the last readable moment and the
    /// first that is not — because a member that stopped one millisecond early
    /// prints plausibly against either half alone. The tier holds the entry
    /// [`Lifetime::Forever`] throughout, so what the second half reads is the
    /// seal and not a store that had already forgotten it.
    #[test]
    fn sealed_entry_past_its_sealed_expiry_is_a_miss_whatever_the_store_says() {
        const KEY: &[u8] = b"past-its-seal";
        let plain = sealed_plaintext(1_000, 1_000, b"hunter2");

        store_put(&sealed_key(KEY), plain, Lifetime::Forever, None);
        let held = store_get(&sealed_key(KEY)).expect("the tier was given no lifetime to run out");

        assert_eq!(
            sealed_value(&held, 999).map(|held| held.value),
            Some(b"hunter2".as_ref()),
            "a millisecond before the sealed expiry, the secret is readable"
        );
        assert!(
            sealed_value(&held, 1_000).is_none(),
            "at the expiry itself it is not, which is where a lifetime of zero lands"
        );
        assert!(
            sealed_value(&held, 1_001).is_none(),
            "and past it the store still holds the entry and still answers nothing"
        );

        store_forget(&sealed_key(KEY));

        // A buffer too short to carry the two numbers ahead of the secret is
        // the same miss, which is what keeps a truncated payload from being
        // read as a secret.
        assert!(sealed_value(b"short", 0).is_none());
    }

    /// Every nonce a sealed entry carries is drawn through
    /// [`crate::random::draw`], so one `#[Test(seed: …)]` reproduces a sealed
    /// entry along with every other draw the test made.
    ///
    /// Both halves, because either alone passes something broken. The seeded
    /// pair is what a generator of this module's own would fail; the scan of
    /// the shipped half is what a *later* draw added beside one would fail,
    /// and it is the same structural assertion `crate::crypto`'s own case
    /// makes about its members' bodies.
    #[test]
    fn every_sealed_entry_nonce_is_drawn_through_core_random() {
        let sealed_under = |seed: u64| {
            let mut ctx = nvs_runtime::Ctx::buffered();
            ctx.set_random_state(seed);
            let aad = bound(SEAL_DOMAIN, b"shop", b"token");
            crate::crypto::seal_under(
                &mut ctx,
                &sealing(),
                &aad,
                &sealed_plaintext(1_000, 1_000, b"hunter2"),
                "test",
            )
            .expect("a short value seals")
        };

        assert_eq!(
            sealed_under(9),
            sealed_under(9),
            "one seed seals one entry the same way, so the nonce came from the seeded generator"
        );
        assert_ne!(
            sealed_under(9),
            sealed_under(10),
            "and another seed draws another nonce, so the first line is not a constant one"
        );

        let shipped: Vec<&str> = include_str!("cache.rs")
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]")
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        assert!(
            !shipped
                .iter()
                .any(|line| line.contains("rand::") || line.contains("random::")),
            "a sealed entry's nonce is `crate::crypto::seal_under`'s draw, so the shipped half \
             of this module names no generator at all"
        );
    }
}
