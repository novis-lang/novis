//! `Core\RateLimit` — `rule:core-classes/ratelimit-two-members`'s
//! limiter for what only the application knows, as both halves of it: `consume`
//! over the shared store, `shed` over this core's own memory, and the
//! `Core\RateLimit\Decision` each answers with.
//!
//! § 1's two members are two *jobs*. `consume` enforces a policy the
//! application promised somebody — a plan quota, a login limit — so its state
//! is the shared store and its answer is coherent across every core and every
//! machine. `shed` drops load to keep a host up, so its state is the core's own
//! memory and an approximate answer is adequate. Two members rather than a flag
//! on one, so which guarantee a program relies on is visible in review.
//!
//! # Decision: the store is `Core\Cache`'s, and this member is its own door
//!
//! There is one shared store in a deployment — `[cache.shared] url` — and a
//! limiter that named a second one would be a second thing to configure for no
//! second guarantee. So this module reaches the store through
//! [`crate::cache`]'s connection, which is one socket per core whether a
//! request caches or limits.
//!
//! What it does **not** do is require the program to have called
//! `Core\Cache::shared()` first. `rule:core-classes/ratelimit-two-members` and `rule:core-classes/ratelimit-unreachable-store-throws` both write
//! `Core\RateLimit::consume(…)` standing alone, and a member that silently
//! needed an unrelated call ahead of it would be an ordering rule held nowhere
//! near either call site. So [`nvs_core_ratelimit_consume`] opens the
//! configured store itself, through the same door
//! [`crate::cache::open_configured`] is for `Core\Cache::shared()`: the grant
//! is asked for here, and the store the deployment configured is dialled here.
//! That is why this member carries a
//! `cache.shared` row in [`crate::registry::CAPABILITIES`] and the `Decision`'s
//! four readers carry none — they read slots, and a slot read performs no
//! effect.
//!
//! # Decision: the GCRA step is one Lua script, and it is ours
//!
//! [`SCRIPT`] is the whole of what the store does: read the stored theoretical
//! arrival time, decide, and write the new one — atomically, because two cores
//! reading and writing the same key over two round trips would admit two
//! requests where the policy admits one, and an approximate `consume` is the
//! thing § 1 says is not a `consume` at all. A `WATCH`/retry loop is the
//! alternative and it is more round trips on the request path exactly when the
//! key is hot, which is when a limiter matters.
//!
//! **The arithmetic that Rust owns is the *parameters*, and the script owns the
//! *step*.** [`window`] turns `limit`, `per` and `burst` into GCRA's emission
//! interval and delay-variation tolerance, and it is the one place that
//! conversion happens — so [`step`] derives its window there rather than
//! restating § 2's arithmetic in a second place, and the five lines that
//! genuinely differ between an in-process decision and an atomic one are all
//! that differ. § 2's "both tiers run the identical algorithm" is that
//! factoring, and `shed_is_the_same_gcra_over_the_cores_own_memory` is where it
//! is checked rather than promised.
//!
//! # Decision: `shed`'s state is `Core\Cache`'s local tier, not a map of its own
//!
//! The per-core half needs somewhere to keep one timestamp per key, and the
//! runtime already has exactly one per-core store: [`crate::cache`]'s local
//! tier, an `rule:core-api/two-cache-tiers` `HashMap` in this thread. `shed` writes there, under
//! [`PREFIX`], for the reason `consume` writes to the one shared store — a
//! second map would be a second footprint to bound, bounded by nothing, and
//! `rule:concurrency/cache-memory-is-charged-to-the-core`'s `nvs.toml` cap is written for *the* local tier rather than
//! for `Core\Cache`'s. One store, one cap, whichever member filled it.
//!
//! Two consequences, both § 1's approximation rather than defects. An entry
//! this tier evicts is an arrival forgotten, so a shed key that loses its
//! timestamp admits a burst — which is why `consume` may not live here, and
//! `shed`'s contract already says its count is per core and so multiplies by
//! the number of them. And the clock is [`crate::time::monotonic_micros`], not
//! the wall clock: a limiter reading the wall clock refuses for as long as an
//! NTP step moved it backwards, and an interval is all GCRA reads.
//!
//! The script counts in **microseconds** rather than nanoseconds, because Lua's
//! numbers are doubles and a nanosecond count of the Unix epoch passed 2⁵³ in
//! 1970 + 104 days: it would be rounded, silently, on every arrival. A
//! microsecond count is exact until the year 2255, and [`window`] refuses a
//! tolerance past 2⁵³ rather than let one be rounded.
//!
//! # Decision: no configuration, and no key that can collide with a cache entry
//!
//! § 4's "no configuration at all" is the design: the limit is an argument
//! because only the application knows whether this is a plan quota or a login
//! throttle, and there is no `[ratelimit]` block for an operator to move it
//! into. `rate_limit_reads_no_directive` is that as a check — this module reads
//! no directive of its own, and the one it reaches indirectly is
//! `[cache.shared] url`, which says where the store is and nothing about the
//! policy.
//!
//! A key is namespaced with [`PREFIX`] before it reaches the store, because the
//! same store holds `Core\Cache` entries: a program caching under `account:1`
//! and limiting under `account:1` must not be one entry, and the two members
//! are written by different people in different files.
//!
//! **What it spends:** one entry per live key — one timestamp, not a window of
//! arrivals, which is § 2's O(1)-per-key and what makes limiting per user
//! affordable at a million users. `consume`'s entry is in the shared store and
//! expires on its own once the key drains; `shed`'s is in this core's local
//! tier, where it is bounded by that tier's cap and by nothing else, since the
//! tier has no TTL. In this process, `consume` spends the request text and one
//! three-integer reply, both released with the call, and `shed` spends the
//! decimal timestamp it stores.

use nvs_runtime::{Fault, Tag, ThrownClass, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\RateLimit";

/// [`DECISION`]'s name — see [`NAME`].
pub(crate) const DECISION_NAME: &str = r"Core\RateLimit\Decision";

/// What every key this module writes is prefixed with, so a limiter's entry and
/// a cache entry of the same name are two entries — see the module doc.
pub(crate) const PREFIX: &str = "nvs:ratelimit:";

/// `Core\Time\Duration`, as the two places below spell it.
const DURATION: CoreTy = CoreTy::Instance(crate::time::DURATION_NAME);

/// `{burst?: uint, cost?: uint}` — § 1's one trailing options shape, which both
/// members take because § 2 gives them one algorithm.
///
/// `burst` defaults to `limit`, which is a value only the call knows, so its
/// [`Const`] is the sentinel [`Const::Null`] that `uint` cannot otherwise hold
/// — the arrangement [`CoreTy::Union`]'s own docs fix for an option whose
/// "not given" is not a value in the declared type. `cost` defaults to a
/// written `1`, because one call weighing one unit is a constant.
const OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "burst",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "cost",
        ty: CoreTy::Uint,
        default: Const::Uint(1),
    },
];

/// § 1's two members: the coherent one, and the approximate one.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "consume",
            names: &["key", "limit", "per"],
            // The key is `Qual::Neutral`, which is § 3's two sentences at once:
            // a `tainted` account id is admitted, because a key is one opaque
            // length-prefixed value on the wire and there is no injection to
            // prevent; and a `secret` one is refused, because every mark but
            // `Qual::Reveal` refuses `secret` and writing a signing key into a
            // store with a lifetime is the durable exposure `rule:security/secret-qualifier` exists to
            // close. Nothing of the key reaches the answer, which is what makes
            // the mark neutral rather than contagious.
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Uint,
                DURATION,
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(DECISION_NAME),
            symbol: "nvs_core_ratelimit_consume",
            doc: Some(&CONSUME_DOC),
        },
        // The same signature to the letter, which is § 2's "both tiers run the
        // identical algorithm" said in the one place a caller reads: moving a
        // call between the two changes the guarantee and nothing else, not even
        // an argument's position.
        CoreMethod {
            name: "shed",
            names: &["key", "limit", "per"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Uint,
                DURATION,
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(DECISION_NAME),
            symbol: "nvs_core_ratelimit_shed",
            doc: Some(&SHED_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\RateLimit`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Limits how often something may happen for one key, such as one account or one API \
            key. `consume` counts in the shared store, so every server gives the same answer. \
            `shed` counts on this core only, to drop load quickly. Both return a \
            `Core\\RateLimit\\Decision`.",
};

/// `Core\RateLimit::consume`'s reference card — `rule:core-api/reference-card`.
const CONSUME_DOC: MethodDoc = MethodDoc {
    short: "Charges `$cost` units against `$key`'s allowance of `$limit` per `$per` in the shared \
            store, and answers whether this arrival is inside the limit.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "What the allowance is per — an account, a tenant, an API key id. `tainted` is \
                   admitted, since a key is one opaque value on the wire; a `secret` is refused, \
                   since keying on one writes it into a store, and the fix is to key on \
                   `Core\\Hash::of` of it.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many units `$per` admits — the drain rate, not a ceiling on any one \
                   instant.",
            shape: &[],
        },
        ParamDoc {
            name: "per",
            desc: "The period `$limit` units are admitted over.",
            shape: &[],
        },
        ParamDoc {
            name: "burst",
            desc: "How much may arrive at once; defaults to `$limit`, which admits a whole \
                   period's worth in one instant.",
            shape: &[],
        },
        ParamDoc {
            name: "cost",
            desc: "What this one call weighs, so an expensive endpoint may charge five units of \
                   the same quota; defaults to 1.",
            shape: &[],
        },
    ],
    ret: "A `Core\\RateLimit\\Decision`. Its `retryAfter` is `null` exactly when it is allowed, \
          and is the exact wait until the arrival would be admitted otherwise — never an \
          estimate, and never rounded up to the next window.",
    errors: &[
        ErrorDoc {
            error: "IOError",
            desc: "The shared store cannot be reached or refused the command. It is never \
                   answered as `allowed`: whether this limiter fails open or closed is knowledge \
                   only the call site has, so the decision is thrown to it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "No `[cache.shared] url` is configured, or `cache.shared` is not granted — a \
                   deployment mistake rather than the world saying no, and \
                   deliberately not the class the fail-open `catch` around this member holds. \
                   Also `$limit`, `$per` or `$burst` at zero, a period too short to divide \
                   into `$limit` units, and a `$cost` larger than `$burst`, which no wait could \
                   admit.",
        },
    ],
};

/// `Core\RateLimit::shed`'s reference card — `rule:core-api/reference-card`.
const SHED_DOC: MethodDoc = MethodDoc {
    short: "Charges `$cost` units against `$key`'s allowance of `$limit` per `$per` in this core's \
            own memory, and answers whether this arrival is inside the limit — the approximate \
            tier, for dropping load rather than for enforcing a promise.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "What the allowance is per. `tainted` is admitted and a `secret` refused, for \
                   `consume`'s reasons.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many units `$per` admits **on this core**: a limit of 100 across eight \
                   cores admits up to 800, which is why a number somebody was promised belongs to \
                   `consume` instead.",
            shape: &[],
        },
        ParamDoc {
            name: "per",
            desc: "The period `$limit` units are admitted over.",
            shape: &[],
        },
        ParamDoc {
            name: "burst",
            desc: "How much may arrive at once; defaults to `$limit`.",
            shape: &[],
        },
        ParamDoc {
            name: "cost",
            desc: "What this one call weighs; defaults to 1.",
            shape: &[],
        },
    ],
    ret: "A `Core\\RateLimit\\Decision`, answered from this core's memory and so reaching no \
          store: there is nothing to be unreachable, and this member does not throw for one. Its \
          arrivals are held in `Core\\Cache`'s local tier, which may forget an entry at any time \
          — a forgotten key admits a burst, which is the approximation the tier is chosen for.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$limit`, `$per` or `$burst` at zero, a period too short to divide into \
               `$limit` units, and a `$cost` larger than `$burst`, which no wait could admit — \
               the same refusals `consume` makes, since both derive one window.",
    }],
};

/// § 3's decision: what was decided, what it was decided against, and the exact
/// wait when it was refused.
///
/// Four readers rather than four readonly properties, which is where this
/// departs from § 3's sketch and has to: a `Core`-owned instance has no
/// property a program can reach ([`CoreTy::Instance`] is the home of that
/// rule), so `$d->allowed` would resolve a class, find no member and reach
/// `nvs-ir` with nothing to call. `Core\Http\Response` is the same shape for
/// the same reason and its `->status()` is the precedent.
pub(crate) const DECISION: CoreClass = CoreClass {
    name: DECISION_NAME,
    doc: Some(&DECISION_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "allowed",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_ratelimit_decision_allowed",
            doc: Some(&ALLOWED_DOC),
        },
        CoreMethod {
            name: "limit",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_ratelimit_decision_limit",
            doc: Some(&LIMIT_DOC),
        },
        CoreMethod {
            name: "remaining",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_ratelimit_decision_remaining",
            doc: Some(&REMAINING_DOC),
        },
        CoreMethod {
            name: "retryAfter",
            names: &[],
            params: &[],
            defaults: &[],
            // `rule:core-api/shape-rules` R4's "absence is `?T`" rather than a sentinel zero: an
            // allowed arrival has no wait, and a wait of zero is a different
            // claim from having none.
            return_ty: CoreTy::Nullable(&DURATION),
            symbol: "nvs_core_ratelimit_decision_retry_after",
            doc: Some(&RETRY_AFTER_DOC),
        },
    ],
    slots: &["allowed", "limit", "remaining", "retryAfter"],
    constants: &[],
};

/// `Core\RateLimit\Decision`'s class card — `rule:core-api/reference-card`.
const DECISION_CARD: ClassDoc = ClassDoc {
    short: "The result of one `Core\\RateLimit::consume` or `shed` call. `allowed` says whether \
            the call was inside the limit. `limit`, `remaining` and `retryAfter` give the numbers \
            for `RateLimit` and `Retry-After` response headers.",
};

/// `Core\RateLimit\Decision::allowed`'s reference card — `rule:core-api/reference-card`.
const ALLOWED_DOC: MethodDoc = MethodDoc {
    short: "Whether this arrival was inside the limit, and so whether its cost was charged.",
    params: &[],
    ret: "`true` when the units were charged, `false` when nothing was charged and the arrival \
          was refused.",
    errors: &[],
};

/// `Core\RateLimit\Decision::limit`'s reference card — `rule:core-api/reference-card`.
const LIMIT_DOC: MethodDoc = MethodDoc {
    short: "The `$limit` the decision was made against, carried back so a `RateLimit` header can \
            be written from the decision alone.",
    params: &[],
    ret: "The limit as it was passed.",
    errors: &[],
};

/// `Core\RateLimit\Decision::remaining`'s reference card — `rule:core-api/reference-card`.
const REMAINING_DOC: MethodDoc = MethodDoc {
    short: "How many further units the store would admit at this instant.",
    params: &[],
    ret: "The units left in the burst allowance, counted after this arrival was charged; `0` \
          when the next unit would have to wait.",
    errors: &[],
};

/// `Core\RateLimit\Decision::retryAfter`'s reference card — `rule:core-api/reference-card`.
const RETRY_AFTER_DOC: MethodDoc = MethodDoc {
    short: "How long until this arrival would be admitted — the exact wait, computed from the \
            store's own clock rather than estimated.",
    params: &[],
    ret: "`null` exactly when the decision is allowed, and otherwise the `Core\\Time\\Duration` \
          until the theoretical arrival time; a `Retry-After` header built from it tells the \
          client when to come back rather than when the window turns over, which is what stops \
          every refused client retrying in the same instant.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ratelimit_consume" => (nvs_core_ratelimit_consume as *const ()).cast(),
        "nvs_core_ratelimit_shed" => (nvs_core_ratelimit_shed as *const ()).cast(),
        "nvs_core_ratelimit_decision_allowed" => {
            (nvs_core_ratelimit_decision_allowed as *const ()).cast()
        }
        "nvs_core_ratelimit_decision_limit" => {
            (nvs_core_ratelimit_decision_limit as *const ()).cast()
        }
        "nvs_core_ratelimit_decision_remaining" => {
            (nvs_core_ratelimit_decision_remaining as *const ()).cast()
        }
        "nvs_core_ratelimit_decision_retry_after" => {
            (nvs_core_ratelimit_decision_retry_after as *const ()).cast()
        }
        _ => return None,
    })
}

/// The GCRA step, atomic because the store runs it as one — the module doc's
/// second decision is why it is ours rather than a client library's.
///
/// `KEYS[1]` is the namespaced key and `ARGV` is `interval`, `tau` and `cost`,
/// all in microseconds and all produced by [`window`]. The reply is three
/// integers: whether the arrival was admitted, how many microseconds until it
/// would be, and how many further units the allowance holds.
///
/// The clock is the **store's**, not the caller's: the whole difference between
/// this member and `shed` is that every core agrees, and a theoretical arrival
/// time compared against N differently-skewed machine clocks agrees only as far
/// as the worst of them. `redis.replicate_commands` is called where it exists
/// because Redis before 5 refused a write after a non-deterministic read
/// without it, and is a no-op in every version since.
const SCRIPT: &str = "\
if redis.replicate_commands then redis.replicate_commands() end
local clock = redis.call('TIME')
local now = tonumber(clock[1]) * 1000000 + tonumber(clock[2])
local interval = tonumber(ARGV[1])
local tau = tonumber(ARGV[2])
local cost = tonumber(ARGV[3])
local tat = tonumber(redis.call('GET', KEYS[1])) or now
if tat < now then tat = now end
local next_tat = tat + cost * interval
local allow_at = next_tat - tau
if now < allow_at then
  local left = math.floor((tau - (tat - now)) / interval)
  if left < 0 then left = 0 end
  return {0, allow_at - now, left}
end
redis.call('SET', KEYS[1], next_tat, 'PX', math.ceil((next_tat - now) / 1000) + 1)
local left = math.floor((tau - (next_tat - now)) / interval)
if left < 0 then left = 0 end
return {1, 0, left}
";

/// The largest count [`SCRIPT`]'s arithmetic holds exactly — Lua's numbers are
/// doubles, so every integer past 2⁵³ is rounded rather than refused.
const EXACT_CEILING: i128 = 1 << 53;

/// GCRA's two parameters, in microseconds: how long one unit takes to drain,
/// and how far ahead of the drain an arrival may run.
///
/// One place, per the module doc, because § 2's "both tiers run the identical
/// algorithm" is a claim about *these two numbers* — the decision step that
/// reads them is five lines and is written where the state is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    /// The emission interval `per / limit`: how long one unit takes to drain.
    interval: i128,
    /// The delay variation tolerance `burst × interval`: how much may arrive at
    /// once. With `burst` at its default of `limit` this is exactly `per`.
    tau: i128,
}

/// § 2's drain rate and burst tolerance, from the limit the caller wrote —
/// `member` being whichever of § 1's two asked, since both derive one window.
///
/// # Errors
///
/// A thrown `RuntimeError` for a limit, a period or a burst of zero, for a
/// period too short to divide into `limit` units, and for a tolerance past what
/// [`SCRIPT`] holds exactly. Each is a limit that can never admit anything or
/// can never be enforced accurately, and refusing is the direction `rule:core-classes/ratelimit-unreachable-store-throws` sets: a limiter that quietly does not limit is worse than no limiter.
///
/// The ceiling is [`SCRIPT`]'s rather than each tier's, so that a limit either
/// tier refuses is a limit both refuse: the two members are one algorithm under
/// § 2, and a window `shed` accepted and `consume` did not would make moving a
/// call between them a rewrite.
fn window(limit: u64, per: i64, burst: u64, member: &str) -> Result<Window, Fault> {
    let refuse = |why: String| Fault::thrown(format!("{NAME}::{member}(): {why}"));
    if limit == 0 {
        return Err(refuse(
            "a `$limit` of 0 admits nothing at any rate, so there is no rate to enforce — a \
             limiter that refuses everything is spelled by not calling the endpoint"
                .to_owned(),
        ));
    }
    if per <= 0 {
        return Err(refuse(
            "a `$per` of zero or less is not a period, so there is nothing for `$limit` units to \
             be admitted over"
                .to_owned(),
        ));
    }
    if burst == 0 {
        return Err(refuse(
            "a `$burst` of 0 admits nothing at any instant, so every arrival would be refused \
             however long it waited"
                .to_owned(),
        ));
    }
    // Microseconds, per the module doc: `SCRIPT` counts in them because a
    // nanosecond count of the epoch is past what a double holds exactly.
    let interval = i128::from(per) / 1000 / i128::from(limit);
    if interval == 0 {
        return Err(refuse(format!(
            "a `$per` of {per}ns divided into {limit} units is under a microsecond each, which is \
             finer than this limiter counts"
        )));
    }
    let tau = interval * i128::from(burst);
    if tau > EXACT_CEILING {
        return Err(refuse(format!(
            "a `$burst` of {burst} at that rate is {tau} microseconds of tolerance, past the \
             {EXACT_CEILING} this limiter's arithmetic holds exactly"
        )));
    }
    Ok(Window { interval, tau })
}

/// Refuses a `cost` the window could never admit, for whichever of § 1's two
/// members asked.
///
/// A cost past `burst` puts the arrival past the tolerance even on a key with
/// no history, so [`step`] would refuse it forever and report a wait after
/// which it is refused again. `retryAfter` is exact or it is nothing, so this
/// is the same refusal `window` makes for a burst of zero.
///
/// # Errors
///
/// A thrown `RuntimeError` when `cost` is larger than `burst`.
fn admissible(cost: u64, burst: u64, member: &str) -> Result<(), Fault> {
    if cost > burst {
        return Err(Fault::thrown(format!(
            "{NAME}::{member}(): a `$cost` of {cost} is more than the `$burst` of {burst}, so the \
             arrival would be refused however long it waited"
        )));
    }
    Ok(())
}

/// [`SCRIPT`]'s five lines, in this process, for the tier that has no store to
/// run them in — `rule:core-classes/ratelimit-gcra`'s "both tiers run the identical algorithm" as one
/// function rather than as a promise.
///
/// Takes the stored arrival time and answers **the three integers [`SCRIPT`]
/// answers**, in the same order and the same microseconds, plus the arrival
/// time to store when the arrival was admitted. Two tiers, one decoder: nothing
/// downstream of here can tell which of them decided, which is what makes
/// moving a call between the members a change of guarantee and not of shape.
fn step(stored: Option<i128>, now: i128, window: Window, cost: u64) -> ([i64; 3], Option<i128>) {
    // Microseconds are wide in an `i128` and narrow in the `i64` a `Duration`
    // holds, and the saturating direction is the safe one for both figures the
    // caller reads: a wait longer than 292,000 years is reported as that, and
    // `window`'s own ceiling is what keeps a real one out of this range.
    let narrowed = |wide: i128| i64::try_from(wide).unwrap_or(i64::MAX);

    let mut tat = stored.unwrap_or(now);
    if tat < now {
        tat = now;
    }
    let next_tat = tat + i128::from(cost) * window.interval;
    let allow_at = next_tat - window.tau;
    if now < allow_at {
        // The refusal charges nothing, so the arrival time is left where it is
        // and there is nothing to store: a client that keeps arriving while
        // refused does not push its own wait further out.
        let left = ((window.tau - (tat - now)) / window.interval).max(0);
        return ([0, narrowed(allow_at - now), narrowed(left)], None);
    }
    let left = ((window.tau - (next_tat - now)) / window.interval).max(0);
    ([1, 0, narrowed(left)], Some(next_tat))
}

/// This core's stored arrival time for `key`, or `None` for a key that has none.
///
/// `None` for a value that is not one of ours, too, which is the local tier's
/// contract rather than a defence: an entry may be absent at any time, and a
/// program that overwrote this key through `Core\Cache::local()` has forgotten
/// one key's arrival on one core — § 1's approximation, in the tier chosen for
/// tolerating it.
fn stored_tat(key: &[u8]) -> Option<i128> {
    let held = crate::cache::store_get(key)?;
    let Ok(text) = String::from_utf8(held) else {
        return None;
    };
    let Ok(tat) = text.parse::<i128>() else {
        return None;
    };
    Some(tat)
}

/// [`DECISION`]'s slots, by index — the layout its `slots` names.
const ALLOWED_SLOT: usize = 0;
const LIMIT_SLOT: usize = 1;
const REMAINING_SLOT: usize = 2;
const RETRY_AFTER_SLOT: usize = 3;

/// One decision, as the instance a program reads it off.
///
/// The **only** place this module builds one, which is § 5 as structure rather
/// than as a rule: every route to a `Decision` runs through here, and every
/// route to here is downstream of the store having answered — so there is no
/// arrangement of this file in which an unreachable store produces one.
fn built(allowed: bool, limit: u64, remaining: u64, retry_after: Option<i64>) -> Value {
    crate::instance::build(
        &DECISION,
        [
            Value::bool(allowed),
            Value::uint(limit),
            Value::uint(remaining),
            retry_after.map_or_else(Value::null, crate::time::duration_of),
        ],
    )
}

/// The store's three integers, as the decision they mean.
///
/// # Errors
///
/// The reply's shape as text, for anything that is not three integers — a store
/// answering something else has run a script that is not [`SCRIPT`], and
/// reading it as a decision would be inventing one.
fn decoded(reply: &[i64], limit: u64) -> Result<Value, String> {
    let [allowed, retry_after, remaining] = reply else {
        return Err(format!(
            "the limiter script answered {} value(s) where it answers three",
            reply.len()
        ));
    };
    let allowed = *allowed != 0;
    let remaining = u64::try_from(*remaining).unwrap_or(0);
    // Microseconds on the wire, nanoseconds in a `Duration`. `null` exactly
    // when allowed, per § 3 — not a zero the caller has to know to read as
    // absence.
    let wait = (!allowed).then(|| retry_after.saturating_mul(1000).max(0));
    Ok(built(allowed, limit, remaining, wait))
}

/// The `string` in argument slot `at`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`crate::cache`]'s own
/// `key_of` gives: the row declares a `string` there, so another tag is
/// compiled code's bug — unreachable from source, because `E0401` refuses the
/// call before any of this runs.
fn key_of<'a>(args: &'a [Value], at: usize, member: &str) -> Result<&'a str, Fault> {
    args[at].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string` key, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// The `uint` in argument slot `at`, or `None` for an option that was omitted.
///
/// # Errors
///
/// A [`Fault::fatal`], for [`key_of`]'s reason — unreachable from source, since
/// `E0401` refuses a non-`uint` argument first and an omitted option arrives as
/// the `Tag::Null` this reads as absence.
fn uint_of(args: &[Value], at: usize, member: &str) -> Result<Option<u64>, Fault> {
    if args[at].tag() == Some(Tag::Null) {
        return Ok(None);
    }
    args[at].as_uint().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `uint`, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// A slot of the receiving `Decision`, retained because it is being answered.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object — unreachable from
/// source, since an instance member's receiver is typed and `E0401` refuses a
/// call on anything else.
fn slot_of(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &DECISION, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\RateLimit::consume(tainted string $key, uint $limit, Duration $per,
    /// {burst?: uint, cost?: uint}): RateLimit\Decision` — `rule:core-classes/ratelimit-two-members`'s
    /// coherent member.
    ///
    /// The door, the window and the step, in that order: the configured store
    /// is opened under `cache.shared`, unscoped, [`window`]
    /// turns the limit into GCRA's two parameters, and [`SCRIPT`] does the one
    /// thing that has to be atomic. Nothing here decides anything — the store
    /// does, and this reads its answer back.
    ///
    /// # Errors
    ///
    /// A thrown `RuntimeError` for a store that is not configured or not
    /// granted, and for a limit that cannot be enforced; a thrown
    /// `IOError` for a store that cannot be reached or that refused the script.
    /// Never an answer: `rule:core-classes/ratelimit-unreachable-store-throws` is that the failure mode belongs to the
    /// call site, which is the only place that knows whether this limiter is a
    /// plan quota to fail open on or a login throttle to fail closed on.
    fn nvs_core_ratelimit_consume(ctx, args: [5]) {
        let key = key_of(args, 0, "consume")?;
        let limit = uint_of(args, 1, "consume")?.unwrap_or(0);
        let per = crate::time::nanos_of(args, 2, "consume")?;
        let burst = uint_of(args, 3, "consume")?.unwrap_or(limit);
        let cost = uint_of(args, 4, "consume")?.unwrap_or(1);

        // Before the door, so a limit that could never be enforced is a
        // refusal rather than a round trip that answers one.
        let window = window(limit, per, burst, "consume")?;
        admissible(cost, burst, "consume")?;

        let member = format!("{NAME}::consume");
        crate::cache::open_configured(
            ctx,
            &member,
            "; a limit enforced across every core has no per-core store to fall back to",
        )?;

        let namespaced = format!("{PREFIX}{key}");
        let arguments = [
            window.interval.to_string(),
            window.tau.to_string(),
            cost.to_string(),
        ];
        let reply = crate::cache::on_shared(NAME, "consume", |open| {
            let argv: Vec<&[u8]> = arguments.iter().map(|text| text.as_bytes()).collect();
            open.eval(SCRIPT, namespaced.as_bytes(), &argv)
        })?;

        decoded(&reply, limit).map_err(|why| {
            Fault::thrown_as(ThrownClass::Io, format!("{member}(): {why}"))
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\RateLimit::shed(tainted string $key, uint $limit, Duration $per,
    /// {burst?: uint, cost?: uint}): RateLimit\Decision` — `rule:core-classes/ratelimit-two-members`'s
    /// approximate member.
    ///
    /// `consume` without the door and without the round trip: the same
    /// arguments derive the same [`window`], [`step`] runs the arithmetic
    /// [`SCRIPT`] runs, and [`decoded`] reads the same three integers. What
    /// differs is where the arrival time lives — this core's own memory, so
    /// **the count is per core and a limit of 100 across eight cores admits up
    /// to 800**, which is the module doc's local-tier decision and § 1's stated
    /// arithmetic.
    ///
    /// Nothing here can fail on the world: there is no store to be unreachable
    /// and no host to be ungranted, which is why the member declares no
    /// capability (`registry::CAPABILITIES` carries the `None` row and `rule:testing/capability-completeness-test` is why that is a row rather than an exemption).
    ///
    /// # Errors
    ///
    /// A thrown `RuntimeError` for a limit that cannot be enforced, from
    /// [`window`] and from nowhere else.
    fn nvs_core_ratelimit_shed(ctx, args: [5]) {
        let key = key_of(args, 0, "shed")?;
        let limit = uint_of(args, 1, "shed")?.unwrap_or(0);
        let per = crate::time::nanos_of(args, 2, "shed")?;
        let burst = uint_of(args, 3, "shed")?.unwrap_or(limit);
        let cost = uint_of(args, 4, "shed")?.unwrap_or(1);

        let window = window(limit, per, burst, "shed")?;
        admissible(cost, burst, "shed")?;

        // The same namespacing as the coherent tier, for the same reason: the
        // store this writes to is the one `Core\Cache::local()` hands out, and
        // a program caching under `account:1` must not find a timestamp there.
        let namespaced = format!("{PREFIX}{key}");
        let now = crate::time::monotonic_micros();
        let (reply, admitted) = step(stored_tat(namespaced.as_bytes()), now, window, cost);
        if let Some(next_tat) = admitted {
            // Under the same `[cache.local] max_size` every other entry on this
            // core is under (`rule:concurrency/cache-memory-is-charged-to-the-core`), which is the whole of what the
            // `None` row in `registry::CAPABILITIES` says bounds this member: a
            // limiter that outgrew the cap would be the footprint a grant could
            // not have bounded anyway.
            crate::cache::store_put(
                namespaced.as_bytes(),
                next_tat.to_string().into_bytes(),
                // The cap is what bounds a limiter's entries, not a clock: a
                // window's own arithmetic already answers for an arrival too
                // old to matter, and an entry the cap forgot is the same
                // approximation `shed` is chosen for.
                crate::cache::Lifetime::Forever,
                crate::cache::local_cap(ctx),
            );
        }

        decoded(&reply, limit).map_err(|why| {
            // Unreachable: `step`'s own type is three integers. The arm exists
            // because `decoded` is the one reader of them for both tiers, and
            // a shape error there would be this crate's bug rather than a
            // store's — which is what `Fault::fatal` says and a thrown
            // `IOError` would not.
            Fault::fatal(format!("{NAME}::shed(): {why}"))
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\RateLimit\Decision::allowed(): bool` — § 3's first field.
    fn nvs_core_ratelimit_decision_allowed(_ctx, args: [1]) {
        slot_of(args, ALLOWED_SLOT, "allowed")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\RateLimit\Decision::limit(): uint` — § 3's second field.
    fn nvs_core_ratelimit_decision_limit(_ctx, args: [1]) {
        slot_of(args, LIMIT_SLOT, "limit")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\RateLimit\Decision::remaining(): uint` — § 3's third field.
    fn nvs_core_ratelimit_decision_remaining(_ctx, args: [1]) {
        slot_of(args, REMAINING_SLOT, "remaining")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\RateLimit\Decision::retryAfter(): ?Duration` — § 3's fourth field,
    /// and the one that is `null` rather than zero when there is no wait.
    fn nvs_core_ratelimit_decision_retry_after(_ctx, args: [1]) {
        slot_of(args, RETRY_AFTER_SLOT, "retryAfter")
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::time::Duration;

    use nvs_runtime::{Ctx, NvsStr, Tag};

    use super::{
        ALLOWED_SLOT, CLASS, CONSUME_DOC, DECISION, PREFIX, SCRIPT, SHED_DOC, Value, Window,
        admissible, decoded, nvs_core_ratelimit_consume, nvs_core_ratelimit_decision_allowed,
        nvs_core_ratelimit_decision_limit, nvs_core_ratelimit_decision_remaining,
        nvs_core_ratelimit_decision_retry_after, nvs_core_ratelimit_shed, step, window,
    };
    use crate::cache::redis::Connection;
    use crate::cache::{Dial, Target};

    /// What the per-core case limits, once, since it is both an argument and
    /// the key an entry is looked for under.
    const SHED_KEY: &str = "account:1";

    /// A listener on loopback and the address it took — `crate::cache::redis`'s
    /// own cases' shape, and the reason that module takes an address rather
    /// than a `Ctx`.
    fn listening() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let address = listener.local_addr().expect("the port it took");
        (listener, address)
    }

    /// Reads whatever one command's bytes are, up to a chunk — enough for a
    /// fake store to see the whole `EVAL` this module sends.
    fn read_command(stream: &mut TcpStream) -> Vec<u8> {
        let mut got = vec![0_u8; 4096];
        let read = stream.read(&mut got).expect("the client's command");
        got.truncate(read);
        got
    }

    /// One arrival at [`SHED_KEY`] on this thread, under a limit of one an
    /// hour, and whether it was admitted.
    ///
    /// The whole member through `nvs_runtime::call` rather than [`step`]
    /// alone, because what is being asserted is *where the arrival is kept* —
    /// which is the half the arithmetic does not have. An hour, so that no
    /// clock reading between two calls can drain the allowance and make the
    /// case's second answer a race.
    fn arrival(ctx: &mut Ctx) -> bool {
        let answered = nvs_runtime::call(
            nvs_core_ratelimit_shed,
            ctx,
            &[
                Value::str(NvsStr::new(SHED_KEY.as_bytes())),
                Value::uint(1),
                crate::time::duration_of(3_600 * 1_000_000_000),
                Value::null(),
                Value::null(),
            ],
        )
        .expect("`shed` reaches no store, so it has nothing to fail on");
        let object = answered.obj_ptr().expect("a decision is an instance");
        crate::instance::slot(object, ALLOWED_SLOT).as_bool() == Some(true)
    }

    /// `rule:core-classes/ratelimit-gcra`: the shared tier is GCRA, which is one stored timestamp and
    /// two derived parameters — the emission interval `per / limit` and the
    /// tolerance `burst × interval`, with `burst` defaulting to `limit` so that
    /// the default tolerance is exactly one period.
    ///
    /// Three claims, because the arithmetic alone would pass a module that
    /// computed GCRA's parameters and then sent something else. The parameters
    /// are § 2's; the script is a single stored timestamp rather than a window
    /// of arrivals; and a whole `EVAL` exchange carries those parameters and
    /// nothing else, with the reply read back as the decision it means.
    #[test]
    fn consume_is_gcra_over_the_shared_store() {
        // `per / limit`, and `burst × interval` — in microseconds.
        assert_eq!(
            window(4, 2_000_000_000, 4, "consume").expect("a limit of 4 per 2s"),
            Window {
                interval: 500_000,
                tau: 2_000_000,
            }
        );
        // The default burst is the limit, so the default tolerance is one whole
        // period: a period's worth may arrive at once.
        let per = 1_000_000_000;
        for limit in [1_u64, 3, 7, 250] {
            let derived = window(limit, per, limit, "consume").expect("a limit over a second");
            assert_eq!(
                derived.tau,
                derived.interval * i128::from(limit),
                "the tolerance is `burst × interval` at every limit"
            );
        }

        // One timestamp per key, not a window of them — the property that makes
        // § 2's memory O(1) per key rather than O(arrivals).
        assert!(
            SCRIPT.contains("local next_tat = tat + cost * interval"),
            "the step is the theoretical arrival time moving forward by the cost"
        );
        assert!(
            SCRIPT.contains("local allow_at = next_tat - tau"),
            "an arrival is admitted once the new arrival time is within the tolerance"
        );
        assert_eq!(
            SCRIPT.matches("redis.call('SET'").count(),
            1,
            "one stored value per key, written once per admitted arrival"
        );

        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            let sent = read_command(&mut stream);
            stream
                .write_all(b"*3\r\n:1\r\n:0\r\n:4\r\n")
                .expect("the reply");
            sent
        });

        let mut connection = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        connection.ensure().expect("the fake store is listening");
        let derived = window(5, 1_000_000_000, 5, "consume").expect("5 per second");
        let reply = connection
            .eval(
                SCRIPT,
                format!("{PREFIX}account:1").as_bytes(),
                &[
                    derived.interval.to_string().as_bytes(),
                    derived.tau.to_string().as_bytes(),
                    b"1",
                ],
            )
            .expect("the fake store answers the three integers");

        let sent = String::from_utf8(server.join().expect("the fake store runs to completion"))
            .expect("the command is text");
        assert!(
            sent.starts_with("*7\r\n$4\r\nEVAL\r\n"),
            "one `EVAL`: {sent}"
        );
        assert!(
            sent.contains("\r\n$1\r\n1\r\n$23\r\nnvs:ratelimit:account:1\r\n"),
            "one key, namespaced away from a cache entry of the same name: {sent}"
        );
        // 5 per second is a 200ms emission interval and, at the default burst,
        // one second of tolerance — both in microseconds, and both as `window`
        // derived them rather than as anything the wire recomputes.
        assert!(
            sent.ends_with("$6\r\n200000\r\n$7\r\n1000000\r\n$1\r\n1\r\n"),
            "the interval, the tolerance and the cost cross as they were derived: {sent}"
        );

        assert_eq!(reply, vec![1, 0, 4]);
        let decision = decoded(&reply, 5).expect("three integers are a decision");
        assert!(decision.obj_ptr().is_some(), "a decision is an instance");
    }

    /// `rule:core-classes/ratelimit-gcra` and `rule:core-classes/ratelimit-two-members`: the whole
    /// member, from a program's five arguments to the `Decision` it reads, over a deployment that
    /// granted `cache.shared` and configured a store.
    ///
    /// Two claims. A limit that can never be enforced is refused **before** the door, so the
    /// store is never dialled for it — asserted by the listener having nothing to accept. And an
    /// arrival the store refuses comes back as that refusal, field by field: the `cost` option
    /// crosses as written, and the wait is the store's microseconds as nanoseconds.
    // covers: Core\RateLimit::consume
    #[test]
    fn consume_refuses_an_unenforceable_limit_undialled_and_reads_the_stores_refusal() {
        let (listener, address) = listening();
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(&format!(
            "[capabilities]\ncache.shared = true\n\n[cache.shared]\nurl = \"redis://{address}\"\n"
        )));
        let arguments = |limit: u64, cost: Value| {
            [
                Value::str(NvsStr::new(b"account:1")),
                Value::uint(limit),
                crate::time::duration_of(1_000_000_000),
                Value::null(),
                cost,
            ]
        };

        let zero = nvs_runtime::call(
            nvs_core_ratelimit_consume,
            &mut ctx,
            &arguments(0, Value::null()),
        );
        assert!(zero.is_err(), "a limit of 0 is refused");
        let said = ctx.take_pending().unwrap_or_default();
        assert!(
            said.contains("a `$limit` of 0"),
            "the refusal names the limit: {said}"
        );
        listener
            .set_nonblocking(true)
            .expect("a listener can be polled");
        assert!(
            listener.accept().is_err(),
            "the refusal came before the door, so nothing dialled the store"
        );
        listener.set_nonblocking(false).expect("and put back");

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the member dials once");
            let sent = read_command(&mut stream);
            stream
                .write_all(b"*3\r\n:0\r\n:1234567\r\n:0\r\n")
                .expect("the reply");
            sent
        });
        let answered = nvs_runtime::call(
            nvs_core_ratelimit_consume,
            &mut ctx,
            &arguments(5, Value::uint(2)),
        )
        .expect("a configured, granted store that answers is a decision");
        let sent = String::from_utf8(server.join().expect("the fake store runs to completion"))
            .expect("the command is text");
        assert!(
            sent.ends_with("$1\r\n2\r\n"),
            "the `cost` option crosses as the script's last argument: {sent}"
        );

        let object = answered.obj_ptr().expect("a decision is an instance");
        assert_eq!(
            crate::instance::slot(object, ALLOWED_SLOT).as_bool(),
            Some(false)
        );
        assert_eq!(
            crate::instance::slot(object, super::LIMIT_SLOT).as_uint(),
            Some(5)
        );
        assert_eq!(
            crate::instance::slot(object, super::REMAINING_SLOT).as_uint(),
            Some(0)
        );
        let wait = crate::instance::slot(object, super::RETRY_AFTER_SLOT);
        assert_eq!(
            crate::time::nanos_of(&[wait], 0, "retryAfter").expect("a refusal carries a wait"),
            1_234_567_000
        );
    }

    /// `rule:core-classes/ratelimit-two-members` and `rule:core-classes/ratelimit-gcra`: `shed` is the same algorithm over this core's own
    /// memory, so what has to hold is that it is GCRA *and* that it is the same
    /// one — a second implementation that drifted from the first would make
    /// moving a call between the members a rewrite rather than a change of
    /// guarantee.
    ///
    /// Sameness is asserted where it can be: one signature to the letter, one
    /// [`window`] derivation, and the same three integers through the same
    /// decoder. GCRA is asserted as behaviour over a sequence — two arrivals
    /// inside a burst of two, the third refused with the exact wait until the
    /// next unit drains, and that arrival admitted once it has.
    // covers: Core\RateLimit::shed
    #[test]
    fn shed_is_the_same_gcra_over_the_cores_own_memory() {
        let [consume, shed] = CLASS.methods else {
            panic!("§ 1 is two members, and they are the whole class");
        };
        assert_eq!(
            consume.names, shed.names,
            "the same arguments in the same order, which is what makes the two names a choice"
        );
        assert_eq!(consume.params.len(), shed.params.len());
        assert!(
            matches!(shed.return_ty, crate::registry::CoreTy::Instance(name) if name == super::DECISION_NAME),
            "and the same `Decision` back, so a caller reads the answer identically"
        );

        // 2 per second, default burst: a 500ms emission interval and one second
        // of tolerance, both from the one `window` the shared tier uses.
        let derived = window(2, 1_000_000_000, 2, "shed").expect("2 per second");
        let (first, after_first) = step(None, 0, derived, 1);
        assert_eq!(first, [1, 0, 1], "the first of two admits, one unit left");
        let (second, after_second) = step(after_first, 0, derived, 1);
        assert_eq!(second, [1, 0, 0], "the second exhausts the burst");
        let (third, after_third) = step(after_second, 0, derived, 1);
        assert_eq!(
            third,
            [0, 500_000, 0],
            "the third is refused, and the wait is the exact drain of one unit"
        );
        assert!(
            after_third.is_none(),
            "a refusal charges nothing, so arriving while refused does not push the wait out"
        );
        let (fourth, _) = step(after_second, 500_000, derived, 1);
        assert_eq!(
            fourth[0], 1,
            "and it is admitted once that unit has drained"
        );

        // The same decoder as the shared tier, so `retryAfter` is `null` exactly
        // when allowed and exact otherwise, in one place for both members.
        let refused = decoded(&third, 2).expect("three integers are a decision");
        let object = refused.obj_ptr().expect("a decision is an instance");
        let wait = crate::instance::slot(object, super::RETRY_AFTER_SLOT);
        assert_eq!(
            crate::time::nanos_of(&[wait], 0, "retryAfter").expect("a `Duration`"),
            500_000_000,
        );
    }

    /// `rule:core-classes/ratelimit-gcra`: a cost past the burst is past the
    /// tolerance even on a key with no history, so [`step`] refuses it with a
    /// wait after which it is refused again. [`admissible`] is what keeps that
    /// wait from reaching a `retryAfter`, for both members, and a cost equal to
    /// the burst is still admitted in one arrival.
    // covers: Core\RateLimit::shed, Core\RateLimit::consume
    #[test]
    fn a_cost_past_the_burst_is_refused_rather_than_given_a_wait() {
        // 1 per hour with a burst of 5: a 3600-second interval.
        let derived = window(1, 3_600_000_000_000, 5, "shed").expect("1 per hour");
        let (refused, _) = step(None, 0, derived, 6);
        assert_eq!(
            refused,
            [0, 3_600_000_000, 5],
            "the step reports an hour's wait"
        );
        let (again, _) = step(None, 3_600_000_000, derived, 6);
        assert_eq!(
            again[0], 0,
            "and an hour later the same arrival is refused again"
        );

        assert!(admissible(6, 5, "shed").is_err());
        assert!(admissible(6, 5, "consume").is_err());
        assert!(admissible(u64::MAX, 5, "shed").is_err());
        assert!(admissible(5, 5, "shed").is_ok());
        let (whole, _) = step(None, 0, derived, 5);
        assert_eq!(
            whole,
            [1, 0, 0],
            "a cost equal to the burst is admitted at once"
        );
    }

    /// `rule:core-classes/ratelimit-two-members`: `shed`'s arrivals live in this core's own memory, which is
    /// the whole of what it trades away — the count is **per core**, and an
    /// arrival the tier forgets is one that never happened. Neither is a
    /// defect, and both are only useful to a caller who is told: a program that
    /// read a `shed` limit as a number somebody was promised has picked the
    /// wrong member, and the contract is the one place that can be said in time.
    ///
    /// Three claims, over the member rather than over [`step`], since what is
    /// at stake here is where the timestamp goes rather than the arithmetic on
    /// it. Two threads are two cores for a `thread_local` tier, so a limit of
    /// one admits one *each* — § 1's multiplication measured at two rather than
    /// restated at eight. An unrelated write that fills `rule:concurrency/cache-memory-is-charged-to-the-core`'s cap
    /// forgets the arrival, and the key then admits a burst GCRA alone would
    /// have refused. And [`SHED_DOC`] states both, plus the absence of the
    /// `IOError` its coherent twin documents.
    // covers: Core\RateLimit::shed
    #[test]
    fn shed_is_per_core_and_approximate_and_says_so() {
        let mut ctx = Ctx::buffered();
        assert!(
            arrival(&mut ctx),
            "the first arrival is inside a limit of one"
        );
        assert!(
            !arrival(&mut ctx),
            "and the second is refused, an hour before the first drains"
        );

        // Where that refusal is remembered: this core's local tier, under this
        // module's prefix rather than under the caller's own key.
        let entry = format!("{PREFIX}{SHED_KEY}");
        assert!(
            crate::cache::store_get(entry.as_bytes()).is_some(),
            "an arrival is an entry in `Core\\Cache`'s local tier, and nothing else holds one"
        );
        assert_eq!(
            crate::cache::store_get(SHED_KEY.as_bytes()),
            None,
            "a program's own cache entry of the same name is a different entry"
        );

        // Per core. The runtime is thread-per-core and the tier is a
        // `thread_local`, so a second thread is a second core for this
        // question — and it has no arrival to be refused against.
        let elsewhere = std::thread::spawn(|| arrival(&mut Ctx::buffered()))
            .join()
            .expect("the second core's thread runs to completion");
        assert!(
            elsewhere,
            "a limit of one admits one per core, which is why a promised number is `consume`'s"
        );

        // Approximate. `rule:concurrency/cache-memory-is-charged-to-the-core`'s cap forgets the entry written longest
        // ago, and an arrival is an ordinary entry: a program caching anything
        // at all can drop one, without knowing this member exists.
        for filler in 0..8 {
            crate::cache::store_put(
                format!("unrelated-{filler}").as_bytes(),
                vec![b'x'; 2048],
                crate::cache::Lifetime::Forever,
                Some(8 * 1024),
            );
        }
        assert_eq!(
            crate::cache::store_get(entry.as_bytes()),
            None,
            "the cap forgot the arrival, which § 1 says may happen at any time for any reason"
        );
        assert!(
            arrival(&mut ctx),
            "a forgotten arrival admits a burst — the approximation the tier is chosen for"
        );

        // And the card says so, in the two fields a caller reads while choosing
        // between the two members.
        let limit = SHED_DOC.params[1];
        assert_eq!(limit.name, "limit");
        assert!(
            limit.desc.contains("on this core") && limit.desc.contains("800"),
            "`limit`'s card has to state that the count multiplies by the number of cores"
        );
        assert!(
            SHED_DOC.ret.contains("forget an entry at any time")
                && SHED_DOC.ret.contains("admits a burst"),
            "and the return's card the eviction a caller would otherwise read as a bug"
        );
        assert!(
            SHED_DOC.errors.iter().all(|entry| entry.error != "IOError"),
            "there is no store to be unreachable, so there is no `IOError` to document"
        );
    }

    /// `rule:core-classes/ratelimit-gcra`: `retryAfter` is the theoretical arrival time minus now,
    /// computed rather than estimated — which is what a sliding-window counter
    /// cannot do, and why every refused client would otherwise retry in the
    /// same instant at the window edge.
    ///
    /// The claim is that the store's own figure crosses unchanged into the
    /// `Duration`, in particular that it is not rounded to `per` and not
    /// rounded to a whole second. A refusal 1.234567s away is 1.234567s away.
    #[test]
    fn retry_after_is_exact_rather_than_a_window_guess() {
        assert!(
            SCRIPT.contains("return {0, allow_at - now, left}"),
            "the wait is the arrival time minus now, and nothing else"
        );

        let refused = decoded(&[0, 1_234_567, 0], 5).expect("a refusal is a decision");
        let object = refused.obj_ptr().expect("a decision is an instance");
        let wait = crate::instance::slot(object, super::RETRY_AFTER_SLOT);
        let nanos = crate::time::nanos_of(&[wait], 0, "retryAfter").expect("a `Duration`");
        assert_eq!(
            nanos, 1_234_567_000,
            "the store's microseconds are the answer's nanoseconds, exactly"
        );

        // And the other half of § 3: allowed carries no wait at all, rather
        // than a zero a caller has to know to read as absence.
        let allowed = decoded(&[1, 0, 4], 5).expect("an allowance is a decision");
        let object = allowed.obj_ptr().expect("a decision is an instance");
        assert!(
            crate::instance::slot(object, super::RETRY_AFTER_SLOT).tag() == Some(Tag::Null),
            "`retryAfter` is `null` exactly when the arrival was allowed"
        );
    }

    /// `rule:core-classes/ratelimit-gcra`: the four readers a program calls
    /// answer the decision's own slots, through the symbols a call site reaches
    /// rather than the slot reads the cases above make.
    ///
    /// One allowed decision and one refused, since each reader's answer differs
    /// between them in a way a reader that read the wrong slot would not match.
    // covers: Core\RateLimit\Decision::allowed, Core\RateLimit\Decision::limit
    // covers: Core\RateLimit\Decision::remaining, Core\RateLimit\Decision::retryAfter
    #[test]
    fn the_four_readers_answer_the_decisions_own_slots() {
        let mut ctx = Ctx::buffered();
        let allowed = decoded(&[1, 0, 4], 5).expect("an allowance is a decision");
        let refused = decoded(&[0, 1_234_567, 0], 5).expect("a refusal is a decision");

        for (decision, admitted, left) in [(allowed, true, 4), (refused, false, 0)] {
            let receiver = [decision];
            let read = |reader: nvs_runtime::NvsFn, ctx: &mut Ctx| {
                nvs_runtime::call(reader, ctx, &receiver).expect("a reader cannot fail")
            };
            assert_eq!(
                read(nvs_core_ratelimit_decision_allowed, &mut ctx).as_bool(),
                Some(admitted)
            );
            assert_eq!(
                read(nvs_core_ratelimit_decision_limit, &mut ctx).as_uint(),
                Some(5),
                "`limit` is the limit the decision was made against, allowed or not"
            );
            assert_eq!(
                read(nvs_core_ratelimit_decision_remaining, &mut ctx).as_uint(),
                Some(left)
            );
            let wait = read(nvs_core_ratelimit_decision_retry_after, &mut ctx);
            if admitted {
                assert!(
                    wait.tag() == Some(Tag::Null),
                    "an allowed decision has no wait"
                );
            } else {
                assert_eq!(
                    crate::time::nanos_of(&[wait], 0, "retryAfter").expect("a `Duration`"),
                    1_234_567_000,
                    "a refused decision's wait is the exact figure it was decoded from"
                );
            }
        }
    }

    /// `rule:core-classes/ratelimit-unreachable-store-throws`: an unreachable store throws, and never decides *allowed*.
    ///
    /// Two claims, because the exchange failing is only half of it. A store
    /// that accepts and closes is an error and not an answer — and this module
    /// has exactly **one** place that builds a `Decision`, reached only from
    /// the reply, so there is no arrangement of the file in which a failure
    /// takes a fallback path to an allowance. The source scan is the half that
    /// still holds after someone adds a second call site.
    #[test]
    fn an_unreachable_store_throws_rather_than_deciding_allowed() {
        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the client dials once");
            drop(stream);
        });

        let mut connection = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        connection.ensure().expect("the fake store is listening");
        let failed = connection.eval(SCRIPT, b"k", &[b"1000", b"1000", b"1"]);
        assert!(
            failed.is_err(),
            "a store that hangs up mid-command is a failure, not an absence"
        );
        server.join().expect("the fake store runs to completion");

        // The scan stops at the test module, as `crate::cache`'s own does:
        // what a member can reach at run time is the shipped half.
        let shipped: Vec<&str> = include_str!("ratelimit.rs")
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]")
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let builds = shipped
            .iter()
            .filter(|line| line.contains("crate::instance::build("))
            .count();
        assert_eq!(
            builds, 1,
            "a `Decision` is built from {builds} places; § 5 holds because there is exactly one \
             and it is downstream of the store's reply"
        );
        assert!(
            !shipped.iter().any(|line| line.contains("unwrap_or_default")
                || line.contains(".ok()")
                || line.contains("is_err()")),
            "a fallback around the store's answer is how a limiter comes to fail open silently"
        );

        // And the card says so, because a caller that cannot read the failure
        // mode will assume the convenient one.
        assert!(
            CONSUME_DOC
                .errors
                .iter()
                .any(|entry| entry.error == "IOError" && entry.desc.contains("never")),
            "the card has to state that an unreachable store is never `allowed`"
        );
    }

    /// `rule:core-classes/ratelimit-two-members`: no configuration at all. The limit is an argument because
    /// only the application knows whether this is a plan quota or a login
    /// throttle, and a `[ratelimit]` block appearing later is the regression
    /// this pins — a directive would move the policy into a root-owned file
    /// that no longer sits beside the plan it belongs to.
    #[test]
    fn rate_limit_reads_no_directive() {
        // The shipped half only, as the scan above: what a member can read at
        // run time is what is above the test module.
        let shipped: String = include_str!("ratelimit.rs")
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]")
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !shipped.contains("ctx.config()"),
            "this module reads no directive; the one it reaches indirectly is `[cache.shared] \
             url`, which says where the store is and nothing about the policy"
        );
        assert!(
            !shipped.contains("\"ratelimit."),
            "a `[ratelimit]` directive key is the configuration block § 4 refuses to have"
        );

        // The policy is the argument list, and the whole of it: three
        // positionals and one bag of two, with no fifth thing to configure.
        let consume = CLASS.methods.first().expect("`consume` is the roster");
        assert_eq!(consume.names, ["key", "limit", "per"]);
        assert_eq!(
            DECISION.slots,
            ["allowed", "limit", "remaining", "retryAfter"],
            "§ 3's four fields, and the decision carries nothing else"
        );
    }
}
