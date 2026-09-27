//! `Core\Regex` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 5, over `rule:core-classes/regex-two-tiers`'s
//! two engines.
//!
//! That ADR's body is the rule and this module is its implementation; nothing
//! about *why* the tiers exist is restated here.
//!
//! # The two crates, and why these two
//!
//! The choice of these two crates is settled, not made here — what belongs
//! here is what each one is doing:
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
//! answer the same tier for the same text, and the checker's answer reaches the
//! call it was settled at: [`crate::registry::PREPARED_MEMBERS`] hands
//! `compile` the tier as a constant argument, so the first
//! compile of a backtracking pattern in a process goes straight to the engine
//! that can express it instead of deriving the routing a second time. What is
//! missing is `[regex] backtracking = "deny"`, which has no `[regex]` block to
//! live in.
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
//! **Whose bytes they are, and the bound on that.** A compiled program is
//! charged to the request that compiled it and outlives that request, so the
//! one whose write fills the cache and clears it is credited with the programs
//! earlier requests paid for: its `Ctx::memory_used` reading falls by that
//! much, and the `[limits] memory` ceiling armed against that reading gives it
//! that much extra headroom, bounded each time by the [`CACHE_CAPACITY`]
//! programs this core holds. No accounting bracket takes that away. [`CACHE`]
//! hands out an `Rc` the calling request holds too, so
//! `nvs_runtime::budget::Detached` — which asks a store for symmetry, the
//! allocation and the release on one balance — has no pair to take, and
//! bracketing the compile the way `Core\Cache`'s tier is bracketed would leave
//! the last `Rc`'s drop the request's, which is the same credit with no bound
//! on it at all. What is never at stake is who wrote the text: a pattern is a
//! sink taking the plain `string` (`rule:security/regex-pattern-is-a-sink`), so
//! the set a core caches is the program's own and never one a request composed.
//! [`crate::cldr`]'s prepared-pattern cache is this same store one grammar
//! along, under this same bound.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use nvs_runtime::{Ctx, Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::granularity::{Cursor, DEFAULT};
use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// This class's own name, for the rosters keyed on one — spec § 5's
/// `Core\Regex`.
pub const NAME: &str = r"Core\Regex";

/// `Core\Regex`'s registry rows, in the spec's own order.
///
/// All eight of them. The four members that section states on a `Match` are
/// [`MATCH`]'s own roster, not this one.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
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

/// `Core\Regex`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Finds, extracts, replaces and splits text with regular expressions. Every method \
            takes a pattern as a `string` or as a `Core\\Regex\\Pattern` that `compile` \
            returns.",
};

/// `Core\Regex::compile`'s reference card — `rule:core-api/reference-card`.
const COMPILE_DOC: MethodDoc = MethodDoc {
    short: "Compiles a regular expression once, with options, and returns a \
            `Core\\Regex\\Pattern`. You can pass the `Pattern` to every `Core\\Regex` method \
            that takes a pattern.",
    params: &[
        ParamDoc {
            name: "pattern",
            desc: "The regular expression, with no delimiters. A `tainted` string does not \
                   compile here. Use `Core\\Regex::quote` to match a user's text literally.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "`true` makes letters match in upper and lower case. PHP writes this as `i`. \
                   The default is `false`.",
            shape: &[],
        },
        ParamDoc {
            name: "multiline",
            desc: "`true` makes `^` and `$` match at the start and end of every line. PHP writes \
                   this as `m`. The default is `false`.",
            shape: &[],
        },
        ParamDoc {
            name: "dotAll",
            desc: "`true` makes `.` match a newline too. PHP writes this as `s`. The default is \
                   `false`.",
            shape: &[],
        },
        ParamDoc {
            name: "ungreedy",
            desc: "`true` makes `*` and `+` match as little as possible, and `*?` as much as \
                   possible. PHP writes this as `U`. The default is `false`.",
            shape: &[],
        },
    ],
    ret: "The compiled `Core\\Regex\\Pattern`. The pattern is checked here, so a mistake in it \
          throws at this line and not at its first use.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression. The message contains the pattern.",
    }],
};

/// `Core\Regex::matches`'s reference card — `rule:core-api/reference-card`.
const MATCHES_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$pattern` matches anywhere in `$subject`. The match can be at any \
            position. Write `^` and `$` in the pattern to match the whole text.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Core\\Regex\\Pattern` from `Core\\Regex::compile`, or a pattern string with \
                   no options. A `tainted` string does not compile here.",
            shape: &[],
        },
    ],
    ret: "`true` if the text contains at least one match, and `false` if it does not.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression, or matching it against `$subject` \
               needs more steps than the limit allows.",
    }],
};

/// `Core\Regex::matchAll`'s reference card — `rule:core-api/reference-card`.
const MATCH_ALL_DOC: MethodDoc = MethodDoc {
    short: "Finds every match of `$pattern` in `$subject` and returns them as an array of \
            `Core\\Regex\\Match`. The matches do not overlap, and they are in the order they \
            appear in the text.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Core\\Regex\\Pattern` from `Core\\Regex::compile`, or a pattern string with \
                   no options. A `tainted` string does not compile here.",
            shape: &[],
        },
    ],
    ret: "One `Core\\Regex\\Match` for each match, from left to right. The array is empty if \
          there is no match.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression, or matching it against `$subject` \
               needs more steps than the limit allows.",
    }],
};

/// `Core\Regex::replace`'s reference card — `rule:core-api/reference-card`.
const REPLACE_DOC: MethodDoc = MethodDoc {
    short: "Replaces every match of `$pattern` in `$subject` with `$replacement`, and returns \
            the new text. In `$replacement`, `$1` is the text of group 1 and `$0` is the whole \
            match.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Core\\Regex\\Pattern` from `Core\\Regex::compile`, or a pattern string with \
                   no options. A `tainted` string does not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "replacement",
            desc: "The text that replaces each match. `$1` to `$99` insert a numbered group, and \
                   `${name}` inserts a named group. Write `${1}0` when a digit follows the \
                   group. `$$` is one `$`. A group the pattern does not have inserts nothing. \
                   PHP's `\\1` is not a group here.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many matches to replace, counted from the start of the text. The default \
                   is every match. `0` replaces nothing.",
            shape: &[],
        },
    ],
    ret: "The text with the matches replaced. It is the same text if there is no match.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression, or matching it against `$subject` \
               needs more steps than the limit allows.",
    }],
};

/// `Core\Regex::replaceWith`'s reference card — `rule:core-api/reference-card`.
const REPLACE_WITH_DOC: MethodDoc = MethodDoc {
    short: "Replaces every match of `$pattern` in `$subject` with the text your function \
            returns for that match, and returns the new text. The function gets one \
            `Core\\Regex\\Match`.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Core\\Regex\\Pattern` from `Core\\Regex::compile`, or a pattern string with \
                   no options. A `tainted` string does not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "fn",
            desc: "A function that gets one `Core\\Regex\\Match` and returns a `string`. It is \
                   called once for each match, from left to right. The text it returns is \
                   inserted as it is, so `$1` in it is two characters.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "How many matches to replace, counted from the start of the text. The default \
                   is every match. `0` replaces nothing. The function is not called for a match \
                   after the limit.",
            shape: &[],
        },
    ],
    ret: "The text with the matches replaced. It is the same text if there is no match.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression, matching it against `$subject` \
               needs more steps than the limit allows, or the new text is larger than the \
               memory limit.",
    }],
};

/// `Core\Regex::split`'s reference card — `rule:core-api/reference-card`.
const SPLIT_DOC: MethodDoc = MethodDoc {
    short: "Splits `$subject` at every match of `$pattern`, and returns the pieces as an \
            array. The matches are not part of any piece.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to split.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Core\\Regex\\Pattern` from `Core\\Regex::compile`, or a pattern string with \
                   no options. A `tainted` string does not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "The largest number of pieces. The last piece contains the rest of the text. \
                   A negative number removes that many pieces from the end. `0` returns the \
                   whole text as one piece. The default is no limit. PHP's `preg_split` reads \
                   `0` and `-1` as no limit.",
            shape: &[],
        },
        ParamDoc {
            name: "keepEmpty",
            desc: "Whether empty pieces are kept. The default is `true`. With `false`, empty \
                   pieces are removed after `limit` is applied.",
            shape: &[],
        },
    ],
    ret: "The pieces, from left to right. If there is no match, the array has one piece: the \
          whole text.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression, or matching it against `$subject` \
               needs more steps than the limit allows.",
    }],
};

/// `Core\Regex::quote`'s reference card — `rule:core-api/reference-card`.
const QUOTE_DOC: MethodDoc = MethodDoc {
    short: "Puts a `\\` before every character that has a special meaning in a pattern. The \
            result is a pattern that matches `$literal` exactly. A `tainted` text is allowed \
            here, and the result can be used as a pattern.",
    params: &[ParamDoc {
        name: "literal",
        desc: "The text to match exactly.",
        shape: &[],
    }],
    ret: "The escaped pattern. A text with no special characters is returned unchanged. The \
          escaped characters are not the same as PHP's `preg_quote`: `&` and `~` are escaped \
          here, and `!`, `:`, `<`, `=`, `>` and `/` are not.",
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
    doc: None,
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
///
/// Its `string` half is `rule:security/unclassified-parameter-refuses-tainted`'s **sink**: a pattern is one of `rule:core-api/shape-rules`
/// R11's four grammars, so its content becomes an instruction the engine
/// executes and a `tainted` one is refused at the call. `Core\Regex::quote` is
/// the [`Qual::Launder`] that answers for it. That arm is the whole union's
/// classification — [`CoreTy::Union`] folds its arms — so these rows refuse a
/// tainted pattern by the mark written here, exactly as `compile`'s own
/// `string` parameter does, rather than by the default an unclassified
/// parameter gets.
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
    doc: Some(&MATCH_CARD),
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

/// `Core\Regex\Match`'s class card — `rule:core-api/reference-card`.
const MATCH_CARD: ClassDoc = ClassDoc {
    short: "One match of a regular expression in a text. It has the matched text, the position \
            where the match starts and the text of each group. `Core\\Regex::match` and \
            `Core\\Regex::matchAll` return it.",
};

/// `Core\Regex\Match::group`'s reference card — `rule:core-api/reference-card`.
const MATCH_GROUP_DOC: MethodDoc = MethodDoc {
    short: "Returns the text of one group, by its number or by its name. PHP reads this as \
            `$matches[$group]` after `preg_match`.",
    params: &[ParamDoc {
        name: "group",
        desc: "The number of the group, or its name. Group `0` is the whole match.",
        shape: &[],
    }],
    ret: "The text of the group. The result is `null` if the pattern has the group but this \
          match did not use it.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The pattern has no group with this number or name.",
    }],
};

/// `Core\Regex\Match::groups`'s reference card — `rule:core-api/reference-card`.
const MATCH_GROUPS_DOC: MethodDoc = MethodDoc {
    short: "Returns the text of every group in one array. A named group is in the array twice: \
            under its name and under its number.",
    params: &[],
    ret: "An array with group `0` first. A group that this match did not use is in the array, \
          and its value is `null`.",
    errors: &[],
};

/// `Core\Regex\Match::offset`'s reference card — `rule:core-api/reference-card`.
const MATCH_OFFSET_DOC: MethodDoc = MethodDoc {
    short: "Returns the position in the text where the whole match starts. The position counts \
            characters, not bytes.",
    params: &[],
    ret: "The position of the first character of the match. A match at the start of the text is \
          at `0`.",
    errors: &[],
};

/// `Core\Regex\Match::text`'s reference card — `rule:core-api/reference-card`.
const MATCH_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Returns the whole text that the pattern matched. This is the same text as \
            `group(0)`.",
    params: &[],
    ret: "The matched text. It is never `null`, but it can be an empty string.",
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
    short: "Finds the first match of `$pattern` in `$subject` and returns it as a \
            `Core\\Regex\\Match`. The `Match` has the matched text, each group and the position \
            of the match.",
    params: &[
        ParamDoc {
            name: "subject",
            desc: "The text to search.",
            shape: &[],
        },
        ParamDoc {
            name: "pattern",
            desc: "A `Core\\Regex\\Pattern` from `Core\\Regex::compile`, or a pattern string with \
                   no options. A `tainted` string does not compile here.",
            shape: &[],
        },
        ParamDoc {
            name: "from",
            desc: "The character position where the search starts. The default is `0`. A \
                   negative position counts from the end of `$subject`.",
            shape: &[],
        },
    ],
    ret: "The first `Core\\Regex\\Match` at or after `from`, or `null` if there is no match.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$pattern` is not a valid regular expression, or matching it against `$subject` \
               needs more steps than the limit allows.",
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

/// `rule:core-classes/regex-two-tiers`'s step budget for the backtracking tier,
/// as the number `[limits] max_regex_steps` falls back to.
///
/// `fancy-regex`'s own default, kept rather than lowered: it is the figure
/// that crate's adversarial-pattern tests are written against, and picking a
/// different one here would be a number with no measurement behind it. What
/// matters for the ADR is that exhausting it *throws*, which
/// [`budget_exhausted`] is.
///
/// This is what a request **starts with** and not a ceiling: the directive is
/// `Runtime`-class (`rule:config/three-changeability-classes`), so a request
/// may widen or narrow it for itself, and a host that wants a bound on how far
/// writes `[limits.hard] max_regex_steps`.
const BACKTRACK_BUDGET: usize = 1_000_000;

/// `[limits] max_regex_steps` — the directive [`step_budget`] reads.
const STEPS_KEY: &str = "max_regex_steps";

/// The step budget this request runs its backtracking patterns under:
/// `[limits] max_regex_steps`, or [`BACKTRACK_BUDGET`] where the configuration
/// states nothing usable.
///
/// Read per call rather than once per request, because `Core\Config::set` may
/// have moved it since the last pattern and a request that widened its own
/// budget has to get the wider one on the next compile.
///
/// **`false` is not a spelling for an unbounded tier.**
/// `rule:config/three-changeability-classes` spells "no ceiling" that way for
/// the limits a request may raise; a pattern allowed to backtrack forever is
/// the hang `rule:core-classes/regex-two-tiers` exists to stop, so it reads as
/// the shipped budget instead. A malformed value reads the same way, for the
/// reason `Ctx`'s own limit readers answer their defaults: the file was parsed
/// and refused once already, at the boundary that could name the line.
fn step_budget(ctx: &Ctx) -> usize {
    let Some(written) = ctx.config().and_then(|config| config.get(STEPS_KEY)) else {
        return BACKTRACK_BUDGET;
    };
    let setting = nvs_config::Setting::Text(written);
    match nvs_config::Quantity::parse(STEPS_KEY, nvs_config::Unit::Count, &setting) {
        Ok(nvs_config::Quantity::Count(steps)) => usize::try_from(steps).unwrap_or(usize::MAX),
        _ => BACKTRACK_BUDGET,
    }
}

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

/// One [`CACHE`] entry: the three things that key a compiled program — the
/// pattern text, the [`PATTERN`] flags and the step budget it was built under —
/// and the program itself.
type Cached = (String, u8, usize, Rc<Compiled>);

thread_local! {
    /// This core's compiled patterns, keyed by the pattern text, **the flags it
    /// was compiled under** and **the step budget it was built with** — the
    /// same text under two [`PATTERN`] flag sets is two programs, and
    /// [`step_budget`] is baked into the backtracking engine when it is built
    /// rather than read per match, so one key would hand the second call the
    /// first one's answer and the second request the first one's ceiling.
    ///
    /// The budget is part of the key rather than checked on a hit because the
    /// alternative is a compiled program whose ceiling is whichever request
    /// reached this core first, which is one request running under another's
    /// configuration. What it spends is one `usize` per entry and a second
    /// compile on a core serving two budgets, which happens only where a
    /// request moved its own.
    ///
    /// A `Vec` rather than a map: it is capacity-bounded and scanned
    /// linearly, which for a few hundred short keys beats hashing them, and
    /// it keeps the "clear when full" policy a one-liner.
    static CACHE: RefCell<Vec<Cached>> = const { RefCell::new(Vec::new()) };
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
fn compiled(pattern: &str, flags: u8, member: &str, budget: usize) -> Result<Rc<Compiled>, Fault> {
    compiled_prepared(pattern, flags, member, None, budget)
}

/// [`compiled`], for the one member whose call site carries
/// [`crate::registry::PREPARED_MEMBERS`]' tier: the same cache, and [`build`]
/// told which engine the compiler already routed this text to.
///
/// # Errors
///
/// [`compiled`]'s, unchanged — a prepared tier picks the engine that compiles
/// the pattern and never whether one does.
fn compiled_prepared(
    pattern: &str,
    flags: u8,
    member: &str,
    prepared: Option<Tier>,
    budget: usize,
) -> Result<Rc<Compiled>, Fault> {
    if let Some(hit) = CACHE.with_borrow(|cache| {
        cache
            .iter()
            .find(|(key, keyed_flags, keyed_budget, _)| {
                key == pattern && *keyed_flags == flags && *keyed_budget == budget
            })
            .map(|(_, _, _, compiled)| Rc::clone(compiled))
    }) {
        return Ok(hit);
    }

    let built = build(pattern, flags, prepared, budget)
        .map_err(|why| Fault::thrown(format!("Core\\Regex::{member}(): {why}")))?;
    // Asked before the program is cached, so a refused request leaves nothing
    // behind, and a cached program has already been paid for.
    nvs_runtime::affordable(
        Some(search_cost(&built, &effective(pattern, flags))),
        "Core\\Regex",
    )?;

    let built = Rc::new(built);
    CACHE.with_borrow_mut(|cache| {
        if cache.len() >= CACHE_CAPACITY {
            cache.clear();
        }
        cache.push((pattern.to_owned(), flags, budget, Rc::clone(&built)));
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
/// # What a prepared tier changes, and what it cannot
///
/// `prepared` is the tier the compiler settled for a literal pattern
/// ([`crate::registry::PREPARED_MEMBERS`]), and [`Tier::Backtracking`] is the
/// only value that changes anything: the linear engine's parser has already
/// refused this text once, while checking, so offering it again would buy the
/// same refusal a second time. The routing rule above is unchanged by it —
/// a prepared tier skips a step whose answer is known, and never routes a
/// pattern anywhere the rule would not have.
///
/// A wrong or stale word is therefore a performance question and not a semantic
/// one: [`Tier::Linear`] is what the untold path does anyway, and a spurious
/// [`Tier::Backtracking`] compiles on an engine that expresses every pattern the
/// linear one does.
///
/// # Errors
///
/// The sentence [`compiled`] throws, without the member prefix a call site
/// adds: the caller that has one is the runtime, and the caller that does not
/// is [`validate`].
fn build(
    pattern: &str,
    flags: u8,
    prepared: Option<Tier>,
    budget: usize,
) -> Result<Compiled, String> {
    let spelled = effective(pattern, flags);
    if prepared == Some(Tier::Backtracking) {
        return backtracking(pattern, &spelled, budget);
    }
    match regex::Regex::new(&spelled) {
        Ok(linear) => Ok(Compiled::Linear(linear)),
        Err(regex::Error::Syntax(_)) => backtracking(pattern, &spelled, budget),
        Err(limit) => Err(format!(
            "`{pattern}` is a pattern the linear engine expresses but is too large for it \
             to build: {limit}"
        )),
    }
}

/// The bytes the linear engine allocates on this core the first time it
/// searches with `built`, which [`compiled_prepared`] asks the request's
/// memory ceiling for in front of the engine.
///
/// The engine's capture table has one row of capture slots per automaton
/// state, and it keeps two tables, so a pattern of thousands of groups asks
/// for gigabytes before it reads one byte of the subject. `regex` allocates
/// that table outside any ask, and a request past its ceiling then ends with
/// no message; asking first ends it with the memory-limit `FATAL` every other
/// breach reports. The figure is `regex-automata`'s own layout for its PikeVM,
/// read off an automaton built from the same text with the same defaults.
///
/// The backtracking tier answers `0`: its memory grows with the steps it
/// takes, and the step budget already bounds those.
///
/// **What it spends:** a second automaton built and dropped, once per pattern
/// per core, on a cache miss only.
fn search_cost(built: &Compiled, spelled: &str) -> usize {
    let Compiled::Linear(_) = built else {
        return 0;
    };
    // `regex` has already built this text under a tighter size limit than
    // the automaton's default, so the build does not fail in practice; if it
    // did, the table would still be counted by the allocator as it grows.
    let Ok(nfa) = regex_automata::nfa::thompson::NFA::new(spelled) else {
        return 0;
    };
    let slots = nfa.group_info().slot_len();
    let table = nfa
        .states()
        .len()
        .saturating_mul(slots)
        .saturating_add(slots.max(nfa.pattern_len().saturating_mul(2)));
    table
        .saturating_mul(2)
        .saturating_mul(std::mem::size_of::<usize>())
}

/// `spelled` on `rule:core-classes/regex-two-tiers`'s second tier, with the
/// caller's step budget on it — [`build`]'s second arm and its prepared
/// shortcut reach the same two lines rather than spelling them twice.
///
/// The budget is fixed when the program is built and cannot be moved per
/// match, which is why [`CACHE`] keys on it.
///
/// # Errors
///
/// The sentence [`build`] answers with for a pattern neither engine compiles;
/// it quotes `pattern` as the program wrote it rather than [`effective`]'s
/// flag-wrapped form.
fn backtracking(pattern: &str, spelled: &str, budget: usize) -> Result<Compiled, String> {
    Ok(Compiled::Backtracking(
        fancy_regex::RegexBuilder::new(spelled)
            .backtrack_limit(budget)
            .build()
            .map_err(|err| {
                format!("`{pattern}` is not a pattern either engine can compile: {err}")
            })?,
    ))
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
/// in is the tier reported here. What *is* flag-sensitive is gap 2's `[regex]
/// backtracking = "deny"`, which refuses the second tier outright rather than
/// re-routing anything into it, and which this fold does not read.
pub fn validate(pattern: &str) -> Result<Tier, String> {
    // The shipped budget and not a configured one: a fold runs while checking,
    // where there is no request whose `[limits]` to read, and the budget cannot
    // change which tier a pattern lands on anyway.
    build(pattern, NO_FLAGS, None, BACKTRACK_BUDGET).map(|compiled| match compiled {
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

impl Tier {
    /// This tier as the word [`PREPARED_NONE`] documents.
    #[must_use]
    pub const fn prepared_code(self) -> i64 {
        match self {
            Self::Linear => PREPARED_LINEAR,
            Self::Backtracking => PREPARED_BACKTRACKING,
        }
    }
}

/// The word [`crate::registry::PREPARED_MEMBERS`]' argument 0 carries when the
/// call site prepared nothing.
///
/// That slot is an ABI between two crates that cannot name each other's types —
/// `nvs_ir::ir::Prepared` encodes, [`prepared_tier`] decodes — so the mapping
/// lives here, beside the decoder, rather than being spelled at both ends.
/// Nothing prepared is the ordinary answer for a pattern the program computed:
/// that roster's own docs say why the slot is there for such a call anyway.
pub const PREPARED_NONE: i64 = 0;
/// [`Tier::Linear`]'s word — see [`PREPARED_NONE`].
pub const PREPARED_LINEAR: i64 = 1;
/// [`Tier::Backtracking`]'s word — see [`PREPARED_NONE`].
pub const PREPARED_BACKTRACKING: i64 = 2;

/// The tier `code` names, and `None` for [`PREPARED_NONE`] or a word this build
/// does not know.
///
/// **An unknown word is read as "nothing was prepared" rather than refused.**
/// What it costs is one routing decision made the ordinary way, while a helper
/// that threw over an ABI word would turn a version skew into a failed request;
/// and `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` puts
/// the compiler build in the artifact's own address, so a cached unit cannot
/// hand this process a word another compiler minted.
#[must_use]
pub fn prepared_tier(code: i64) -> Option<Tier> {
    match code {
        PREPARED_LINEAR => Some(Tier::Linear),
        PREPARED_BACKTRACKING => Some(Tier::Backtracking),
        _ => None,
    }
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
fn budget_exhausted(member: &str, pattern: &str, budget: usize, err: &fancy_regex::Error) -> Fault {
    Fault::thrown(format!(
        "Core\\Regex::{member}(): `{pattern}` exhausted the backtracking budget of \
         {budget} steps against this subject ({err})"
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

/// One part of `Core\Regex::replace`'s replacement, as [`template`] reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece<'a> {
    /// Text copied as it is.
    Text(&'a str),
    /// The text of one group, by number. A number the pattern has no group
    /// for, and a group that did not take part in the match, insert nothing.
    Group(usize),
}

/// `Core\Regex::replace`'s replacement, read the way `preg_replace` reads it.
///
/// A bare `$` takes **at most two digits** and nothing else: `$1st` is group 1
/// and then `st`, `$123` is group 12 and then `3`, and `$a` is the two
/// characters `$a`. `${n}` and `${name}` are the braced references, `$$` is
/// one `$`, and a `$` that starts none of these is itself. Both engines' own
/// expanders read a bare `$` as the *longest* run of name characters, which is
/// why neither is handed the template. A name `names` does not have becomes a
/// group number no match has, so it inserts nothing, as a missing number does.
fn template<'a>(replacement: &'a str, names: &[Option<&str>]) -> Vec<Piece<'a>> {
    let number = |digits: &str| digits.parse::<usize>().unwrap_or(usize::MAX);
    let bytes = replacement.as_bytes();
    let mut pieces = Vec::new();
    let mut at = 0;
    while let Some(found) = replacement[at..].find('$') {
        let dollar = at + found;
        if dollar > at {
            pieces.push(Piece::Text(&replacement[at..dollar]));
        }
        let after = &bytes[dollar + 1..];
        let digits = after
            .iter()
            .take(2)
            .take_while(|b| b.is_ascii_digit())
            .count();
        // The length of the name between `${` and `}`, when there is one.
        let braced = (after.first() == Some(&b'{'))
            .then(|| after.iter().skip(1).position(|&b| b == b'}'))
            .flatten()
            .filter(|&close| {
                close > 0
                    && after[1..=close]
                        .iter()
                        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
            });
        if after.first() == Some(&b'$') {
            pieces.push(Piece::Text("$"));
            at = dollar + 2;
        } else if digits > 0 {
            let digits = &replacement[dollar + 1..dollar + 1 + digits];
            pieces.push(Piece::Group(number(digits)));
            at = dollar + 1 + digits.len();
        } else if let Some(close) = braced {
            let name = &replacement[dollar + 2..dollar + 2 + close];
            let group = if name.bytes().all(|b| b.is_ascii_digit()) {
                number(name)
            } else {
                names
                    .iter()
                    .position(|held| *held == Some(name))
                    .unwrap_or(usize::MAX)
            };
            pieces.push(Piece::Group(group));
            at = dollar + 3 + close;
        } else {
            pieces.push(Piece::Text("$"));
            at = dollar + 1;
        }
    }
    if at < replacement.len() {
        pieces.push(Piece::Text(&replacement[at..]));
    }
    pieces
}

/// Appends `gap`, the text before one match, and that match's expansion of
/// `pieces` to `out`, after [`grow`] has asked for the room. `group` returns
/// a group's text by number.
fn expand<'s>(
    out: &mut String,
    gap: &str,
    pieces: &[Piece<'_>],
    group: impl Fn(usize) -> Option<&'s str>,
) -> Result<(), Fault> {
    let wanted = pieces
        .iter()
        .map(|piece| match *piece {
            Piece::Text(text) => text.len(),
            Piece::Group(number) => group(number).map_or(0, str::len),
        })
        .fold(gap.len(), usize::saturating_add);
    grow(out, wanted, "Core\\Regex::replace")?;
    out.push_str(gap);
    for piece in pieces {
        match *piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Group(number) => out.push_str(group(number).unwrap_or_default()),
        }
    }
    Ok(())
}

/// Makes room in `out` for `add` more bytes, and asks the request's memory
/// budget **before** the allocation rather than after it.
///
/// A replacement repeats text the program never allocated — `$0` written a
/// thousand times over a long match — so a result built first and checked
/// afterwards can reach many times the request's limit before the check. The
/// room grows by doubling, as `String`'s own does, and the doubled size is
/// what is asked for.
///
/// # Errors
///
/// [`nvs_runtime::affordable`]'s two: a `FATAL` past the limit, and a throw
/// for a size no process could hold.
fn grow(out: &mut String, add: usize, member: &str) -> Result<(), Fault> {
    let needed = out.len().checked_add(add);
    if needed.is_some_and(|needed| needed <= out.capacity()) {
        return Ok(());
    }
    let doubled = needed.map(|needed| needed.max(out.capacity().saturating_mul(2)));
    let size = nvs_runtime::affordable(doubled, member)?;
    out.reserve_exact(size - out.len());
    Ok(())
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
    ///
    /// **Argument 0 is [`crate::registry::PREPARED_MEMBERS`]' word**, ahead of
    /// the pattern: the tier the compiler settled for a literal, and
    /// [`PREPARED_NONE`] for a pattern the program computed. It is read only on
    /// a cache miss and decides only which engine is offered the text first,
    /// which is [`build`]'s own account of what a prepared tier can change. A
    /// word this build does not know is [`prepared_tier`]'s `None` and costs
    /// nothing but the routing it would have derived anyway.
    fn nvs_core_regex_compile(ctx, args: [6]) {
        let prepared = prepared_tier(args[0].as_int().unwrap_or(PREPARED_NONE));
        let pattern = text(&args[1], "compile", "the pattern")?;
        let mut flags = NO_FLAGS;
        for (slot, option, flag) in [
            (2, "the `caseInsensitive` option", FLAG_CASE_INSENSITIVE),
            (3, "the `multiline` option", FLAG_MULTILINE),
            (4, "the `dotAll` option", FLAG_DOT_ALL),
            (5, "the `ungreedy` option", FLAG_UNGREEDY),
        ] {
            if boolean(&args[slot], "compile", option)? {
                flags |= flag;
            }
        }
        compiled_prepared(pattern, flags, "compile", prepared, step_budget(ctx))?;
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
    fn nvs_core_regex_matches(ctx, args: [2]) {
        let subject = text(&args[0], "matches", "the subject")?;
        let given = pattern_of(&args[1], "matches")?;
        let pattern = text(&given.text, "matches", "the pattern")?;
        let budget = step_budget(ctx);
        let found = match &*compiled(pattern, given.flags, "matches", budget)? {
            Compiled::Linear(re) => re.is_match(subject),
            Compiled::Backtracking(re) => re
                .is_match(subject)
                .map_err(|err| budget_exhausted("matches", pattern, budget, &err))?,
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
///
/// `offsets` is the subject the engine matched over, as the one thing this
/// needs it for: the match's byte position in [`DEFAULT`]'s unit. A member
/// reporting a run of matches passes **one** cursor through all of them, which
/// is what keeps the whole run's conversion O(n) — [`Cursor`] owns the ordering
/// that buys it.
fn built_match(offsets: &mut Cursor<'_>, names: &[Option<&str>], captured: &Captured<'_>) -> Value {
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
        .map_or(0, |(byte, _)| offsets.index_of_byte(byte));
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
/// in an Novis array is spelled. A `string` key is borrowed from the argument,
/// so a lookup by name allocates nothing.
fn group_key<'a>(value: &'a Value, member: &str) -> Result<std::borrow::Cow<'a, [u8]>, Fault> {
    if let Some(number) = value.as_int() {
        return Ok(std::borrow::Cow::Owned(number.to_string().into_bytes()));
    }
    if let Some(bytes) = value.as_str_bytes() {
        return Ok(std::borrow::Cow::Borrowed(bytes));
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
    fn nvs_core_regex_match(ctx, args: [3]) {
        let subject = text(&args[0], "match", "the subject")?;
        let given = pattern_of(&args[1], "match")?;
        let pattern = text(&given.text, "match", "the pattern")?;
        let from = integer(&args[2], "match", "the `from` option")?;
        let start = start_byte(subject, from);

        let budget = step_budget(ctx);
        let compiled = compiled(pattern, given.flags, "match", budget)?;
        let names = names_of(&compiled);
        let found = match &*compiled {
            Compiled::Linear(re) => re
                .captures_at(subject, start)
                .map(|caps| linear_groups(&caps)),
            Compiled::Backtracking(re) => re
                .captures_from_pos(subject, start)
                .map_err(|err| budget_exhausted("match", pattern, budget, &err))?
                .map(|caps| backtracking_groups(&caps)),
        };
        Ok(found.map_or_else(Value::null, |captured| {
            built_match(&mut DEFAULT.cursor(subject), &names, &captured)
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
    /// **Every match's `offset` is converted by one cursor**, so reporting
    /// positions for *k* matches costs one walk of the subject rather than
    /// *k* of its prefixes: the matches arrive in increasing byte order, which
    /// is exactly what [`Cursor`] asks for.
    fn nvs_core_regex_match_all(ctx, args: [2]) {
        let subject = text(&args[0], "matchAll", "the subject")?;
        let given = pattern_of(&args[1], "matchAll")?;
        let pattern = text(&given.text, "matchAll", "the pattern")?;

        let budget = step_budget(ctx);
        let compiled = compiled(pattern, given.flags, "matchAll", budget)?;
        let names = names_of(&compiled);
        let mut offsets = DEFAULT.cursor(subject);
        let mut out = NvsArray::new();
        match &*compiled {
            Compiled::Linear(re) => {
                for caps in re.captures_iter(subject) {
                    out.append(built_match(&mut offsets, &names, &linear_groups(&caps)));
                }
            }
            Compiled::Backtracking(re) => {
                for caps in re.captures_iter(subject) {
                    let caps =
                        caps.map_err(|err| budget_exhausted("matchAll", pattern, budget, &err))?;
                    out.append(built_match(&mut offsets, &names, &backtracking_groups(&caps)));
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
    /// `$`. A bare `$` reads at most two digits and a `$` that starts no
    /// reference is itself, as PHP's does; [`template`] is that reading.
    /// PHP additionally accepts `\1`; it is not accepted here, because
    /// `\1` inside a double-quoted Novis string is already an escape the lexer
    /// reads, so the same source text would mean two different things
    /// depending on the quote used to write it. A group reference that names
    /// no group expands to the empty string, as PHP's does.
    ///
    /// `limit` counts *replacements*, defaults to every one, and a limit of
    /// `0` replaces nothing — which is the reading the option's `uint` type
    /// forces and the one PHP's own `preg_replace` gives it.
    ///
    /// The result is built here rather than by either engine's `replacen`,
    /// so [`grow`] asks the memory budget before every allocation of it. A
    /// template that reads no group searches with `find_iter`, which builds
    /// no capture set per match.
    fn nvs_core_regex_replace(ctx, args: [4]) {
        let subject = text(&args[0], "replace", "the subject")?;
        let given = pattern_of(&args[1], "replace")?;
        let pattern = text(&given.text, "replace", "the pattern")?;
        let replacement = text(&args[2], "replace", "the replacement")?;
        let limit = unsigned(&args[3], "replace", "the `limit` option")?;
        if limit == 0 {
            return produced(subject);
        }
        let count = usize::try_from(limit).unwrap_or(usize::MAX);

        let budget = step_budget(ctx);
        let compiled = compiled(pattern, given.flags, "replace", budget)?;
        let pieces = template(replacement, &names_of(&compiled));
        let reads_groups = pieces
            .iter()
            .any(|piece| matches!(piece, Piece::Group(number) if *number > 0));
        let mut out = String::new();
        let mut cursor = 0;
        match &*compiled {
            Compiled::Linear(re) if !reads_groups => {
                for whole in re.find_iter(subject).take(count) {
                    let gap = &subject[cursor..whole.start()];
                    expand(&mut out, gap, &pieces, |number| {
                        (number == 0).then(|| whole.as_str())
                    })?;
                    cursor = whole.end();
                }
            }
            Compiled::Linear(re) => {
                for caps in re.captures_iter(subject).take(count) {
                    let Some(whole) = caps.get(0) else { continue };
                    let gap = &subject[cursor..whole.start()];
                    expand(&mut out, gap, &pieces, |number| {
                        caps.get(number).map(|group| group.as_str())
                    })?;
                    cursor = whole.end();
                }
            }
            Compiled::Backtracking(re) => {
                for caps in re.captures_iter(subject).take(count) {
                    let caps =
                        caps.map_err(|err| budget_exhausted("replace", pattern, budget, &err))?;
                    let Some(whole) = caps.get(0) else { continue };
                    let gap = &subject[cursor..whole.start()];
                    expand(&mut out, gap, &pieces, |number| {
                        caps.get(number).map(|group| group.as_str())
                    })?;
                    cursor = whole.end();
                }
            }
        }
        grow(&mut out, subject.len() - cursor, "Core\\Regex::replace")?;
        out.push_str(&subject[cursor..]);
        produced(&out)
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
    offsets: &mut Cursor<'_>,
    names: &[Option<&str>],
    captured: &Captured<'_>,
) -> Result<String, Fault> {
    let matched = built_match(offsets, names, captured);
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

        let budget = step_budget(ctx);
        let compiled = compiled(pattern, given.flags, "replaceWith", budget)?;
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
                        caps.map_err(|err| budget_exhausted("replaceWith", pattern, budget, &err))?;
                    found.push(backtracking_groups(&caps));
                }
            }
        }

        // Every piece of the result is asked for by `grow` before it is
        // allocated, as `Core\Regex::replace`'s is.
        let mut out = String::new();
        let mut offsets = DEFAULT.cursor(subject);
        let mut cursor = 0;
        for captured in &found {
            let Some((start, whole)) = captured.first().copied().flatten() else {
                continue;
            };
            let answer = replacement_for(ctx, args[2], &mut offsets, &names, captured)?;
            let gap = &subject[cursor..start];
            grow(&mut out, gap.len().saturating_add(answer.len()), "Core\\Regex::replaceWith")?;
            out.push_str(gap);
            out.push_str(&answer);
            cursor = start + whole.len();
        }
        grow(&mut out, subject.len() - cursor, "Core\\Regex::replaceWith")?;
        out.push_str(&subject[cursor..]);
        produced(&out)
    }
}

/// Hands `each` every piece `pattern` splits `subject` into, in order, at
/// most `pieces` of them with the last holding the unsplit remainder.
///
/// **No piece is collected here.** A caller turns each one into a value as it
/// arrives, so the split holds no `Vec` of sixteen bytes per piece beside the
/// result, which for an empty pattern is sixteen times the subject.
fn each_piece<'a>(
    compiled: &Compiled,
    subject: &'a str,
    pieces: Option<usize>,
    pattern: &str,
    budget: usize,
    mut each: impl FnMut(&'a str),
) -> Result<(), Fault> {
    let pieces = pieces.unwrap_or(usize::MAX);
    match compiled {
        Compiled::Linear(re) => re.splitn(subject, pieces).for_each(each),
        Compiled::Backtracking(re) => {
            for piece in re.splitn(subject, pieces) {
                each(piece.map_err(|err| budget_exhausted("split", pattern, budget, &err))?);
            }
        }
    }
    Ok(())
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
    fn nvs_core_regex_split(ctx, args: [4]) {
        let subject = text(&args[0], "split", "the subject")?;
        let given = pattern_of(&args[1], "split")?;
        let pattern = text(&given.text, "split", "the pattern")?;
        let limit = integer(&args[2], "split", "the `limit` option")?;
        let keep_empty = boolean(&args[3], "split", "the `keepEmpty` option")?;

        let budget = step_budget(ctx);
        let compiled = compiled(pattern, given.flags, "split", budget)?;
        // A limit of `0` means one piece, not none — see the docs above.
        let (wanted, kept) = if limit >= 0 {
            (Some(usize::try_from(limit).unwrap_or(usize::MAX).max(1)), usize::MAX)
        } else {
            // A negative limit needs the count before the first piece is kept,
            // so the split runs twice: once counting, once building.
            let mut all = 0_usize;
            each_piece(&compiled, subject, None, pattern, budget, |_| all += 1)?;
            let dropped = usize::try_from(limit.unsigned_abs()).unwrap_or(usize::MAX);
            (None, all.saturating_sub(dropped))
        };

        let mut out = NvsArray::new();
        let mut seen = 0_usize;
        each_piece(&compiled, subject, wanted, pattern, budget, |piece| {
            seen += 1;
            if seen <= kept && (keep_empty || !piece.is_empty()) {
                out.append(Value::str(NvsStr::new(piece.as_bytes())));
            }
        })?;
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
        let compiled = built(r"(?<word>[a-z]+)(\d+)?", NO_FLAGS, "match").expect("compiles");
        let names = names_of(&compiled);
        let Compiled::Linear(re) = &*compiled else {
            panic!("a plain pattern lands in the linear tier")
        };
        let caps = re.captures_at("  abc", 0).expect("matches");
        let value = built_match(&mut DEFAULT.cursor("  abc"), &names, &linear_groups(&caps));
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

    /// The four `Match` members, called the way a program calls them, over one
    /// match found after a two-byte `é`: `offset` counts graphemes, `text` is
    /// group `0`, `groups` lists a named group under its name and then its
    /// number, and `group` answers `null` for a declared group the match did
    /// not reach and throws for one the pattern never declared.
    // covers: Core\Regex\Match::group, Core\Regex\Match::groups, Core\Regex\Match::offset, Core\Regex\Match::text
    #[test]
    fn the_four_match_members_read_one_match_by_grapheme_and_by_group() {
        let mut ctx = Ctx::buffered();
        let subject = Value::str(NvsStr::new("é abc".as_bytes()));
        let pattern = Value::str(NvsStr::new(br"(?<word>[a-z]+)(\d+)?"));
        let found = nvs_runtime::call(
            nvs_core_regex_match,
            &mut ctx,
            &[subject, pattern, Value::int(0)],
        )
        .expect("matches");
        let text_of = |value: Value| value.as_str_bytes().map(<[u8]>::to_vec);

        let offset =
            nvs_runtime::call(nvs_core_regex_match_offset, &mut ctx, &[found]).expect("offset");
        assert_eq!(
            offset.as_int(),
            Some(2),
            "`é` is one grapheme and two bytes"
        );

        let text = nvs_runtime::call(nvs_core_regex_match_text, &mut ctx, &[found]).expect("text");
        assert_eq!(text_of(text), Some(b"abc".to_vec()));

        let groups =
            nvs_runtime::call(nvs_core_regex_match_groups, &mut ctx, &[found]).expect("groups");
        let held = crate::arr::borrowed(groups.array_ptr().expect("an array"));
        assert_eq!(
            held.keys(),
            [
                b"0".to_vec(),
                b"word".to_vec(),
                b"1".to_vec(),
                b"2".to_vec()
            ]
        );
        assert!(held.get(b"2").is_some_and(|v| v.tag() == Some(Tag::Null)));

        let word = Value::str(NvsStr::new(b"word"));
        let by_name = nvs_runtime::call(nvs_core_regex_match_group, &mut ctx, &[found, word])
            .expect("a name");
        assert_eq!(text_of(by_name), Some(b"abc".to_vec()));
        let unreached = nvs_runtime::call(
            nvs_core_regex_match_group,
            &mut ctx,
            &[found, Value::int(2)],
        )
        .expect("declared");
        assert_eq!(unreached.tag(), Some(Tag::Null));
        assert!(
            nvs_runtime::call(
                nvs_core_regex_match_group,
                &mut ctx,
                &[found, Value::int(3)]
            )
            .is_err()
        );
        let message = ctx.take_pending().expect("a throw leaves its message");
        assert_eq!(
            message,
            "Core\\Regex\\Match::group(): the pattern declares no group `3`"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns every reference the calls above returned"
        )]
        unsafe {
            for value in [by_name, text, groups, found, word, subject, pattern] {
                value.release();
            }
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
                        tiered(pattern, flags, None).expect("a linear pattern compiles"),
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
                    tiered(pattern, NO_FLAGS, None).expect("the second tier compiles it"),
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
        let refused = tiered(&too_big, NO_FLAGS, None)
            .expect_err("5,000 unicode classes is past the size limit");
        assert!(refused.contains("too large for it to build"), "{refused}");
    }

    /// `crate::registry::PREPARED_MEMBERS`' word, both halves of it: it
    /// round-trips, and what it changes in [`build`] is which engine is offered
    /// the text first and nothing else.
    #[test]
    fn a_prepared_tier_routes_and_decides_nothing() {
        assert_eq!(
            prepared_tier(Tier::Linear.prepared_code()),
            Some(Tier::Linear)
        );
        assert_eq!(
            prepared_tier(Tier::Backtracking.prepared_code()),
            Some(Tier::Backtracking)
        );
        // The zero word and a word this build does not know are one answer, so
        // a compiler-version skew costs a routing decision rather than a
        // request — the reason `prepared_tier` states.
        assert_eq!(prepared_tier(PREPARED_NONE), None);
        assert_eq!(prepared_tier(i64::MAX), None);

        // A lookbehind told what it is goes straight to the engine that
        // expresses it, landing where the untold path lands.
        assert!(matches!(
            tiered(r"(?<=USD )\d+", NO_FLAGS, Some(Tier::Backtracking))
                .expect("the second tier compiles a lookbehind"),
            Compiled::Backtracking(_)
        ));

        // And a word that disagrees with the text is a routing question rather
        // than a semantic one: the backtracker expresses every pattern the
        // linear engine does, so a pattern wrongly claimed for it still
        // compiles and still matches.
        assert!(matches!(
            tiered("[a-z]+", NO_FLAGS, Some(Tier::Backtracking))
                .expect("the backtracker expresses a character class too"),
            Compiled::Backtracking(_)
        ));
    }

    /// A pattern neither engine can compile throws rather than matching
    /// nothing — `rule:core-classes/regex-syntax`'s "never silently ignored".
    #[test]
    fn a_pattern_neither_engine_accepts_throws() {
        let err = built("(unclosed", NO_FLAGS, "matches")
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
        let held = built(CATASTROPHIC, NO_FLAGS, "matches").expect("compiles");
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
        let Fault::Thrown(class, message) =
            budget_exhausted("matches", CATASTROPHIC, BACKTRACK_BUDGET, &err)
        else {
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
        let held = built(CATASTROPHIC, NO_FLAGS, "matches").expect("compiles");
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
        let first = built(r"^cached-\w+$", NO_FLAGS, "matches").expect("compiles");
        let again = built(r"^cached-\w+$", NO_FLAGS, "matches").expect("compiles");
        assert!(Rc::ptr_eq(&first, &again));
    }

    /// A pattern's capture table is priced before the engine allocates it:
    /// a few groups cost kilobytes, five thousand cost more than a gigabyte,
    /// and a request under a 64 MiB ceiling is refused the second pattern
    /// with nothing cached and without the gigabyte ever being held.
    // covers: Core\Regex::match
    #[test]
    fn a_search_whose_capture_table_the_request_cannot_afford_is_refused_first() {
        let few = tiered(r"(\d+)-(\d+)", NO_FLAGS, None).expect("compiles");
        assert!(search_cost(&few, r"(\d+)-(\d+)") < 64 * 1024);
        let many = "(a)".repeat(5000);
        let wide = tiered(&many, NO_FLAGS, None).expect("compiles");
        assert!(search_cost(&wide, &many) > 1 << 30);
        let backtracking = tiered(r"(a)\1", NO_FLAGS, None).expect("compiles");
        assert_eq!(search_cost(&backtracking, r"(a)\1"), 0);

        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(64 << 20);
        let subject = Value::str(NvsStr::new("a".repeat(5000).as_bytes()));
        let pattern = Value::str(NvsStr::new(many.as_bytes()));
        let args = [subject, pattern, Value::int(0)];
        assert!(nvs_runtime::call(nvs_core_regex_match, &mut ctx, &args).is_err());
        assert!(
            ctx.memory_used() < 64 << 20,
            "{} bytes held",
            ctx.memory_used()
        );
        assert!(CACHE.with_borrow(|cache| cache.iter().all(|(key, ..)| *key != many)));
        #[expect(unsafe_code, reason = "this frame owns the two strings it built")]
        unsafe {
            subject.release();
            pattern.release();
        }
    }

    /// The four flags are part of the key, not of the text: one pattern under
    /// two flag sets is two compiled programs, and each behaves as its flags
    /// say — the property `regex-compile-carries-the-four-flags.nvst` then
    /// pins through the members, one flag at a time.
    #[test]
    fn the_flags_are_part_of_the_key_and_reach_the_engine() {
        let plain = built("^flagged-a+$", NO_FLAGS, "compile").expect("compiles");
        let folded = built("^flagged-a+$", FLAG_CASE_INSENSITIVE, "compile").expect("compiles");
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
            let held = built(pattern, flags, "compile").expect("compiles");
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
            built(&format!("bounded-{nth}"), NO_FLAGS, "matches").expect("compiles");
        }
        CACHE.with_borrow(|cache| assert!(cache.len() <= CACHE_CAPACITY, "{}", cache.len()));
    }

    /// `Core\Regex::compile` builds the program before it returns, so the
    /// first match against the handle is already a cache hit, and the
    /// `Pattern` carries the text as the program wrote it beside the flags
    /// [`pattern_of`] reads back. A pattern neither engine takes throws from
    /// `compile` itself, quoting the written text rather than [`effective`]'s
    /// flag-wrapped form.
    // covers: Core\Regex::compile
    #[test]
    fn compile_builds_eagerly_and_answers_the_written_text_beside_its_flags() {
        let mut ctx = Ctx::buffered();
        let (on, off) = (Value::bool(true), Value::bool(false));
        let written = Value::str(NvsStr::new(br"^order-\d+$"));
        let args = [Value::int(PREPARED_NONE), written, on, off, on, off];
        let pattern = nvs_runtime::call(nvs_core_regex_compile, &mut ctx, &args).expect("compiles");

        let given = pattern_of(&pattern, "match").expect("a `Pattern` is a pattern");
        assert_eq!(given.text.as_str_bytes(), Some(&br"^order-\d+$"[..]));
        assert_eq!(given.flags, FLAG_CASE_INSENSITIVE | FLAG_DOT_ALL);
        let held = CACHE.with_borrow(|cache| {
            cache.iter().any(|(text, flags, budget, _)| {
                text == r"^order-\d+$" && *flags == given.flags && *budget == BACKTRACK_BUDGET
            })
        });
        assert!(held, "compile returned before the program was cached");

        let unclosed = Value::str(NvsStr::new(b"(unclosed"));
        let args = [Value::int(PREPARED_NONE), unclosed, on, off, off, off];
        assert!(nvs_runtime::call(nvs_core_regex_compile, &mut ctx, &args).is_err());
        let message = ctx.take_pending().expect("a throw leaves its message");
        assert!(message.contains("`(unclosed`"), "{message}");
        assert!(!message.contains("(?i:"), "{message}");

        #[expect(
            unsafe_code,
            reason = "this frame owns the two strings it built and the `Pattern` `compile` returned"
        )]
        unsafe {
            pattern.release();
            written.release();
            unclosed.release();
        }
    }

    /// `Core\Regex::match` starts at the grapheme `from` names, counts a
    /// negative one from the end, and answers `null` from the end onwards. The
    /// `Match` reports its offset in graphemes rather than bytes, so the
    /// two-byte `é` ahead of each run of digits moves it by one, not by two.
    // covers: Core\Regex::match
    #[test]
    fn match_searches_from_a_grapheme_index_and_answers_null_past_the_last_match() {
        let mut ctx = Ctx::buffered();
        let subject = Value::str(NvsStr::new("é1 é22".as_bytes()));
        let digits = Value::str(NvsStr::new(br"\d+"));
        let mut found = Vec::new();
        for from in [0, 2, -2, 5, 6, 100] {
            let args = [subject, digits, Value::int(from)];
            let answer = nvs_runtime::call(nvs_core_regex_match, &mut ctx, &args).expect("answers");
            found.push(answer.obj_ptr().map(|object| {
                let groups = crate::instance::slot(object, GROUPS_SLOT);
                let held = crate::arr::borrowed(groups.array_ptr().expect("an array"));
                let whole = held
                    .get(b"0")
                    .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec))
                    .expect("group 0 is the whole match");
                let offset = crate::instance::slot(object, OFFSET_SLOT).as_int();
                (whole, offset.expect("an offset"))
            }));
            #[expect(
                unsafe_code,
                reason = "this frame owns the one reference `match` returned"
            )]
            unsafe {
                answer.release();
            }
        }
        assert_eq!(
            found,
            [
                Some((b"1".to_vec(), 1)),
                Some((b"22".to_vec(), 4)),
                Some((b"22".to_vec(), 4)),
                Some((b"2".to_vec(), 5)),
                None,
                None,
            ]
        );
        #[expect(unsafe_code, reason = "this frame owns the two strings it built")]
        unsafe {
            subject.release();
            digits.release();
        }
    }

    /// `Core\Regex::matches` is unanchored on both engines, so only `^` and
    /// `$` make it ask about the whole subject, and a backtracking pattern
    /// that exhausts its step budget throws rather than answering `false`: a
    /// search that gave up has not shown the subject holds no match.
    // covers: Core\Regex::matches
    #[test]
    fn matches_is_unanchored_on_both_engines_and_throws_when_the_budget_runs_out() {
        let mut ctx = Ctx::buffered();
        let mut ask = |subject: &str, pattern: &str| {
            let subject = Value::str(NvsStr::new(subject.as_bytes()));
            let pattern = Value::str(NvsStr::new(pattern.as_bytes()));
            let answer = nvs_runtime::call(nvs_core_regex_matches, &mut ctx, &[subject, pattern]);
            #[expect(unsafe_code, reason = "this frame owns the two strings it built")]
            unsafe {
                subject.release();
                pattern.release();
            }
            answer.ok().map(|found| found.as_bool().expect("a bool"))
        };
        assert_eq!(ask("AB-1234 extra", r"[A-Z]{2}-\d{4}"), Some(true));
        assert_eq!(ask("AB-1234 extra", r"^[A-Z]{2}-\d{4}$"), Some(false));
        assert_eq!(ask("x 11 y", r"(\d)\1"), Some(true));
        assert_eq!(ask("x 12 y", r"^(\d)\1$"), Some(false));
        let slow = format!("{}b", "a".repeat(40));
        assert_eq!(ask(&slow, r"^(a|a?)+\1$"), None);
    }

    /// `Core\Regex::matchAll` answers every match in subject order, with the
    /// grapheme offsets its one [`Cursor`] converts, and the two engines agree:
    /// `(\d)\1*` needs a backreference, so it runs on the backtracking engine
    /// and must report the same runs at the same offsets as the linear `\d+`
    /// over a subject whose runs are each one repeated digit. A pattern that
    /// matches nowhere answers an empty array rather than `null`.
    // covers: Core\Regex::matchAll
    #[test]
    fn match_all_answers_every_match_at_its_grapheme_offset_on_both_engines() {
        let mut ctx = Ctx::buffered();
        let subject = Value::str(NvsStr::new("é1 é22 x é333".as_bytes()));
        let mut answers = Vec::new();
        for pattern in [r"\d+", r"(\d)\1*", "z"] {
            let pattern = Value::str(NvsStr::new(pattern.as_bytes()));
            let args = [subject, pattern];
            let answer =
                nvs_runtime::call(nvs_core_regex_match_all, &mut ctx, &args).expect("answers");
            let list = crate::arr::borrowed(answer.array_ptr().expect("an array"));
            let found: Vec<_> = (0..list.count())
                .map(|nth| {
                    let object = list
                        .get(nth.to_string().as_bytes())
                        .and_then(|one| one.obj_ptr())
                        .expect("a `Match`");
                    let groups = crate::instance::slot(object, GROUPS_SLOT);
                    let whole = crate::arr::borrowed(groups.array_ptr().expect("an array"))
                        .get(b"0")
                        .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec))
                        .expect("group 0 is the whole match");
                    let offset = crate::instance::slot(object, OFFSET_SLOT).as_int();
                    (whole, offset.expect("an offset"))
                })
                .collect();
            answers.push(found);
            #[expect(
                unsafe_code,
                reason = "this frame owns the array `matchAll` returned and the pattern it built"
            )]
            unsafe {
                answer.release();
                pattern.release();
            }
        }
        let runs = vec![
            (b"1".to_vec(), 1),
            (b"22".to_vec(), 4),
            (b"333".to_vec(), 10),
        ];
        assert_eq!(answers, [runs.clone(), runs, Vec::new()]);
        #[expect(unsafe_code, reason = "this frame owns the subject it built")]
        unsafe {
            subject.release();
        }
    }

    /// `Core\Regex::replace` reads its replacement the way `preg_replace`
    /// does on both engines: a bare `$` takes at most two digits, so `$1st` is
    /// group 1 and then `st` rather than a group named `1st`, and a `$` that
    /// starts no reference is inserted as itself. `(\d)(?!\1)` needs a
    /// backreference, so the second pattern runs on the backtracking engine
    /// and must expand the template exactly as the linear one does. A result
    /// larger than the request's memory limit stops before it is built.
    // covers: Core\Regex::replace
    #[test]
    fn replace_reads_a_group_reference_as_php_does_on_both_engines() {
        use Piece::{Group, Text};
        let names = [None, None, Some("name")];
        assert_eq!(template("plain", &names), [Text("plain")]);
        assert_eq!(
            template("$1st ${name} $$ $a $123 ${nope} $", &names),
            [
                Group(1),
                Text("st "),
                Group(2),
                Text(" "),
                Text("$"),
                Text(" "),
                Text("$"),
                Text("a "),
                Group(12),
                Text("3 "),
                Group(usize::MAX),
                Text(" "),
                Text("$"),
            ]
        );
        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(1 << 20);
        let mut replace = |subject: &str, pattern: &str, replacement: &str, limit: u64| {
            let args = [
                Value::str(NvsStr::new(subject.as_bytes())),
                Value::str(NvsStr::new(pattern.as_bytes())),
                Value::str(NvsStr::new(replacement.as_bytes())),
                Value::uint(limit),
            ];
            let answer = nvs_runtime::call(nvs_core_regex_replace, &mut ctx, &args)
                .map(|out| {
                    let text = out.as_text().expect("a string").to_owned();
                    #[expect(unsafe_code, reason = "this frame owns the string `replace` returned")]
                    unsafe {
                        out.release();
                    }
                    text
                })
                .ok();
            #[expect(unsafe_code, reason = "this frame owns the three strings it built")]
            unsafe {
                args[0].release();
                args[1].release();
                args[2].release();
            }
            answer
        };
        for pattern in [r"(\d)", r"(\d)(?!\1)"] {
            let got = replace("1 2", pattern, "[$1st] $a $", u64::MAX);
            assert_eq!(got.as_deref(), Some("[1st] $a $ [2st] $a $"), "{pattern}");
            let got = replace("1 2", pattern, "<$0>", 1);
            assert_eq!(got.as_deref(), Some("<1> 2"), "{pattern}");
        }
        let slow = format!("{}b", "a".repeat(40));
        assert_eq!(replace(&slow, r"^(a|a?)+\1$", "x", u64::MAX), None);
        // 1000 copies of a 4 KiB match is about 4 MiB, four times the limit.
        let long = "a".repeat(4096);
        assert_eq!(replace(&long, ".+", &"$0".repeat(1000), u64::MAX), None);
    }

    /// A one-parameter closure calling `invoke`: the arity and tag slots and
    /// the invoke row are all `nvs_runtime::call_closure` reads. The table is
    /// leaked because a descriptor's address is its identity and must outlive
    /// every instance made from it.
    fn closure_of(invoke: nvs_runtime::NvsFn) -> Value {
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives every instance made from it"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(1));
        object.set_field(
            nvs_runtime::CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY)),
        );
        Value::object(object)
    }

    /// Releases the receiver and the `Match` `call_closure` retained for a
    /// one-parameter callee, and answers `answer`.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and one argument, each retained for this \
                  callee to release, and the address of a live `Value` for the result"
    )]
    unsafe fn answered(args: *const Value, out: *mut Value, answer: Value) -> i32 {
        unsafe {
            (*args).release();
            (*args.add(1)).release();
            *out = answer;
        }
        nvs_runtime::OK
    }

    /// `fn(Match $m) => "[" . $m->text() . "]"`.
    #[expect(unsafe_code, reason = "forwarding this callee's own contract")]
    unsafe extern "C" fn bracketed(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let matched = unsafe { *args.add(1) };
        let groups = crate::instance::slot(matched.obj_ptr().expect("a `Match`"), GROUPS_SLOT);
        let whole = crate::arr::borrowed(groups.array_ptr().expect("an array"))
            .get(b"0")
            .and_then(|v| v.as_text().map(str::to_owned))
            .expect("group 0 is the whole match");
        let answer = Value::str(NvsStr::new(format!("[{whole}]").as_bytes()));
        unsafe { answered(args, out, answer) }
    }

    /// `fn(Match $m) => 7` — an answer that is not a string.
    #[expect(unsafe_code, reason = "forwarding this callee's own contract")]
    unsafe extern "C" fn answers_an_int(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        unsafe { answered(args, out, Value::int(7)) }
    }

    /// `fn(Match $m) => Core\Str::repeat("x", 65536)`.
    #[expect(unsafe_code, reason = "forwarding this callee's own contract")]
    unsafe extern "C" fn answers_64_kib(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        let answer = Value::str(NvsStr::new("x".repeat(64 * 1024).as_bytes()));
        unsafe { answered(args, out, answer) }
    }

    /// `Core\Regex::replaceWith` calls a real closure once per replaced match
    /// on both engines, and inserts its answer literally: the `$1` a callback
    /// answers stays two characters. `limit` counts replacements, an answer
    /// that is not a string throws, and a result larger than the request's
    /// memory limit stops before it is built.
    // covers: Core\Regex::replaceWith
    #[test]
    fn replace_with_inserts_the_callback_answer_literally_on_both_engines() {
        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(1 << 20);
        let mut replace_with =
            |subject: &str, pattern: &str, invoke: nvs_runtime::NvsFn, limit: u64| {
                let args = [
                    Value::str(NvsStr::new(subject.as_bytes())),
                    Value::str(NvsStr::new(pattern.as_bytes())),
                    closure_of(invoke),
                    Value::uint(limit),
                ];
                let answer = nvs_runtime::call(nvs_core_regex_replace_with, &mut ctx, &args)
                    .map(|out| {
                        let text = out.as_text().expect("a string").to_owned();
                        #[expect(
                            unsafe_code,
                            reason = "this frame owns the string `replaceWith` returned"
                        )]
                        unsafe {
                            out.release();
                        }
                        text
                    })
                    .ok();
                #[expect(
                    unsafe_code,
                    reason = "this frame owns the two strings and the closure it built"
                )]
                unsafe {
                    args[0].release();
                    args[1].release();
                    args[2].release();
                }
                answer
            };
        // `(\d)\1*` needs a backreference, so it runs on the backtracking engine.
        for pattern in [r"\d+", r"(\d)\1*"] {
            let got = replace_with("a1b22c", pattern, bracketed, u64::MAX);
            assert_eq!(got.as_deref(), Some("a[1]b[22]c"), "{pattern}");
            let got = replace_with("a1b22c", pattern, bracketed, 1);
            assert_eq!(got.as_deref(), Some("a[1]b22c"), "{pattern}");
            let got = replace_with("a1b22c", pattern, bracketed, 0);
            assert_eq!(got.as_deref(), Some("a1b22c"), "{pattern}");
        }
        let got = replace_with("x$1y", r"\$\d", bracketed, u64::MAX);
        assert_eq!(got.as_deref(), Some("x[$1]y"));
        assert_eq!(replace_with("a1", r"\d", answers_an_int, u64::MAX), None);
        // 100 answers of 64 KiB are about 6 MiB, six times the limit.
        let many = "a".repeat(100);
        assert_eq!(replace_with(&many, "a", answers_64_kib, u64::MAX), None);
    }

    /// `Core\Regex::split` reads `limit` by `Core\Str::split`'s three-sign
    /// rule on both engines: positive caps the pieces with the last holding
    /// the rest, negative drops pieces off the end, and `0` is one piece.
    /// `keepEmpty: false` drops empty pieces after the limit, and a
    /// backtracking pattern that runs out of steps throws.
    // covers: Core\Regex::split
    #[test]
    fn split_reads_limit_by_the_three_sign_rule_on_both_engines() {
        let mut ctx = Ctx::buffered();
        let mut split = |subject: &str, pattern: &str, limit: i64, keep_empty: bool| {
            let args = [
                Value::str(NvsStr::new(subject.as_bytes())),
                Value::str(NvsStr::new(pattern.as_bytes())),
                Value::int(limit),
                Value::bool(keep_empty),
            ];
            let answer = nvs_runtime::call(nvs_core_regex_split, &mut ctx, &args)
                .map(|out| {
                    let list = crate::arr::borrowed(out.array_ptr().expect("an array"));
                    let pieces: Vec<String> = (0..list.count())
                        .map(|nth| {
                            list.get(nth.to_string().as_bytes())
                                .and_then(|piece| piece.as_text().map(str::to_owned))
                                .expect("a string piece")
                        })
                        .collect();
                    #[expect(unsafe_code, reason = "this frame owns the array `split` returned")]
                    unsafe {
                        out.release();
                    }
                    pieces
                })
                .ok();
            #[expect(unsafe_code, reason = "this frame owns the two strings it built")]
            unsafe {
                args[0].release();
                args[1].release();
            }
            answer
        };
        // `(\d)\1*` needs a backreference, so it runs on the backtracking engine.
        for pattern in [r"\d+", r"(\d)\1*"] {
            let all = split("a1b22c3", pattern, i64::MAX, true);
            assert_eq!(
                all.as_deref(),
                Some(&["a", "b", "c", ""].map(String::from)[..]),
                "{pattern}"
            );
            let two = split("a1b22c3", pattern, 2, true);
            assert_eq!(
                two.as_deref(),
                Some(&["a", "b22c3"].map(String::from)[..]),
                "{pattern}"
            );
            let dropped = split("a1b22c3", pattern, -2, true);
            assert_eq!(
                dropped.as_deref(),
                Some(&["a", "b"].map(String::from)[..]),
                "{pattern}"
            );
            let one = split("a1b22c3", pattern, 0, true);
            assert_eq!(
                one.as_deref(),
                Some(&["a1b22c3"].map(String::from)[..]),
                "{pattern}"
            );
            let kept = split("1a22", pattern, i64::MAX, false);
            assert_eq!(
                kept.as_deref(),
                Some(&["a"].map(String::from)[..]),
                "{pattern}"
            );
        }
        let slow = format!("{}b", "a".repeat(40));
        assert_eq!(split(&slow, r"^(a|a?)+\1$", i64::MAX, true), None);
    }

    /// `Core\Regex::quote` escapes every character either engine gives a
    /// meaning to, so its result compiles on the linear engine and matches
    /// its own literal, inside a larger text, and nothing else.
    // covers: Core\Regex::quote
    #[test]
    fn quote_escapes_every_meta_character_so_the_result_matches_only_its_literal() {
        let mut ctx = Ctx::buffered();
        let literal = r"#$&()*+-.?[\]^{|}~ a.b";
        let argument = Value::str(NvsStr::new(literal.as_bytes()));
        let quoted =
            nvs_runtime::call(nvs_core_regex_quote, &mut ctx, &[argument]).expect("quotes");
        let pattern = quoted.as_text().expect("a string").to_owned();
        assert_eq!(pattern, r"\#\$\&\(\)\*\+\-\.\?\[\\\]\^\{\|\}\~ a\.b");
        let held = built(&pattern, NO_FLAGS, "quote").expect("compiles");
        let Compiled::Linear(re) = &*held else {
            panic!("a quoted literal needs no backtracking")
        };
        assert!(re.is_match(literal));
        assert!(re.is_match(&format!("before {literal} after")));
        assert!(!re.is_match(r"#$&()*+-.?[\]^{|}~ axb"));
        #[expect(
            unsafe_code,
            reason = "this frame owns the string it built and the one `quote` returned"
        )]
        unsafe {
            argument.release();
            quoted.release();
        }
    }

    /// [`compiled`] at the shipped budget, which is what every case above
    /// wants: none of them is about a configured one, and threading the
    /// constant through each call would say so once per line.
    fn built(pattern: &str, flags: u8, member: &str) -> Result<Rc<Compiled>, Fault> {
        compiled(pattern, flags, member, BACKTRACK_BUDGET)
    }

    /// [`build`] at the same budget, for the cases that ask which tier a
    /// pattern lands on — a question no budget changes.
    fn tiered(pattern: &str, flags: u8, prepared: Option<Tier>) -> Result<Compiled, String> {
        build(pattern, flags, prepared, BACKTRACK_BUDGET)
    }

    /// The budget is `[limits] max_regex_steps` and an unconfigured request
    /// gets [`BACKTRACK_BUDGET`].
    ///
    /// Four readings and then the engine: nothing written is the constant, a
    /// narrower number and a wider one are each taken as written, and `false`
    /// is not a spelling for a tier that may backtrack forever. The last two
    /// assertions are what makes the directive more than a parsed key — the
    /// number reaches the built program, and two budgets on one core are two
    /// entries in [`CACHE`] rather than one request running under another's
    /// ceiling.
    #[test]
    fn the_regex_step_budget_is_a_limits_directive_with_the_constant_as_its_default() {
        assert_eq!(step_budget(&Ctx::buffered()), BACKTRACK_BUDGET);

        let configured = |written: &str| {
            let mut ctx = Ctx::buffered();
            ctx.set_config(crate::tests::granting(written));
            step_budget(&ctx)
        };
        assert_eq!(configured("[limits]\nmax_regex_steps = 64\n"), 64);
        assert_eq!(
            configured("[limits]\nmax_regex_steps = 4000000\n"),
            4_000_000
        );
        assert_eq!(
            configured("[limits]\nmax_regex_steps = false\n"),
            BACKTRACK_BUDGET,
            "`false` is not a spelling for a tier that may backtrack forever"
        );

        let narrow = compiled(CATASTROPHIC, NO_FLAGS, "matches", 64).expect("compiles");
        let Compiled::Backtracking(re) = &*narrow else {
            panic!("a backreference is not a pattern the linear engine expresses")
        };
        assert!(
            re.is_match(&format!("{}b", "a".repeat(40))).is_err(),
            "a narrowed budget is the one the engine runs under"
        );

        let shipped = built(CATASTROPHIC, NO_FLAGS, "matches").expect("compiles");
        assert!(
            !Rc::ptr_eq(&shipped, &narrow),
            "the budget is part of the cache key, or one request runs under another's"
        );
    }
}
