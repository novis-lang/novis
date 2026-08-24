//! `Core\Regex` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 5, over [ADR 0056](../../../../docs/adr/0056-regex-engine-policy.md)'s
//! two engines.
//!
//! That ADR's body is the rule and this module is its implementation; nothing
//! about *why* the tiers exist is restated here.
//!
//! # The two crates, and why these two
//!
//! `docs/agent/loop-goal.md` § *Standing decisions* names both, so the choice
//! is recorded rather than made here — what belongs here is what each one is
//! doing:
//!
//! * **`regex`** is ADR 0056 § 1's linear tier. A finite-automata engine with
//!   no backtracking to exhaust, so [`Compiled::Linear`] needs no budget and
//!   is given none.
//! * **`fancy-regex`** is § 2's backtracking tier, reached only by a pattern
//!   the linear engine cannot express. It is built *on* `regex` — it delegates
//!   every non-fancy sub-expression to it — so the two agree on syntax, on
//!   Unicode tables and on what a character class means, which two unrelated
//!   engines would not. That shared core is the reason this pair rather than,
//!   say, `pcre2` behind a C shim: ADR 0051 § 4's second question asks what a
//!   C dependency buys, and the answer here would be a *second* opinion about
//!   pattern syntax, which is what ADR 0056 exists to avoid.
//!
//! # Tiering happens at the first call, not while checking
//!
//! ADR 0056 § 3 makes a **literal** pattern's tier a compile-time fact, over
//! [ADR 0057](../../../../docs/adr/0057-intrinsic-literal-folding.md)'s
//! literal-folding mechanism. That mechanism is not built, so today every
//! pattern — literal or assembled — takes the run-time path in [`compiled`]:
//! the linear engine is offered the pattern first and the backtracking engine
//! gets it only if the linear one refuses. The tier a given pattern lands in
//! is therefore already the tier § 3 will report; what is missing is the
//! *reporting*, the compile error for a malformed literal, and
//! `[regex] backtracking = "deny"`. Recorded as gap 1 below.
//!
//! # What a compiled pattern costs, and where it is held
//!
//! Compiling is orders of magnitude more expensive than matching, and the
//! spec's own members take a pattern by value at every call — so
//! `Regex::matches($line, "^\\d+$")` inside a loop would recompile per
//! iteration. [`compiled`] therefore memoizes, in a **thread-local** cache of
//! at most [`CACHE_CAPACITY`] entries.
//!
//! Thread-local is not a compromise here: the runtime is thread-per-core and
//! shared-nothing, so a per-thread cache needs no lock on the hot path and
//! cannot become cross-request state
//! ([ADR 0059](../../../../docs/adr/0059-cross-request-state-is-explicit.md)
//! forbids that) — a compiled pattern is derived from the pattern text alone,
//! observable only as speed.
//!
//! **What it spends:** one compiled program per distinct pattern per core,
//! bounded at [`CACHE_CAPACITY`], which is O(cache) rather than O(requests
//! served). The cache is cleared wholesale when it fills rather than evicted
//! by recency: an LRU costs a per-hit write on the hot path to buy a better
//! answer for a working set that does not fit, and a program with more than
//! [`CACHE_CAPACITY`] live patterns on one core is one this cache was never
//! going to serve.
//!
//! # Known gaps
//!
//! 1. **`compile` and `replaceWith` are not registered.** Both are stated in
//!    terms of `Pattern`, spec § 5's *option-carrying* handle: unlike
//!    [`MATCH`], whose whole state is values MWL already holds, a `Pattern` is
//!    a pattern plus four compilation flags, and the flags have to reach
//!    [`compiled`]'s cache key before either member means anything.
//!    `replaceWith` additionally hands its callback a `Match`, which
//!    `mwl_runtime::call_closure` can already carry.
//!
//!    The consequence for the six members that *are* registered is that their
//!    `Pattern|string $pattern` parameter is registered as `string` alone. It
//!    widens to the union the spec writes the moment `Pattern` is expressible;
//!    a program written against the narrow spelling keeps compiling.
//! 2. **ADR 0056 § 4's sink is not enforced.** The pattern parameter must
//!    demand the plain, unqualified `string`, and nothing in
//!    [`crate::registry`] can state a qualifier at all — `tainted` and
//!    `secret` are grammar and checker rows without a `Core`-facing half yet.
//!    A registry row that cannot say "plain `string` only" accepts a tainted
//!    pattern, which is the one place this module is currently *less* safe
//!    than that ADR requires.
//! 3. **The step budget is a constant, not a directive.** ADR 0056 § 2 puts
//!    the default in `mwl.toml` under ADR 0005's ordinary rules, and there is
//!    no configuration subsystem before M6. [`BACKTRACK_BUDGET`] is that
//!    default, stated once, and reading it from config is a change to that one
//!    line.
//! 4. **`matchAll` converts each match's offset over the subject's prefix**,
//!    so reporting positions for *k* matches in an *n*-byte subject is O(n·k)
//!    rather than O(n) — [`crate::granularity::Unit::index_of_byte`] counts
//!    from the start each time. The matches arrive in increasing order, so the
//!    fix is a cursor that counts only the gap since the previous one; it is
//!    not written because a cluster can in principle span a match boundary, and
//!    getting that edge right is worth its own slice rather than a line here.

use std::cell::RefCell;
use std::rc::Rc;

use mwl_runtime::{Fault, HelperResult, MwlArray, MwlStr, Tag, Value};

use crate::granularity::DEFAULT;
use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Regex`'s registry rows, in the spec's own order.
///
/// Six of § 5's eight members; gap 1 above owns which two are missing and what
/// they wait on. The four members that section states on a `Match` are
/// [`MATCH`]'s own roster, not this one.
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Regex",
    methods: &[
        CoreMethod {
            name: "matches",
            params: &[CoreTy::Str, CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_regex_matches",
        },
        CoreMethod {
            name: "match",
            params: &[CoreTy::Str, CoreTy::Str, CoreTy::Options(MATCH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(MATCH_NAME)),
            symbol: "mwl_core_regex_match",
        },
        CoreMethod {
            name: "matchAll",
            params: &[CoreTy::Str, CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(MATCH_NAME)),
            symbol: "mwl_core_regex_match_all",
        },
        CoreMethod {
            name: "replace",
            params: &[
                CoreTy::Str,
                CoreTy::Str,
                CoreTy::Str,
                CoreTy::Options(REPLACE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_regex_replace",
        },
        CoreMethod {
            name: "split",
            params: &[CoreTy::Str, CoreTy::Str, CoreTy::Options(SPLIT_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "mwl_core_regex_split",
        },
        CoreMethod {
            name: "quote",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_regex_quote",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Regex\Match`'s fully-qualified name, written once — [`MATCH`]
/// declares it and every [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
const MATCH_NAME: &str = r"Core\Regex\Match";

/// Spec § 5's `Core\Regex\Match` — the first `Core`-owned instance, and the
/// shape that replaces `preg_match`'s `$matches` out-parameter (ADR 0063 R3
/// forbids one) together with `PREG_OFFSET_CAPTURE`.
///
/// Four members over two slots, and no static member at all: a `Match` is only
/// ever produced by [`mwl_core_regex_match`] or [`mwl_core_regex_match_all`].
/// [`crate::instance`] owns what a `Core`-owned instance *is*; what belongs
/// here is what this one holds.
///
/// **The groups are materialized when the match is made, not when they are
/// asked for.** Slot `groups` is the whole `array<?string>` the spec's
/// `groups()` answers with, and every other member reads it: `group(k)` is a
/// key lookup in it, `text()` is `group(0)`, and only `offset` is stored
/// beside it. The alternative — keeping the subject and each group's byte
/// range, and cutting a string per `group()` call — would make a `Match` cheap
/// to produce and repeatedly expensive to read, and would leave it holding a
/// reference to a subject the caller has otherwise finished with. **What it
/// spends:** one array plus one `string` per participating group (two for a
/// named one, which is stored under both its name and its number, exactly as
/// `preg_match` returns it), per match, charged to the request.
pub const MATCH: CoreClass = CoreClass {
    name: MATCH_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "group",
            params: &[CoreTy::Union(&[CoreTy::Int, CoreTy::Str])],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_regex_match_group",
        },
        CoreMethod {
            name: "groups",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Nullable(&CoreTy::Str)),
            symbol: "mwl_core_regex_match_groups",
        },
        CoreMethod {
            name: "offset",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_regex_match_offset",
        },
        CoreMethod {
            name: "text",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_regex_match_text",
        },
    ],
    slots: &["groups", "offset"],
    constants: &[],
};

/// [`MATCH`]'s `groups` slot, by index — what every member below reads.
///
/// A constant rather than a `MATCH.slot("groups")` call at each use: the index
/// is fixed at compile time and the lookup is a string compare per member call.
/// `the_slot_constants_match_the_registered_layout` holds the two together.
const GROUPS_SLOT: usize = 0;

/// [`MATCH`]'s `offset` slot, by index — see [`GROUPS_SLOT`].
const OFFSET_SLOT: usize = 1;

/// `Core\Regex::match`'s `{from?: int}` — the position in the subject to start
/// searching at, defaulting to its beginning.
///
/// A [`crate::granularity::DEFAULT`]-unit index, like every other `string`
/// position MWL takes or hands back, and negative counts from the end under
/// [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R8. It is
/// **not** `preg_match`'s `$offset`, which counts bytes and documents that a
/// value inside a multi-byte character is undefined behaviour.
///
/// The match is still made against the whole subject, so a look-behind or a
/// `^` sees what precedes `from` — the same treatment both engines' own
/// "search from" entry points give, and the only one under which
/// `Regex::match($s, $p, {from: $m->offset() + 1})` finds the second match
/// rather than a different pattern's.
const MATCH_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "from",
    ty: CoreTy::Int,
    default: Const::Int(0),
}];

/// `Core\Regex::replace`'s `{limit?: uint}` — how many matches to replace,
/// defaulting to `uint`'s maximum, which is "every one".
///
/// The same spelling, and the same reason, as `Core\Str::replace`'s: a
/// sentinel `0` would collide with the perfectly sensible "replace nothing"
/// a computed limit can produce, and [`Const`] has no `null` that a `uint`
/// option could carry.
const REPLACE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "limit",
    ty: CoreTy::Uint,
    default: Const::Uint(u64::MAX),
}];

/// `Core\Regex::split`'s `{limit?: int, keepEmpty?: bool}`.
///
/// `limit` has **`Core\Str::split`'s three-sign rule**, which that member's
/// own docs state and this one does not repeat — one word, one meaning, across
/// the two members that carry it. [`mwl_core_regex_split`] owns the single
/// place that rule and PHP's `preg_split` disagree.
const SPLIT_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "limit",
        ty: CoreTy::Int,
        default: Const::Int(i64::MAX),
    },
    CoreOption {
        name: "keepEmpty",
        ty: CoreTy::Bool,
        default: Const::Bool(true),
    },
];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_regex_matches" => (mwl_core_regex_matches as *const ()).cast(),
        "mwl_core_regex_match" => (mwl_core_regex_match as *const ()).cast(),
        "mwl_core_regex_match_all" => (mwl_core_regex_match_all as *const ()).cast(),
        "mwl_core_regex_match_group" => (mwl_core_regex_match_group as *const ()).cast(),
        "mwl_core_regex_match_groups" => (mwl_core_regex_match_groups as *const ()).cast(),
        "mwl_core_regex_match_offset" => (mwl_core_regex_match_offset as *const ()).cast(),
        "mwl_core_regex_match_text" => (mwl_core_regex_match_text as *const ()).cast(),
        "mwl_core_regex_replace" => (mwl_core_regex_replace as *const ()).cast(),
        "mwl_core_regex_split" => (mwl_core_regex_split as *const ()).cast(),
        "mwl_core_regex_quote" => (mwl_core_regex_quote as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The two tiers
// ============================================================================

/// ADR 0056 § 2's step budget for the backtracking tier, and gap 3's constant.
///
/// `fancy-regex`'s own default, kept rather than lowered: it is the figure
/// that crate's adversarial-pattern tests are written against, and picking a
/// different one here would be a number with no measurement behind it. What
/// matters for the ADR is that exhausting it *throws*, which
/// [`budget_exhausted`] is.
const BACKTRACK_BUDGET: usize = 1_000_000;

/// How many compiled patterns one core holds before the cache is cleared —
/// this module's own docs own the reasoning and what it spends.
const CACHE_CAPACITY: usize = 256;

/// One compiled pattern, in whichever tier ADR 0056 § 1 placed it.
///
/// The tier is not a user-visible property of the value: no member below
/// branches on it for anything but which engine's API to call, and neither
/// answers a different question from the other.
#[derive(Debug)]
enum Compiled {
    /// § 1's linear tier — no budget, because it needs none.
    Linear(regex::Regex),
    /// § 2's budgeted backtracking tier, reached only because the linear
    /// engine refused this pattern.
    Backtracking(fancy_regex::Regex),
}

thread_local! {
    /// This core's compiled patterns, keyed by the pattern text.
    ///
    /// A `Vec` rather than a map: it is capacity-bounded and scanned
    /// linearly, which for a few hundred short keys beats hashing them, and
    /// it keeps the "clear when full" policy a one-liner.
    static CACHE: RefCell<Vec<(String, Rc<Compiled>)>> = const { RefCell::new(Vec::new()) };
}

/// The compiled form of `pattern`, from this core's cache or freshly built.
///
/// # Errors
///
/// A `Fault::thrown` naming the pattern when **neither** engine can compile
/// it, which is ADR 0056 § 5's "a construct neither engine supports is
/// diagnosed, never silently ignored" — at run time today, since gap 1 above
/// owns the compile-time half. The message carries the backtracking engine's
/// own complaint, because it is the more permissive of the two: a pattern the
/// linear engine merely could not *express* has already been handed on by the
/// time this fails.
fn compiled(pattern: &str, member: &str) -> Result<Rc<Compiled>, Fault> {
    if let Some(hit) = CACHE.with_borrow(|cache| {
        cache
            .iter()
            .find(|(key, _)| key == pattern)
            .map(|(_, compiled)| Rc::clone(compiled))
    }) {
        return Ok(hit);
    }

    let built = match regex::Regex::new(pattern) {
        Ok(linear) => Compiled::Linear(linear),
        Err(_) => Compiled::Backtracking(
            fancy_regex::RegexBuilder::new(pattern)
                .backtrack_limit(BACKTRACK_BUDGET)
                .build()
                .map_err(|err| {
                    Fault::thrown(format!(
                        "Core\\Regex::{member}(): `{pattern}` is not a pattern either engine can \
                         compile: {err}"
                    ))
                })?,
        ),
    };

    let built = Rc::new(built);
    CACHE.with_borrow_mut(|cache| {
        if cache.len() >= CACHE_CAPACITY {
            cache.clear();
        }
        cache.push((pattern.to_owned(), Rc::clone(&built)));
    });
    Ok(built)
}

/// ADR 0056 § 2's throw: the backtracking tier ran out of steps.
///
/// Never a falsy return and never a truncated search — the whole point of that
/// section is that PHP's `pcre.backtrack_limit` turns a hang into a wrong
/// answer, and this turns it into a `Throwable` the request can catch.
///
/// A `fancy_regex::Error` that is *not* the budget is still a throw: at this
/// point the pattern has already compiled, so the only remaining runtime
/// failures are the budget and a stack overflow in the backtracker, and both
/// mean "this pattern cannot be run against this subject."
fn budget_exhausted(member: &str, pattern: &str, err: &fancy_regex::Error) -> Fault {
    Fault::thrown(format!(
        "Core\\Regex::{member}(): `{pattern}` exhausted the backtracking budget of \
         {BACKTRACK_BUDGET} steps against this subject ({err})"
    ))
}

// ============================================================================
// Argument decoding
// ============================================================================

/// One `string` argument's text. Both failures are `FATAL` for
/// `Core\Str`'s reasons, which that module's own `text` states.
fn text<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    let bytes = value.as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex::{member} expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })?;
    std::str::from_utf8(bytes).map_err(|_| {
        Fault::fatal(format!(
            "Core\\Regex::{member} received a `string` that is not valid UTF-8, which ADR 0009 \
             guarantees it cannot be"
        ))
    })
}

/// One `int` argument.
fn integer(value: &Value, member: &str, position: &str) -> Result<i64, Fault> {
    value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex::{member} expected {:?} for {position}, got tag {}",
            Tag::Int,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument.
fn unsigned(value: &Value, member: &str, position: &str) -> Result<u64, Fault> {
    value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex::{member} expected {:?} for {position}, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })
}

/// One `bool` argument.
fn boolean(value: &Value, member: &str, position: &str) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex::{member} expected {:?} for {position}, got tag {}",
            Tag::Bool,
            value.tag_byte()
        ))
    })
}

/// A freshly built `string` result.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(MwlStr::new(text.as_bytes())))
}

// ============================================================================
// The members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Regex::matches(string $subject, string $pattern): bool` —
    /// replacing `preg_match` used as a predicate.
    ///
    /// The pattern is unanchored, as PHP's is: it asks whether the subject
    /// *contains* a match, and `^`/`$` are how a call asks for more.
    fn mwl_core_regex_matches(_ctx, args: [2]) {
        let subject = text(&args[0], "matches", "the subject")?;
        let pattern = text(&args[1], "matches", "the pattern")?;
        let found = match &*compiled(pattern, "matches")? {
            Compiled::Linear(re) => re.is_match(subject),
            Compiled::Backtracking(re) => re
                .is_match(subject)
                .map_err(|err| budget_exhausted("matches", pattern, &err))?,
        };
        Ok(Value::bool(found))
    }
}

// ============================================================================
// `Core\Regex\Match` — the object the two matching members produce
// ============================================================================

/// One match reduced to the one shape both engines answer in: every group the
/// pattern declares, `None` where it did not participate, and each
/// participating one's byte offset into the subject plus its text.
type Captured<'a> = Vec<Option<(usize, &'a str)>>;

/// [`Captured`] from the linear tier's own capture set.
fn linear_groups<'a>(caps: &regex::Captures<'a>) -> Captured<'a> {
    (0..caps.len())
        .map(|number| {
            caps.get(number)
                .map(|found| (found.start(), found.as_str()))
        })
        .collect()
}

/// [`Captured`] from the backtracking tier's own capture set.
fn backtracking_groups<'a>(caps: &fancy_regex::Captures<'a, str>) -> Captured<'a> {
    (0..caps.len())
        .map(|number| {
            caps.get(number)
                .map(|found| (found.start(), found.as_str()))
        })
        .collect()
}

/// Every group's name, indexed by group number — `None` for an unnamed group,
/// and always `None` at index 0, which is the whole match.
fn names_of(compiled: &Compiled) -> Vec<Option<&str>> {
    match compiled {
        Compiled::Linear(re) => re.capture_names().collect(),
        Compiled::Backtracking(re) => re.capture_names().collect(),
    }
}

/// The byte offset a search starting at unit index `from` begins at.
///
/// Negative counts from the end ([`MATCH_OPTIONS`]), and either end saturates:
/// a `from` past the subject searches an empty remainder and finds nothing,
/// which composes with a loop where a throw would not.
fn start_byte(subject: &str, from: i64) -> usize {
    let index = if from < 0 {
        let total = i64::try_from(DEFAULT.length(subject)).unwrap_or(i64::MAX);
        total.saturating_add(from).max(0)
    } else {
        from
    };
    DEFAULT.byte_of_index(subject, usize::try_from(index).unwrap_or(usize::MAX))
}

/// One `Core\Regex\Match` over `captured`, as the value a member returns.
///
/// [`MATCH`]'s own docs own the shape and what it spends; what is here is the
/// order, which is `preg_match`'s: a named group is written under its name and
/// then under its number, so a program migrating from PHP reads the same array
/// back.
fn built_match(subject: &str, names: &[Option<&str>], captured: &Captured<'_>) -> Value {
    let text_of = |group: Option<(usize, &str)>| {
        group.map_or_else(Value::null, |(_, text)| {
            Value::str(MwlStr::new(text.as_bytes()))
        })
    };
    let mut groups = MwlArray::new();
    for (number, group) in captured.iter().enumerate() {
        if let Some(name) = names.get(number).copied().flatten() {
            groups.set(MwlStr::new(name.as_bytes()), text_of(*group));
        }
        groups.append(text_of(*group));
    }
    // The whole match always participates, so the `0` below is unreachable for
    // any capture set an engine produced.
    let offset = captured
        .first()
        .copied()
        .flatten()
        .map_or(0, |(byte, _)| DEFAULT.index_of_byte(subject, byte));
    crate::instance::build(
        &MATCH,
        [
            Value::array(groups),
            Value::int(i64::try_from(offset).unwrap_or(i64::MAX)),
        ],
    )
}

/// The group array one of [`MATCH`]'s members reads, borrowed from its
/// receiver.
fn group_array(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<MwlArray>, Fault> {
    let receiver = crate::instance::receiver(args[0], &MATCH, member)?;
    let groups = crate::instance::slot(receiver, GROUPS_SLOT);
    let array = groups.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex\\Match::{member} found tag {} in its `groups` slot",
            groups.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// One `int|string` group key as the bytes an array is keyed by — an integer
/// group is stored under its decimal rendering, which is how every integer key
/// in an MWL array is spelled.
fn group_key(value: &Value, member: &str) -> Result<Vec<u8>, Fault> {
    if let Some(number) = value.as_int() {
        return Ok(number.to_string().into_bytes());
    }
    if let Some(bytes) = value.as_str_bytes() {
        return Ok(bytes.to_vec());
    }
    Err(Fault::fatal(format!(
        "Core\\Regex\\Match::{member} expected an `int|string` group, got tag {}",
        value.tag_byte()
    )))
}

mwl_runtime::mwl_helper! {
    /// `Core\Regex::match(string $subject, string $pattern, {from?: int}): ?Match`
    /// — replacing `preg_match`, its `$matches` out-parameter and
    /// `PREG_OFFSET_CAPTURE` at once.
    ///
    /// `null` is "no match", which ADR 0063 R5 makes the only absence
    /// spelling — there is no `0`/`false`/`1` return to read, and no error code
    /// beside it, because a pattern that cannot run throws
    /// ([`budget_exhausted`], [`compiled`]).
    fn mwl_core_regex_match(_ctx, args: [3]) {
        let subject = text(&args[0], "match", "the subject")?;
        let pattern = text(&args[1], "match", "the pattern")?;
        let from = integer(&args[2], "match", "the `from` option")?;
        let start = start_byte(subject, from);

        let compiled = compiled(pattern, "match")?;
        let names = names_of(&compiled);
        let found = match &*compiled {
            Compiled::Linear(re) => re
                .captures_at(subject, start)
                .map(|caps| linear_groups(&caps)),
            Compiled::Backtracking(re) => re
                .captures_from_pos(subject, start)
                .map_err(|err| budget_exhausted("match", pattern, &err))?
                .map(|caps| backtracking_groups(&caps)),
        };
        Ok(found.map_or_else(Value::null, |captured| {
            built_match(subject, &names, &captured)
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Regex::matchAll(string $subject, string $pattern): array<Match>` —
    /// replacing `preg_match_all` and both of its ordering flags.
    ///
    /// One `Match` per match, in the order they occur: `PREG_SET_ORDER`'s
    /// shape, since `PREG_PATTERN_ORDER`'s transpose is a differently-shaped
    /// return from the same member, which ADR 0063 R7 refuses.
    /// `Core\Arr::map($matches, fn($m) => $m->group(1))` is the transpose, in
    /// one line, when a caller wants it.
    ///
    /// **Each match's `offset` is converted from bytes independently**, which
    /// costs a pass over the subject's prefix per match — gap 4 below owns
    /// that.
    fn mwl_core_regex_match_all(_ctx, args: [2]) {
        let subject = text(&args[0], "matchAll", "the subject")?;
        let pattern = text(&args[1], "matchAll", "the pattern")?;

        let compiled = compiled(pattern, "matchAll")?;
        let names = names_of(&compiled);
        let mut out = MwlArray::new();
        match &*compiled {
            Compiled::Linear(re) => {
                for caps in re.captures_iter(subject) {
                    out.append(built_match(subject, &names, &linear_groups(&caps)));
                }
            }
            Compiled::Backtracking(re) => {
                for caps in re.captures_iter(subject) {
                    let caps =
                        caps.map_err(|err| budget_exhausted("matchAll", pattern, &err))?;
                    out.append(built_match(subject, &names, &backtracking_groups(&caps)));
                }
            }
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `$match->group(int|string $group): ?string` — one group's text, or
    /// `null` where the pattern declares that group but this match did not
    /// reach it.
    ///
    /// A group the **pattern** does not declare is a different question, and
    /// **throws**: ADR 0063 R5's `?T` says "this match has no such text", while
    /// R4's throw says "there is no such group to ask about." PHP answers both
    /// with an absent array entry, which is why `preg_match` code so often
    /// reads a typo as an empty capture.
    fn mwl_core_regex_match_group(_ctx, args: [2]) {
        let groups = group_array(args, "group")?;
        let key = group_key(&args[1], "group")?;
        let found = groups.get(&key).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Regex\\Match::group(): the pattern declares no group `{}`",
                String::from_utf8_lossy(&key)
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "the group array owns the reference this borrowed read \
                      returned, so the caller needs one of its own"
        )]
        unsafe {
            found.retain();
        }
        Ok(found)
    }
}

mwl_runtime::mwl_helper! {
    /// `$match->groups(): array<?string>` — every group at once, in
    /// `preg_match`'s own order and shape ([`built_match`]).
    fn mwl_core_regex_match_groups(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &MATCH, "groups")?;
        let groups = crate::instance::slot(receiver, GROUPS_SLOT);
        #[expect(
            unsafe_code,
            reason = "the receiver's slot owns the reference this borrowed read \
                      returned, so the caller needs one of its own"
        )]
        unsafe {
            groups.retain();
        }
        Ok(groups)
    }
}

mwl_runtime::mwl_helper! {
    /// `$match->offset(): int` — where the whole match starts in the subject,
    /// counted in [`crate::granularity::DEFAULT`]'s unit like every other
    /// `string` position, not in `PREG_OFFSET_CAPTURE`'s bytes.
    fn mwl_core_regex_match_offset(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &MATCH, "offset")?;
        Ok(crate::instance::slot(receiver, OFFSET_SLOT))
    }
}

mwl_runtime::mwl_helper! {
    /// `$match->text(): string` — the whole match's text, which is group `0`.
    ///
    /// Not nullable: group 0 participates in every match an engine reports, so
    /// a missing slot here is a corrupted instance rather than an absent value.
    fn mwl_core_regex_match_text(_ctx, args: [1]) {
        let groups = group_array(args, "text")?;
        let whole = groups.get(b"0").ok_or_else(|| {
            Fault::fatal("Core\\Regex\\Match::text() found no group `0` on this match")
        })?;
        #[expect(
            unsafe_code,
            reason = "the group array owns the reference this borrowed read \
                      returned, so the caller needs one of its own"
        )]
        unsafe {
            whole.retain();
        }
        Ok(whole)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Regex::replace(string $subject, string $pattern, string $replacement, {limit?: uint}): string`
    /// — replacing `preg_replace`.
    ///
    /// **`$1` and `${name}` are the group spellings**, and `$$` is a literal
    /// `$`. PHP additionally accepts `\1`; it is not accepted here, because
    /// `\1` inside a double-quoted MWL string is already an escape the lexer
    /// reads, so the same source text would mean two different things
    /// depending on the quote used to write it. A group reference that names
    /// no group expands to the empty string, as PHP's does.
    ///
    /// `limit` counts *replacements*, defaults to every one, and a limit of
    /// `0` replaces nothing — which is the reading the option's `uint` type
    /// forces and the one PHP's own `preg_replace` gives it.
    fn mwl_core_regex_replace(_ctx, args: [4]) {
        let subject = text(&args[0], "replace", "the subject")?;
        let pattern = text(&args[1], "replace", "the pattern")?;
        let replacement = text(&args[2], "replace", "the replacement")?;
        let limit = unsigned(&args[3], "replace", "the `limit` option")?;
        if limit == 0 {
            return produced(subject);
        }
        // Both engines spell "every match" as `0`, so an unlimited call has to
        // say so rather than passing a huge count.
        let count = usize::try_from(limit).unwrap_or(usize::MAX);
        let count = if limit == u64::MAX { 0 } else { count };

        let replaced = match &*compiled(pattern, "replace")? {
            Compiled::Linear(re) => re.replacen(subject, count, replacement).into_owned(),
            Compiled::Backtracking(re) => re
                .try_replacen(subject, count, replacement)
                .map_err(|err| budget_exhausted("replace", pattern, &err))?
                .into_owned(),
        };
        produced(&replaced)
    }
}

/// Every piece `pattern` splits `subject` into, at most `pieces` of them with
/// the last holding the unsplit remainder.
fn pieces_of<'a>(
    compiled: &Compiled,
    subject: &'a str,
    pieces: Option<usize>,
    pattern: &str,
) -> Result<Vec<&'a str>, Fault> {
    match (compiled, pieces) {
        (Compiled::Linear(re), Some(pieces)) => Ok(re.splitn(subject, pieces).collect()),
        (Compiled::Linear(re), None) => Ok(re.split(subject).collect()),
        (Compiled::Backtracking(re), Some(pieces)) => re
            .splitn(subject, pieces)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| budget_exhausted("split", pattern, &err)),
        (Compiled::Backtracking(re), None) => re
            .split(subject)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| budget_exhausted("split", pattern, &err)),
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Regex::split(string $subject, string $pattern, {limit?: int, keepEmpty?: bool}): array<string>`
    /// — replacing `preg_split` and its four flags.
    ///
    /// `limit` is **`Core\Str::split`'s rule**, stated there: positive is at
    /// most that many pieces with the last holding the remainder, negative
    /// drops that many pieces off the end, and zero yields the subject
    /// unsplit. That is `explode`'s reading of the word, not `preg_split`'s,
    /// and the difference is deliberate. `preg_split` reads `0` and `-1` as
    /// "no limit" and gives any *other* negative no documented meaning at all
    /// — PHP 8.5 answers `preg_split('/\d/', 'a1b2c3d', -2)` with the subject
    /// unsplit — so there is no coherent rule to be compatible with below
    /// zero, and being compatible at zero would leave two members named
    /// `split` disagreeing about one word. One divergence, at `0`, and it is
    /// the rarer call.
    ///
    /// `keepEmpty` defaults to `true`, which is `preg_split` with no flags;
    /// `false` is its `PREG_SPLIT_NO_EMPTY`, and drops empty pieces **after**
    /// the limit has been applied — so `limit` always counts the pieces the
    /// split produced, not the ones that survived.
    ///
    /// `PREG_SPLIT_DELIM_CAPTURE` and `PREG_SPLIT_OFFSET_CAPTURE` have no
    /// option: the first returns a differently-shaped array from the same
    /// member, which ADR 0063 R7 refuses, and the second is what `matchAll`
    /// answers.
    fn mwl_core_regex_split(_ctx, args: [4]) {
        let subject = text(&args[0], "split", "the subject")?;
        let pattern = text(&args[1], "split", "the pattern")?;
        let limit = integer(&args[2], "split", "the `limit` option")?;
        let keep_empty = boolean(&args[3], "split", "the `keepEmpty` option")?;

        let compiled = compiled(pattern, "split")?;
        let mut pieces = if limit >= 0 {
            // A limit of `0` means one piece, not none — see the docs above.
            let wanted = usize::try_from(limit).unwrap_or(usize::MAX).max(1);
            pieces_of(&compiled, subject, Some(wanted), pattern)?
        } else {
            let all = pieces_of(&compiled, subject, None, pattern)?;
            let dropped = usize::try_from(limit.unsigned_abs()).unwrap_or(usize::MAX);
            let kept = all.len().saturating_sub(dropped);
            all.into_iter().take(kept).collect()
        };
        if !keep_empty {
            pieces.retain(|piece| !piece.is_empty());
        }

        let mut out = MwlArray::new();
        for piece in pieces {
            out.append(Value::str(MwlStr::new(piece.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Regex::quote(string $literal): string` — replacing `preg_quote`,
    /// and ADR 0056 § 4's one laundering member for the pattern sink.
    ///
    /// Escapes every character either engine gives a meaning to, so the result
    /// matches `$literal` and nothing else. PHP's optional `$delimiter`
    /// argument has no equivalent, because ADR 0056 § 5 removed the
    /// `/…/` delimiter syntax it existed for: a pattern here is a pattern, not
    /// a pattern wrapped in punctuation.
    fn mwl_core_regex_quote(_ctx, args: [1]) {
        let literal = text(&args[0], "quote", "the literal")?;
        produced(&regex::escape(literal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two slot constants every member below indexes with are the slots
    /// [`MATCH`] declares — the one place the fast path and the registered
    /// layout are held together.
    #[test]
    fn the_slot_constants_match_the_registered_layout() {
        assert_eq!(GROUPS_SLOT, MATCH.slot("groups"));
        assert_eq!(OFFSET_SLOT, MATCH.slot("offset"));
    }

    /// A named group is readable under its name *and* its number, and a group
    /// the pattern declares but the subject did not reach is present and
    /// `null` — the two properties `group`'s throw-versus-`null` split rests
    /// on.
    #[test]
    fn a_match_carries_every_declared_group_named_and_numbered() {
        let compiled = compiled(r"(?<word>[a-z]+)(\d+)?", "match").expect("compiles");
        let names = names_of(&compiled);
        let Compiled::Linear(re) = &*compiled else {
            panic!("a plain pattern lands in the linear tier")
        };
        let caps = re.captures_at("  abc", 0).expect("matches");
        let value = built_match("  abc", &names, &linear_groups(&caps));
        let object = value.obj_ptr().expect("a Match is an object");
        let groups = crate::instance::slot(object, GROUPS_SLOT);
        let held = crate::arr::borrowed(groups.array_ptr().expect("an array"));
        assert_eq!(
            held.get(b"0")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"abc".to_vec())
        );
        assert_eq!(
            held.get(b"word")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"abc".to_vec())
        );
        // Declared, unmatched: present and null, which is what makes
        // `group(2)` answer `null` while `group(3)` throws.
        assert!(held.get(b"2").is_some_and(|v| v.tag() == Some(Tag::Null)));
        assert!(held.get(b"3").is_none());
        // Two leading spaces, so the offset is not the trivial zero.
        assert_eq!(crate::instance::slot(object, OFFSET_SLOT).as_int(), Some(2));
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference `built_match` produced"
        )]
        unsafe {
            value.release();
        }
    }

    /// The tier is chosen by the pattern, never by the caller — ADR 0056 § 1.
    /// A plain pattern lands linear; one with a lookahead the linear engine
    /// cannot express falls to the backtracking tier rather than failing.
    #[test]
    fn a_pattern_lands_in_the_tier_its_features_require() {
        assert!(matches!(
            *compiled(r"\d+", "matches").expect("a plain pattern compiles"),
            Compiled::Linear(_)
        ));
        assert!(matches!(
            *compiled(r"foo(?=bar)", "matches").expect("a lookahead compiles on the second tier"),
            Compiled::Backtracking(_)
        ));
    }

    /// A pattern neither engine can compile throws rather than matching
    /// nothing — ADR 0056 § 5's "never silently ignored".
    #[test]
    fn a_pattern_neither_engine_accepts_throws() {
        let err = compiled("(unclosed", "matches").expect_err("an unclosed group is not a pattern");
        assert!(format!("{err:?}").contains("(unclosed"), "{err:?}");
    }

    /// A compiled pattern is held, so a loop over one pattern compiles it
    /// once — this module's own docs own what that spends.
    #[test]
    fn one_pattern_is_compiled_once_per_core() {
        let first = compiled(r"^cached-\w+$", "matches").expect("compiles");
        let again = compiled(r"^cached-\w+$", "matches").expect("compiles");
        assert!(Rc::ptr_eq(&first, &again));
    }

    /// The cache is bounded: filling it past its capacity clears it rather
    /// than growing without limit.
    #[test]
    fn the_cache_never_grows_past_its_capacity() {
        for nth in 0..=CACHE_CAPACITY {
            compiled(&format!("bounded-{nth}"), "matches").expect("compiles");
        }
        CACHE.with_borrow(|cache| assert!(cache.len() <= CACHE_CAPACITY, "{}", cache.len()));
    }
}
