//! `Core\Regex` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 5, over `rule:core-classes/regex-two-tiers`'s
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
//! * **`regex`** is `rule:core-classes/regex-two-tiers`'s linear tier. A finite-automata engine with
//!   no backtracking to exhaust, so [`Compiled::Linear`] needs no budget and
//!   is given none.
//! * **`fancy-regex`** is § 2's backtracking tier, reached only by a pattern
//!   the linear engine cannot express. It is built *on* `regex` — it delegates
//!   every non-fancy sub-expression to it — so the two agree on syntax, on
//!   Unicode tables and on what a character class means, which two unrelated
//!   engines would not. That shared core is the reason this pair rather than,
//!   say, `pcre2` behind a C shim: `rule:packaging/a-c-dependency-answers-two-questions`'s second question asks what a
//!   C dependency buys, and the answer here would be a *second* opinion about
//!   pattern syntax, which is what `rule:core-classes/regex-two-tiers` exists to avoid.
//!
//! # Tiering happens while checking for a literal, at the first call otherwise
//!
//! `rule:core-classes/regex-literal-tiering` makes a **literal** pattern's tier a compile-time fact, over
//! `rule:expressions/intrinsic-literals`'s
//! literal-folding mechanism: `nvs_types::intrinsics` runs [`validate`] on the
//! literal, refuses a malformed one as a compile error, and records the tier it
//! landed in. An assembled pattern takes the run-time path in [`compiled`]:
//! the linear engine is offered the pattern first and the backtracking engine
//! gets it only if the linear engine's *parser* refused a construct, which is
//! [`build`]'s routing rule and the one place that decision lives. Both paths
//! answer the same tier for the same text. What is missing is the *reporting*
//! of the recorded tier past the checker, and `[regex] backtracking = "deny"`,
//! which has no `[regex]` block to live in — gap 2 below.
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
//! (`rule:concurrency/cross-request-state-is-explicit`
//! forbids that) — a compiled pattern is derived from the pattern text and
//! its flags alone, observable only as speed.
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
//! 1. **`rule:security/regex-pattern-is-a-sink`'s sink is enforced, and seven of the eight rows enforce
//!    it without saying so.** `compile`'s pattern parameter carries
//!    `Qual::Sink`; the seven members that take `Pattern|string` carry no
//!    classification at all, because `nvs_types::core_lib`'s `qual_of` answers
//!    `None` for a [`crate::registry::CoreTy::Union`]. `None` and `Sink`
//!    refuse a qualified argument alike — `nvs_types::expr::quals`'
//!    `admits_tainted_argument` — so a `tainted` pattern is refused at all
//!    eight, which is what that ADR asks for. What is left is the *spelling*:
//!    a union has nowhere to hold a mark, so those seven refuse by the default
//!    rather than by a rule a reader can find, and a union that ever wanted
//!    [`Qual::Launder`] would have no slot for it.
//!    Decided: Yes: qual_of reads the parameter's declared Qual whatever its type — The refusal becomes
//!    readable, and a union can hold Launder; one registry-shape change.
//!    — owner: unowned-closures
//! 2. **The step budget is a constant, not a directive.** `rule:core-classes/regex-two-tiers` puts
//!    the default in `nvs.toml` under `rule:config/three-changeability-classes`'s ordinary rules, and no
//!    key for it parses: `crates/nvs-config/src/tree.rs` is the one home for
//!    what does, and it names none. [`BACKTRACK_BUDGET`] is that default,
//!    stated once, and reading it from config is a change to that one line.
//!    — owner: M6
//! 3. **`matchAll` converts each match's offset over the subject's prefix**,
//!    so reporting positions for *k* matches in an *n*-byte subject is O(n·k)
//!    rather than O(n) — [`crate::granularity::Unit::index_of_byte`] counts
//!    from the start each time. The matches arrive in increasing order, so the
//!    fix is a cursor that counts only the gap since the previous one; it is
//!    not written because a cluster can in principle span a match boundary, and
//!    getting that edge right is worth its own slice rather than a line here.
//!    — owner: unowned
//! 4. **This core's compiled-pattern cache is a cross-request store no
//!    accounting bracket can take.** [`CACHE`] holds an `Rc` the compiling
//!    request holds too, so a pattern's bytes have two owners and nothing can
//!    put its allocation and its release on the same balance, which is what
//!    `nvs_runtime::budget::Detached` asks of every store that opens one. What
//!    it costs today is the request whose write fills the cache and clears it:
//!    it is credited with every pattern earlier requests compiled, lowering the
//!    balance its own ceiling is armed against, bounded by [`CACHE_CAPACITY`]
//!    patterns. Bracketing it the way `Core\Cache`'s tier is bracketed would be
//!    worse rather than better — the compile would be the process's while the
//!    last `Rc`'s drop stayed the request's, which is the same credit with no
//!    bound on it at all. It waits on per-request provenance, which is the
//!    request arena.
//!    — owner: M6

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::granularity::DEFAULT;
use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Regex`'s registry rows, in the spec's own order.
///
/// All eight of them. The four members that section states on a `Match` are
/// [`MATCH`]'s own roster, not this one.
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Regex",
    methods: &[
        CoreMethod {
            name: "compile",
            names: &["pattern"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Options(COMPILE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(PATTERN_NAME),
            symbol: "nvs_core_regex_compile",
            doc: Some(&COMPILE_DOC),
        },
        CoreMethod {
            name: "matches",
            names: &["subject", "pattern"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Union(PATTERN_OR_STRING),
            ],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_regex_matches",
            doc: Some(&MATCHES_DOC),
        },
        CoreMethod {
            name: "match",
            names: &["subject", "pattern"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Union(PATTERN_OR_STRING),
                CoreTy::Options(MATCH_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(MATCH_NAME)),
            symbol: "nvs_core_regex_match",
            doc: Some(&MATCH_DOC),
        },
        CoreMethod {
            name: "matchAll",
            names: &["subject", "pattern"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Union(PATTERN_OR_STRING),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(MATCH_NAME)),
            symbol: "nvs_core_regex_match_all",
            doc: Some(&MATCH_ALL_DOC),
        },
        CoreMethod {
            name: "replace",
            names: &["subject", "pattern", "replacement"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Union(PATTERN_OR_STRING),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(REPLACE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_regex_replace",
            doc: Some(&REPLACE_DOC),
        },
        CoreMethod {
            name: "replaceWith",
            names: &["subject", "pattern", "fn"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Union(PATTERN_OR_STRING),
                CoreTy::CallableSig(&[CoreTy::Instance(MATCH_NAME)], &CoreTy::Str),
                CoreTy::Options(REPLACE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_regex_replace_with",
            doc: Some(&REPLACE_WITH_DOC),
        },
        CoreMethod {
            name: "split",
            names: &["subject", "pattern"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Union(PATTERN_OR_STRING),
                CoreTy::Options(SPLIT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_regex_split",
            doc: Some(&SPLIT_DOC),
        },
        CoreMethod {
            name: "quote",
            names: &["literal"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_regex_quote",
            doc: Some(&QUOTE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Regex::compile`'s reference card — `rule:core-api/reference-card`.
const COMPILE_DOC: MethodDoc = MethodDoc {
    short: "Compiles `$pattern` under the four flags into a `Pattern` handle every other member \
            takes in place of a pattern string — PCRE's `/…/imsU` delimiter-and-modifier \
            syntax, as named options.",
    params: &[
        ParamDoc {
            name: "pattern",
            desc: "The pattern text; a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "Match letters regardless of case — PCRE's `i`; the default is case-sensitive.",
            shape: &[],
        },
        ParamDoc {
            name: "multiline",
            desc: "Let `^` and `$` match at every line boundary rather than only at the ends of \
                   the subject — PCRE's `m`.",
            shape: &[],
        },
        ParamDoc {
            name: "dotAll",
            desc: "Let `.` match a newline too — PCRE's `s`.",
            shape: &[],
        },
        ParamDoc {
            name: "ungreedy",
            desc: "Swap the greediness of every quantifier, so `*` is lazy and `*?` is greedy — \
                   PCRE's `U`.",
            shape: &[],
        },
    ],
    ret: "The `Pattern`, compiled eagerly so a malformed pattern fails here rather than at its \
          first use.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one.",
    }],
};

/// `Core\Regex::matches`'s reference card — `rule:core-api/reference-card`.
const MATCHES_DOC: MethodDoc = MethodDoc {
    short: "Answers whether `$pattern` matches anywhere in `$subject` — `preg_match` used as a \
            predicate. The pattern is unanchored, so `^` and `$` are how a call asks for more.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Pattern` from `Core\\Regex::compile`, or a pattern string compiled with no \
                   flags; the pattern is a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
    ],
    ret: "`true` when the subject contains at least one match, `false` otherwise.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one, or \
               the backtracking engine exhausted its step budget against this subject.",
    }],
};

/// `Core\Regex::matchAll`'s reference card — `rule:core-api/reference-card`.
const MATCH_ALL_DOC: MethodDoc = MethodDoc {
    short: "Finds every non-overlapping match of `$pattern` in `$subject`, one `Match` each in \
            the order they occur — `preg_match_all` in `PREG_SET_ORDER`'s shape, with each \
            match's groups and offset on it.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Pattern` from `Core\\Regex::compile`, or a pattern string compiled with no \
                   flags; the pattern is a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
    ],
    ret: "The matches in subject order; an empty array when the pattern matches nowhere.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one, or \
               the backtracking engine exhausted its step budget against this subject.",
    }],
};

/// `Core\Regex::replace`'s reference card — `rule:core-api/reference-card`.
const REPLACE_DOC: MethodDoc = MethodDoc {
    short: "Replaces up to `limit` matches of `$pattern` in `$subject` with `$replacement`, as \
            `preg_replace` does; in the replacement `$1` and `${name}` are group references and \
            `$$` is a literal `$`.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Pattern` from `Core\\Regex::compile`, or a pattern string compiled with no \
                   flags; the pattern is a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
        ParamDoc {
            name: "replacement",
            desc: "The template each match becomes; a reference to a group the pattern does not \
                   declare expands to the empty string, and PHP's `\\1` spelling is not a \
                   reference.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many matches to replace, counted from the start of the subject; the \
                   default is every one, and `0` replaces nothing.",
            shape: &[],
        },
    ],
    ret: "The subject with its matches replaced — unchanged when the pattern matches nowhere or \
          `limit` is `0`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one, or \
               the backtracking engine exhausted its step budget against this subject.",
    }],
};

/// `Core\Regex::replaceWith`'s reference card — `rule:core-api/reference-card`.
const REPLACE_WITH_DOC: MethodDoc = MethodDoc {
    short: "Replaces up to `limit` matches of `$pattern` in `$subject` with what `$fn` answers \
            for each, as `preg_replace_callback` does; the callback receives one `Match` and \
            its answer is inserted literally, with no group expansion.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Pattern` from `Core\\Regex::compile`, or a pattern string compiled with no \
                   flags; the pattern is a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
        ParamDoc {
            name: "fn",
            desc: "A `callable(Match): string` called once per replaced match, in subject order, \
                   after every match has been found.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many matches to replace, counted from the start of the subject; the \
                   default is every one, `0` replaces nothing, and `$fn` is never called for a \
                   match beyond it.",
            shape: &[],
        },
    ],
    ret: "The subject with its matches replaced — unchanged when the pattern matches nowhere or \
          `limit` is `0`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one, or \
               the backtracking engine exhausted its step budget against this subject.",
    }],
};

/// `Core\Regex::split`'s reference card — `rule:core-api/reference-card`.
const SPLIT_DOC: MethodDoc = MethodDoc {
    short: "Splits `$subject` at every match of `$pattern`, as `preg_split` does, under \
            `Core\\Str::split`'s reading of `limit`.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to split.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Pattern` from `Core\\Regex::compile`, or a pattern string compiled with no \
                   flags; the pattern is a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "`Core\\Str::split`'s three-sign rule: positive is at most that many pieces \
                   with the last holding the remainder, negative drops that many pieces off the \
                   end, and `0` yields the subject unsplit — not `preg_split`'s reading of `0` \
                   and `-1` as no limit.",
            shape: &[],
        },
        ParamDoc {
            name: "keepEmpty",
            desc: "Whether empty pieces are kept; `false` is `PREG_SPLIT_NO_EMPTY`, and drops \
                   them after `limit` has been applied.",
            shape: &[],
        },
    ],
    ret: "The pieces in order; the whole subject as one piece when the pattern matches nowhere.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one, or \
               the backtracking engine exhausted its step budget against this subject.",
    }],
};

/// `Core\Regex::quote`'s reference card — `rule:core-api/reference-card`.
const QUOTE_DOC: MethodDoc = MethodDoc {
    short: "Escapes every character either engine gives a meaning to in `$literal`, as \
            `preg_quote` does, so the result is a pattern matching that literal and nothing \
            else — the launder for the pattern sink, so its result is accepted where a \
            `tainted` string is not.",
    params: &[ParamDoc {
        name: "literal",
        desc: "The text to match literally.",
        shape: &[],
    }],
    ret: "The escaped pattern; a string with no meta character comes back unchanged. The escaped \
          set is not `preg_quote`'s — `&` and `~` are escaped here, `!:<=>` and `/` are not — \
          so only what each result matches is comparable.",
    errors: &[],
};

/// `Core\Regex\Pattern`'s fully-qualified name, written once — see
/// [`MATCH_NAME`] for why.
const PATTERN_NAME: &str = r"Core\Regex\Pattern";

/// Spec § 5's `Core\Regex\Pattern` — a pattern plus the four compilation
/// flags, which is what PCRE's `/…/imsU` delimiter-and-modifier syntax carried
/// and Novis has no syntax for.
///
/// **Two slots and no member of its own.** A `Pattern` is a *handle*: nothing
/// asks it a question, and every one of § 5's matching members takes one where
/// it also takes a plain `string`. So its state is read by `Core\Regex`'s
/// members rather than by its own, which is the one carve-out in
/// `registry`'s `a_class_with_slots_has_instance_members_and_the_reverse` and
/// is named there.
///
/// The slots are the pattern **as the program wrote it** and the flags as a
/// bitmask, not a compiled program: [`crate::instance`]'s first decision is
/// that a `Core` instance holds only values Novis already holds, and the
/// compiled form lives in this module's per-core cache, which
/// [`compiled`] reaches with exactly this pair. Keeping the original text
/// rather than the flag-folded one ([`effective`]) is what lets a throw quote
/// what the call site wrote.
///
/// **What it spends:** one object, two slots and one `string` copy of the
/// pattern per `compile` call, charged to the request. The compiled program it
/// stands for is the cache's, shared by every call that names the same pair.
pub const PATTERN: CoreClass = CoreClass {
    name: PATTERN_NAME,
    methods: &[],
    instance: &[],
    slots: &["pattern", "flags"],
    constants: &[],
};

/// [`PATTERN`]'s `pattern` slot, by index — see [`GROUPS_SLOT`].
const PATTERN_TEXT_SLOT: usize = 0;

/// [`PATTERN`]'s `flags` slot, by index — see [`GROUPS_SLOT`].
const PATTERN_FLAGS_SLOT: usize = 1;

/// Spec § 5's `Pattern|string $pattern`, written once because five members
/// declare it — a compiled handle, or the text of one taken at its defaults.
///
/// Both spellings are the same question asked twice, which is why the spec
/// admits either: `Regex::matches($s, "^\\d+$")` needs no handle at all, and
/// `Regex::compile` exists for the call that wants flags or wants the
/// pattern's validity checked at one place. [`pattern_of`] is where the two
/// meet again.
/// Its `string` half is `rule:security/unclassified-parameter-refuses-tainted`'s **sink**: a pattern is one of `rule:core-api/shape-rules`
/// R11's four grammars, so its content becomes an instruction the engine
/// executes and a `tainted` one is refused at the call. `Core\Regex::quote` is
/// the [`Qual::Launder`] that answers for it.
const PATTERN_OR_STRING: &[CoreTy] = &[CoreTy::Instance(PATTERN_NAME), CoreTy::Text(Qual::Sink)];

/// `Core\Regex::compile`'s four flags, all defaulting to off.
///
/// Named options rather than a modifier string, because `/…/imsU` is a second
/// grammar inside a string literal that nothing can check: a typo in it is a
/// silently different pattern, and a `x` PHP accepts and this table does not
/// would be silently dropped. The four are exactly PCRE's `i`, `m`, `s` and
/// `U`; `x` (extended) has no option because the whitespace it ignores is not
/// a thing an Novis pattern carries, and `u` (Unicode) is not optional — both
/// engines are Unicode-aware always.
const COMPILE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "caseInsensitive",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "multiline",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "dotAll",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "ungreedy",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `{caseInsensitive: true}` — PCRE's `i`.
const FLAG_CASE_INSENSITIVE: u8 = 1;

/// `{multiline: true}` — PCRE's `m`.
const FLAG_MULTILINE: u8 = 1 << 1;

/// `{dotAll: true}` — PCRE's `s`.
const FLAG_DOT_ALL: u8 = 1 << 2;

/// `{ungreedy: true}` — PCRE's `U`.
const FLAG_UNGREEDY: u8 = 1 << 3;

/// No flag set at all, which is what a plain `string` pattern carries.
const NO_FLAGS: u8 = 0;

/// `Core\Regex\Match`'s fully-qualified name, written once — [`MATCH`]
/// declares it and every [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
const MATCH_NAME: &str = r"Core\Regex\Match";

/// Spec § 5's `Core\Regex\Match` — the first `Core`-owned instance, and the
/// shape that replaces `preg_match`'s `$matches` out-parameter (`rule:core-api/shape-rules` R3
/// forbids one) together with `PREG_OFFSET_CAPTURE`.
///
/// Four members over two slots, and no static member at all: a `Match` is only
/// ever produced by [`nvs_core_regex_match`] or [`nvs_core_regex_match_all`].
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
            names: &["group"],
            params: &[CoreTy::Union(&[CoreTy::Int, CoreTy::Text(Qual::Neutral)])],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_regex_match_group",
            doc: Some(&MATCH_GROUP_DOC),
        },
        CoreMethod {
            name: "groups",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Nullable(&CoreTy::Str)),
            symbol: "nvs_core_regex_match_groups",
            doc: Some(&MATCH_GROUPS_DOC),
        },
        CoreMethod {
            name: "offset",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_regex_match_offset",
            doc: Some(&MATCH_OFFSET_DOC),
        },
        CoreMethod {
            name: "text",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_regex_match_text",
            doc: Some(&MATCH_TEXT_DOC),
        },
    ],
    slots: &["groups", "offset"],
    constants: &[],
};

/// `Core\Regex\Match::group`'s reference card — `rule:core-api/reference-card`.
const MATCH_GROUP_DOC: MethodDoc = MethodDoc {
    short: "Answers one group's text by number or by name — `$matches[$group]` read after \
            `preg_match`.",
    params: &[ParamDoc {
        name: "group",
        desc: "The group's number, `0` for the whole match, or its name.",
        shape: &[],
    }],
    ret: "The group's text, or `null` where the pattern declares the group and this match did \
          not reach it.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$group` names a group the pattern does not declare.",
    }],
};

/// `Core\Regex\Match::groups`'s reference card — `rule:core-api/reference-card`.
const MATCH_GROUPS_DOC: MethodDoc = MethodDoc {
    short: "Answers every group at once in `preg_match`'s own order — a named group under its \
            name and then under its number — as `$matches` reads under \
            `PREG_UNMATCHED_AS_NULL`.",
    params: &[],
    ret: "The array, group `0` first; a declared group this match did not reach is present and \
          `null` rather than absent.",
    errors: &[],
};

/// `Core\Regex\Match::offset`'s reference card — `rule:core-api/reference-card`.
const MATCH_OFFSET_DOC: MethodDoc = MethodDoc {
    short: "Answers where the whole match starts in the subject — `PREG_OFFSET_CAPTURE`'s \
            position, counted in graphemes rather than bytes.",
    params: &[],
    ret: "The grapheme index of the match's first character; `0` for a match at the start of the \
          subject.",
    errors: &[],
};

/// `Core\Regex\Match::text`'s reference card — `rule:core-api/reference-card`.
const MATCH_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Answers the whole match's text, which is group `0`.",
    params: &[],
    ret: "The matched text; never `null`, since the whole match participates in every match an \
          engine reports.",
    errors: &[],
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
/// position Novis takes or hands back, and negative counts from the end under
/// `rule:core-api/shape-rules` R8. It is
/// **not** `preg_match`'s `$offset`, which counts bytes and documents that a
/// value inside a multi-byte character is undefined behaviour.
///
/// The match is still made against the whole subject, so a look-behind or a
/// `^` sees what precedes `from` — the same treatment both engines' own
/// "search from" entry points give, and the only one under which
/// `Regex::match($s, $p, {from: $m->offset() + 1})` finds the second match
/// rather than a different pattern's.
/// `Core\Regex::match`'s reference card (`rule:core-api/reference-card`) — the member that proves
/// [`ErrorDoc`], because it throws: both of its errors are the `RuntimeError`
/// [`compiled`] and [`budget_exhausted`] raise, and they are two entries
/// rather than one because a reader wants to know *when*.
const MATCH_DOC: MethodDoc = MethodDoc {
    short: "Finds the first match of `$pattern` in `$subject` at or after `from`, as a `Match` \
            carrying its groups and offset — `preg_match` with `$matches` and \
            `PREG_OFFSET_CAPTURE` folded into the return.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Pattern` from `Core\\Regex::compile`, or a pattern string compiled with no \
                   flags; the pattern is a sink, so a `tainted` string is refused at the call.",
            shape: &[],
        },
        ParamDoc {
            name: "from",
            desc: "The grapheme index the search starts at; negative counts from the end, and an \
                   index past the end starts at the end.",
            shape: &[],
        },
    ],
    ret: "The first `Match`, or `null` when the pattern matches nowhere at or after `from`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` compiles under neither the linear engine nor the backtracking one, or \
               the backtracking engine exhausted its step budget against this subject.",
    }],
};

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
/// the two members that carry it. [`nvs_core_regex_split`] owns the single
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
        "nvs_core_regex_compile" => (nvs_core_regex_compile as *const ()).cast(),
        "nvs_core_regex_matches" => (nvs_core_regex_matches as *const ()).cast(),
        "nvs_core_regex_match" => (nvs_core_regex_match as *const ()).cast(),
        "nvs_core_regex_match_all" => (nvs_core_regex_match_all as *const ()).cast(),
        "nvs_core_regex_match_group" => (nvs_core_regex_match_group as *const ()).cast(),
        "nvs_core_regex_match_groups" => (nvs_core_regex_match_groups as *const ()).cast(),
        "nvs_core_regex_match_offset" => (nvs_core_regex_match_offset as *const ()).cast(),
        "nvs_core_regex_match_text" => (nvs_core_regex_match_text as *const ()).cast(),
        "nvs_core_regex_replace" => (nvs_core_regex_replace as *const ()).cast(),
        "nvs_core_regex_replace_with" => (nvs_core_regex_replace_with as *const ()).cast(),
        "nvs_core_regex_split" => (nvs_core_regex_split as *const ()).cast(),
        "nvs_core_regex_quote" => (nvs_core_regex_quote as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The two tiers
// ============================================================================

/// `rule:core-classes/regex-two-tiers`'s step budget for the backtracking tier, and gap 3's constant.
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

/// One compiled pattern, in whichever tier `rule:core-classes/regex-two-tiers` placed it.
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
    /// This core's compiled patterns, keyed by the pattern text **and the
    /// flags it was compiled under** — the same text under two [`PATTERN`]
    /// flag sets is two programs, and one key would hand the second call the
    /// first one's answer.
    ///
    /// A `Vec` rather than a map: it is capacity-bounded and scanned
    /// linearly, which for a few hundred short keys beats hashing them, and
    /// it keeps the "clear when full" policy a one-liner.
    static CACHE: RefCell<Vec<(String, u8, Rc<Compiled>)>> = const { RefCell::new(Vec::new()) };
}

/// `pattern` as the engines are given it: the text a program wrote, wrapped in
/// the one inline flag group its [`PATTERN`] flags amount to, or borrowed
/// unchanged when it carries none.
///
/// **A group rather than a leading `(?ims)` directive**, because a directive
/// applies only to the end of the enclosing group — so `(?i)a|B` would leave
/// the second alternative case-sensitive, which is not what `/a|B/i` means.
/// `(?i:a|B)` has no such edge. It is also the only mechanism both tiers
/// share: `fancy-regex`'s builder has no `swap_greed` at all, while both
/// parsers read `U` inline, and one spelling across the two is what keeps a
/// pattern's meaning from depending on which tier it landed in.
///
/// A non-capturing group changes no group number and no anchor, so a `Match`
/// built from the result reports exactly the groups the program declared.
fn effective(pattern: &str, flags: u8) -> Cow<'_, str> {
    if flags == NO_FLAGS {
        return Cow::Borrowed(pattern);
    }
    let mut spelled = String::with_capacity(pattern.len() + 8);
    spelled.push_str("(?");
    for (flag, letter) in [
        (FLAG_CASE_INSENSITIVE, 'i'),
        (FLAG_MULTILINE, 'm'),
        (FLAG_DOT_ALL, 's'),
        (FLAG_UNGREEDY, 'U'),
    ] {
        if flags & flag != 0 {
            spelled.push(letter);
        }
    }
    spelled.push(':');
    spelled.push_str(pattern);
    spelled.push(')');
    Cow::Owned(spelled)
}

/// The compiled form of `pattern` under `flags`, from this core's cache or
/// freshly built.
///
/// # Errors
///
/// A `Fault::thrown` naming the pattern when **neither** engine can compile
/// it, which is `rule:core-classes/regex-syntax`'s "a construct neither engine supports is
/// diagnosed, never silently ignored" — at run time today, since gap 1 above
/// owns the compile-time half. The message carries the backtracking engine's
/// own complaint, because it is the more permissive of the two: a pattern the
/// linear engine merely could not *express* has already been handed on by the
/// time this fails, and it quotes the text the program wrote rather than
/// [`effective`]'s flag-wrapped form.
///
/// The other refusal is a pattern the linear engine *does* express and cannot
/// fit under its own size limit; [`build`]'s routing rule says why that is a
/// throw rather than a quiet move to the second tier.
fn compiled(pattern: &str, flags: u8, member: &str) -> Result<Rc<Compiled>, Fault> {
    if let Some(hit) = CACHE.with_borrow(|cache| {
        cache
            .iter()
            .find(|(key, keyed_flags, _)| key == pattern && *keyed_flags == flags)
            .map(|(_, _, compiled)| Rc::clone(compiled))
    }) {
        return Ok(hit);
    }

    let built = build(pattern, flags)
        .map_err(|why| Fault::thrown(format!("Core\\Regex::{member}(): {why}")))?;

    let built = Rc::new(built);
    CACHE.with_borrow_mut(|cache| {
        if cache.len() >= CACHE_CAPACITY {
            cache.clear();
        }
        cache.push((pattern.to_owned(), flags, Rc::clone(&built)));
    });
    Ok(built)
}

/// `pattern` under `flags`, offered to `rule:core-classes/regex-two-tiers`'s two engines in that order —
/// the whole of what "compiling a pattern" is, with no cache and no `Fault`
/// around it so that both callers can reach it.
///
/// # The routing rule
///
/// **A pattern changes tier for one reason only: the linear engine's parser
/// refused a construct.** That is the whole of `rule:core-classes/regex-two-tiers`'s "if the linear
/// engine can express it", and stating it as *which error* rather than *any
/// error* is what makes the tier a semantic property of the pattern instead of
/// a performance heuristic. `regex` refuses for two kinds of reason and only
/// one of them is about expressiveness:
///
/// * [`regex::Error::Syntax`] is the parser, and it covers both a construct
///   finite automata cannot express (a lookaround, a backreference) and a
///   pattern that is simply malformed. Both are handed on, and the second is
///   refused again by the backtracker a line later — so a malformed pattern
///   still reaches § 5's diagnostic and no accepted pattern is lost.
/// * Every other variant is the linear engine hitting a **limit** while
///   building a program it could express perfectly well —
///   [`regex::Error::CompiledTooBig`] today, and the enum is `non_exhaustive`,
///   so an unknown variant is treated the same way. Re-tiering there would
///   make a pattern's exposure to backtracking a function of how large its
///   automaton happens to be, which is exactly the silent, size-dependent
///   move this ADR exists to prevent: the pattern is refused instead, and the
///   program is told which engine ran out of room.
///
/// # Errors
///
/// The sentence [`compiled`] throws, without the member prefix a call site
/// adds: the caller that has one is the runtime, and the caller that does not
/// is [`validate`].
fn build(pattern: &str, flags: u8) -> Result<Compiled, String> {
    let spelled = effective(pattern, flags);
    match regex::Regex::new(&spelled) {
        Ok(linear) => Ok(Compiled::Linear(linear)),
        Err(regex::Error::Syntax(_)) => Ok(Compiled::Backtracking(
            fancy_regex::RegexBuilder::new(&spelled)
                .backtrack_limit(BACKTRACK_BUDGET)
                .build()
                .map_err(|err| {
                    format!("`{pattern}` is not a pattern either engine can compile: {err}")
                })?,
        )),
        Err(limit) => Err(format!(
            "`{pattern}` is a pattern the linear engine expresses but is too large for it \
             to build: {limit}"
        )),
    }
}

/// Which tier `pattern` compiles on, or the refusal — for a caller that wants
/// `rule:core-classes/regex-literal-tiering`'s compile-time fact and not the automaton —
/// `rule:expressions/intrinsic-literals`'s fold,
/// which reads a **literal** pattern while checking and reports § 3's
/// diagnostic instead of the throw [`compiled`] would have made.
///
/// # Errors
///
/// [`build`]'s own sentence, unchanged, so a pattern refused while checking is
/// exactly one the first call would have thrown on (§ 4).
///
/// The flags are **not** read: they reach [`effective`] as a `(?ims:…)` group
/// wrapped around the pattern, which is balanced whatever the pattern is, so
/// it can turn no accepted pattern into a refused one nor the other way round.
/// Reading them would mean folding `Core\Regex::compile`'s options bag as
/// well, for an answer that cannot differ.
///
/// The [`Tier`] is sound for the same reason and not merely the refusal: a
/// wrapper the linear engine can express cannot move a pattern the linear
/// engine refused, nor the other way round, so the tier a flagged call lands
/// in is the tier reported here. What *is* flag-sensitive is gap 1's `[regex]
/// backtracking = "deny"`, which refuses the second tier outright rather than
/// re-routing anything into it, and which this fold does not read.
pub fn validate(pattern: &str) -> Result<Tier, String> {
    build(pattern, NO_FLAGS).map(|compiled| match compiled {
        Compiled::Linear(_) => Tier::Linear,
        Compiled::Backtracking(_) => Tier::Backtracking,
    })
}

/// Which of [`build`]'s two engines a pattern compiles on — its routing rule
/// as a value, for the caller that wants the fact and not the automaton.
///
/// A tier is a property of the **pattern**, not of the call, which is the
/// whole reason this can be answered while checking: `build` changes tier only
/// when the linear engine's *parser* refuses a construct, so the same text
/// routes the same way however many times it is offered and whatever budget
/// the machine has that day. [`validate`]'s own doc says why the flags do not
/// disturb it either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// § 1's linear-time default: no backtracking, no budget, no pattern-
    /// dependent blow-up.
    Linear,
    /// § 2's budgeted backtracking tier, reached only because the linear
    /// engine could not express this pattern — a lookaround or a
    /// backreference — and never because a program was large.
    Backtracking,
}

/// `rule:core-classes/regex-two-tiers`'s throw: the backtracking tier ran out of steps.
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

/// One `string` argument's text. The one failure is `FATAL` for `Core\Str`'s
/// reasons, which that module's own `text` states — including why there is no
/// second, encoding one: the tag [`Value::as_text`] checks is `rule:types/bytes`'s UTF-8
/// guarantee itself.
fn text<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex::{member} expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// A `Pattern|string` argument, decoded to what [`compiled`] takes.
///
/// The text comes back as a [`Value`] rather than a `&str` because a
/// [`PATTERN`]'s is one of its slots: the borrow would be of the slot *read*,
/// which is a local, while the buffer behind it belongs to the receiver and
/// lives for the length of the call. The caller runs it through [`text`] as it
/// would any other `string` argument.
struct Given {
    /// The pattern as the program wrote it, borrowed for the call.
    text: Value,
    /// The flags it carries — [`NO_FLAGS`] for a plain `string`, which is the
    /// spelling that takes every default.
    flags: u8,
}

/// [`Given`] from whichever half of `Pattern|string` the call site wrote.
///
/// **No reference is taken on either path**: [`crate::instance::slot`] borrows
/// exactly as `InstKind::FieldGet` does, and the argument's own reference
/// belongs to the caller, so nothing here owes a release.
///
/// # Errors
///
/// A `Fault::fatal` for anything that is neither, which compiled code cannot
/// produce — `nvs_types` has already checked this parameter against the union.
fn pattern_of(value: &Value, member: &str) -> Result<Given, Fault> {
    if value.as_str_bytes().is_some() {
        return Ok(Given {
            text: *value,
            flags: NO_FLAGS,
        });
    }
    let receiver = value.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Regex::{member} expected a `Pattern` or a `string` for the pattern, got tag {}",
            value.tag_byte()
        ))
    })?;
    let flags = crate::instance::slot(receiver, PATTERN_FLAGS_SLOT)
        .as_int()
        .and_then(|held| u8::try_from(held).ok())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Regex::{member} received a `Pattern` whose flags are not ones \
                 `Core\\Regex::compile` wrote"
            ))
        })?;
    Ok(Given {
        text: crate::instance::slot(receiver, PATTERN_TEXT_SLOT),
        flags,
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
    Ok(Value::str(NvsStr::new(text.as_bytes())))
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Regex::compile(string $pattern, {caseInsensitive?, multiline?,
    /// dotAll?, ungreedy?}): Pattern` — replacing PCRE's
    /// `/…/imsU` delimiter-and-modifier syntax, which Novis has no grammar for.
    ///
    /// **Compiles eagerly**, and throws here rather than at the first match if
    /// neither engine can take the pattern: a `compile` that deferred every
    /// failure to whichever member later used the handle would report the
    /// mistake at a line that did not make it. The compiled program goes
    /// straight into this core's cache under the pair the returned [`PATTERN`]
    /// carries, so the first match against it is already a hit.
    fn nvs_core_regex_compile(_ctx, args: [5]) {
        let pattern = text(&args[0], "compile", "the pattern")?;
        let mut flags = NO_FLAGS;
        for (slot, option, flag) in [
            (1, "the `caseInsensitive` option", FLAG_CASE_INSENSITIVE),
            (2, "the `multiline` option", FLAG_MULTILINE),
            (3, "the `dotAll` option", FLAG_DOT_ALL),
            (4, "the `ungreedy` option", FLAG_UNGREEDY),
        ] {
            if boolean(&args[slot], "compile", option)? {
                flags |= flag;
            }
        }
        compiled(pattern, flags, "compile")?;
        Ok(crate::instance::build(
            &PATTERN,
            [
                Value::str(NvsStr::new(pattern.as_bytes())),
                Value::int(i64::from(flags)),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Regex::matches(string $subject, string $pattern): bool` —
    /// replacing `preg_match` used as a predicate.
    ///
    /// The pattern is unanchored, as PHP's is: it asks whether the subject
    /// *contains* a match, and `^`/`$` are how a call asks for more.
    fn nvs_core_regex_matches(_ctx, args: [2]) {
        let subject = text(&args[0], "matches", "the subject")?;
        let given = pattern_of(&args[1], "matches")?;
        let pattern = text(&given.text, "matches", "the pattern")?;
        let found = match &*compiled(pattern, given.flags, "matches")? {
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

/// The byte offset a search starting at unit index `from` begins at —
/// [`MATCH_OPTIONS`]'s option, read through the one place a signed position
/// becomes a byte offset.
fn start_byte(subject: &str, from: i64) -> usize {
    DEFAULT.byte_of_signed_index(subject, from)
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
            Value::str(NvsStr::new(text.as_bytes()))
        })
    };
    let mut groups = NvsArray::new();
    for (number, group) in captured.iter().enumerate() {
        if let Some(name) = names.get(number).copied().flatten() {
            groups.set(NvsStr::new(name.as_bytes()), text_of(*group));
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
fn group_array(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
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
/// in an Novis array is spelled.
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

nvs_runtime::nvs_helper! {
    /// `Core\Regex::match(string $subject, string $pattern, {from?: int}): ?Match`
    /// — replacing `preg_match`, its `$matches` out-parameter and
    /// `PREG_OFFSET_CAPTURE` at once.
    ///
    /// `null` is "no match", which `rule:core-api/shape-rules` R5 makes the only absence
    /// spelling — there is no `0`/`false`/`1` return to read, and no error code
    /// beside it, because a pattern that cannot run throws
    /// ([`budget_exhausted`], [`compiled`]).
    fn nvs_core_regex_match(_ctx, args: [3]) {
        let subject = text(&args[0], "match", "the subject")?;
        let given = pattern_of(&args[1], "match")?;
        let pattern = text(&given.text, "match", "the pattern")?;
        let from = integer(&args[2], "match", "the `from` option")?;
        let start = start_byte(subject, from);

        let compiled = compiled(pattern, given.flags, "match")?;
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

nvs_runtime::nvs_helper! {
    /// `Core\Regex::matchAll(string $subject, string $pattern): array<Match>` —
    /// replacing `preg_match_all` and both of its ordering flags.
    ///
    /// One `Match` per match, in the order they occur: `PREG_SET_ORDER`'s
    /// shape, since `PREG_PATTERN_ORDER`'s transpose is a differently-shaped
    /// return from the same member, which `rule:core-api/shape-rules` R7 refuses.
    /// `Core\Arr::map($matches, fn($m) => $m->group(1))` is the transpose, in
    /// one line, when a caller wants it.
    ///
    /// **Each match's `offset` is converted from bytes independently**, which
    /// costs a pass over the subject's prefix per match — gap 3 above owns
    /// that.
    fn nvs_core_regex_match_all(_ctx, args: [2]) {
        let subject = text(&args[0], "matchAll", "the subject")?;
        let given = pattern_of(&args[1], "matchAll")?;
        let pattern = text(&given.text, "matchAll", "the pattern")?;

        let compiled = compiled(pattern, given.flags, "matchAll")?;
        let names = names_of(&compiled);
        let mut out = NvsArray::new();
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

nvs_runtime::nvs_helper! {
    /// `$match->group(int|string $group): ?string` — one group's text, or
    /// `null` where the pattern declares that group but this match did not
    /// reach it.
    ///
    /// A group the **pattern** does not declare is a different question, and
    /// **throws**: `rule:core-api/shape-rules` R5's `?T` says "this match has no such text", while
    /// R4's throw says "there is no such group to ask about." PHP answers both
    /// with an absent array entry, which is why `preg_match` code so often
    /// reads a typo as an empty capture.
    fn nvs_core_regex_match_group(_ctx, args: [2]) {
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

nvs_runtime::nvs_helper! {
    /// `$match->groups(): array<?string>` — every group at once, in
    /// `preg_match`'s own order and shape ([`built_match`]).
    ///
    /// **The shape is `$matches` under `PREG_UNMATCHED_AS_NULL`, not under
    /// PHP's default**, and `rule:core-api/shape-rules` R11 is why that is a decision rather than
    /// a default: there are no `PREG_*` constants, so one of the two readings
    /// has to be the only one. PHP's default trims *trailing* unmatched groups
    /// out of the array and writes `""` for the ones in the middle, conflating
    /// "not declared", "declared and did not participate" and "participated
    /// and captured nothing" — the first two are exactly what
    /// [`nvs_core_regex_match_group`]'s throw-versus-`null` split is built on,
    /// so the flagged reading is the only one that can carry it.
    /// `tests/differential/core/regex-match-groups-is-preg_match-s-matches-under-unmatched-as-null.nvst`
    /// counts the difference: over twelve rows the two readings part on six of
    /// them, six entries short in total.
    fn nvs_core_regex_match_groups(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `$match->offset(): int` — where the whole match starts in the subject,
    /// counted in [`crate::granularity::DEFAULT`]'s unit like every other
    /// `string` position, not in `PREG_OFFSET_CAPTURE`'s bytes.
    fn nvs_core_regex_match_offset(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &MATCH, "offset")?;
        Ok(crate::instance::slot(receiver, OFFSET_SLOT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$match->text(): string` — the whole match's text, which is group `0`.
    ///
    /// Not nullable: group 0 participates in every match an engine reports, so
    /// a missing slot here is a corrupted instance rather than an absent value.
    fn nvs_core_regex_match_text(_ctx, args: [1]) {
        let groups = group_array(args, "text")?;
        // Unreachable from source: no program constructs a `Core\Regex\Match`,
        // and `built_match` — its only builder — appends group `0` first,
        // because the whole match participates in every capture set an engine
        // reports. Its own comment says so at the other end of the same rule.
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

nvs_runtime::nvs_helper! {
    /// `Core\Regex::replace(string $subject, string $pattern, string $replacement, {limit?: uint}): string`
    /// — replacing `preg_replace`.
    ///
    /// **`$1` and `${name}` are the group spellings**, and `$$` is a literal
    /// `$`. PHP additionally accepts `\1`; it is not accepted here, because
    /// `\1` inside a double-quoted Novis string is already an escape the lexer
    /// reads, so the same source text would mean two different things
    /// depending on the quote used to write it. A group reference that names
    /// no group expands to the empty string, as PHP's does.
    ///
    /// `limit` counts *replacements*, defaults to every one, and a limit of
    /// `0` replaces nothing — which is the reading the option's `uint` type
    /// forces and the one PHP's own `preg_replace` gives it.
    fn nvs_core_regex_replace(_ctx, args: [4]) {
        let subject = text(&args[0], "replace", "the subject")?;
        let given = pattern_of(&args[1], "replace")?;
        let pattern = text(&given.text, "replace", "the pattern")?;
        let replacement = text(&args[2], "replace", "the replacement")?;
        let limit = unsigned(&args[3], "replace", "the `limit` option")?;
        if limit == 0 {
            return produced(subject);
        }
        // Both engines spell "every match" as `0`, so an unlimited call has to
        // say so rather than passing a huge count.
        let count = usize::try_from(limit).unwrap_or(usize::MAX);
        let count = if limit == u64::MAX { 0 } else { count };

        let replaced = match &*compiled(pattern, given.flags, "replace")? {
            Compiled::Linear(re) => re.replacen(subject, count, replacement).into_owned(),
            Compiled::Backtracking(re) => re
                .try_replacen(subject, count, replacement)
                .map_err(|err| budget_exhausted("replace", pattern, &err))?
                .into_owned(),
        };
        produced(&replaced)
    }
}

/// What the callback answers for one match, as text this frame owns.
///
/// Exactly two references are created here and both are released here: the
/// [`MATCH`] this frame builds for the callback, and the `string` the callback
/// answers with, which `nvs_runtime::call_closure` hands back as one fresh
/// reference. Each release is written *before* the `?` that could carry the
/// failure out — a throw from inside the callback and an answer of the wrong
/// tag are the two edges a plain `?` would otherwise leak past.
fn replacement_for(
    ctx: &mut nvs_runtime::Ctx,
    callback: Value,
    subject: &str,
    names: &[Option<&str>],
    captured: &Captured<'_>,
) -> Result<String, Fault> {
    let matched = built_match(subject, names, captured);
    let answered = nvs_runtime::call_closure(ctx, callback, &[matched]);
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `built_match` produced, \
                  and `call_closure` retained its own for the length of the call"
    )]
    unsafe {
        matched.release();
    }
    let answered = answered?;
    let copied = text(&answered, "replaceWith", "the callback's answer").map(str::to_owned);
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `call_closure` returned"
    )]
    unsafe {
        answered.release();
    }
    copied
}

nvs_runtime::nvs_helper! {
    /// `Core\Regex::replaceWith(string $subject, Pattern|string $pattern, callable $fn, {limit?: uint}): string`
    /// — replacing `preg_replace_callback` and `preg_replace_callback_array`.
    ///
    /// The callback is handed **one [`MATCH`]**, not PHP's positional array,
    /// so `$m->group("year")` reads here exactly as it does on the result of
    /// [`nvs_core_regex_match`] — including the throw for a group the pattern
    /// never declared. `preg_replace_callback_array`'s several-patterns form
    /// is a loop over this member rather than a second shape of argument,
    /// which `rule:core-api/shape-rules` R7 is the rule for.
    ///
    /// **What the callback answers is inserted literally.** A `$1` in it is
    /// two characters rather than a group reference, which is the one place
    /// this member reads differently from [`nvs_core_regex_replace`]'s
    /// template — the callback already held every group, so a second
    /// expansion pass over its answer could only corrupt text it chose.
    ///
    /// `limit` is `replace`'s option with `replace`'s meaning: it counts
    /// replacements, `0` performs none, and the callback is never called for a
    /// match beyond it.
    ///
    /// **Every match is found before the first call.** Stepping the engine's
    /// own iterator and running user code between steps would hold a live
    /// borrow of an engine across a call that can reach `Core\Regex` again;
    /// collecting first spends one [`Captured`] per match for the length of
    /// the call, which is AGENTS.md's ordering buying priority 4 with
    /// priority 5.
    fn nvs_core_regex_replace_with(ctx, args: [4]) {
        let subject = text(&args[0], "replaceWith", "the subject")?;
        let given = pattern_of(&args[1], "replaceWith")?;
        let pattern = text(&given.text, "replaceWith", "the pattern")?;
        let limit = unsigned(&args[3], "replaceWith", "the `limit` option")?;
        if limit == 0 {
            return produced(subject);
        }
        let count = usize::try_from(limit).unwrap_or(usize::MAX);

        let compiled = compiled(pattern, given.flags, "replaceWith")?;
        let names = names_of(&compiled);
        let mut found: Vec<Captured<'_>> = Vec::new();
        match &*compiled {
            Compiled::Linear(re) => {
                for caps in re.captures_iter(subject).take(count) {
                    found.push(linear_groups(&caps));
                }
            }
            Compiled::Backtracking(re) => {
                for caps in re.captures_iter(subject).take(count) {
                    let caps =
                        caps.map_err(|err| budget_exhausted("replaceWith", pattern, &err))?;
                    found.push(backtracking_groups(&caps));
                }
            }
        }

        let mut out = String::with_capacity(subject.len());
        let mut cursor = 0;
        for captured in &found {
            let Some((start, whole)) = captured.first().copied().flatten() else {
                continue;
            };
            out.push_str(&subject[cursor..start]);
            out.push_str(&replacement_for(ctx, args[2], subject, &names, captured)?);
            cursor = start + whole.len();
        }
        out.push_str(&subject[cursor..]);
        produced(&out)
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

nvs_runtime::nvs_helper! {
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
    /// member, which `rule:core-api/shape-rules` R7 refuses, and the second is what `matchAll`
    /// answers.
    fn nvs_core_regex_split(_ctx, args: [4]) {
        let subject = text(&args[0], "split", "the subject")?;
        let given = pattern_of(&args[1], "split")?;
        let pattern = text(&given.text, "split", "the pattern")?;
        let limit = integer(&args[2], "split", "the `limit` option")?;
        let keep_empty = boolean(&args[3], "split", "the `keepEmpty` option")?;

        let compiled = compiled(pattern, given.flags, "split")?;
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

        let mut out = NvsArray::new();
        for piece in pieces {
            out.append(Value::str(NvsStr::new(piece.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Regex::quote(string $literal): string` — replacing `preg_quote`,
    /// and `rule:security/regex-pattern-is-a-sink`'s one laundering member for the pattern sink.
    ///
    /// Escapes every character either engine gives a meaning to, so the result
    /// matches `$literal` and nothing else. PHP's optional `$delimiter`
    /// argument has no equivalent, because `rule:core-classes/regex-syntax` removed the
    /// `/…/` delimiter syntax it existed for: a pattern here is a pattern, not
    /// a pattern wrapped in punctuation.
    ///
    /// **The escaped set is not `preg_quote`'s and a port must not compare the
    /// two outputs.** Over printable ASCII this escapes 18 characters,
    /// `#$&()*+-.?[\]^{|}~`, where `preg_quote($c, "/")` escapes 22,
    /// `!#$()*+-./:<=>?[\]^{|}`. `&` and `~` are meta here because the Rust
    /// engine reads `&&` and `~~` as character-class set operators and PCRE
    /// does not; `!:<=>` are meta to neither engine and PCRE's launderer
    /// escapes them anyway; `/` is escaped there only because a delimiter was
    /// handed in. What the two do agree on is the property both are for — the
    /// result matches its own literal, matches it inside a larger subject, and
    /// matches nothing else — which
    /// `tests/differential/core/regex-quote-and-preg_quote-escape-different-sets-and-match-the-same-literals.nvst`
    /// counts over the whole ASCII table rather than comparing row by row.
    fn nvs_core_regex_quote(_ctx, args: [1]) {
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
        let compiled = compiled(r"(?<word>[a-z]+)(\d+)?", NO_FLAGS, "match").expect("compiles");
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

    /// The tier is chosen by the pattern, never by the caller, and it is a
    /// **semantic** property of the pattern rather than a performance
    /// heuristic — `rule:core-classes/regex-two-tiers`, and [`build`]'s routing rule.
    ///
    /// Asserted over two whole tables by collecting the strays rather than
    /// read off one line: a pattern that quietly changed tier still answers
    /// correctly, so counting is the only way the move is visible at all.
    #[test]
    fn a_pattern_the_linear_engine_expresses_never_reaches_the_backtracker() {
        // The constructs finite automata do express, one row each.
        const LINEAR: &[&str] = &[
            r"^\d+$",
            r"\b\w+\b",
            r"[a-z]+@[a-z]+\.[a-z]{2,}",
            r"(?:foo|bar)*baz",
            r"(?<year>\d{4})-(?<month>\d{2})-(\d{2})",
            r"\p{Greek}+",
            r"[[:alpha:]]+",
            r"a{2,5}?",
            r"[^\x00-\x1f]+",
            r"\s*,\s*",
            r"\Aabc\z",
            r"(?i)mixed|CASE",
            r"(?s).+",
            r"(?m)^$",
        ];
        // The four lookarounds and the two backreference spellings — the
        // whole of what a finite automaton cannot do, and so the whole of
        // what may legitimately move.
        const REQUIRES_BACKTRACKING: &[&str] = &[
            r"foo(?=bar)",
            r"foo(?!bar)",
            r"(?<=foo)bar",
            r"(?<!foo)bar",
            r"^(\w+)\s+\1$",
            r"(?<word>\w+)\s+\k<word>",
        ];

        // Under every combination of the four flags: `effective` wraps the
        // pattern in a balanced group, so no flag can turn a pattern the
        // linear engine expresses into one it does not.
        let every_flag = FLAG_CASE_INSENSITIVE | FLAG_MULTILINE | FLAG_DOT_ALL | FLAG_UNGREEDY;
        for flags in NO_FLAGS..=every_flag {
            let strayed: Vec<&str> = LINEAR
                .iter()
                .copied()
                .filter(|pattern| {
                    !matches!(
                        build(pattern, flags).expect("a linear pattern compiles"),
                        Compiled::Linear(_)
                    )
                })
                .collect();
            assert!(
                strayed.is_empty(),
                "reached the backtracker under flags {flags}: {strayed:?}"
            );
        }

        let stayed: Vec<&str> = REQUIRES_BACKTRACKING
            .iter()
            .copied()
            .filter(|pattern| {
                !matches!(
                    build(pattern, NO_FLAGS).expect("the second tier compiles it"),
                    Compiled::Backtracking(_)
                )
            })
            .collect();
        assert!(
            stayed.is_empty(),
            "the linear engine claimed to express: {stayed:?}"
        );

        // And the rule is expressiveness, never size: a pattern the linear
        // engine expresses but cannot fit under its own program-size limit is
        // refused rather than handed to an engine with a step budget.
        // Re-tiering there would make a pattern's exposure to backtracking a
        // function of how large its automaton happens to be, which nothing in
        // the source says and no reader could predict.
        let too_big = r"\p{L}".repeat(5_000);
        let refused =
            build(&too_big, NO_FLAGS).expect_err("5,000 unicode classes is past the size limit");
        assert!(refused.contains("too large for it to build"), "{refused}");
    }

    /// A pattern neither engine can compile throws rather than matching
    /// nothing — `rule:core-classes/regex-syntax`'s "never silently ignored".
    #[test]
    fn a_pattern_neither_engine_accepts_throws() {
        let err = compiled("(unclosed", NO_FLAGS, "matches")
            .expect_err("an unclosed group is not a pattern");
        assert!(format!("{err:?}").contains("(unclosed"), "{err:?}");
    }

    /// The catastrophic pair every backtracking-budget assertion below runs
    /// against: a backreference puts the pattern on the second tier and stops
    /// `fancy-regex` delegating the loop back to the linear engine, and forty
    /// `a`s followed by a `b` the pattern can never reach is 2^40 paths — far
    /// enough past [`BACKTRACK_BUDGET`] that no machine's speed enters into
    /// it, which is `rule:core-classes/regex-two-tiers`'s own reason for bounding steps and not seconds.
    const CATASTROPHIC: &str = r"^(a|a?)+\1$";

    /// `rule:core-classes/regex-two-tiers`: the backtracking tier runs under [`BACKTRACK_BUDGET`],
    /// attached where the program is **built** — so it is on every pattern
    /// that reaches the tier, rather than on the ones a member remembered to
    /// bound. A subject inside the budget still answers.
    #[test]
    fn a_backtracking_pattern_runs_under_a_throwing_step_budget() {
        let held = compiled(CATASTROPHIC, NO_FLAGS, "matches").expect("compiles");
        let Compiled::Backtracking(re) = &*held else {
            panic!("a backreference is not a pattern the linear engine expresses")
        };
        assert!(
            re.is_match("aa").is_ok(),
            "a short subject finishes inside the budget"
        );

        let subject = format!("{}b", "a".repeat(40));
        let err = re.is_match(&subject).expect_err("exhausts the budget");

        // The budget the program was built with is the one the throw names,
        // so a message can never quote a bound that is not the enforced one.
        let Fault::Thrown(class, message) = budget_exhausted("matches", CATASTROPHIC, &err) else {
            panic!("§ 2 is an ordinary catchable throw, not a resource-limit fatal")
        };
        assert!(matches!(class, nvs_runtime::ThrownClass::Runtime));
        assert!(message.contains(&BACKTRACK_BUDGET.to_string()), "{message}");
        assert!(message.contains(CATASTROPHIC), "{message}");
    }

    /// `rule:core-classes/regex-two-tiers`'s load-bearing half: exhausting the budget **throws**,
    /// and never answers "no match".
    ///
    /// A call site that wrote the pattern as a check reads a falsy answer as
    /// *permitted*, so returning one turns a denial-of-service into an
    /// authorization bypass. PHP's `pcre.backtrack_limit` returns `false` for
    /// both "gave up" and "did not match", which is the behaviour this pins
    /// against — and it is asserted over every way a member reads the tier,
    /// because the distinction is lost the first time one of them swallows
    /// the error into an empty result.
    #[test]
    fn a_step_budget_exhaustion_throws_rather_than_returning_no_match() {
        let held = compiled(CATASTROPHIC, NO_FLAGS, "matches").expect("compiles");
        let Compiled::Backtracking(re) = &*held else {
            panic!("a backreference is not a pattern the linear engine expresses")
        };
        let subject = format!("{}b", "a".repeat(40));

        let predicate = re.is_match(&subject);
        assert!(predicate.is_err(), "answered {predicate:?} rather than Err");
        assert!(re.find(&subject).is_err(), "found a first match");
        assert!(re.captures(&subject).is_err(), "captured a first match");
    }

    /// A compiled pattern is held, so a loop over one pattern compiles it
    /// once — this module's own docs own what that spends.
    #[test]
    fn one_pattern_is_compiled_once_per_core() {
        let first = compiled(r"^cached-\w+$", NO_FLAGS, "matches").expect("compiles");
        let again = compiled(r"^cached-\w+$", NO_FLAGS, "matches").expect("compiles");
        assert!(Rc::ptr_eq(&first, &again));
    }

    /// The four flags are part of the key, not of the text: one pattern under
    /// two flag sets is two compiled programs, and each behaves as its flags
    /// say — the property `regex-compile-carries-the-four-flags.nvst` then
    /// pins through the members, one flag at a time.
    #[test]
    fn the_flags_are_part_of_the_key_and_reach_the_engine() {
        let plain = compiled("^flagged-a+$", NO_FLAGS, "compile").expect("compiles");
        let folded = compiled("^flagged-a+$", FLAG_CASE_INSENSITIVE, "compile").expect("compiles");
        assert!(!Rc::ptr_eq(&plain, &folded));

        let matched = |held: &Compiled, subject: &str| match held {
            Compiled::Linear(re) => re.is_match(subject),
            Compiled::Backtracking(re) => re.is_match(subject).expect("within budget"),
        };
        assert!(!matched(&plain, "flagged-AAA"));
        assert!(matched(&folded, "flagged-AAA"));
    }

    /// Each flag is the PCRE modifier it is named for, over both tiers: the
    /// group [`effective`] wraps is the one spelling `regex` and `fancy-regex`
    /// both read, so a pattern means the same thing whichever tier took it.
    #[test]
    fn each_flag_is_the_pcre_modifier_it_replaces() {
        // `m` — `$` reaches a line ending rather than only the subject's end.
        assert_eq!(effective("a$", FLAG_MULTILINE), "(?m:a$)");
        // `s` — `.` covers a newline; `U` — the quantifiers swap greed.
        assert_eq!(
            effective("a.+b", FLAG_DOT_ALL | FLAG_UNGREEDY),
            "(?sU:a.+b)"
        );
        // No flag at all borrows the text rather than rewriting it.
        assert!(matches!(effective("a$", NO_FLAGS), Cow::Borrowed("a$")));

        let run = |pattern: &str, flags: u8, subject: &str| {
            let held = compiled(pattern, flags, "compile").expect("compiles");
            match &*held {
                Compiled::Linear(re) => re.find(subject).map(|found| found.as_str().to_owned()),
                Compiled::Backtracking(re) => re
                    .find(subject)
                    .expect("within budget")
                    .map(|found| found.as_str().to_owned()),
            }
        };
        assert_eq!(run("^b", FLAG_MULTILINE, "a\nbc"), Some("b".to_owned()));
        assert_eq!(run("^b", NO_FLAGS, "a\nbc"), None);
        assert_eq!(run("a.c", FLAG_DOT_ALL, "a\nc"), Some("a\nc".to_owned()));
        assert_eq!(run("<.+>", FLAG_UNGREEDY, "<a><b>"), Some("<a>".to_owned()));
        assert_eq!(run("<.+>", NO_FLAGS, "<a><b>"), Some("<a><b>".to_owned()));
        // A flag group applies to every alternative, which is why it is a
        // group rather than a leading `(?i)` directive.
        assert_eq!(
            run("x|Y", FLAG_CASE_INSENSITIVE, "zy"),
            Some("y".to_owned())
        );
    }

    /// The cache is bounded: filling it past its capacity clears it rather
    /// than growing without limit.
    #[test]
    fn the_cache_never_grows_past_its_capacity() {
        for nth in 0..=CACHE_CAPACITY {
            compiled(&format!("bounded-{nth}"), NO_FLAGS, "matches").expect("compiles");
        }
        CACHE.with_borrow(|cache| assert!(cache.len() <= CACHE_CAPACITY, "{}", cache.len()));
    }
}
