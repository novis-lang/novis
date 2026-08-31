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
//! keeps its `net.connect` row, which lands with the connection that needs it.
//!
//! # What is not here yet
//!
//! **The shared tier's store.** [`nvs_core_cache_shared`] answers the refusal
//! its own doc states and nothing else, because there is no configured store
//! for it to reach; the Redis client, its `net.connect` row in
//! [`crate::registry::CAPABILITIES`] and the tier slot's second value arrive
//! together.
//!
//! **A TTL, an eviction and a `forget`.** § 3's cap is what evicts, and a
//! lifetime is meaningless before something enforces one. `put` grows the
//! trailing options shape ADR 0063 R2 puts last when that lands, which is an
//! addition to the row rather than a change to it.
//!
//! **What it spends:** per core, one map entry per live key — the key's bytes
//! plus its payload's — held until it is overwritten and charged to the core
//! rather than to any request, which is § 3's O(cores × working set) and
//! deliberately not O(requests served).

use std::cell::RefCell;
use std::collections::HashMap;

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

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
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "No shared store is configured, or the configured one cannot be reached — an \
               unreachable store throws rather than answering as though the entry were absent.",
    }],
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
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The value cannot cross: it is or holds a closure, or an object with a `secret` \
               property that was not revealed.",
    }],
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
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "The entry names a class this program cannot resolve — the same refusal \
               `Core\\Serialize::decode` makes, and the ordinary consequence of a deployment \
               whose classes changed under a store that outlives them.",
    }],
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

/// Refuses a store that is not the local tier.
///
/// # Errors
///
/// A [`Fault::fatal`], because the only member that builds a store today is
/// [`nvs_core_cache_local`] — a second tier arrives with the member that can
/// produce one, and until then this is unreachable from source.
fn local_store(args: &[Value], member: &str) -> Result<(), Fault> {
    let receiver = crate::instance::receiver(args[0], &STORE, member)?;
    let tier = crate::instance::slot(receiver, TIER_SLOT);
    if tier.as_text() == Some(LOCAL_TIER) {
        return Ok(());
    }
    Err(Fault::fatal(format!(
        "{STORE_NAME}::{member} found a `tier` slot that is not `{LOCAL_TIER}`"
    )))
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
    /// It throws rather than answering a store that would silently behave like
    /// the local one: the two tiers make different promises, and a `shared`
    /// that quietly served per-core entries would be the accident § 1 splits
    /// the members to prevent.
    ///
    /// # Errors
    ///
    /// Always, until the Redis client and its `[cache.shared]` configuration
    /// land — see this module's own *What is not here yet*.
    fn nvs_core_cache_shared(_ctx, _args: [0]) {
        Err(Fault::thrown(format!(
            "{NAME}::shared(): no shared store is configured, so there is nothing coherent to \
             answer with — configure one, or use `{NAME}::local()` and accept its contract"
        )))
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
    /// because a value the program itself built is the program's bug.
    fn nvs_core_cache_put(_ctx, args: [3]) {
        local_store(args, "put")?;
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

        store_put(&key, payload);
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
    /// the same resolver — the program's own class table.
    fn nvs_core_cache_get(ctx, args: [2]) {
        local_store(args, "get")?;
        let key = key_of(args, 1, "get")?.as_bytes().to_vec();

        let Some(payload) = store_get(&key) else {
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
    use super::{CLASS, GET_DOC, LOCAL_DOC, SHARED_DOC, store_get, store_put};

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
