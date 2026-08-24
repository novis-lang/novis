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
//! 1. **`compile`, `match`, `matchAll` and `replaceWith` are not registered.**
//!    All four are stated in terms of a `Core`-owned *object* — spec § 5's
//!    `Pattern` and `Match` — and [`crate::registry::CoreTy`] has no variant
//!    for one: a `Core` class is a namespace for static members today, with no
//!    instance representation and no instance-method dispatch. That is one
//!    piece of work covering all four, plus `Pattern` as an option-carrying
//!    handle, and it is what `docs/agent/handoff.md` names next.
//!
//!    The consequence for the four members that *are* registered is that their
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

use std::cell::RefCell;
use std::rc::Rc;

use mwl_runtime::{Fault, HelperResult, MwlArray, MwlStr, Tag, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Regex`'s registry rows, in the spec's own order.
///
/// Four of § 5's eight members; gap 1 above owns which four are missing and
/// what they wait on.
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
    constants: &[],
};

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
