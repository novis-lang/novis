//! `Core\Str` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 1, over `nvs_runtime`'s reference-counted `NvsStr`.
//!
//! Every member here is pure (`rule:core-api/shape-rules` R3) and borrows its subject rather than
//! consuming it — see [`crate`]'s own docs for why that falls out of being a
//! helper rather than being a rule this module states.
//!
//! # `string` is valid UTF-8, so this module never validates
//!
//! `rule:types/bytes` guarantees a
//! `string`'s bytes are valid UTF-8, which is what lets every member below
//! reach for `&str` operations directly. [`text`] is the one place an argument
//! becomes one, and it checks the **tag** and nothing else: the guarantee is a
//! property of that tag, discharged once behind one `unsafe` in
//! `nvs_runtime`'s `string` module (its § *Reading the payload as text*).
//! Re-deriving it here would be an O(n) pass per argument at every call site
//! in this file, over a buffer the runtime already holds the answer for — and
//! a debug build re-validates inside that one reader, so a producer that ever
//! broke the invariant fails the suite rather than being caught a member at a
//! time.
//!
//! The wrong *tag* is still reported, as a contained `FATAL` rather than a
//! `panic!`: it means the checker let a call through it should have refused,
//! and `rule:errors/escalation-ladder`'s ladder wants such a bug reported, not left to take the
//! process down.
//!
//! # A result is written once
//!
//! A member that builds its result **writes it straight into the allocation it
//! is answered from**, through [`built`] and `nvs_runtime`'s `NvsStr::build`.
//! The spelling it replaces — accumulate into a `String`, hand that to
//! [`produced`] — allocates twice and copies every byte twice, because
//! [`produced`] can only copy the text it is given. [`produced`] stays for the
//! members whose result is already sitting in a borrowed slice of the subject,
//! where that copy is the only one there is.
//!
//! `repeat`, `padStart`/`padEnd` and `reverse` know their length exactly and so
//! allocate exactly once, which `tests/allocation_policy.rs` holds them to.
//! `replace` starts the writer at its subject's length and lets it grow by the
//! same doubling the `String` did, so a guess that is wrong costs what it
//! always cost and never the final copy.
//!
//! **Buying an exact length with a second pass is a loss, and was measured as
//! one** — twice, which is why it is written down rather than left to be
//! rediscovered. Counting `replace`'s matches before writing them took
//! `05-string-replace` from 0.92× to 0.74× against PHP; walking a cycle of
//! `padEnd`'s padding to measure what a second walk then wrote took
//! `07-string-normalize` from 0.50× to 0.45×. A read is not free, and at these
//! sizes it is not cheaper than the `memcpy` it saves. Both are arithmetic now,
//! and `docs/perf/userland-gap.md` § E holds the numbers.
//!
//! **`join` is the member this rule does not reach**, and it is unchanged: its
//! length costs a walk of the subject's slots, which is the expensive half of
//! the member, and all three ways round that measured worse than the `String`
//! it builds — a writer at a guessed capacity grows two or three times per
//! call (87.3 ms of work to 90.0), a `Vec` of borrowed pieces spends an
//! allocation per call (105.1), and measuring first walks the slots twice
//! (92.5).
//!
//! Every length that *is* computed goes through `nvs_runtime::affordable`,
//! which is the one seam a per-request ceiling attaches to. That seam refuses
//! only a size past `isize::MAX`, so a member whose capacity is a **count off
//! its call site** — `repeat`, `padStart`, `padEnd` — writes through
//! [`built_fallibly`] and asks the allocator as well, rather than aborting the
//! process on a count the seam allowed and the machine cannot serve.
//!
//! # Granularity is decided elsewhere, and read from one place
//!
//! `length`, `at` and `padStart`/`padEnd`'s `$length` all count in
//! [`crate::granularity::DEFAULT`] — `rule:types/string-is-utf8`'s answer, stated in that
//! module and in that ADR, never restated here. Every other member is
//! granularity-independent.
//!
//! # Case conversion is Unicode's, not PHP's
//!
//! `lower`/`upper`/`upperFirst`/`lowerFirst` use Rust's full Unicode case
//! mappings, so they answer for `straße`/`ÄRGER` what PHP's `mb_strtoupper`
//! answers and *not* what its byte-wise `strtolower`/`ucfirst` do. The spec's
//! **Replaces** column lists both PHP spellings against one Novis member on
//! purpose (R13: "no member takes an encoding argument"), so the byte-wise
//! behaviour has no surviving spelling to be compatible with — a deliberate
//! divergence, and the shape a test case's `--ORACLE-DIVERGES--` section exists
//! to record (`crates/nvs-test/src/lib.rs`).

use std::cmp::Ordering;

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, StrWriter, Tag, Value};
use unicode_normalization::UnicodeNormalization;

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\NormalForm`'s fully-qualified name, written once for
/// [`NORMAL_FORM`], for the registry row that takes one, and for the message
/// quoting it.
pub(crate) const NORMAL_FORM_NAME: &str = r"Core\NormalForm";

/// `Core\Str::length`'s reference card (`rule:core-api/reference-card`) — the first member
/// documented in the registry, and the simplest: one parameter, no options,
/// nothing thrown.
const LENGTH_DOC: MethodDoc = MethodDoc {
    short: "Counts the graphemes in `$s` — user-perceived characters, the unit every \
            `Core\\Str` member counts in — so a combining sequence counts once and this is \
            never a byte count.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to measure.",
        shape: &[],
    }],
    ret: "The grapheme count; `0` for the empty string.",
    errors: &[],
};

/// Spec § 1's `NormalForm` — UAX #15's four normal forms, and the only
/// argument [`nvs_core_str_normalize`] takes beside its subject.
///
/// The order is the spec's own, which is also the order the two axes fall
/// out in: composed before decomposed, canonical before compatibility. The
/// integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`], and they are **ABI** —
/// [`normal_form_of`] indexes this table with the integer compiled code
/// wrote, so reordering the list is a behaviour change rather than a cosmetic
/// one.
pub(crate) const NORMAL_FORM: CoreEnum = CoreEnum {
    name: NORMAL_FORM_NAME,
    cases: &[("Nfc", 0), ("Nfd", 1), ("Nfkc", 2), ("Nfkd", 3)],
    doc: Some(&NORMAL_FORM_DOC),
};

/// [`NORMAL_FORM`]'s reference card — `rule:core-api/reference-card`.
const NORMAL_FORM_DOC: EnumDoc = EnumDoc {
    short: "Which of UAX #15's four normal forms `Core\\Str::normalize` rewrites into — composed \
            or decomposed on one axis, canonical or compatibility on the other.",
    cases: &[
        CaseDoc {
            name: "Nfc",
            desc: "Canonical composition: `é` is one code point, and only canonical equivalents \
                   are unified — the form to store and compare text in.",
        },
        CaseDoc {
            name: "Nfd",
            desc: "Canonical decomposition: `é` is `e` plus a combining acute, and only canonical \
                   equivalents are unified.",
        },
        CaseDoc {
            name: "Nfkc",
            desc: "Compatibility composition: `Nfc`, and a character that merely renders like \
                   another — `ﬁ`, `①` — is unified with it too, which does not round-trip.",
        },
        CaseDoc {
            name: "Nfkd",
            desc: "Compatibility decomposition: `Nfd`, plus the same compatibility unification \
                   `Nfkc` applies.",
        },
    ],
};

/// `Core\Str`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Functions for text: measure it, search it, cut it, change its case and build new \
            strings. Every position and length counts characters the way a person sees them, \
            so an accented letter or an emoji counts as one.",
};

/// `Core\Str`'s registry rows, in the spec's own order.
///
/// Declared beside the implementations rather than in one flat table, so
/// adding a member touches this file and nothing else. [`crate::registry`]'s
/// `CLASSES` lists this const; that list grows one line per *class*, never one
/// per member.
///
/// # `rule:security/unclassified-parameter-refuses-tainted`'s classification, over this class
///
/// [`Qual`]'s own docs hold the rule every class is classified by. Applied
/// here it lands three ways, and the third is the only one that is not
/// mechanical:
///
/// * **Neutral** wherever the answer is a `bool`, a count or an ordering —
///   `length`, `isEmpty`, `contains`, `startsWith`, `endsWith`, `indexOf`,
///   `lastIndexOf`, `countOf`, `compare`, and `codePoints`, whose
///   `array<uint>` carries no byte of the subject that a sink could read.
/// * **Contagious** everywhere else, including the needle of a member that
///   answers a slice: `before`'s separator never appears in the answer, but
///   *which* slice is answered is the needle's doing, and laundering by
///   influence is not something this class is allowed to do.
/// * **Sink** on `format`'s template, which is one of `rule:core-api/shape-rules` R11's four
///   grammars — § 1's corollary makes every one of the four a sink, and the
///   variadic arguments it renders stay data.
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Str",
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "length",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_str_length",
            doc: Some(&LENGTH_DOC),
        },
        CoreMethod {
            name: "at",
            names: &["s", "index"],
            params: &[CoreTy::Text(Qual::Contagious), CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_at",
            doc: Some(&AT_DOC),
        },
        CoreMethod {
            name: "isEmpty",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_str_is_empty",
            doc: Some(&IS_EMPTY_DOC),
        },
        CoreMethod {
            name: "contains",
            names: &["haystack", "needle"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_str_contains",
            doc: Some(&CONTAINS_DOC),
        },
        CoreMethod {
            name: "startsWith",
            names: &["s", "prefix"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_str_starts_with",
            doc: Some(&STARTS_WITH_DOC),
        },
        CoreMethod {
            name: "endsWith",
            names: &["s", "suffix"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_str_ends_with",
            doc: Some(&ENDS_WITH_DOC),
        },
        CoreMethod {
            name: "slice",
            names: &["s", "offset", "length"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Int,
                CoreTy::Nullable(&CoreTy::Int),
            ],
            defaults: &[Const::Null],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_slice",
            doc: Some(&SLICE_DOC),
        },
        CoreMethod {
            name: "indexOf",
            names: &["haystack", "needle"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(INDEX_OF_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "nvs_core_str_index_of",
            doc: Some(&INDEX_OF_DOC),
        },
        CoreMethod {
            name: "lastIndexOf",
            names: &["haystack", "needle"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(LAST_INDEX_OF_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "nvs_core_str_last_index_of",
            doc: Some(&LAST_INDEX_OF_DOC),
        },
        CoreMethod {
            name: "countOf",
            names: &["haystack", "needle"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_str_count_of",
            doc: Some(&COUNT_OF_DOC),
        },
        CoreMethod {
            name: "compare",
            names: &["a", "b"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(COMPARE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_str_compare",
            doc: Some(&COMPARE_DOC),
        },
        CoreMethod {
            name: "before",
            names: &["s", "needle"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(AROUND_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_str_before",
            doc: Some(&BEFORE_DOC),
        },
        CoreMethod {
            name: "after",
            names: &["s", "needle"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(AROUND_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_str_after",
            doc: Some(&AFTER_DOC),
        },
        CoreMethod {
            name: "join",
            names: &["parts", "separator"],
            params: &[CoreTy::Array(&CoreTy::Str), CoreTy::Text(Qual::Contagious)],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_join",
            doc: Some(&JOIN_DOC),
        },
        CoreMethod {
            name: "split",
            names: &["s", "separator"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(SPLIT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_str_split",
            doc: Some(&SPLIT_DOC),
        },
        CoreMethod {
            name: "chunk",
            names: &["s", "size"],
            params: &[CoreTy::Text(Qual::Contagious), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_str_chunk",
            doc: Some(&CHUNK_DOC),
        },
        CoreMethod {
            name: "lines",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_str_lines",
            doc: Some(&LINES_DOC),
        },
        CoreMethod {
            name: "graphemes",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_str_graphemes",
            doc: Some(&GRAPHEMES_DOC),
        },
        CoreMethod {
            name: "codePoints",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Uint),
            symbol: "nvs_core_str_code_points",
            doc: Some(&CODE_POINTS_DOC),
        },
        CoreMethod {
            name: "replace",
            names: &["s", "search", "replacement"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(REPLACE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_replace",
            doc: Some(&REPLACE_DOC),
        },
        CoreMethod {
            name: "replaceAll",
            names: &["s", "pairs"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Array(&CoreTy::Str),
                CoreTy::Options(REPLACE_ALL_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_replace_all",
            doc: Some(&REPLACE_ALL_DOC),
        },
        CoreMethod {
            name: "replaceRange",
            names: &["s", "offset", "length", "replacement"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Int,
                CoreTy::Nullable(&CoreTy::Int),
                CoreTy::Text(Qual::Contagious),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_replace_range",
            doc: Some(&REPLACE_RANGE_DOC),
        },
        CoreMethod {
            name: "padStart",
            names: &["s", "length", "padding"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Uint,
                CoreTy::Text(Qual::Contagious),
            ],
            defaults: &[Const::Str(" ")],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_pad_start",
            doc: Some(&PAD_START_DOC),
        },
        CoreMethod {
            name: "padEnd",
            names: &["s", "length", "padding"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Uint,
                CoreTy::Text(Qual::Contagious),
            ],
            defaults: &[Const::Str(" ")],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_pad_end",
            doc: Some(&PAD_END_DOC),
        },
        CoreMethod {
            name: "trim",
            names: &["s"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(TRIM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_trim",
            doc: Some(&TRIM_DOC),
        },
        CoreMethod {
            name: "trimStart",
            names: &["s"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(TRIM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_trim_start",
            doc: Some(&TRIM_START_DOC),
        },
        CoreMethod {
            name: "trimEnd",
            names: &["s"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(TRIM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_trim_end",
            doc: Some(&TRIM_END_DOC),
        },
        CoreMethod {
            name: "repeat",
            names: &["s", "times"],
            params: &[CoreTy::Text(Qual::Contagious), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_repeat",
            doc: Some(&REPEAT_DOC),
        },
        CoreMethod {
            name: "reverse",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_reverse",
            doc: Some(&REVERSE_DOC),
        },
        CoreMethod {
            name: "wrap",
            names: &["s", "width"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Uint,
                CoreTy::Options(WRAP_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_wrap",
            doc: Some(&WRAP_DOC),
        },
        CoreMethod {
            name: "lower",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_lower",
            doc: Some(&LOWER_DOC),
        },
        CoreMethod {
            name: "upper",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_upper",
            doc: Some(&UPPER_DOC),
        },
        CoreMethod {
            name: "upperFirst",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_upper_first",
            doc: Some(&UPPER_FIRST_DOC),
        },
        CoreMethod {
            name: "lowerFirst",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_lower_first",
            doc: Some(&LOWER_FIRST_DOC),
        },
        CoreMethod {
            name: "fold",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_fold",
            doc: Some(&FOLD_DOC),
        },
        CoreMethod {
            name: "normalize",
            names: &["s", "form"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Enum(NORMAL_FORM_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_normalize",
            doc: Some(&NORMALIZE_DOC),
        },
        CoreMethod {
            name: "fromCodePoint",
            names: &["codePoint"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_from_code_point",
            doc: Some(&FROM_CODE_POINT_DOC),
        },
        CoreMethod {
            name: "fromCodePoints",
            names: &["codePoints"],
            params: &[CoreTy::Array(&CoreTy::Uint)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_from_code_points",
            doc: Some(&FROM_CODE_POINTS_DOC),
        },
        CoreMethod {
            name: "format",
            names: &["template", "arguments"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Variadic(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_str_format",
            doc: Some(&FORMAT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Str::at`'s reference card — `rule:core-api/reference-card`.
const AT_DOC: MethodDoc = MethodDoc {
    short: "Answers the one character at `$index`, as `$s[$i]` and `mb_substr($s, $i, 1)` do — \
            counted in graphemes, the unit every `Core\\Str` member counts in, and never a \
            byte.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to index into.",
            shape: &[],
        },
        ParamDoc {
            name: "index",
            desc: "The position of the character; a negative one counts from the end.",
            shape: &[],
        },
    ],
    ret: "The character, as a one-grapheme string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$index` addresses nothing — it lies at or past the string's length in either \
               direction.",
    }],
};

/// `Core\Str::isEmpty`'s reference card — `rule:core-api/reference-card`.
const IS_EMPTY_DOC: MethodDoc = MethodDoc {
    short: "Answers whether `$s` holds no characters at all — the `$s === \"\"` test.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to test.",
        shape: &[],
    }],
    ret: "`true` for the empty string, `false` for any other.",
    errors: &[],
};

/// `Core\Str::contains`'s reference card — `rule:core-api/reference-card`.
const CONTAINS_DOC: MethodDoc = MethodDoc {
    short: "Answers whether `$needle` occurs anywhere in `$haystack`, as `str_contains` does.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The string searched in.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The string searched for, matched case-sensitively.",
            shape: &[],
        },
    ],
    ret: "`true` when it occurs; an empty needle is contained in every string, the empty one \
          included.",
    errors: &[],
};

/// `Core\Str::startsWith`'s reference card — `rule:core-api/reference-card`.
const STARTS_WITH_DOC: MethodDoc = MethodDoc {
    short: "Answers whether `$s` begins with `$prefix`, as `str_starts_with` does.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to test.",
            shape: &[],
        },
        ParamDoc {
            name: "prefix",
            desc: "The text it must begin with, matched case-sensitively.",
            shape: &[],
        },
    ],
    ret: "`true` when it does; an empty prefix begins every string.",
    errors: &[],
};

/// `Core\Str::endsWith`'s reference card — `rule:core-api/reference-card`.
const ENDS_WITH_DOC: MethodDoc = MethodDoc {
    short: "Answers whether `$s` ends with `$suffix`, as `str_ends_with` does.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to test.",
            shape: &[],
        },
        ParamDoc {
            name: "suffix",
            desc: "The text it must end with, matched case-sensitively.",
            shape: &[],
        },
    ],
    ret: "`true` when it does; an empty suffix ends every string.",
    errors: &[],
};

/// `Core\Str::slice`'s reference card — `rule:core-api/reference-card`.
const SLICE_DOC: MethodDoc = MethodDoc {
    short: "Cuts the part of `$s` that starts at `$offset` and runs for `$length` characters, as \
            `substr` and `mb_substr` do — counted in graphemes, so no slice ever splits a \
            character.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to cut from.",
            shape: &[],
        },
        ParamDoc {
            name: "offset",
            desc: "Where the slice begins; a negative offset counts from the end.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "How many characters to take; a negative one stops that many from the end, and \
                   `null` runs to the end.",
            shape: &[],
        },
    ],
    ret: "The selected text, or `\"\"` for a window that is empty or lies past either end; `$s` \
          is unchanged.",
    errors: &[],
};

/// `Core\Str::indexOf`'s reference card — `rule:core-api/reference-card`.
const INDEX_OF_DOC: MethodDoc = MethodDoc {
    short: "Finds the first occurrence of `$needle` in `$haystack` and answers its position, as \
            `strpos`, `stripos`, `mb_strpos` and `mb_stripos` do.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The string searched in.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The string searched for; an empty one matches where the search starts.",
            shape: &[],
        },
        ParamDoc {
            name: "from",
            desc: "The position the search starts at; a negative one counts from the end, and \
                   the default is `0`.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "Match through Unicode's simple lower-case mapping of each character rather \
                   than exactly; the default is `false`.",
            shape: &[],
        },
    ],
    ret: "The grapheme position of the first occurrence at or after `from`, usable as `slice`'s \
          offset; `null` when the needle does not occur there — never `false`.",
    errors: &[],
};

/// `Core\Str::lastIndexOf`'s reference card — `rule:core-api/reference-card`.
const LAST_INDEX_OF_DOC: MethodDoc = MethodDoc {
    short: "Finds the last occurrence of `$needle` in `$haystack` and answers its position, as \
            `strrpos`, `strripos` and `mb_strrpos` do.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The string searched in.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The string searched for.",
            shape: &[],
        },
        ParamDoc {
            name: "before",
            desc: "Only an occurrence that ends at or before this position counts; a negative \
                   one counts from the end, and the default is the whole string.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "Match through Unicode's simple lower-case mapping of each character rather \
                   than exactly; the default is `false`.",
            shape: &[],
        },
    ],
    ret: "The grapheme position of the last such occurrence — occurrences may overlap, so \
          `lastIndexOf(\"aaa\", \"aa\")` is `1`; `null` when none occurs — never `false`.",
    errors: &[],
};

/// `Core\Str::countOf`'s reference card — `rule:core-api/reference-card`.
const COUNT_OF_DOC: MethodDoc = MethodDoc {
    short: "Counts the non-overlapping occurrences of `$needle` in `$haystack`, as \
            `substr_count` does.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The string searched in.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The string counted, matched case-sensitively; never empty.",
            shape: &[],
        },
    ],
    ret: "The count, `0` when the needle does not occur; `countOf(\"aaa\", \"aa\")` is `1`, \
          because a count partitions the subject where `lastIndexOf` does not.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$needle` is empty.",
    }],
};

/// `Core\Str::compare`'s reference card — `rule:core-api/reference-card`.
const COMPARE_DOC: MethodDoc = MethodDoc {
    short: "Orders `$a` against `$b`, as `strcmp`, `strcasecmp`, `strnatcmp` and \
            `strnatcasecmp` do — the two options pick which of the four.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The first string.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The second string.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "Compare Unicode's simple lower-case mapping of each character instead, so \
                   `ß` and `SS` still differ; the default is `false`.",
            shape: &[],
        },
        ParamDoc {
            name: "natural",
            desc: "Order embedded digit runs by their numeric value, so `\"img2\"` sorts before \
                   `\"img12\"` — a different ordering, not a variant of the default; the \
                   default is `false`.",
            shape: &[],
        },
    ],
    ret: "`-1`, `0` or `1` — the sign only, never a byte difference.",
    errors: &[],
};

/// `Core\Str::before`'s reference card — `rule:core-api/reference-card`.
const BEFORE_DOC: MethodDoc = MethodDoc {
    short: "Returns the part of `$s` before the first `$needle`, without the needle. With \
            `{last: true}`, it cuts at the last `$needle`. Replaces PHP's `strstr($s, $needle, \
            true)`.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to cut.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The separator to cut at. The search is case-sensitive. The needle is not part \
                   of the result.",
            shape: &[],
        },
        ParamDoc {
            name: "last",
            desc: "When `true`, cuts at the last needle. The default is `false`, which cuts at \
                   the first one.",
            shape: &[],
        },
    ],
    ret: "The text before the needle. If the needle is not in `$s`, the result is `null`. If \
          the needle is at the start, the result is an empty string.",
    errors: &[],
};

/// `Core\Str::after`'s reference card — `rule:core-api/reference-card`.
const AFTER_DOC: MethodDoc = MethodDoc {
    short: "Returns the part of `$s` after the first `$needle`, without the needle. With \
            `{last: true}`, it cuts at the last `$needle`. Replaces PHP's `strstr` and `strrchr`.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to cut.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The separator to cut at. The search is case-sensitive. The needle is not part \
                   of the result.",
            shape: &[],
        },
        ParamDoc {
            name: "last",
            desc: "When `true`, cuts at the last needle. The default is `false`, which cuts at \
                   the first one.",
            shape: &[],
        },
    ],
    ret: "The text after the needle. If the needle is not in `$s`, the result is `null`. If \
          the needle is at the end, the result is an empty string.",
    errors: &[],
};

/// `Core\Str::join`'s reference card — `rule:core-api/reference-card`.
const JOIN_DOC: MethodDoc = MethodDoc {
    short: "Concatenates the strings in `$parts` with `$separator` between each neighbouring \
            pair, as `implode` does.",
    params: &[
        ParamDoc {
            name: "parts",
            desc: "The strings to join, in slot order.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "What goes between two neighbouring parts; the default is the empty string.",
            shape: &[],
        },
    ],
    ret: "The joined string; `\"\"` for an empty array.",
    errors: &[],
};

/// `Core\Str::split`'s reference card — `rule:core-api/reference-card`.
const SPLIT_DOC: MethodDoc = MethodDoc {
    short: "Splits `$s` at every occurrence of `$separator`, as `explode` does.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to split.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "The text to split at, matched case-sensitively; never empty.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "At most this many pieces when positive, the last holding the unsplit \
                   remainder; every piece but the last `-limit` of them when negative; the \
                   subject unsplit when `0`. The default is no limit.",
            shape: &[],
        },
    ],
    ret: "The pieces in order, without the separator; `[\"\"]` for the empty string, and `[]` \
          when a negative limit drops every piece.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$separator` is empty.",
    }],
};

/// `Core\Str::chunk`'s reference card — `rule:core-api/reference-card`.
const CHUNK_DOC: MethodDoc = MethodDoc {
    short: "Divides `$s` into pieces of `$size` characters each, as `str_split`, `mb_str_split` \
            and `chunk_split` do — counted in graphemes, so no chunk ever splits a character.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to divide.",
            shape: &[],
        },
        ParamDoc {
            name: "size",
            desc: "How many characters each chunk holds; at least `1`.",
            shape: &[],
        },
    ],
    ret: "The chunks in order, only the last of them possibly shorter; `[]` for the empty \
          string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$size` is `0`.",
    }],
};

/// `Core\Str::lines`'s reference card — `rule:core-api/reference-card`.
const LINES_DOC: MethodDoc = MethodDoc {
    short: "Splits `$s` into its lines, as `explode(PHP_EOL, …)` does — at `\\n`, `\\r\\n` and \
            a lone `\\r` alike, whatever the platform.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to split.",
        shape: &[],
    }],
    ret: "The lines without their terminators; a trailing terminator adds no final empty line, \
          an interior empty line is still a line, and the empty string has no lines at all.",
    errors: &[],
};

/// `Core\Str::graphemes`'s reference card — `rule:core-api/reference-card`.
const GRAPHEMES_DOC: MethodDoc = MethodDoc {
    short: "Splits `$s` into its extended grapheme clusters — the unit `length` counts and `at` \
            indexes — as the split half of intl's `grapheme_*` family does.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to split.",
        shape: &[],
    }],
    ret: "One string per grapheme, in order; `[]` for the empty string.",
    errors: &[],
};

/// `Core\Str::codePoints`'s reference card — `rule:core-api/reference-card`.
const CODE_POINTS_DOC: MethodDoc = MethodDoc {
    short: "Lists the Unicode scalar values of `$s`, as `mb_str_split` plus `mb_ord` does — \
            code points rather than graphemes, so a combining sequence is several.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to read.",
        shape: &[],
    }],
    ret: "One `uint` per code point, in order, each in `0..=0x10FFFF` and never a surrogate; \
          `[]` for the empty string.",
    errors: &[],
};

/// `Core\Str::replace`'s reference card — `rule:core-api/reference-card`.
const REPLACE_DOC: MethodDoc = MethodDoc {
    short: "Replaces every occurrence of `$search` in `$s` with `$replacement`, as `str_replace` \
            and `str_ireplace` do — non-overlapping, left to right, and the replacement is \
            never rescanned.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to rewrite.",
            shape: &[],
        },
        ParamDoc {
            name: "search",
            desc: "The text to look for; an empty one matches nothing.",
            shape: &[],
        },
        ParamDoc {
            name: "replacement",
            desc: "The text put in its place.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "Match through Unicode's simple lower-case mapping of each character rather \
                   than exactly; the default is `false`.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "Replace at most this many occurrences, from the left; `0` replaces nothing, \
                   and the default is every one.",
            shape: &[],
        },
    ],
    ret: "The rewritten string; `$s` unchanged when nothing matched.",
    errors: &[],
};

/// `Core\Str::replaceAll`'s reference card — `rule:core-api/reference-card`.
const REPLACE_ALL_DOC: MethodDoc = MethodDoc {
    short: "Substitutes a whole table at once — `$pairs` keyed needle to replacement — as \
            `strtr` and the array form of `str_replace` do: one pass, the longest matching \
            needle wins at each position, and a replacement is never rescanned.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to rewrite.",
            shape: &[],
        },
        ParamDoc {
            name: "pairs",
            desc: "The substitutions, each key the text to find and its value the text put \
                   there; an empty key is skipped.",
            shape: &[],
        },
        ParamDoc {
            name: "caseInsensitive",
            desc: "Match through Unicode's simple lower-case mapping of each character, where \
                   a tie goes to the pair written first; the default is `false`.",
            shape: &[],
        },
    ],
    ret: "The rewritten string; `$s` unchanged for an empty table or when nothing matched.",
    errors: &[],
};

/// `Core\Str::replaceRange`'s reference card — `rule:core-api/reference-card`.
const REPLACE_RANGE_DOC: MethodDoc = MethodDoc {
    short: "Puts `$replacement` in place of the window `slice` would answer for the same \
            `$offset` and `$length`, as `substr_replace` does.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to rewrite.",
            shape: &[],
        },
        ParamDoc {
            name: "offset",
            desc: "Where the window begins; a negative offset counts from the end.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "How many characters the window covers; a negative one stops that many from \
                   the end, and `null` runs to the end.",
            shape: &[],
        },
        ParamDoc {
            name: "replacement",
            desc: "The text put in the window's place; `\"\"` removes the window.",
            shape: &[],
        },
    ],
    ret: "The rewritten string; an empty window — a `$length` of `0`, or one reaching back past \
          the offset — makes this an insertion at that position.",
    errors: &[],
};

/// `Core\Str::padStart`'s reference card — `rule:core-api/reference-card`.
const PAD_START_DOC: MethodDoc = MethodDoc {
    short: "Prepends copies of `$padding` to `$s` until it is `$length` characters long, as \
            `str_pad` with `STR_PAD_LEFT` does — counted in graphemes.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to pad.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "The length to reach.",
            shape: &[],
        },
        ParamDoc {
            name: "padding",
            desc: "The text repeated to fill the shortfall, cut at its own end when it does not \
                   divide evenly; the default is one space.",
            shape: &[],
        },
    ],
    ret: "The padded string; `$s` unchanged when it is already `$length` long or longer.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$padding` is empty while `$s` is shorter than `$length`, or the result would be \
               larger than this process can hold.",
    }],
};

/// `Core\Str::padEnd`'s reference card — `rule:core-api/reference-card`.
const PAD_END_DOC: MethodDoc = MethodDoc {
    short: "Appends copies of `$padding` to `$s` until it is `$length` characters long, as \
            `str_pad` with `STR_PAD_RIGHT` does — counted in graphemes.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to pad.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "The length to reach.",
            shape: &[],
        },
        ParamDoc {
            name: "padding",
            desc: "The text repeated to fill the shortfall, cut at its own end when it does not \
                   divide evenly; the default is one space.",
            shape: &[],
        },
    ],
    ret: "The padded string; `$s` unchanged when it is already `$length` long or longer.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$padding` is empty while `$s` is shorter than `$length`, or the result would be \
               larger than this process can hold.",
    }],
};

/// `Core\Str::trim`'s reference card — `rule:core-api/reference-card`.
const TRIM_DOC: MethodDoc = MethodDoc {
    short: "Strips every leading and trailing character drawn from `characters` off `$s`, as \
            `trim` does — matched by character, and without `trim`'s `a..z` range syntax.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to trim.",
            shape: &[],
        },
        ParamDoc {
            name: "characters",
            desc: "The set of characters to strip, each one literal; the default is space, tab, \
                   newline, carriage return, NUL and vertical tab.",
            shape: &[],
        },
    ],
    ret: "The trimmed string; `$s` unchanged when neither end holds one of the characters.",
    errors: &[],
};

/// `Core\Str::trimStart`'s reference card — `rule:core-api/reference-card`.
const TRIM_START_DOC: MethodDoc = MethodDoc {
    short: "Strips every leading character drawn from `characters` off `$s`, as `ltrim` does — \
            matched by character, and without `ltrim`'s `a..z` range syntax.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to trim.",
            shape: &[],
        },
        ParamDoc {
            name: "characters",
            desc: "The set of characters to strip, each one literal; the default is space, tab, \
                   newline, carriage return, NUL and vertical tab.",
            shape: &[],
        },
    ],
    ret: "The trimmed string; `$s` unchanged when it does not begin with one of the characters.",
    errors: &[],
};

/// `Core\Str::trimEnd`'s reference card — `rule:core-api/reference-card`.
const TRIM_END_DOC: MethodDoc = MethodDoc {
    short: "Strips every trailing character drawn from `characters` off `$s`, as `rtrim` and \
            `chop` do — matched by character, and without `rtrim`'s `a..z` range syntax.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to trim.",
            shape: &[],
        },
        ParamDoc {
            name: "characters",
            desc: "The set of characters to strip, each one literal; the default is space, tab, \
                   newline, carriage return, NUL and vertical tab.",
            shape: &[],
        },
    ],
    ret: "The trimmed string; `$s` unchanged when it does not end with one of the characters.",
    errors: &[],
};

/// `Core\Str::repeat`'s reference card — `rule:core-api/reference-card`.
const REPEAT_DOC: MethodDoc = MethodDoc {
    short: "Concatenates `$times` copies of `$s`, as `str_repeat` does.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to repeat.",
            shape: &[],
        },
        ParamDoc {
            name: "times",
            desc: "How many copies to write.",
            shape: &[],
        },
    ],
    ret: "The repeated string; `\"\"` when `$times` is `0` or `$s` is empty.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result would be larger than this process can hold.",
    }],
};

/// `Core\Str::reverse`'s reference card — `rule:core-api/reference-card`.
const REVERSE_DOC: MethodDoc = MethodDoc {
    short: "Reverses the order of the characters in `$s`, as `strrev` does — by grapheme rather \
            than by byte, so `\"café\"` becomes `\"éfac\"` and a combining mark stays on its \
            letter.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to reverse.",
        shape: &[],
    }],
    ret: "The reversed string; the same length as `$s`, and `\"\"` for the empty string.",
    errors: &[],
};

/// `Core\Str::wrap`'s reference card — `rule:core-api/reference-card`.
const WRAP_DOC: MethodDoc = MethodDoc {
    short: "Breaks `$s` into lines no longer than `$width` characters by inserting `breakWith` \
            at spaces, as `wordwrap` does — counted in graphemes.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The text to wrap.",
            shape: &[],
        },
        ParamDoc {
            name: "width",
            desc: "The longest line allowed, in characters; `0` breaks at every space.",
            shape: &[],
        },
        ParamDoc {
            name: "breakWith",
            desc: "The text inserted at each break, and a line reset wherever it already occurs \
                   in `$s`; the default is `\"\\n\"`, without `wordwrap`'s leading space.",
            shape: &[],
        },
        ParamDoc {
            name: "cutLongWords",
            desc: "Break a word longer than `$width` in the middle rather than letting it \
                   overrun the line; the default is `false`.",
            shape: &[],
        },
    ],
    ret: "The wrapped text.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`breakWith` is empty, or `$width` is `0` with `cutLongWords` set.",
    }],
};

/// `Core\Str::lower`'s reference card — `rule:core-api/reference-card`.
const LOWER_DOC: MethodDoc = MethodDoc {
    short: "Lower-cases `$s` through Unicode's full lowercase mapping, as `mb_strtolower` does; \
            there is no byte-wise `strtolower` twin.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to lower-case.",
        shape: &[],
    }],
    ret: "The lower-cased string, possibly a different length from `$s`.",
    errors: &[],
};

/// `Core\Str::upper`'s reference card — `rule:core-api/reference-card`.
const UPPER_DOC: MethodDoc = MethodDoc {
    short: "Upper-cases `$s` through Unicode's full uppercase mapping, as `mb_strtoupper` does, \
            so `straße` becomes `STRASSE`; there is no byte-wise `strtoupper` twin.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to upper-case.",
        shape: &[],
    }],
    ret: "The upper-cased string, possibly a different length from `$s`.",
    errors: &[],
};

/// `Core\Str::upperFirst`'s reference card — `rule:core-api/reference-card`.
const UPPER_FIRST_DOC: MethodDoc = MethodDoc {
    short: "Upper-cases the first character of `$s` and copies the rest through, as `ucfirst` \
            does — with Unicode's mapping, so a leading `ß` expands to `SS`.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string whose first character changes.",
        shape: &[],
    }],
    ret: "The string with its first character upper-cased; `\"\"` for the empty string.",
    errors: &[],
};

/// `Core\Str::lowerFirst`'s reference card — `rule:core-api/reference-card`.
const LOWER_FIRST_DOC: MethodDoc = MethodDoc {
    short: "Lower-cases the first character of `$s` and copies the rest through, as `lcfirst` \
            does — with Unicode's mapping rather than a byte's.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string whose first character changes.",
        shape: &[],
    }],
    ret: "The string with its first character lower-cased; `\"\"` for the empty string.",
    errors: &[],
};

/// `Core\Str::fold`'s reference card — `rule:core-api/reference-card`.
const FOLD_DOC: MethodDoc = MethodDoc {
    short: "Case-folds `$s` through Unicode's default full folding, as \
            `mb_convert_case($s, MB_CASE_FOLD)` does — a comparison key rather than text to \
            show, so `ß` becomes `ss` and `ﬁ` becomes `fi`.",
    params: &[ParamDoc {
        name: "s",
        desc: "The string to fold.",
        shape: &[],
    }],
    ret: "The folded string; two strings that differ only by case fold to the same one, which \
          `compare`'s `{caseInsensitive: true}` cannot promise.",
    errors: &[],
};

/// `Core\Str::normalize`'s reference card — `rule:core-api/reference-card`.
const NORMALIZE_DOC: MethodDoc = MethodDoc {
    short: "Rewrites `$s` into the UAX #15 normal form `$form`, as `Normalizer::normalize` does, \
            so two encodings of the same text compare equal.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The string to normalize.",
            shape: &[],
        },
        ParamDoc {
            name: "form",
            desc: "Which of the four forms: `Nfc` or `Nfd` for a canonical one, `Nfkc` or `Nfkd` \
                   for a compatibility one.",
            shape: &[],
        },
    ],
    ret: "The normalized string; an ASCII subject comes back unchanged under every form.",
    errors: &[],
};

/// `Core\Str::fromCodePoint`'s reference card — `rule:core-api/reference-card`.
const FROM_CODE_POINT_DOC: MethodDoc = MethodDoc {
    short: "Builds the one-character string for the Unicode scalar value `$codePoint`, as \
            `mb_chr` does; `chr`'s byte lives on `Core\\Bytes` instead.",
    params: &[ParamDoc {
        name: "codePoint",
        desc: "The scalar value, at most `0x10FFFF` and never a surrogate.",
        shape: &[],
    }],
    ret: "A string of that one code point.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$codePoint` is not a Unicode scalar value — above `0x10FFFF`, in the surrogate \
               range `0xD800..=0xDFFF`, or negative.",
    }],
};

/// `Core\Str::fromCodePoints`'s reference card — `rule:core-api/reference-card`.
const FROM_CODE_POINTS_DOC: MethodDoc = MethodDoc {
    short: "Builds the string whose code points are `$codePoints`, in order — `codePoints`' \
            inverse, as `implode(array_map(\"mb_chr\", …))` does.",
    params: &[ParamDoc {
        name: "codePoints",
        desc: "The scalar values, in order; each at most `0x10FFFF` and never a surrogate.",
        shape: &[],
    }],
    ret: "The string; `\"\"` for an empty array, and nothing at all when an element is refused.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "An element is not a Unicode scalar value — above `0x10FFFF`, in the surrogate \
               range `0xD800..=0xDFFF`, or negative.",
    }],
};

/// `Core\Str::format`'s reference card — `rule:core-api/reference-card`.
const FORMAT_DOC: MethodDoc = MethodDoc {
    short: "Fills the `printf` template `$template` from `$arguments`, as `sprintf` and \
            `vsprintf` do — the closed conversion list `%s %d %u %f %e %g %x %X %o %b %%` with \
            `printf`'s flags, width, precision and `%1$s` positions, and none of its locale \
            reading.",
    params: &[
        ParamDoc {
            name: "template",
            desc: "The `printf` template — a taint sink, so it must be trusted text; a literal \
                   one has its placeholders checked at compile time.",
            shape: &[],
        },
        ParamDoc {
            name: "arguments",
            desc: "The values the placeholders consume, in order or by `%1$s` position; every \
                   one must be read by at least one placeholder.",
            shape: &[],
        },
    ],
    ret: "The filled-in text; a width or precision counts graphemes, and `%f` always writes `.` \
          as the decimal separator.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The template holds a malformed or unknown placeholder, names more arguments than \
               were passed, leaves an argument no placeholder reads, or reaches a value with no \
               reading for its conversion — an array for `%d`, or a `decimal` that is not whole \
               for an integer conversion.",
    }],
};

/// `Core\Str::split`'s `{limit?: int}` — [`nvs_core_str_split`]'s own docs own
/// what each sign of it means and why the default is `int`'s maximum.
const SPLIT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "limit",
    ty: CoreTy::Int,
    default: Const::Int(i64::MAX),
}];

/// `Core\Str::trim`/`trimStart`/`trimEnd`'s `{characters?: string}`, shared by
/// all three — one bag, so the three members cannot drift apart on either the
/// option's name or its default.
///
/// The default is PHP's own `trim` set: space, tab, newline, carriage return,
/// NUL and vertical tab. [`trimmed`] owns the two places the match itself
/// diverges from PHP's.
const TRIM_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "characters",
    ty: CoreTy::Text(Qual::Contagious),
    default: Const::Str(" \t\n\r\0\u{0b}"),
}];

/// `Core\Str::replace`'s `{caseInsensitive?: bool, limit?: uint}`.
///
/// [`nvs_core_str_replace`]'s own docs own both defaults — in particular why
/// "every occurrence" is spelled as `uint`'s maximum rather than as a sentinel
/// `0` or a `null` the registry cannot state yet.
const REPLACE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "caseInsensitive",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "limit",
        ty: CoreTy::Uint,
        default: Const::Uint(u64::MAX),
    },
];

/// `Core\Str::replaceAll`'s `{caseInsensitive?: bool}`.
///
/// Deliberately **not** [`REPLACE_OPTIONS`]: a `limit` over a whole
/// substitution table would have to say *which* pair it counts, and the spec's
/// row at § 1 does not offer one. `replace` keeps its `limit` because there is
/// exactly one needle to count.
const REPLACE_ALL_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "caseInsensitive",
    ty: CoreTy::Bool,
    default: Const::Bool(false),
}];

/// `Core\Str::indexOf`'s `{from?: int, caseInsensitive?: bool}`.
///
/// `from` is a **position**, so it obeys `rule:core-api/shape-rules` R8's sign rule and reads
/// through [`crate::granularity::Unit::byte_of_signed_index`] — the same option,
/// spelled the same way and meaning the same thing, as `Core\Regex::match`'s.
/// Its default is the start of the subject, which is a search of the whole of
/// it.
const INDEX_OF_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "from",
        ty: CoreTy::Int,
        default: Const::Int(0),
    },
    CoreOption {
        name: "caseInsensitive",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Str::lastIndexOf`'s `{before?: int, caseInsensitive?: bool}` — the
/// mirror of [`INDEX_OF_OPTIONS`], whose bound runs the other way.
///
/// The default is `int`'s maximum rather than a sentinel, which
/// [`crate::granularity::Unit::byte_of_index`] saturates to the subject's whole
/// length: "no bound at all", spelled the way `Core\Str::replace`'s `limit`
/// already spells it, because no string this process can hold is that long.
const LAST_INDEX_OF_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "before",
        ty: CoreTy::Int,
        default: Const::Int(i64::MAX),
    },
    CoreOption {
        name: "caseInsensitive",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Str::compare`'s `{caseInsensitive?: bool, natural?: bool}`.
///
/// The two are independent, and all four combinations name one of PHP's four
/// comparison functions: neither is `strcmp`, `caseInsensitive` alone is
/// `strcasecmp`, `natural` alone is `strnatcmp`, and both together are
/// `strnatcasecmp`. `natural` selects a **different ordering** rather than a
/// variant of the same one, which is why the spec's own prose under § 1's
/// *Comparison* table calls it out; [`nvs_core_str_compare`] owns what that
/// ordering is.
const COMPARE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "caseInsensitive",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "natural",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Str::before`/`after`'s `{last?: bool}`, shared by both — one bag, so
/// the two members cannot drift apart on which occurrence they cut at.
const AROUND_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "last",
    ty: CoreTy::Bool,
    default: Const::Bool(false),
}];

/// `Core\Str::wrap`'s `{breakWith?: string, cutLongWords?: bool}` — PHP's
/// `wordwrap` third and fourth arguments, named rather than positional.
///
/// `breakWith` defaults to `"\n"` and **not** to PHP's `" \n"`: that default of
/// PHP's is a two-character break inserted verbatim, which leaves a trailing
/// space on every wrapped line. [`nvs_core_str_wrap`] owns the rest.
const WRAP_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "breakWith",
        ty: CoreTy::Text(Qual::Contagious),
        default: Const::Str("\n"),
    },
    CoreOption {
        name: "cutLongWords",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain.
///
/// [`crate::symbols`] chains one of these per domain, so a new class adds an
/// arm here rather than to a single workspace-wide match.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_str_length" => (nvs_core_str_length as *const ()).cast(),
        "nvs_core_str_at" => (nvs_core_str_at as *const ()).cast(),
        "nvs_core_str_is_empty" => (nvs_core_str_is_empty as *const ()).cast(),
        "nvs_core_str_contains" => (nvs_core_str_contains as *const ()).cast(),
        "nvs_core_str_starts_with" => (nvs_core_str_starts_with as *const ()).cast(),
        "nvs_core_str_ends_with" => (nvs_core_str_ends_with as *const ()).cast(),
        "nvs_core_str_slice" => (nvs_core_str_slice as *const ()).cast(),
        "nvs_core_str_index_of" => (nvs_core_str_index_of as *const ()).cast(),
        "nvs_core_str_last_index_of" => (nvs_core_str_last_index_of as *const ()).cast(),
        "nvs_core_str_count_of" => (nvs_core_str_count_of as *const ()).cast(),
        "nvs_core_str_compare" => (nvs_core_str_compare as *const ()).cast(),
        "nvs_core_str_before" => (nvs_core_str_before as *const ()).cast(),
        "nvs_core_str_after" => (nvs_core_str_after as *const ()).cast(),
        "nvs_core_str_reverse" => (nvs_core_str_reverse as *const ()).cast(),
        "nvs_core_str_wrap" => (nvs_core_str_wrap as *const ()).cast(),
        "nvs_core_str_join" => (nvs_core_str_join as *const ()).cast(),
        "nvs_core_str_split" => (nvs_core_str_split as *const ()).cast(),
        "nvs_core_str_chunk" => (nvs_core_str_chunk as *const ()).cast(),
        "nvs_core_str_lines" => (nvs_core_str_lines as *const ()).cast(),
        "nvs_core_str_graphemes" => (nvs_core_str_graphemes as *const ()).cast(),
        "nvs_core_str_code_points" => (nvs_core_str_code_points as *const ()).cast(),
        "nvs_core_str_from_code_point" => (nvs_core_str_from_code_point as *const ()).cast(),
        "nvs_core_str_from_code_points" => (nvs_core_str_from_code_points as *const ()).cast(),
        "nvs_core_str_replace" => (nvs_core_str_replace as *const ()).cast(),
        "nvs_core_str_replace_all" => (nvs_core_str_replace_all as *const ()).cast(),
        "nvs_core_str_replace_range" => (nvs_core_str_replace_range as *const ()).cast(),
        "nvs_core_str_trim" => (nvs_core_str_trim as *const ()).cast(),
        "nvs_core_str_trim_start" => (nvs_core_str_trim_start as *const ()).cast(),
        "nvs_core_str_trim_end" => (nvs_core_str_trim_end as *const ()).cast(),
        "nvs_core_str_pad_start" => (nvs_core_str_pad_start as *const ()).cast(),
        "nvs_core_str_pad_end" => (nvs_core_str_pad_end as *const ()).cast(),
        "nvs_core_str_repeat" => (nvs_core_str_repeat as *const ()).cast(),
        "nvs_core_str_lower" => (nvs_core_str_lower as *const ()).cast(),
        "nvs_core_str_upper" => (nvs_core_str_upper as *const ()).cast(),
        "nvs_core_str_upper_first" => (nvs_core_str_upper_first as *const ()).cast(),
        "nvs_core_str_lower_first" => (nvs_core_str_lower_first as *const ()).cast(),
        "nvs_core_str_fold" => (nvs_core_str_fold as *const ()).cast(),
        "nvs_core_str_normalize" => (nvs_core_str_normalize as *const ()).cast(),
        "nvs_core_str_format" => (nvs_core_str_format as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument's text.
///
/// One failure, `FATAL` rather than `THROWN`: a non-string argument means the
/// checker let a call through it should have refused, which is not something a
/// program can catch its way out of.
///
/// There is no *encoding* failure to report, and this is where that shows in
/// the cost. The tag [`Value::as_text`] checks is itself `rule:types/bytes`'s UTF-8
/// guarantee — `nvs_runtime`'s `string` module owns the argument in its
/// § *Reading the payload as text* — so re-deriving it here would be an O(n)
/// pass per argument, at every call site in this file, over a buffer the
/// runtime already knows the answer for.
fn text<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Str::{member} expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument, unchanged.
fn unsigned(value: &Value, member: &str, position: &str) -> Result<u64, Fault> {
    value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Str::{member} expected {:?} for {position}, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })
}

/// One `int` argument.
fn integer(value: &Value, member: &str, position: &str) -> Result<i64, Fault> {
    value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Str::{member} expected {:?} for {position}, got tag {}",
            Tag::Int,
            value.tag_byte()
        ))
    })
}

/// One `bool` argument.
fn boolean(value: &Value, member: &str, position: &str) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Str::{member} expected {:?} for {position}, got tag {}",
            Tag::Bool,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument, as a `usize`.
fn count(value: &Value, member: &str, position: &str) -> Result<usize, Fault> {
    let raw = unsigned(value, member, position)?;
    usize::try_from(raw).map_err(|_| {
        Fault::thrown(format!(
            "Core\\Str::{member}: {position} is {raw}, which is larger than any string this \
             process could hold"
        ))
    })
}

/// A freshly built `string` result, copied out of the borrowed text it is
/// already sitting in.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(NvsStr::new(text.as_bytes())))
}

/// A freshly built `string` result, written straight into the allocation it is
/// answered from — `capacity` being what the member can say up front, exactly
/// where it knows and as a starting point where it guesses.
///
/// See this module's § *A result is written once*.
fn built(capacity: usize, write: impl FnOnce(&mut StrWriter<'_>)) -> HelperResult {
    Ok(Value::str(NvsStr::build(capacity, write)))
}

/// [`built`], for a member whose capacity is a **count off its own call site**
/// rather than a bound on a subject already in memory.
///
/// Two checks, and they answer different questions, exactly as
/// `nvs_core_random_bytes` runs them: `nvs_runtime::affordable` is the policy
/// seam every count-shaped argument passes through and refuses only a size
/// past `isize::MAX`, so every count below it that the machine cannot serve
/// reaches the allocator — where an abort takes the process and every
/// in-flight request with it, for a refusal a caller may well want to handle.
/// [`NvsStr::try_build`] asks instead.
///
/// `capacity` must be the result's *exact* length; a writer that exceeds it
/// grows through the aborting path, which is why [`built`] stays the spelling
/// for a member whose capacity is only a guess or an upper bound.
fn built_fallibly(
    capacity: usize,
    member: &str,
    write: impl FnOnce(&mut StrWriter<'_>),
) -> HelperResult {
    NvsStr::try_build(capacity, write)
        .map(Value::str)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}: the result is larger than any string this process could hold"
            ))
        })
}

/// The values an `array` argument holds, in slot order, each **borrowed** from
/// the array rather than retained — which is what `nvs_array_value_at` writes,
/// and why nothing here releases one.
///
/// The cursor exists as an iterator so [`nvs_core_str_join`]'s body is the join
/// and not the walk; the `unsafe` the walk needs is stated once, here. It is
/// `pub(crate)` for the same reason — [`crate::cli`]'s live region walks a
/// frame's rows with it rather than restating that obligation in a second
/// place.
pub(crate) struct Elements {
    /// The array being walked. Live for this iterator's whole life, which is
    /// [`Elements::of`]'s obligation on its caller.
    array: *const nvs_runtime::ArrayHeader,
    /// The slot the next step starts looking from.
    from: usize,
}

impl Elements {
    /// The elements of `array`, which must stay live and unwritten for as long
    /// as the iterator does. A helper's own `Tag::Array` argument satisfies
    /// both: it owns a reference for the length of the call, and no member
    /// reading one also writes it.
    pub(crate) fn of(array: *const nvs_runtime::ArrayHeader) -> Self {
        Self { array, from: 0 }
    }
}

impl Iterator for Elements {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        #[expect(
            unsafe_code,
            reason = "`Elements::of`'s caller states the array is live for \
                      this iterator's life, and `from` only ever advances \
                      past a slot this same cursor reported"
        )]
        let (slot, value) = unsafe {
            let slot =
                usize::try_from(nvs_runtime::nvs_array_next_slot(self.array, self.from)).ok()?;
            let mut value = Value::null();
            nvs_runtime::nvs_array_value_at(self.array, slot, &raw mut value);
            (slot, value)
        };
        self.from = slot + 1;
        Some(value)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::length(string $s): uint` — how many characters the string
    /// holds, replacing PHP's `strlen` *and* `mb_strlen` at once.
    ///
    /// The unit is [`crate::granularity::DEFAULT`], which is `rule:types/string-is-utf8`'s
    /// decision and is stated there and nowhere else. What that means for a
    /// PHP program being ported is the divergence row in that ADR's
    /// *Consequences*: `strlen("café")` is 5 and this is 4.
    ///
    /// **One** pass over the string's bytes, which is the floor the unit sets
    /// rather than a number worth improving: reading the argument is a tag
    /// check ([`Value::as_text`]), and `granularity`'s fast-path test fuses
    /// what used to be an `is_ascii` scan and a separate search for `\r`. An
    /// ASCII subject's count is then `len`, in O(1).
    ///
    /// **And one pass per string, not per call**: the answer is kept in the
    /// string's own header, so a `length` inside a loop over the same subject
    /// pays the scan once — `rule:types/bytes`'s *Consequences* asked for exactly that,
    /// and `granularity::Unit::length_of` is the seam it arrives through.
    fn nvs_core_str_length(_ctx, args: [1]) {
        let subject = text(&args[0], "length", "the subject")?;
        // `try_from` rather than `as`: `usize` is no wider than `u64` on any
        // target `deny.toml` builds for, so this cannot lose a digit, and
        // spelling it this way keeps the cast lints this crate denies from
        // needing a silence. That is also why the `Err` arm below is
        // unreachable from source, and for a reason no diagnostic states: the
        // conversion is total on every target this builds for, so the arm is
        // the price of not writing `as` rather than a boundary a program
        // reaches by holding a long enough string.
        let length = u64::try_from(crate::granularity::DEFAULT.length_of(&args[0], subject))
            .map_err(|_| Fault::fatal("Core\\Str::length counted past `uint`"))?;
        Ok(Value::uint(length))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::at(string $s, int $index): string` — the one character at
    /// `$index`, replacing PHP's `$s[$i]` and `mb_substr($s, $i, 1)`.
    ///
    /// Two things separate it from PHP's `$s[$i]`, and both follow from rules
    /// already taken rather than being decided here:
    ///
    /// * **The unit is a character, not a byte** — [`crate::granularity`],
    ///   whose own docs record why a byte-indexed `at` cannot exist at all
    ///   under `rule:types/bytes`'s UTF-8 invariant.
    /// * **An index that addresses nothing throws**, rather than PHP's warning
    ///   plus `""`. The declared return type is `string`, not `?string`, and
    ///   `rule:core-api/shape-rules` R4/R5
    ///   make that the difference between the two: absence would have to be
    ///   spelled in the type.
    ///
    /// A negative index counts from the end, which is R8's range rule applied
    /// to a range of one.
    fn nvs_core_str_at(_ctx, args: [2]) {
        let subject = text(&args[0], "at", "the subject")?;
        let index = integer(&args[1], "at", "the index")?;
        let unit = crate::granularity::DEFAULT;
        let found = unit.at(subject, index).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Str::at(): index {index} is outside a string of {} characters",
                unit.length(subject)
            ))
        })?;
        produced(found)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::isEmpty(string $s): bool` — whether the string holds no
    /// characters at all, replacing PHP's `$s === ""` idiom.
    fn nvs_core_str_is_empty(_ctx, args: [1]) {
        Ok(Value::bool(text(&args[0], "isEmpty", "the subject")?.is_empty()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::contains(string $haystack, string $needle): bool` —
    /// replacing PHP's `str_contains` and `strstr` used as a predicate.
    ///
    /// An empty needle is contained in every string, including the empty one,
    /// which is both Rust's and PHP 8's answer.
    fn nvs_core_str_contains(_ctx, args: [2]) {
        let haystack = text(&args[0], "contains", "the subject")?;
        let needle = text(&args[1], "contains", "the needle")?;
        Ok(Value::bool(haystack.contains(needle)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::startsWith(string $s, string $prefix): bool` — replacing
    /// PHP's `str_starts_with`.
    fn nvs_core_str_starts_with(_ctx, args: [2]) {
        let subject = text(&args[0], "startsWith", "the subject")?;
        let prefix = text(&args[1], "startsWith", "the prefix")?;
        Ok(Value::bool(subject.starts_with(prefix)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::endsWith(string $s, string $suffix): bool` — replacing PHP's
    /// `str_ends_with`.
    fn nvs_core_str_ends_with(_ctx, args: [2]) {
        let subject = text(&args[0], "endsWith", "the subject")?;
        let suffix = text(&args[1], "endsWith", "the suffix")?;
        Ok(Value::bool(subject.ends_with(suffix)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::join(array<string> $parts, string $separator = ""): string`
    /// — replacing PHP's `implode`/`join`.
    ///
    /// The **first `Core` member with an optional parameter**: a call that
    /// omits the separator has the empty string materialized at the call site
    /// from `nvs_stdlib::registry::CoreMethod::defaults`, so this body always
    /// receives two arguments and knows nothing about defaults at all — see
    /// `nvs_types::defaults` for why the caller does that work.
    ///
    /// PHP's legacy argument-swapped `implode($glue, $array)` form has no
    /// counterpart: `rule:core-api/shape-rules` R1 puts the subject first, and R20 leaves no room
    /// for a second spelling of one operation.
    fn nvs_core_str_join(_ctx, args: [2]) {
        // Unreachable from source: parameter 0 is `array<string>` in `CLASS`
        // above, so a non-container subject is `E0401: expected
        // array<string>, found mixed` at the checker. Same judgement as
        // `crate::arr`'s `nvs_core_arr_count`, which states it in full.
        let parts = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::join expected {:?} for the subject, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let separator = text(&args[1], "join", "the separator")?;

        // The one member § E named that is **left as it was**, because all
        // three alternatives measured slower than accumulating into a `String`
        // and copying it: a writer at a guessed capacity grows two or three
        // times per call, a piece list spends a `Vec` per call, and measuring
        // the length first walks the slots twice. This module's § *A result is
        // written once* records the numbers. What makes `join` different from
        // the members above it is that its length costs a slot walk to learn,
        // and that walk is the expensive part of the member.
        let mut out = String::new();
        for (at, value) in Elements::of(parts).enumerate() {
            if at > 0 {
                out.push_str(separator);
            }
            out.push_str(text(&value, "join", "an element")?);
        }
        produced(&out)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::split(string $s, string $separator, {limit?: int}): array<string>`
    /// — replacing PHP's `explode`, whose third argument becomes the one
    /// option here.
    ///
    /// `limit` keeps every one of `explode`'s three behaviours, because they
    /// are three different questions and PHP answers all of them through one
    /// integer:
    ///
    /// * **positive** — at most that many pieces, the last one holding the
    ///   whole unsplit remainder.
    /// * **negative** — every piece except the last `-limit` of them, which is
    ///   an empty array when there are not that many.
    /// * **zero** — one piece, i.e. the subject unsplit. PHP's own reading,
    ///   and kept rather than "split nothing" so a computed limit behaves the
    ///   same here as it does there.
    ///
    /// It defaults to `int`'s maximum, which is "no limit" — a subject that
    /// fits in memory can never produce that many pieces. Same decision, and
    /// the same reasons, as [`nvs_core_str_replace`]'s own `limit`.
    ///
    /// **An empty separator throws**, as PHP's `explode` does: there is no
    /// sensible piece boundary, and returning the subject unsplit would hide
    /// a computed separator that came out empty by mistake.
    fn nvs_core_str_split(_ctx, args: [3]) {
        let subject = text(&args[0], "split", "the subject")?;
        let separator = text(&args[1], "split", "the separator")?;
        let limit = integer(&args[2], "split", "the `limit` option")?;
        if separator.is_empty() {
            return Err(Fault::thrown(
                "Core\\Str::split(): the separator must not be empty",
            ));
        }

        let mut out = NvsArray::new();
        if limit >= 0 {
            // A limit of `0` means one piece, not none — see the docs above.
            let pieces = usize::try_from(limit).unwrap_or(usize::MAX).max(1);
            for piece in subject.splitn(pieces, separator) {
                out.append(Value::str(NvsStr::new(piece.as_bytes())));
            }
        } else {
            let dropped = usize::try_from(limit.unsigned_abs()).unwrap_or(usize::MAX);
            let all: Vec<&str> = subject.split(separator).collect();
            for piece in all.get(..all.len().saturating_sub(dropped)).unwrap_or(&[]) {
                out.append(Value::str(NvsStr::new(piece.as_bytes())));
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::chunk(string $s, uint $size): array<string>` — replacing
    /// PHP's `str_split`, `mb_str_split` and `chunk_split`.
    ///
    /// One member for all three because they differ only in what they count
    /// and what they do with the pieces: `str_split` counts bytes,
    /// `mb_str_split` counts code points, and `chunk_split` joins the same
    /// pieces back with a separator, which is `Str::join` on this member's
    /// answer. The unit here is [`crate::granularity::DEFAULT`] like every
    /// other length in this class, so the last chunk is the only short one and
    /// **no chunk ever splits a grapheme cluster** — the failure a byte-counted
    /// `str_split` produces, and the reason it cannot be used on text at all.
    ///
    /// **A size of `0` throws**, as PHP 8's `str_split` does: there is no chunk
    /// count that answers it, and any other reading — the subject unsplit, an
    /// empty array — hides a computed size that came out zero by mistake.
    ///
    /// An empty subject is **no chunks**, not one empty one. That parts company
    /// with [`nvs_core_str_split`], which answers `[""]`, and deliberately: a
    /// separator-split asks "what lies between the separators" and there is one
    /// such region, while this asks "how does the text divide" and empty text
    /// divides into nothing. PHP 8.2 made `str_split("")` the same `[]`.
    fn nvs_core_str_chunk(_ctx, args: [2]) {
        let subject = text(&args[0], "chunk", "the subject")?;
        let size = count(&args[1], "chunk", "the chunk size")?;
        if size == 0 {
            return Err(Fault::thrown(
                "Core\\Str::chunk(): the chunk size must be at least 1",
            ));
        }

        let mut out = NvsArray::new();
        let mut start = 0usize;
        let mut at = 0usize;
        let mut held = 0usize;
        for piece in crate::granularity::DEFAULT.pieces(subject) {
            at += piece.len();
            held += 1;
            if held == size {
                out.append(Value::str(NvsStr::new(&subject.as_bytes()[start..at])));
                start = at;
                held = 0;
            }
        }
        if start < subject.len() {
            out.append(Value::str(NvsStr::new(&subject.as_bytes()[start..])));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::lines(string $s): array<string>` — replacing
    /// `explode(PHP_EOL, …)` and the splitting half of `file()`.
    ///
    /// See [`line_pieces`] for the three terminators it accepts and why the
    /// platform's own line ending is never consulted.
    fn nvs_core_str_lines(_ctx, args: [1]) {
        let subject = text(&args[0], "lines", "the subject")?;
        let mut out = NvsArray::new();
        for line in line_pieces(subject.as_bytes()) {
            out.append(Value::str(NvsStr::new(line)));
        }
        Ok(Value::array(out))
    }
}

/// `subject`'s lines, without their terminators.
///
/// **`\n`, `\r\n` and a lone `\r` all end a line**, on every platform, and
/// nothing here reads the host's own line ending — which is why Novis has no
/// `PHP_EOL` equivalent to pass in. Text arriving over a request, out of a
/// file written elsewhere, or off a Windows editor is the ordinary case, and a
/// member that split on one of the three would answer with a `\r` still glued
/// to every line of the other two.
///
/// **A trailing terminator does not produce a final empty line**, so
/// `lines("a\n")` is one line rather than two and a file that ends the way a
/// text file is supposed to end does not need its last element discarded. An
/// interior empty line is still a line: `lines("a\n\nb")` is three. An empty
/// subject has no lines at all.
///
/// No grapheme cluster is split by any of this: `\r\n` is one cluster under
/// UAX #29's GB3 and is consumed whole, and a lone `\r` is a cluster of its
/// own.
///
/// **It divides octets, not validated text**, because [`crate::io`]'s `lines`
/// is the other caller and a file `Core\IO::read` hands back was never asked
/// to be UTF-8 — the class's own module doc owns that. All three terminators
/// are ASCII and no multi-byte sequence contains an ASCII byte, so the pieces
/// of valid text are the same either way and this is the one place that
/// decides what a line is.
pub(crate) fn line_pieces(subject: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let (mut start, mut at) = (0usize, 0usize);
    while at < subject.len() {
        match subject[at] {
            b'\n' => {
                out.push(&subject[start..at]);
                at += 1;
                start = at;
            }
            b'\r' => {
                out.push(&subject[start..at]);
                at += usize::from(subject.get(at + 1) == Some(&b'\n')) + 1;
                start = at;
            }
            _ => at += 1,
        }
    }
    if start < subject.len() {
        out.push(&subject[start..]);
    }
    out
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::graphemes(string $s): array<string>` — the split half of
    /// intl's `grapheme_*` family.
    ///
    /// [`crate::granularity::Unit::Grapheme`] written out as a member. This is
    /// the unit every other length in this class already counts in
    /// ([`crate::granularity::DEFAULT`]), so `graphemes($s)` is exactly what
    /// `Str::at` walks and `Str::length` counts — a program that needs the
    /// pieces themselves does not have to reimplement the boundary rule to get
    /// them, which is the mistake `str_split` invites.
    ///
    /// An empty subject is **no pieces**, matching [`nvs_core_str_chunk`] for
    /// the same reason: empty text divides into nothing.
    fn nvs_core_str_graphemes(_ctx, args: [1]) {
        let subject = text(&args[0], "graphemes", "the subject")?;
        let mut out = NvsArray::new();
        for piece in crate::granularity::Unit::Grapheme.pieces(subject) {
            out.append(Value::str(NvsStr::new(piece.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::codePoints(string $s): array<uint>` — replacing
    /// `mb_str_split` + `mb_ord` and `unpack("N*", …)`.
    ///
    /// One member rather than the two steps PHP needs, and it answers the
    /// scalar values rather than one-code-point strings: a program asking for
    /// code points wants the numbers, and `Str::graphemes` is already the
    /// member that answers pieces of text. Every element is a Unicode scalar
    /// value, so it is in `0..=0x10FFFF` and never a surrogate — `string` is
    /// guaranteed well-formed UTF-8 (`rule:types/bytes`), which is what makes this
    /// total where PHP's `mb_ord` has a failure mode.
    ///
    /// **This is [`crate::granularity::Unit::CodePoint`], not the class
    /// default**, and that is the point of the member: it is the one place a
    /// program asks for scalar values on purpose rather than by accident.
    /// `codePoints("é\u{0301}")` is two, where `graphemes` of the same subject
    /// is one.
    fn nvs_core_str_code_points(_ctx, args: [1]) {
        let subject = text(&args[0], "codePoints", "the subject")?;
        let mut out = NvsArray::new();
        for piece in crate::granularity::Unit::CodePoint.pieces(subject) {
            let point = piece.chars().next().expect("a code point piece is one char");
            out.append(Value::uint(u64::from(point as u32)));
        }
        Ok(Value::array(out))
    }
}

/// `subject` with every leading and/or trailing character drawn from
/// `characters` removed — the shared body of `trim`/`trimStart`/`trimEnd`.
///
/// Two deliberate divergences from PHP's `trim`, both consequences of `rule:types/bytes`
/// making a `string` text rather than bytes, and of `rule:core-api/shape-rules` R13 refusing a
/// mini-language inside an argument:
///
/// * The set is matched by **character**, not by byte, so a multi-byte
///   character can be trimmed and a lone continuation byte can never be.
/// * PHP's `"a..z"` range syntax is **not** interpreted. A `.` in the set is a
///   `.`, and nothing else.
fn trimmed<'a>(subject: &'a str, characters: &str, start: bool, end: bool) -> &'a str {
    let mut out = subject;
    if start {
        out = out.trim_start_matches(|c| characters.contains(c));
    }
    if end {
        out = out.trim_end_matches(|c| characters.contains(c));
    }
    out
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::trim(string $s, {characters?: string}): string` — replacing
    /// PHP's `trim`. [`trimmed`] owns the two divergences from it, and
    /// `crate::registry`'s `TRIM_OPTIONS` owns the default set.
    fn nvs_core_str_trim(_ctx, args: [2]) {
        let subject = text(&args[0], "trim", "the subject")?;
        let characters = text(&args[1], "trim", "the `characters` option")?;
        produced(trimmed(subject, characters, true, true))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::trimStart(string $s, {characters?: string}): string` —
    /// replacing PHP's `ltrim`. See [`nvs_core_str_trim`].
    fn nvs_core_str_trim_start(_ctx, args: [2]) {
        let subject = text(&args[0], "trimStart", "the subject")?;
        let characters = text(&args[1], "trimStart", "the `characters` option")?;
        produced(trimmed(subject, characters, true, false))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::trimEnd(string $s, {characters?: string}): string` —
    /// replacing PHP's `rtrim`/`chop`. See [`nvs_core_str_trim`].
    fn nvs_core_str_trim_end(_ctx, args: [2]) {
        let subject = text(&args[0], "trimEnd", "the subject")?;
        let characters = text(&args[1], "trimEnd", "the `characters` option")?;
        produced(trimmed(subject, characters, false, true))
    }
}

/// Where `needle` next occurs in `haystack`, and how many bytes it matched —
/// the two halves a replacement needs, and the reason this is not just
/// `str::find`: a case-insensitive match can be a different byte length from
/// the needle it matched (`İ` is two bytes more than `i`), so the length has to
/// come out of the match rather than out of the pattern.
///
/// Case-insensitivity is compared one `char` at a time through Unicode's
/// simple lowercase mapping, not full case folding — so `ß` does not match
/// `SS`. That is the same boundary [`map_first`] already sits on, and it is
/// what keeps a match's byte length derivable from the subject alone.
fn find_from(haystack: &str, needle: &str, case_insensitive: bool) -> Option<(usize, usize)> {
    if !case_insensitive {
        return haystack.find(needle).map(|at| (at, needle.len()));
    }
    haystack
        .char_indices()
        .find_map(|(at, _)| match_at(&haystack[at..], needle).map(|len| (at, len)))
}

/// How many bytes of `rest` `needle` matches at its start, case-insensitively,
/// or `None` for no match — see [`find_from`].
fn match_at(rest: &str, needle: &str) -> Option<usize> {
    let mut subject = rest.char_indices();
    let mut matched = 0usize;
    for wanted in needle.chars() {
        let (at, found) = subject.next()?;
        if !found.to_lowercase().eq(wanted.to_lowercase()) {
            return None;
        }
        matched = at + found.len_utf8();
    }
    Some(matched)
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::replace(string $s, string $search, string $replacement, {caseInsensitive?: bool, limit?: uint}): string`
    /// — replacing PHP's `str_replace` **and** `str_ireplace`, which are one
    /// member here because `rule:core-api/shape-rules` R13/R20 leave no room for a second
    /// spelling of one operation.
    ///
    /// Matches are non-overlapping and taken left to right, and the
    /// replacement is never rescanned — `replace("aaa", "aa", "a")` is `"aa"`,
    /// PHP's answer too.
    ///
    /// Two option decisions, both recorded here because the spec's table
    /// states the option's *type* and not its default:
    ///
    /// * **`limit` defaults to `uint`'s maximum**, which is "every
    ///   occurrence" — a string that fits in memory can never hold that many.
    ///   A sentinel `0` would have been a magic value, and `?uint = null` is a
    ///   second spelling of "every occurrence" where the spec's table gives
    ///   `limit` a plain `uint`. A `limit` of `0` therefore means
    ///   exactly what it says: replace nothing.
    /// * **An empty `$search` replaces nothing**, rather than inserting the
    ///   replacement between every character or looping forever. PHP returns
    ///   the subject unchanged too.
    fn nvs_core_str_replace(_ctx, args: [5]) {
        let subject = text(&args[0], "replace", "the subject")?;
        let search = text(&args[1], "replace", "the search string")?;
        let replacement = text(&args[2], "replace", "the replacement")?;
        let case_insensitive = boolean(&args[3], "replace", "the `caseInsensitive` option")?;
        let limit = unsigned(&args[4], "replace", "the `limit` option")?;

        if search.is_empty() || limit == 0 {
            return produced(subject);
        }
        // The subject's own length is the capacity the accumulation starts
        // with, exactly as the `String` this replaced started with — but the
        // buffer being filled *is* the one the result is answered from, so
        // there is no copy of the whole accumulation at the end. Counting the
        // matches first to make the length exact was measured and is a loss:
        // a second `find_from` pass over the subject costs more than the copy
        // it saves.
        built(subject.len(), |out| {
            let mut rest = subject;
            let mut done = 0u64;
            while done < limit {
                let Some((at, matched)) = find_from(rest, search, case_insensitive) else {
                    break;
                };
                out.push_str(&rest[..at]);
                out.push_str(replacement);
                rest = &rest[at + matched..];
                done += 1;
            }
            out.push_str(rest);
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::replaceAll(string $s, array<string> $pairs, {caseInsensitive?: bool}): string`
    /// — replacing PHP's `str_replace` with array arguments **and** its
    /// `strtr`, which are one member here because they are one operation:
    /// substituting a whole table in a single pass.
    ///
    /// `$pairs` is keyed **needle → replacement**, so this is the first
    /// `Core` member to read an array's keys as well as its values; PHP's
    /// two-parallel-arrays spelling of `str_replace` has no counterpart,
    /// because a pair whose halves can be different lengths is a bug the type
    /// system cannot see.
    ///
    /// **`strtr`'s reading, not `str_replace`'s**, and the reason is that only
    /// one of the two is independent of the order the pairs were written:
    ///
    /// * The subject is scanned once, left to right. At each position the
    ///   **longest** matching needle wins, and the text it produced is never
    ///   rescanned — so `replaceAll("ab", ["a" => "b", "b" => "a"])` is `"ba"`
    ///   rather than `"aa"` or `"bb"`.
    /// * `str_replace`'s array form instead runs each pair over the whole
    ///   subject in turn, feeding every earlier replacement to every later
    ///   pair. That makes the answer depend on the literal order of an array
    ///   whose order is otherwise never observable here, which is exactly the
    ///   silent-surprise shape `rule:core-api/shape-rules` R20 exists to keep out.
    /// * A tie is therefore impossible without `caseInsensitive`: two distinct
    ///   needles cannot match the same span exactly. With it they can, and the
    ///   pair written first wins.
    ///
    /// **An empty needle is skipped**, matching [`nvs_core_str_replace`]'s
    /// answer for the same input rather than inserting its replacement between
    /// every character. An empty `$pairs` returns the subject unchanged.
    ///
    /// Cost: one pass over the subject, times the number of pairs, with no
    /// index built — the table a `strtr` call carries is a handful of entries
    /// in every use this library has, and an Aho-Corasick automaton would
    /// spend more building itself than it saves. Revisit against a measured
    /// call site, not against this comment.
    fn nvs_core_str_replace_all(_ctx, args: [3]) {
        let subject = text(&args[0], "replaceAll", "the subject")?;
        // Unreachable from source: parameter 1 is `array<string>` in `CLASS`
        // above, so a non-container table is `E0401: expected array<string>,
        // found mixed` at the checker. The `found a key that is not valid
        // UTF-8` guard further down is a different question and keeps its
        // line on the ratchet.
        let pairs = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::replaceAll expected {:?} for the pairs, got tag {}",
                Tag::Array,
                args[1].tag_byte()
            ))
        })?;
        let case_insensitive = boolean(&args[2], "replaceAll", "the `caseInsensitive` option")?;

        // Copied out rather than borrowed: a key is handed over as its own
        // reference, and holding one per entry for the length of the scan
        // would owe a release on every early return below.
        let mut table: Vec<(String, String)> = Vec::new();
        let mut from = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, key, value) = unsafe {
                let slot = nvs_runtime::nvs_array_next_slot(pairs, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let key = NvsStr::from_raw(nvs_runtime::nvs_array_key_at(pairs, slot));
                let mut value = Value::null();
                nvs_runtime::nvs_array_value_at(pairs, slot, &raw mut value);
                (slot, key, value)
            };
            from = slot + 1;

            // An array key is `int|string` and neither form can be invalid
            // UTF-8: `nvs_runtime`'s `key_at` renders a packed slot's key as
            // its own decimal digits, a hashed entry's key is the `string` it
            // was written with, and a `string` is guaranteed well-formed UTF-8
            // (`rule:types/bytes`). Probed with `array<string> $pairs = [1 => "z"];`,
            // which arrives here as the needle `"1"`; a `bytes` key reaches no
            // call at all, refused while lowering (`rule:types/arrays` owns which
            // pass should say so). Nothing a program writes reaches this arm —
            // unreachable from source.
            let needle = std::str::from_utf8(key.as_bytes()).map_err(|_| {
                Fault::fatal(
                    "Core\\Str::replaceAll found a key that is not valid UTF-8".to_owned(),
                )
            })?;
            let replacement = text(&value, "replaceAll", "a replacement")?;
            if !needle.is_empty() {
                table.push((needle.to_owned(), replacement.to_owned()));
            }
        }
        if table.is_empty() {
            return produced(subject);
        }

        let mut out = String::with_capacity(subject.len());
        let mut at = 0usize;
        while at < subject.len() {
            let rest = &subject[at..];
            let mut best: Option<(usize, &str)> = None;
            for (needle, replacement) in &table {
                let matched = if case_insensitive {
                    match_at(rest, needle)
                } else {
                    rest.starts_with(needle.as_str()).then_some(needle.len())
                };
                let Some(matched) = matched else { continue };
                if best.is_none_or(|(longest, _)| matched > longest) {
                    best = Some((matched, replacement.as_str()));
                }
            }
            if let Some((matched, replacement)) = best {
                out.push_str(replacement);
                at += matched;
            } else {
                // No pair starts here, so this character is kept as it stands
                // and the next position is the next character's — never the
                // next byte's, since a needle can only begin on a boundary.
                let kept = rest.chars().next().expect("`rest` is non-empty");
                out.push(kept);
                at += kept.len_utf8();
            }
        }
        produced(&out)
    }
}

/// The byte range an `int $offset` and a `?int $length` name in `subject`,
/// counted in [`crate::granularity::DEFAULT`] and read under `rule:core-api/shape-rules` R8's
/// sign rule — which is PHP's here as well:
///
/// * A **negative offset** counts from the end, and one before the start
///   clamps to it.
/// * A **negative length** stops that many characters short of the end.
/// * A **null length** runs to the end of the subject. That is the type saying
///   what a sentinel would otherwise have to, `rule:core-api/shape-rules` R5 reaching a
///   *parameter*; `nvs_stdlib::registry::Const::Null` is what a call site
///   materializes for a `slice` that omits it.
///
/// The end never precedes the start: a window that closes before it opens is
/// empty, which is `""` for [`nvs_core_str_slice`] and a pure insertion for
/// [`nvs_core_str_replace_range`]. Both members read the rule from here rather
/// than each stating it, because `substr` and `substr_replace` disagreeing
/// about a negative length is exactly the PHP surprise this shared reading
/// removes.
fn window(
    subject: &str,
    offset: &Value,
    length: &Value,
    member: &str,
) -> Result<(usize, usize), Fault> {
    let offset = integer(offset, member, "the offset")?;
    let unit = crate::granularity::DEFAULT;
    let total = unit.length(subject);

    let start = unit.byte_of_signed_index(subject, offset);
    let end = match length.tag() {
        Some(Tag::Null) => subject.len(),
        _ => {
            let length = integer(length, member, "the length")?;
            if length < 0 {
                // Counted from the *end*, not from the start: this is the one
                // place R8's sign rule means "stop short of" rather than
                // "begin at".
                let from_end = i64::try_from(total).unwrap_or(i64::MAX) + length;
                unit.byte_of_index(subject, usize::try_from(from_end).unwrap_or(0))
            } else {
                let from = unit.index_of_byte(subject, start);
                let to = usize::try_from(length)
                    .unwrap_or(usize::MAX)
                    .saturating_add(from);
                unit.byte_of_index(subject, to)
            }
        }
    };
    Ok((start, end.max(start)))
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::slice(string $s, int $offset, ?int $length = null): string`
    /// — replacing PHP's `substr` and `mb_substr`.
    ///
    /// **The first `Core` member whose default is `null`.** Both arguments are
    /// read by [`window`], which owns what each sign means.
    ///
    /// An offset past the end is `""` rather than a throw — PHP 8's answer, and
    /// the one that composes with a loop.
    fn nvs_core_str_slice(_ctx, args: [3]) {
        let subject = text(&args[0], "slice", "the subject")?;
        let (start, end) = window(subject, &args[1], &args[2], "slice")?;
        produced(subject.get(start..end).unwrap_or(""))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::replaceRange(string $s, int $offset, ?int $length, string $replacement): string`
    /// — replacing PHP's `substr_replace`, minus its by-reference and
    /// array-of-subjects forms (`rule:core-api/shape-rules` R3 makes every member pure, R20
    /// leaves one spelling per operation).
    ///
    /// This is [`nvs_core_str_slice`]'s window with the slice *substituted*
    /// rather than returned, and it reads its two positional arguments through
    /// the same [`window`] — so `replaceRange($s, $o, $n, "")` removes exactly
    /// what `slice($s, $o, $n)` returns, for every sign of every argument.
    /// `substr` and `substr_replace` hold that identity too, on all ninety
    /// windows `str-slice-and-replace-range-match-substr-and-substr_replace`
    /// sweeps; what one shared reading buys is that the two members cannot
    /// come apart later, not that PHP's pair got it wrong.
    ///
    /// `$length` is **required**, unlike `slice`'s: writing `null` for "to the
    /// end" is one character, and a default would put an optional parameter
    /// before a mandatory one.
    ///
    /// An empty window is an insertion at that position, which is how a
    /// `$length` of `0` — or a negative one that reaches back past the offset —
    /// reads. PHP's answer too.
    fn nvs_core_str_replace_range(_ctx, args: [4]) {
        let subject = text(&args[0], "replaceRange", "the subject")?;
        let replacement = text(&args[3], "replaceRange", "the replacement")?;
        let (start, end) = window(subject, &args[1], &args[2], "replaceRange")?;

        let mut out =
            String::with_capacity(subject.len().saturating_sub(end - start) + replacement.len());
        out.push_str(subject.get(..start).unwrap_or(subject));
        out.push_str(replacement);
        out.push_str(subject.get(end..).unwrap_or(""));
        produced(&out)
    }
}

/// [`find_from`] rebased onto the whole subject: where `needle` next occurs at
/// or after byte offset `cursor`, and how many bytes it matched.
///
/// `None` once `cursor` has walked past the subject's end, which is what lets
/// every scan below be a `while let` with no separate bound check.
fn find_at(
    haystack: &str,
    needle: &str,
    case_insensitive: bool,
    cursor: usize,
) -> Option<(usize, usize)> {
    let rest = haystack.get(cursor..)?;
    find_from(rest, needle, case_insensitive).map(|(at, matched)| (cursor + at, matched))
}

/// The byte offset a scan resumes at after a match of `matched` bytes at `at`.
///
/// Always a character boundary, and always **past** `at`: an empty needle
/// matches at every position, so advancing by the match's own length would
/// never terminate. Pass `0` for `matched` to walk overlapping occurrences,
/// which is what `lastIndexOf` needs and `countOf` must not do.
fn after_match(haystack: &str, at: usize, matched: usize) -> usize {
    if matched > 0 {
        return at + matched;
    }
    at + haystack[at..].chars().next().map_or(1, char::len_utf8)
}

/// A byte offset into `subject` as the `uint` position a member answers with —
/// [`crate::granularity::DEFAULT`]'s unit, which is what `rule:types/string-is-utf8` makes
/// every `string` position Novis hands out.
fn position(subject: &str, byte: usize) -> HelperResult {
    // Unreachable from source for `nvs_core_str_length`'s reason, which states
    // it in full: `usize` is no wider than `u64` on any target `deny.toml`
    // builds for, so the conversion is total and the `Err` arm is what writing
    // `try_from` rather than `as` costs.
    let index = u64::try_from(crate::granularity::DEFAULT.index_of_byte(subject, byte))
        .map_err(|_| Fault::fatal("Core\\Str counted a position past `uint`"))?;
    Ok(Value::uint(index))
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::indexOf(string $haystack, string $needle, {from?: int, caseInsensitive?: bool}): ?uint`
    /// — replacing PHP's `strpos`, `stripos`, `mb_strpos` and `mb_stripos`, all
    /// four at once, because `rule:core-api/shape-rules` R13 makes the encoding question moot and
    /// R20 leaves no room for a second spelling of one operation.
    ///
    /// **Absence is `null`, not `false`.** That is R5, and it is the single
    /// biggest correctness win in this member: PHP's `strpos(...) == false` bug
    /// family cannot be written, because a `?uint` has no falsy member that
    /// `0` could be confused with.
    ///
    /// The answer counts in [`crate::granularity::DEFAULT`], so it is directly
    /// usable as `Core\Str::slice`'s offset — the property that would break if
    /// this reported the engine's byte offset instead.
    fn nvs_core_str_index_of(_ctx, args: [4]) {
        let subject = text(&args[0], "indexOf", "the subject")?;
        let needle = text(&args[1], "indexOf", "the needle")?;
        let from = integer(&args[2], "indexOf", "the `from` option")?;
        let case_insensitive = boolean(&args[3], "indexOf", "the `caseInsensitive` option")?;

        let start = crate::granularity::DEFAULT.byte_of_signed_index(subject, from);
        match find_at(subject, needle, case_insensitive, start) {
            None => Ok(Value::null()),
            Some((at, _)) => position(subject, at),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::lastIndexOf(string $haystack, string $needle, {before?: int, caseInsensitive?: bool}): ?uint`
    /// — replacing PHP's `strrpos`, `strripos` and `mb_strrpos`. See
    /// [`nvs_core_str_index_of`] for what the two members share.
    ///
    /// **Occurrences may overlap**, so `lastIndexOf("aaa", "aa")` is 1 and not
    /// 0 — PHP's `strrpos` answers 1 too, and the last occurrence of something
    /// is a question about positions rather than about a partition.
    ///
    /// `before` bounds the search: only an occurrence that **ends at or before**
    /// that position is considered, so it names the end of the window rather
    /// than a place to start scanning from. [`LAST_INDEX_OF_OPTIONS`] owns why
    /// its default is `int`'s maximum.
    fn nvs_core_str_last_index_of(_ctx, args: [4]) {
        let subject = text(&args[0], "lastIndexOf", "the subject")?;
        let needle = text(&args[1], "lastIndexOf", "the needle")?;
        let before = integer(&args[2], "lastIndexOf", "the `before` option")?;
        let case_insensitive = boolean(&args[3], "lastIndexOf", "the `caseInsensitive` option")?;

        let bound = crate::granularity::DEFAULT.byte_of_signed_index(subject, before);
        let mut best = None;
        let mut cursor = 0usize;
        while let Some((at, matched)) = find_at(subject, needle, case_insensitive, cursor) {
            if at + matched > bound {
                break;
            }
            best = Some(at);
            cursor = after_match(subject, at, 0);
        }
        match best {
            None => Ok(Value::null()),
            Some(at) => position(subject, at),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::countOf(string $haystack, string $needle): uint` — replacing
    /// PHP's `substr_count`.
    ///
    /// Matches are **non-overlapping**, which is what separates this from
    /// [`nvs_core_str_last_index_of`]'s scan: `countOf("aaa", "aa")` is 1, PHP's
    /// answer too, because a count partitions the subject where a position
    /// search does not. An empty needle throws rather than answering the
    /// character count, as PHP's own `ValueError` does.
    fn nvs_core_str_count_of(_ctx, args: [2]) {
        let subject = text(&args[0], "countOf", "the subject")?;
        let needle = text(&args[1], "countOf", "the needle")?;
        if needle.is_empty() {
            return Err(Fault::thrown(
                "Core\\Str::countOf(): the needle must not be empty",
            ));
        }
        let mut found = 0u64;
        let mut cursor = 0usize;
        while let Some((at, matched)) = find_at(subject, needle, false, cursor) {
            found += 1;
            cursor = after_match(subject, at, matched);
        }
        Ok(Value::uint(found))
    }
}

/// The character at byte offset `at`, or `None` at the end of the subject —
/// which [`natural_order`] reads exactly where PHP's comparator reads its
/// terminating NUL, so it sorts below every character.
fn char_at(subject: &str, at: usize) -> Option<char> {
    subject[at..].chars().next()
}

/// The ASCII digit at byte offset `at`, or `None` for anything else.
///
/// A digit run is ASCII by definition here: what makes one a *number* rather
/// than ordinary characters is that this member adds it up, and `٣` is not a
/// digit it knows how to add up. Every other character goes down
/// [`compare_chars`], where it is ordered rather than counted.
fn digit_at(subject: &str, at: usize) -> Option<u8> {
    subject
        .as_bytes()
        .get(at)
        .copied()
        .filter(u8::is_ascii_digit)
}

/// Past the whitespace run that starts at `at`.
fn skip_whitespace(subject: &str, mut at: usize) -> usize {
    while let Some(found) = char_at(subject, at) {
        if !found.is_whitespace() {
            break;
        }
        at += found.len_utf8();
    }
    at
}

/// Past the zeros PHP's `leading` flag drops — at the very start of a subject
/// only, a `0` followed by another digit is not part of the number at all,
/// which is why `compare("01", "1", {natural: true})` is `0` while
/// `compare("a01", "a1", {natural: true})` is negative.
fn skip_leading_zeros(subject: &str, mut at: usize) -> usize {
    let bytes = subject.as_bytes();
    while bytes.get(at) == Some(&b'0') && bytes.get(at + 1).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    at
}

/// Two digit runs, neither of which begins with a `0`: the longer run is the
/// larger number, so the first differing digit decides only once both runs
/// turn out to be the same length. Both cursors end past their own run.
fn compare_integral(a: &str, ai: &mut usize, b: &str, bi: &mut usize) -> Ordering {
    let mut bias = Ordering::Equal;
    loop {
        match (digit_at(a, *ai), digit_at(b, *bi)) {
            (None, None) => return bias,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                if bias == Ordering::Equal {
                    bias = x.cmp(&y);
                }
                *ai += 1;
                *bi += 1;
            }
        }
    }
}

/// Two digit runs, at least one of which begins with a `0` — read as the
/// digits *after* a decimal point, so the first difference wins outright and
/// the shorter run is the smaller number. That is what orders `1.5` before
/// `1.10` and `a0010` before `a10`.
fn compare_fractional(a: &str, ai: &mut usize, b: &str, bi: &mut usize) -> Ordering {
    loop {
        match (digit_at(a, *ai), digit_at(b, *bi)) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x != y => return x.cmp(&y),
            _ => {
                *ai += 1;
                *bi += 1;
            }
        }
    }
}

/// One character against another, case-folded or not — Unicode's *simple*
/// lower-case mapping, the same boundary [`match_at`] already sits on.
fn compare_chars(a: char, b: char, case_insensitive: bool) -> Ordering {
    if case_insensitive {
        a.to_lowercase().cmp(b.to_lowercase())
    } else {
        a.cmp(&b)
    }
}

/// A subject as the sequence of characters its case-insensitive ordering
/// compares — [`compare_chars`]'s mapping over the whole of it, without
/// building a second string to hold the result.
fn folded(subject: &str) -> impl Iterator<Item = char> + '_ {
    subject.chars().flat_map(char::to_lowercase)
}

/// `{natural: true}`'s ordering: a run of digits compares as a number and
/// everything else compares as characters, which is what puts `img2` before
/// `img12`.
///
/// This is a faithful port of PHP's `strnatcmp`, quirks included, because the
/// spec's § 1 table names that function as what the option replaces and a
/// program being migrated is entitled to the same order it already sorts in.
/// Three of those quirks are not obvious and are pinned by
/// `tests/conformance/core/str-compare-orders-two-ways.nvst`:
///
/// * **A whitespace run is not significant, except at the very end.** Each
///   subject skips its own run before every comparison, so `"a b"` and
///   `"a  b"` are equal — but the walk stops the moment one subject runs out,
///   which is checked *before* the next skip, so `"x "` is greater than `"x"`.
/// * **A run beginning with `0` is a fraction** ([`compare_fractional`]),
///   except at the start of the subject ([`skip_leading_zeros`]).
/// * **An empty subject is ordered by length alone** — PHP guards it ahead of
///   the walk, which is the only reason `" "` is greater than `""` rather
///   than equal to it after the whitespace skip.
///
/// Two deliberate widenings, both the same one the rest of this module makes:
/// whitespace is Unicode's `White_Space` rather than C's `isspace`, and case
/// folding is Unicode's simple lower-case mapping rather than ASCII
/// `toupper`. Both agree with PHP over ASCII and are better outside it.
fn natural_order(a: &str, b: &str, case_insensitive: bool) -> Ordering {
    if a.is_empty() || b.is_empty() {
        return a.len().cmp(&b.len());
    }
    let (mut ai, mut bi) = (skip_leading_zeros(a, 0), skip_leading_zeros(b, 0));
    loop {
        ai = skip_whitespace(a, ai);
        bi = skip_whitespace(b, bi);
        let (mut ca, mut cb) = (char_at(a, ai), char_at(b, bi));

        if let (Some(x), Some(y)) = (ca, cb)
            && x.is_ascii_digit()
            && y.is_ascii_digit()
        {
            let run = if x == '0' || y == '0' {
                compare_fractional(a, &mut ai, b, &mut bi)
            } else {
                compare_integral(a, &mut ai, b, &mut bi)
            };
            if run != Ordering::Equal {
                return run;
            }
            match (ai == a.len(), bi == b.len()) {
                (true, true) => return Ordering::Equal,
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                (false, false) => {}
            }
            // The character that ended two equal runs is compared here rather
            // than at the top of the next turn, so it is *not* whitespace-
            // skipped: `"1\t"` is less than `"1 "` where `"a\t"` and `"a "`
            // are equal. PHP's own loop has this asymmetry and programs sort
            // by it.
            ca = char_at(a, ai);
            cb = char_at(b, bi);
        }

        let ord = match (ca, cb) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(x), Some(y)) => compare_chars(x, y, case_insensitive),
        };
        if ord != Ordering::Equal {
            return ord;
        }
        ai += ca.map_or(0, char::len_utf8);
        bi += cb.map_or(0, char::len_utf8);
        match (ai >= a.len(), bi >= b.len()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::compare(string $a, string $b, {caseInsensitive?: bool, natural?: bool}): int`
    /// — replacing all four of PHP's `strcmp`, `strcasecmp`, `strnatcmp` and
    /// `strnatcasecmp`, plus the comparator behind `natsort`/`natcasesort`,
    /// because `rule:core-api/shape-rules` R20 leaves no room for four spellings of one
    /// operation. `strncmp`'s length-limited form is `Core\Str::slice` first.
    ///
    /// **The answer is `-1`, `0` or `1` and never a byte difference.** PHP 8
    /// already normalized `strcmp` that way, and the only consumer of this
    /// member is a comparator — a magnitude would be a number callers could
    /// come to depend on without it meaning anything.
    ///
    /// The default ordering is over characters, which for UTF-8 is also over
    /// bytes; `{caseInsensitive: true}` compares Unicode's simple lower-case
    /// mapping of each instead, the same boundary [`match_at`] sits on, so
    /// `ß` and `SS` are still different. `{natural: true}` is a **different
    /// ordering** rather than a variant of this one — [`natural_order`] owns
    /// what it is, and the spec's own prose under § 1's *Comparison* table
    /// says so with `compare("img12", "img2")` as the sign that flips.
    ///
    /// There is no locale-sensitive third ordering: `strcoll` has nothing to
    /// read a locale from here (`rule:core-api/tier-placement`), which § 1 states.
    fn nvs_core_str_compare(_ctx, args: [4]) {
        let left = text(&args[0], "compare", "the first subject")?;
        let right = text(&args[1], "compare", "the second subject")?;
        let case_insensitive = boolean(&args[2], "compare", "the `caseInsensitive` option")?;
        let natural = boolean(&args[3], "compare", "the `natural` option")?;

        let ordering = match (natural, case_insensitive) {
            (true, fold) => natural_order(left, right, fold),
            (false, true) => folded(left).cmp(folded(right)),
            (false, false) => left.cmp(right),
        };
        Ok(Value::int(match ordering {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }))
    }
}

/// Where the occurrence `before`/`after` cut at begins, and how many bytes it
/// matched — the first one, or the last when `last` is set.
///
/// Case-sensitive: neither member declares a `caseInsensitive` option, because
/// the spec gives them one option and it is this one.
fn cut_at(subject: &str, needle: &str, last: bool) -> Option<(usize, usize)> {
    if !last {
        return find_from(subject, needle, false);
    }
    let mut best = None;
    let mut cursor = 0usize;
    while let Some(found) = find_at(subject, needle, false, cursor) {
        best = Some(found);
        cursor = after_match(subject, found.0, 0);
    }
    best
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::before(string $s, string $needle, {last?: bool}): ?string` —
    /// everything up to the first occurrence of `$needle`, replacing PHP's
    /// `strstr($h, $n, true)` and `strrchr` used as a prefix.
    ///
    /// The needle itself is not included, and a needle that does not occur is
    /// `null` rather than PHP's `false` (`rule:core-api/shape-rules` R5) — spec § 1's *Extraction*
    /// prose is the home for both, and for what `{last: true}` changes.
    fn nvs_core_str_before(_ctx, args: [3]) {
        let subject = text(&args[0], "before", "the subject")?;
        let needle = text(&args[1], "before", "the needle")?;
        let last = boolean(&args[2], "before", "the `last` option")?;
        match cut_at(subject, needle, last) {
            None => Ok(Value::null()),
            Some((at, _)) => produced(&subject[..at]),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::after(string $s, string $needle, {last?: bool}): ?string` —
    /// everything past the first occurrence of `$needle`, replacing PHP's
    /// `strstr`, `stristr` and `strrchr`.
    ///
    /// The needle is not included, which is the one place this diverges from
    /// `strstr` — spec § 1's *Extraction* prose owns that rule and the port of
    /// a program that wanted PHP's shape. Absence is `null`, not `false`.
    fn nvs_core_str_after(_ctx, args: [3]) {
        let subject = text(&args[0], "after", "the subject")?;
        let needle = text(&args[1], "after", "the needle")?;
        let last = boolean(&args[2], "after", "the `last` option")?;
        match cut_at(subject, needle, last) {
            None => Ok(Value::null()),
            Some((at, matched)) => produced(&subject[at + matched..]),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::reverse(string $s): string` — replacing PHP's `strrev`.
    ///
    /// **Grapheme-aware**, which PHP's byte-wise `strrev` is not: reversing
    /// `"café"` there produces invalid UTF-8, and here it produces `"éfac"`.
    /// The unit is [`crate::granularity::DEFAULT`], so a combining mark stays
    /// attached to the letter it modifies.
    ///
    /// Spends one `Vec` of borrowed pieces per call — [`crate::granularity`]'s
    /// iterator is forward-only, and a reverse needs the last piece first. The
    /// pieces are only ever *read* backwards, so that `Vec` is the whole of
    /// what this member spends beyond its result: a reversal is the same bytes
    /// in a different order, so the length is the subject's and [`built`]
    /// writes them once.
    fn nvs_core_str_reverse(_ctx, args: [1]) {
        let subject = text(&args[0], "reverse", "the subject")?;
        let pieces: Vec<&str> = crate::granularity::DEFAULT.pieces(subject).collect();
        built(subject.len(), |out| {
            for piece in pieces.iter().rev() {
                out.push_str(piece);
            }
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::wrap(string $s, uint $width, {breakWith?: string, cutLongWords?: bool}): string`
    /// — replacing PHP's `wordwrap`, whose third and fourth arguments become
    /// this member's two options.
    ///
    /// [`wrapped`] owns the algorithm, which is PHP's own. Two arguments are
    /// refused here rather than there, because neither has an answer:
    ///
    /// * **An empty `breakWith`** would insert nothing at a break, so the
    ///   result would be the subject with the wrapping silently dropped.
    /// * **A zero `width` with `cutLongWords`** asks for a break before every
    ///   character *and* after it, which does not terminate. PHP raises
    ///   `ValueError` for the same pair; a zero width without cutting is fine
    ///   and breaks at every space.
    fn nvs_core_str_wrap(_ctx, args: [4]) {
        let subject = text(&args[0], "wrap", "the subject")?;
        let width = count(&args[1], "wrap", "the width")?;
        let break_with = text(&args[2], "wrap", "the `breakWith` option")?;
        let cut = boolean(&args[3], "wrap", "the `cutLongWords` option")?;
        if break_with.is_empty() {
            return Err(Fault::thrown(
                "Core\\Str::wrap(): the `breakWith` option must not be empty",
            ));
        }
        if width == 0 && cut {
            return Err(Fault::thrown(
                "Core\\Str::wrap(): a width of 0 cannot cut long words, since every character \
                 would have to be broken both before and after",
            ));
        }
        produced(&wrapped(subject, width, break_with, cut))
    }
}

/// `subject` with `break_with` inserted so no line exceeds `width` units —
/// PHP's `wordwrap`, unit for unit.
///
/// **The width counts in [`crate::granularity::DEFAULT`]**, not in bytes, which
/// is the one thing this does not inherit from PHP: a wrapped column of text is
/// exactly the place where counting `"é"` as two would misalign the output.
/// That is what the `starts` table is for — one byte offset per unit, plus a
/// sentinel for the end, so a unit-counted line has a byte-slicable range.
///
/// A break string already present in the subject **resets the line**, so a
/// paragraph that is already wrapped is re-wrapped rather than measured as one
/// long line. It is matched by bytes and only accepted when it ends on a unit
/// boundary — a break that splits a grapheme cluster is not a line ending.
fn wrapped(subject: &str, width: usize, break_with: &str, cut: bool) -> String {
    let unit = crate::granularity::DEFAULT;
    let mut starts: Vec<usize> = Vec::new();
    let mut at = 0usize;
    for piece in unit.pieces(subject) {
        starts.push(at);
        at += piece.len();
    }
    starts.push(subject.len());
    let total = starts.len() - 1;

    let mut out = String::with_capacity(subject.len());
    // Both are unit indices: where the line being measured began, and the last
    // space seen on it. A `last_space` at or before `line_start` is one from a
    // line already emitted, which is how "this line has no space to break at"
    // is spelled.
    let mut line_start = 0usize;
    let mut last_space: Option<usize> = None;
    let mut current = 0usize;
    while current < total {
        let byte = starts[current];
        if subject[byte..].starts_with(break_with)
            && let Ok(after) = starts.binary_search(&(byte + break_with.len()))
        {
            out.push_str(&subject[starts[line_start]..starts[after]]);
            line_start = after;
            last_space = None;
            current = after;
            continue;
        }
        let over = current - line_start >= width;
        if &subject[byte..starts[current + 1]] == " " {
            if over {
                out.push_str(&subject[starts[line_start]..byte]);
                out.push_str(break_with);
                line_start = current + 1;
            }
            last_space = Some(current);
        } else if over && last_space.is_none_or(|space| line_start >= space) {
            if cut {
                out.push_str(&subject[starts[line_start]..byte]);
                out.push_str(break_with);
                line_start = current;
                last_space = None;
            }
        } else if over {
            let space = last_space.expect("the previous arm covered the absent case");
            out.push_str(&subject[starts[line_start]..starts[space]]);
            out.push_str(break_with);
            line_start = space + 1;
            last_space = None;
        }
        current += 1;
    }
    out.push_str(&subject[starts[line_start]..]);
    out
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::padStart(string $s, uint $length, string $padding = " "): string`
    /// — replacing PHP's `str_pad` with `STR_PAD_LEFT`.
    ///
    /// A subject already at least `$length` long comes back unchanged, and a
    /// padding run that does not divide evenly is cut at **the run's own
    /// end** — [`write_run`] writes whole copies of the padding and then a
    /// prefix of one more, so the dropped piece is the one abutting the
    /// subject here and the one ending the result in [`nvs_core_str_pad_end`].
    /// Both are PHP's behaviour, verified against 8.5. This comment used to
    /// say "the end nearest the subject" for the pair, which is true of only
    /// one of them; `str-pad-members-place-one-run-they-both-agree-on.nvst`
    /// asserts the run is a function of the shortfall alone and pins the two
    /// sides against each other.
    fn nvs_core_str_pad_start(_ctx, args: [3]) {
        let subject = text(&args[0], "padStart", "the subject")?;
        let length = count(&args[1], "padStart", "the target length")?;
        let padding = text(&args[2], "padStart", "the padding")?;
        let (run, fill) = padding_run(subject, length, padding, "Core\\Str::padStart")?;
        built_fallibly(fill + subject.len(), "Core\\Str::padStart", |out| {
            write_run(out, padding, run);
            out.push_str(subject);
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::padEnd(string $s, uint $length, string $padding = " "): string`
    /// — replacing PHP's `str_pad` with `STR_PAD_RIGHT`. See
    /// [`nvs_core_str_pad_start`] for the shared rules.
    fn nvs_core_str_pad_end(_ctx, args: [3]) {
        let subject = text(&args[0], "padEnd", "the subject")?;
        let length = count(&args[1], "padEnd", "the target length")?;
        let padding = text(&args[2], "padEnd", "the padding")?;
        let (run, fill) = padding_run(subject, length, padding, "Core\\Str::padEnd")?;
        built_fallibly(subject.len() + fill, "Core\\Str::padEnd", |out| {
            out.push_str(subject);
            write_run(out, padding, run);
        })
    }
}

/// The run of padding `padStart`/`padEnd` prepend or append — `padding`
/// repeated and then cut to exactly the shortfall, counted in
/// [`crate::granularity::DEFAULT`] — as **how many pieces it is, and how many
/// bytes they occupy**, which is all [`write_run`] and the result's length
/// between them need. Building the run as a `String` here is what the member
/// then had to copy a second time.
///
/// Empty padding throws rather than looping: it can never close a shortfall,
/// and PHP's own `str_pad` refuses it too.
fn padding_run(
    subject: &str,
    length: usize,
    padding: &str,
    member: &str,
) -> Result<(usize, usize), Fault> {
    let unit = crate::granularity::DEFAULT;
    let have = unit.length(subject);
    if have >= length {
        return Ok((0, 0));
    }
    if padding.is_empty() {
        return Err(Fault::thrown(format!(
            "{member}(): the padding is empty, so it can never reach the requested length"
        )));
    }
    // The run is `length - have` pieces, each at most the whole padding, so
    // that product is an upper bound on the bytes about to be collected. It
    // had no check at all before: `$length` is a `uint` off the call site, so
    // `padStart("x", n, "y")` would build an n-byte string with nothing
    // between it and the allocator.
    let run = length - have;
    // `member` arrives already qualified, so the path that succeeds formats
    // nothing: a `format!` here was one allocation per pad.
    nvs_runtime::affordable(run.checked_mul(padding.len()), member)?;
    // The run is whole copies of the padding and then a prefix of one more, so
    // both its length and [`write_run`]'s writing are arithmetic on the pieces
    // of `padding` alone — walking `run` pieces of a cycle to measure what a
    // second walk then writes was measured and is a loss.
    let pieces = unit.length(padding);
    let tail: usize = unit.pieces(padding).take(run % pieces).map(str::len).sum();
    // Cannot overflow: the check above bounds the whole run, and this is it.
    Ok((run, (run / pieces) * padding.len() + tail))
}

/// Writes the run [`padding_run`] measured: `run` pieces of `padding`, which is
/// `run / pieces` whole copies of it and then the first `run % pieces` of one
/// more.
fn write_run(out: &mut StrWriter<'_>, padding: &str, run: usize) {
    if run == 0 {
        // A subject already at or past the target length, where `padding_run`
        // answers before it has refused an empty padding — so this is also
        // what keeps the piece count below non-zero.
        return;
    }
    let unit = crate::granularity::DEFAULT;
    let pieces = unit.length(padding);
    for _ in 0..run / pieces {
        out.push_str(padding);
    }
    let tail: usize = unit.pieces(padding).take(run % pieces).map(str::len).sum();
    out.push_str(&padding[..tail]);
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::repeat(string $s, uint $times): string` — replacing PHP's
    /// `str_repeat`. Zero times is the empty string, as in PHP.
    ///
    /// An empty subject short-circuits the loop rather than running it, for
    /// the reason `Core\Bytes::repeat`'s own doc comment gives: the size check
    /// below is about how large the result is, so an empty subject passes it
    /// for every count there is, and the loop would then run a caller-supplied
    /// `uint` of iterations appending nothing.
    fn nvs_core_str_repeat(_ctx, args: [2]) {
        let subject = text(&args[0], "repeat", "the subject")?;
        let times = count(&args[1], "repeat", "the repeat count")?;
        // The size goes through the one shared check first, so a repeat too
        // large to hold is an ordinary throw rather than the contained FATAL a
        // panicking allocation would be — and then the allocation itself is
        // asked, because that check answers a different question. See
        // [`built_fallibly`].
        let len = nvs_runtime::affordable(
            subject.len().checked_mul(times),
            "Core\\Str::repeat",
        )?;
        let runs = if subject.is_empty() { 0 } else { times };
        built_fallibly(len, "Core\\Str::repeat", |out| {
            for _ in 0..runs {
                out.push_str(subject);
            }
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::lower(string $s): string` — replacing PHP's `strtolower` and
    /// `mb_strtolower`. Unicode's full lowercase mapping; see this module's
    /// docs for why there is only one of them.
    fn nvs_core_str_lower(_ctx, args: [1]) {
        produced(&text(&args[0], "lower", "the subject")?.to_lowercase())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::upper(string $s): string` — replacing PHP's `strtoupper` and
    /// `mb_strtoupper`.
    fn nvs_core_str_upper(_ctx, args: [1]) {
        produced(&text(&args[0], "upper", "the subject")?.to_uppercase())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::upperFirst(string $s): string` — replacing PHP's `ucfirst`.
    ///
    /// Only the first character changes; the rest is copied through, which is
    /// what separates this from title casing (`ucwords`, which the spec's § 1
    /// note keeps out of `Core` entirely because word segmentation is
    /// locale-dependent).
    fn nvs_core_str_upper_first(_ctx, args: [1]) {
        produced(&map_first(text(&args[0], "upperFirst", "the subject")?, true))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::lowerFirst(string $s): string` — replacing PHP's `lcfirst`.
    fn nvs_core_str_lower_first(_ctx, args: [1]) {
        produced(&map_first(text(&args[0], "lowerFirst", "the subject")?, false))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::fold(string $s): string` — replacing PHP's
    /// `mb_convert_case($s, MB_CASE_FOLD)`, and the one member here that
    /// exists **for comparison rather than for display**.
    ///
    /// Folding is Unicode's *default full* case folding (UAX #44's `C` and `F`
    /// mappings, `caseless`'s table at Unicode 16.0), which is a different
    /// function from the lower-case mapping [`nvs_core_str_lower`] applies:
    /// folding turns `ß` into `ss` and `ﬁ` into `fi`, because its whole job is
    /// to make two strings that differ only by case *equal*, and `lower` has
    /// to leave a word looking like a word. So the answer is not text to show
    /// anyone — it is a key.
    ///
    /// This is therefore the strict form of `compare`'s
    /// `{caseInsensitive: true}`, whose per-character simple mapping cannot
    /// see `ß` and `SS` as the same: `compare(fold($a), fold($b)) == 0` is the
    /// caseless test that does. Two members rather than a third option,
    /// because folding is a value a caller can hold on to — a lookup key
    /// folds once and is compared many times.
    fn nvs_core_str_fold(_ctx, args: [1]) {
        produced(&caseless::default_case_fold_str(
            text(&args[0], "fold", "the subject")?,
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::normalize(string $s, NormalForm $form): string` — replacing
    /// PHP's `Normalizer::normalize`.
    ///
    /// UAX #15's four forms over two independent axes, which is why this is
    /// one member with an enum rather than four members: **composed or
    /// decomposed** says whether `é` is one code point or `e` plus a
    /// combining acute, and **canonical or compatibility** says whether a
    /// character that merely *renders* like another is unified with it — `ﬁ`
    /// to `fi`, `①` to `1`. Only the canonical pair round-trips; a `K` form
    /// discards a distinction the original made, which is the same
    /// "comparison key, not a rendering" caveat [`nvs_core_str_fold`] carries
    /// and for the same reason.
    ///
    /// Distinct from folding, and both are needed: folding removes case, and
    /// normalizing removes the choice of encoding. Two strings that look
    /// identical on screen can differ in either, so a lookup key that has to
    /// survive both text-editor round trips and case wants
    /// `fold(normalize($s, NormalForm::Nfc))`.
    ///
    /// ASCII takes the buffer unchanged: no ASCII character has a canonical
    /// or a compatibility decomposition, so all four forms are the identity
    /// there, and the check costs a scan where the general path costs a
    /// `String`.
    fn nvs_core_str_normalize(_ctx, args: [2]) {
        let subject = text(&args[0], "normalize", "the subject")?;
        // The form is read before the fast path rather than after it, so a
        // value that is no case is a fatal for every subject and not only for
        // the ones that reach the table.
        let form = normal_form_of(&args[1])?;
        if subject.is_ascii() {
            return produced(subject);
        }
        produced(&match form {
            NormalForm::Nfc => subject.nfc().collect::<String>(),
            NormalForm::Nfd => subject.nfd().collect::<String>(),
            NormalForm::Nfkc => subject.nfkc().collect::<String>(),
            NormalForm::Nfkd => subject.nfkd().collect::<String>(),
        })
    }
}

/// [`NORMAL_FORM`]'s cases, as the thing the implementation actually branches
/// on.
///
/// A second spelling of one roster is what
/// `the_normal_form_enum_and_its_rust_twin_are_one_roster` exists to refuse,
/// and it is worth the test: the alternative is matching on the raw integer,
/// where a reordered [`NORMAL_FORM`] compiles and answers the wrong form.
#[derive(Clone, Copy)]
enum NormalForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

/// The `Core\NormalForm` case in slot 1, or the `FATAL` a value that is no
/// case is.
///
/// # Errors
///
/// A [`Fault::fatal`], not a throw, for [`text`]'s reason: the checker placed
/// this argument and compiled code wrote the integer, so anything else here
/// is a runtime-contract violation rather than something a program can cause.
fn normal_form_of(value: &Value) -> Result<NormalForm, Fault> {
    match value.as_int() {
        Some(0) => Ok(NormalForm::Nfc),
        Some(1) => Ok(NormalForm::Nfd),
        Some(2) => Ok(NormalForm::Nfkc),
        Some(3) => Ok(NormalForm::Nfkd),
        // Unreachable from source: `normalize`'s parameter 1 is
        // `CoreTy::Enum(NORMAL_FORM_NAME)` in `CLASS` above, so a value that
        // is no case is `E0401: expected Core\NormalForm, found …` at the
        // checker — probed with a `mixed` binding and with the bare `int`
        // literal `1`, which is the near miss worth checking since the
        // discriminant compiled code writes for a case is exactly an integer.
        _ => Err(Fault::fatal(format!(
            "Core\\Str::normalize expected a `{NORMAL_FORM_NAME}` case, got tag {} value {:?}",
            value.tag_byte(),
            value.as_int()
        ))),
    }
}

/// One code point, as the `char` it names, or the throw that says why it names
/// none.
///
/// **Not every `uint` is a scalar value**, and this is where that is enforced
/// once for both `fromCodePoint` and `fromCodePoints`. Two ranges are refused:
/// anything above U+10FFFF, and the surrogate range U+D800..=U+DFFF, which
/// UTF-8 cannot encode and which is the exact hole a UTF-16 round trip leaks.
/// PHP's `mb_chr` answers `false` for both; Novis throws, because a `string` is
/// guaranteed well-formed UTF-8 (`rule:types/bytes`) and a substituted replacement
/// character would be the silent-lossy conversion
/// `rule:types/conversion` refuses
/// everywhere else.
fn scalar_value(point: i128, member: &str) -> Result<char, Fault> {
    u32::try_from(point)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Str::{member}(): {point} is not a Unicode scalar value — a code point is \
                 at most 1114111 and is never in the surrogate range 55296..57343"
            ))
        })
}

/// One code-point argument, whichever integer tag it arrives under.
///
/// **Both tags are accepted on purpose**, unlike [`unsigned`], which is what
/// every other `uint` parameter in this class reads through. A written `int`
/// literal in an `array<uint>` position type-checks — `Str::fromCodePoints([97,
/// 98])` is the obvious call, and covariance on read admits it — and reaches
/// this crate still tagged [`Tag::Int`], so refusing it would answer the
/// most natural spelling of the member with a fatal rather than a value. A
/// negative one is no more a scalar value than 1114112 is, so it takes the same
/// throw from [`scalar_value`] rather than a second message.
fn code_point(value: &Value, member: &str, position: &str) -> Result<i128, Fault> {
    value
        .as_uint()
        .map(i128::from)
        .or_else(|| value.as_int().map(i128::from))
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::{member} expected {:?} for {position}, got tag {}",
                Tag::Uint,
                value.tag_byte()
            ))
        })
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::fromCodePoint(uint $codePoint): string` — replacing `chr`
    /// and `mb_chr`.
    ///
    /// One member for both because Novis has only one text type: PHP's `chr`
    /// builds a *byte*, which is what makes it the wrong half of the pair as
    /// soon as the argument exceeds 127, and that operation lives on
    /// `Core\Bytes` here rather than under a name that looks like text.
    ///
    /// The argument is a scalar value, not a byte — see [`scalar_value`] for
    /// the two ranges that throw and why this does not substitute.
    fn nvs_core_str_from_code_point(_ctx, args: [1]) {
        let point = code_point(&args[0], "fromCodePoint", "the code point")?;
        let mut buffer = [0u8; 4];
        produced(scalar_value(point, "fromCodePoint")?.encode_utf8(&mut buffer))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::fromCodePoints(array<uint> $codePoints): string` —
    /// replacing `implode(array_map("mb_chr", …))`.
    ///
    /// [`nvs_core_str_code_points`]'s inverse, and the pair round-trips: a
    /// subject through `codePoints` and back is the same `string`, since both
    /// halves refuse everything UTF-8 cannot hold. The array form exists
    /// rather than leaving it to `Str::join` because building one string of
    /// *n* code points through *n* one-character strings allocates *n* times
    /// for a result whose length is known — this fills one buffer.
    ///
    /// **A bad element throws and nothing is produced**, rather than the
    /// prefix that was valid: a half-built string is the failure mode that
    /// gets written to a socket before anyone checks.
    fn nvs_core_str_from_code_points(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is `array<uint>` in `CLASS`
        // above, so a non-container argument is `E0401: expected array<uint>,
        // found mixed` at the checker. Same judgement as
        // [`nvs_core_str_join`], and as `crate::arr`'s `nvs_core_arr_count`
        // which states it in full.
        let points = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::fromCodePoints expected {:?} for the code points, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;

        let mut out = String::new();
        let mut from = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, value) = unsafe {
                let slot = nvs_runtime::nvs_array_next_slot(points, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let mut value = Value::null();
                nvs_runtime::nvs_array_value_at(points, slot, &raw mut value);
                (slot, value)
            };
            from = slot + 1;
            let point = code_point(&value, "fromCodePoints", "an element")?;
            out.push(scalar_value(point, "fromCodePoints")?);
        }
        produced(&out)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Str::format(string $template, mixed ...$arguments): string` —
    /// replacing PHP's `sprintf`, `vsprintf`, `printf`, `vprintf`, `fprintf`
    /// and `vfprintf` at once, since none of the six differs in anything but
    /// where its answer goes.
    ///
    /// The **first `Core` member with a variadic parameter**, so the second
    /// argument slot is not one value per written argument but a single
    /// `Tag::Array` holding all of them, built at the call site by
    /// `nvs_ir::lower::lower_variadic_tail` — see
    /// [`crate::registry::CoreTy::Variadic`] for why that shape rather than a
    /// second calling convention. A call that writes no argument at all still
    /// receives an array here, empty rather than absent.
    ///
    /// The template grammar, every refusal and the one thing still owed
    /// (`rule:expressions/intrinsic-literals`'s compile-time check of a *literal* template) are
    /// [`crate::format`]'s, which is the whole of this member.
    fn nvs_core_str_format(_ctx, args: [2]) {
        let template = text(&args[0], "format", "the template")?;
        // Parameter 1 is `CoreTy::Variadic`, and
        // `nvs_ir::lower::lower_call_args` *builds* the array this slot holds
        // out of every argument from that position on, so no source
        // expression reaches the slot and no call — well-typed or not — could
        // put another tag here: unreachable from source for a stronger reason
        // than the `E0401` that refuses the template above. Same judgement as
        // `crate::path`'s `join`, which states it in full.
        let arguments = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::format expected {:?} for the argument list, got tag {}",
                Tag::Array,
                args[1].tag_byte()
            ))
        })?;

        let mut collected = Vec::new();
        let mut from = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, value) = unsafe {
                let slot = nvs_runtime::nvs_array_next_slot(arguments, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let mut value = Value::null();
                nvs_runtime::nvs_array_value_at(arguments, slot, &raw mut value);
                (slot, value)
            };
            from = slot + 1;
            collected.push(value);
        }

        produced(&crate::format::format(template, &collected)?)
    }
}

/// `subject` with its first character case-mapped and the rest copied
/// through — the shared body of `upperFirst`/`lowerFirst`.
///
/// A character whose mapping is more than one character (`ß` → `SS`) expands,
/// which is Unicode's answer and the one PHP's byte-wise `ucfirst` cannot
/// give.
fn map_first(subject: &str, upper: bool) -> String {
    let mut chars = subject.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out: String = if upper {
        first.to_uppercase().collect()
    } else {
        first.to_lowercase().collect()
    };
    out.push_str(chars.as_str());
    out
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsArray, NvsStr, OutputSink, Value, call};

    /// Runs one member through the `rule:errors/propagation` boundary compiled code reaches it
    /// at, releasing every string this test built afterwards — the helper
    /// convention borrows, so the caller still owns them.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        args: &[Value],
    ) -> Result<Value, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(member, &mut ctx, args);
        for arg in args {
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built for each \
                          argument, and the helper borrowed rather than \
                          consumed it"
            )]
            unsafe {
                arg.release();
            }
        }
        result
    }

    /// The returned string's bytes, releasing the reference the helper handed
    /// back.
    fn taken(result: Value) -> String {
        let text = String::from_utf8(
            result
                .as_str_bytes()
                .expect("the member returned a string")
                .to_vec(),
        )
        .expect("the member returned UTF-8");
        #[expect(
            unsafe_code,
            reason = "a returned heap value carries one fresh reference, which \
                      the caller owns"
        )]
        unsafe {
            result.release();
        }
        text
    }

    fn s(text: &str) -> Value {
        Value::str(NvsStr::new(text.as_bytes()))
    }

    #[test]
    fn case_conversion_uses_unicodes_mapping_not_a_byte_wise_one() {
        assert_eq!(
            taken(run(super::nvs_core_str_upper, &[s("straße")]).expect("upper never fails")),
            "STRASSE"
        );
        assert_eq!(
            taken(run(super::nvs_core_str_lower, &[s("ÄRGER")]).expect("lower never fails")),
            "ärger"
        );
        assert_eq!(
            taken(
                run(super::nvs_core_str_upper_first, &[s("ärger")])
                    .expect("upperFirst never fails")
            ),
            "Ärger"
        );
        assert_eq!(
            taken(
                run(super::nvs_core_str_lower_first, &[s("ÄRGER")])
                    .expect("lowerFirst never fails")
            ),
            "äRGER"
        );
    }

    #[test]
    fn case_conversion_of_the_empty_string_is_the_empty_string() {
        assert_eq!(
            taken(run(super::nvs_core_str_upper_first, &[s("")]).expect("no failure")),
            ""
        );
    }

    #[test]
    fn the_predicates_answer_what_php_8_answers() {
        for (haystack, needle, contains, starts, ends) in [
            ("abcdef", "cd", true, false, false),
            ("abcdef", "abc", true, true, false),
            ("abcdef", "def", true, false, true),
            ("abcdef", "", true, true, true),
            ("abcdef", "zz", false, false, false),
        ] {
            let call_it = |f: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32| {
                run(f, &[s(haystack), s(needle)])
                    .expect("a predicate never fails")
                    .as_bool()
                    .expect("a predicate returns a bool")
            };
            assert_eq!(call_it(super::nvs_core_str_contains), contains, "{needle}");
            assert_eq!(call_it(super::nvs_core_str_starts_with), starts, "{needle}");
            assert_eq!(call_it(super::nvs_core_str_ends_with), ends, "{needle}");
        }
    }

    #[test]
    fn join_walks_the_array_in_insertion_order() {
        let mut parts = NvsArray::new();
        parts.set(NvsStr::new(b"0"), s("a"));
        parts.set(NvsStr::new(b"1"), s("b"));
        parts.set(NvsStr::new(b"2"), s("c"));
        assert_eq!(
            taken(
                run(super::nvs_core_str_join, &[Value::array(parts), s("|")]).expect("no failure")
            ),
            "a|b|c"
        );
    }

    /// The pieces `split` produces, read back in order.
    fn split_at(subject: &str, separator: &str, limit: i64) -> Vec<String> {
        let result = run(
            super::nvs_core_str_split,
            &[s(subject), s(separator), Value::int(limit)],
        )
        .expect("a non-empty separator never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the \
                      handle takes over and releases on drop"
        )]
        let array =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("split returns an array")) };
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let piece = array.value_at(slot).expect("a live slot holds a value");
            out.push(
                String::from_utf8(
                    piece
                        .as_str_bytes()
                        .expect("every piece is a string")
                        .to_vec(),
                )
                .expect("every piece is UTF-8"),
            );
            from = slot + 1;
        }
        out
    }

    /// Every row verified against PHP 8.5's `explode`, whose third argument is
    /// this member's one option.
    #[test]
    fn split_matches_phps_explode_at_every_sign_of_the_limit() {
        assert_eq!(split_at("a b c", " ", i64::MAX), ["a", "b", "c"]);
        assert_eq!(split_at("a b c", " ", 2), ["a", "b c"]);
        // Zero means one piece, not none — PHP's own reading.
        assert_eq!(split_at("a b c", " ", 0), ["a b c"]);
        assert_eq!(split_at("a b c", " ", -1), ["a", "b"]);
        assert!(split_at("a b c", " ", -9).is_empty());
        // A separator that never occurs yields the subject, unsplit.
        assert_eq!(split_at("abc", "x", i64::MAX), ["abc"]);
        // An empty subject is one empty piece, not zero pieces.
        assert_eq!(split_at("", " ", i64::MAX), [""]);
    }

    /// An empty separator has no piece boundary to find, so it throws rather
    /// than quietly handing the subject back — PHP raises `ValueError` too.
    #[test]
    fn an_empty_split_separator_throws() {
        let status = run(super::nvs_core_str_split, &[s("a b"), s(""), Value::int(9)])
            .expect_err("an empty separator is refused");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    /// The three terminators, the interior empty line that survives and the
    /// trailing one that does not — [`super::line_pieces`]'s own contract.
    #[test]
    fn lines_end_on_any_of_the_three_terminators() {
        fn lines(subject: &str) -> Vec<&str> {
            super::line_pieces(subject.as_bytes())
                .into_iter()
                .map(|line| std::str::from_utf8(line).expect("a piece of valid text"))
                .collect()
        }
        assert_eq!(lines("a\nb"), ["a", "b"]);
        assert_eq!(lines("a\r\nb"), ["a", "b"]);
        assert_eq!(lines("a\rb"), ["a", "b"]);
        assert_eq!(lines("a\r\n\nb\r"), ["a", "", "b"]);
        // A trailing terminator ends the last line rather than opening one.
        assert_eq!(lines("a\n"), ["a"]);
        assert_eq!(lines("\n"), [""]);
        assert!(lines("").is_empty());
    }

    /// `chunk` counts in [`crate::granularity::DEFAULT`], so a chunk boundary
    /// never lands inside a cluster — which is the whole reason `str_split`
    /// cannot be used on text.
    #[test]
    fn chunk_divides_by_cluster_and_never_inside_one() {
        let chunk = |subject: &str, size: u64| -> Vec<String> {
            let result = run(super::nvs_core_str_chunk, &[s(subject), Value::uint(size)])
                .expect("chunk answers an array");
            #[expect(
                unsafe_code,
                reason = "the helper returned one fresh reference, which the \
                          handle takes over and releases on drop"
            )]
            let array =
                unsafe { NvsArray::from_raw(result.array_ptr().expect("chunk returns an array")) };
            let mut out = Vec::new();
            let mut from = 0usize;
            while let Some(slot) = array.next_slot(from) {
                let piece = array.value_at(slot).expect("a live slot holds a value");
                out.push(
                    String::from_utf8(
                        piece
                            .as_str_bytes()
                            .expect("every chunk is a string")
                            .to_vec(),
                    )
                    .expect("every chunk is UTF-8"),
                );
                from = slot + 1;
            }
            out
        };
        assert_eq!(chunk("abcde", 2), ["ab", "cd", "e"]);
        assert_eq!(chunk("abc", 9), ["abc"]);
        assert!(chunk("", 2).is_empty());
        // Four bytes, one cluster: a byte-counted split would halve it.
        assert_eq!(chunk("é\u{0301}x", 1), ["é\u{0301}", "x"]);
    }

    /// The three trims share one option bag, so they can only differ in which
    /// end they strip. The default set is PHP's, and a written one replaces it
    /// rather than adding to it.
    #[test]
    fn the_three_trims_strip_the_ends_they_name() {
        let php_default = " \t\n\r\0\u{0b}";
        let trim = |member, subject, characters| {
            taken(run(member, &[s(subject), s(characters)]).expect("trimming never fails"))
        };
        assert_eq!(
            trim(super::nvs_core_str_trim, " \thi\n ", php_default),
            "hi"
        );
        assert_eq!(
            trim(super::nvs_core_str_trim_start, "  hi  ", php_default),
            "hi  "
        );
        assert_eq!(
            trim(super::nvs_core_str_trim_end, "  hi  ", php_default),
            "  hi"
        );
        assert_eq!(trim(super::nvs_core_str_trim, "xxhixx", "x"), "hi");
        // Only the characters named: a written set replaces the default.
        assert_eq!(trim(super::nvs_core_str_trim, " xhix ", "x"), " xhix ");
        // `rule:core-api/shape-rules` R13: `a..z` is three characters, not a range.
        assert_eq!(trim(super::nvs_core_str_trim, "abc", "a..z"), "bc");
    }

    /// `replace` with both options at their defaults, which is what a call
    /// site that writes no bag at all passes. Every row verified against PHP
    /// 8.5's `str_replace`.
    fn replaced(subject: &str, search: &str, replacement: &str) -> String {
        taken(
            run(
                super::nvs_core_str_replace,
                &[
                    s(subject),
                    s(search),
                    s(replacement),
                    Value::bool(false),
                    Value::uint(u64::MAX),
                ],
            )
            .expect("replace never fails"),
        )
    }

    #[test]
    fn replace_substitutes_every_occurrence_left_to_right() {
        assert_eq!(replaced("a-b-c", "-", "+"), "a+b+c");
        // The replacement is never rescanned, so this is "aa" and not "a".
        assert_eq!(replaced("aaa", "aa", "a"), "aa");
        assert_eq!(replaced("abc", "z", "y"), "abc");
        // An empty search replaces nothing rather than looping.
        assert_eq!(replaced("abc", "", "x"), "abc");
        assert_eq!(replaced("", "a", "b"), "");
    }

    /// `after` answers the slice past the first occurrence, or past the last
    /// under `{last: true}`, with the needle left out; absence is `null` and a
    /// needle at the very end is the empty string, so the two stay apart.
    // covers: Core\Str::after
    #[test]
    fn after_cuts_past_the_first_or_last_needle_and_answers_null_when_absent() {
        let after = |subject: &str, needle: &str, last: bool| -> Option<String> {
            let result = run(
                super::nvs_core_str_after,
                &[s(subject), s(needle), Value::bool(last)],
            )
            .expect("after never fails");
            match result.tag() {
                Some(nvs_runtime::Tag::Null) => None,
                _ => Some(taken(result)),
            }
        };
        assert_eq!(after("key=a=b", "=", false).as_deref(), Some("a=b"));
        assert_eq!(after("key=a=b", "=", true).as_deref(), Some("b"));
        assert_eq!(after("key=", "=", false).as_deref(), Some(""));
        assert_eq!(after("key", "=", false), None);
        assert_eq!(after("key", "=", true), None);
        // Case-sensitive: the option set is `last` alone.
        assert_eq!(after("Name: x", "name", false), None);
        // A needle longer than the subject is absent, not an error.
        assert_eq!(after("ab", "abc", true), None);
        // Multi-byte text is cut on the needle's own bytes.
        assert_eq!(after("größe: 5", "ß", false).as_deref(), Some("e: 5"));
    }

    /// `before` is `after`'s mirror over the same `cut_at`: the slice up to the
    /// first occurrence, or up to the last under `{last: true}`, and `null`
    /// for absence while a needle at the very start answers the empty string.
    // covers: Core\Str::before
    #[test]
    fn before_cuts_up_to_the_first_or_last_needle_and_answers_null_when_absent() {
        let before = |subject: &str, needle: &str, last: bool| -> Option<String> {
            let result = run(
                super::nvs_core_str_before,
                &[s(subject), s(needle), Value::bool(last)],
            )
            .expect("before never fails");
            match result.tag() {
                Some(nvs_runtime::Tag::Null) => None,
                _ => Some(taken(result)),
            }
        };
        assert_eq!(before("a.b.c", ".", false).as_deref(), Some("a"));
        assert_eq!(before("a.b.c", ".", true).as_deref(), Some("a.b"));
        assert_eq!(before(".hidden", ".", false).as_deref(), Some(""));
        assert_eq!(before("abc", ".", false), None);
        assert_eq!(before("abc", ".", true), None);
        assert_eq!(before("Key: x", "key", false), None);
        assert_eq!(before("größe: 5", "ß", true).as_deref(), Some("grö"));
        // Overlapping occurrences are all walked, so the last "aa" starts at 2.
        assert_eq!(before("aaaa", "aa", true).as_deref(), Some("aa"));
    }

    /// The `caseInsensitive` option is what makes this member subsume
    /// `str_ireplace` as well, and the match's byte length comes out of the
    /// subject — the whole reason `find_from` returns one.
    #[test]
    fn replace_is_case_insensitive_only_when_the_option_says_so() {
        let run_ci = |ci: bool| {
            taken(
                run(
                    super::nvs_core_str_replace,
                    &[
                        s("Hello HELLO hello"),
                        s("hello"),
                        s("hi"),
                        Value::bool(ci),
                        Value::uint(u64::MAX),
                    ],
                )
                .expect("replace never fails"),
            )
        };
        assert_eq!(run_ci(false), "Hello HELLO hi");
        assert_eq!(run_ci(true), "hi hi hi");
    }

    /// `limit` counts replacements, and `0` means none — the consequence of
    /// spelling "every occurrence" as `uint`'s maximum rather than as a
    /// sentinel zero.
    #[test]
    fn replace_stops_after_the_limit_and_does_nothing_at_zero() {
        let capped = |limit: u64| {
            taken(
                run(
                    super::nvs_core_str_replace,
                    &[
                        s("a-b-c-d"),
                        s("-"),
                        s("+"),
                        Value::bool(false),
                        Value::uint(limit),
                    ],
                )
                .expect("replace never fails"),
            )
        };
        assert_eq!(capped(0), "a-b-c-d");
        assert_eq!(capped(1), "a+b-c-d");
        assert_eq!(capped(2), "a+b+c-d");
        assert_eq!(capped(99), "a+b+c+d");
    }

    /// The separator lands between elements even when one of them is empty —
    /// the case a "have I written anything yet" flag would get wrong.
    #[test]
    fn join_separates_an_empty_leading_element_too() {
        let mut parts = NvsArray::new();
        parts.set(NvsStr::new(b"0"), s(""));
        parts.set(NvsStr::new(b"1"), s("b"));
        assert_eq!(
            taken(
                run(super::nvs_core_str_join, &[Value::array(parts), s("-")]).expect("no failure")
            ),
            "-b"
        );
    }

    #[test]
    fn joining_nothing_is_the_empty_string() {
        assert_eq!(
            taken(
                run(
                    super::nvs_core_str_join,
                    &[Value::array(NvsArray::new()), s(",")]
                )
                .expect("no failure")
            ),
            ""
        );
    }

    /// Every row verified against PHP 8.5's own `str_pad`.
    #[test]
    fn padding_matches_php_including_the_truncated_run() {
        for (subject, length, padding, start, end) in [
            ("7", 3u64, "0", "007", "700"),
            ("abc", 2, "0", "abc", "abc"),
            ("ab", 7, "xyz", "xyzxyab", "abxyzxy"),
        ] {
            assert_eq!(
                taken(
                    run(
                        super::nvs_core_str_pad_start,
                        &[s(subject), Value::uint(length), s(padding)]
                    )
                    .expect("no failure")
                ),
                start
            );
            assert_eq!(
                taken(
                    run(
                        super::nvs_core_str_pad_end,
                        &[s(subject), Value::uint(length), s(padding)]
                    )
                    .expect("no failure")
                ),
                end
            );
        }
    }

    #[test]
    fn padding_with_an_empty_run_throws_rather_than_looping() {
        let status = run(
            super::nvs_core_str_pad_start,
            &[s("ab"), Value::uint(5), s("")],
        )
        .expect_err("an empty padding can never reach the length");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    #[test]
    fn repeating_zero_times_is_the_empty_string() {
        assert_eq!(
            taken(run(super::nvs_core_str_repeat, &[s("ab"), Value::uint(0)]).expect("no failure")),
            ""
        );
        assert_eq!(
            taken(run(super::nvs_core_str_repeat, &[s("ab"), Value::uint(3)]).expect("no failure")),
            "ababab"
        );
    }

    /// A repetition too large to allocate is a throw, not the panic
    /// `str::repeat` would raise — contained either way, but only one of the
    /// two is something a program can catch.
    #[test]
    fn an_unrepresentable_repetition_throws() {
        let status = run(
            super::nvs_core_str_repeat,
            &[s("ab"), Value::uint(u64::MAX)],
        )
        .expect_err("no string that long can exist");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    #[test]
    fn a_non_string_argument_is_a_contained_fault() {
        let status =
            run(super::nvs_core_str_upper, &[Value::int(7)]).expect_err("an int is not a string");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    #[test]
    fn is_empty_answers_for_both_shapes() {
        assert_eq!(
            run(super::nvs_core_str_is_empty, &[s("")])
                .expect("no failure")
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            run(super::nvs_core_str_is_empty, &[s("a")])
                .expect("no failure")
                .as_bool(),
            Some(false)
        );
    }

    /// `length` counts characters, which is the answer PHP needs two functions
    /// and a correct `mb_internal_encoding` to reach — and does not reach for
    /// the last row at all, since `strlen` says 25 and `mb_strlen` says 5.
    // covers: Core\Str::length
    #[test]
    fn length_counts_characters_not_bytes_or_code_points() {
        for (subject, want) in [
            ("", 0u64),
            ("nvs", 3),
            ("cafe\u{301}", 4),
            ("\u{1f1e6}\u{1f1f9}", 1),
            ("\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}", 1),
        ] {
            assert_eq!(
                run(super::nvs_core_str_length, &[s(subject)])
                    .expect("no failure")
                    .as_uint(),
                Some(want),
                "length({subject:?})"
            );
        }
    }

    /// `at` addresses the same unit `length` counts, from either end.
    #[test]
    fn at_indexes_characters_from_either_end() {
        for (subject, index, want) in [
            ("nvs", 0i64, "n"),
            ("nvs", 2, "s"),
            ("nvs", -1, "s"),
            ("cafe\u{301}", 3, "e\u{301}"),
            ("cafe\u{301}", -1, "e\u{301}"),
        ] {
            assert_eq!(
                taken(
                    run(super::nvs_core_str_at, &[s(subject), Value::int(index)])
                        .expect("no failure")
                ),
                want,
                "at({subject:?}, {index})"
            );
        }
    }

    /// An index outside the string throws rather than answering `""` the way
    /// PHP's `$s[$i]` does — the return type is `string`, so there is nothing
    /// for an absence to be.
    #[test]
    fn an_index_outside_the_string_throws() {
        for index in [3i64, -4, i64::MAX, i64::MIN] {
            let status = run(super::nvs_core_str_at, &[s("nvs"), Value::int(index)])
                .expect_err("the index addresses nothing");
            assert_eq!(status, nvs_runtime::THROWN, "at(\"nvs\", {index})");
        }
        let status = run(super::nvs_core_str_at, &[s(""), Value::int(0)])
            .expect_err("the empty string has no characters");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    /// [`super::NORMAL_FORM`]'s integers are ABI, and this is what says so:
    /// each case's own constant, fed to the member as compiled code would
    /// write it, must answer in the form of that name.
    ///
    /// One subject distinguishes all four, which is the property that makes
    /// the check worth anything — `ﬁ` is a compatibility ligature (only a `K`
    /// form unifies it with `fi`) and `é` is precomposed (only a `D` form
    /// splits it into `e` plus a combining acute).
    #[test]
    fn the_normal_form_enum_and_its_rust_twin_are_one_roster() {
        for ((case, value), want) in super::NORMAL_FORM.cases.iter().zip([
            ("Nfc", "\u{fb01}\u{e9}"),
            ("Nfd", "\u{fb01}e\u{301}"),
            ("Nfkc", "fi\u{e9}"),
            ("Nfkd", "fie\u{301}"),
        ]) {
            assert_eq!(*case, want.0, "the case table's order changed");
            assert_eq!(
                taken(
                    run(
                        super::nvs_core_str_normalize,
                        &[s("\u{fb01}\u{e9}"), Value::int(*value)],
                    )
                    .expect("normalize never fails")
                ),
                want.1,
                "{case}"
            );
        }
        // The whole roster, so a fifth case added without a branch is a
        // failure here rather than a fatal at the first call site.
        let status = run(
            super::nvs_core_str_normalize,
            &[
                s("é"),
                Value::int(
                    i64::try_from(super::NORMAL_FORM.cases.len()).expect("four fits in an i64"),
                ),
            ],
        )
        .expect_err("an integer that is no case is a contract violation");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    /// Every row verified against PHP 8.5's `wordwrap`, which [`super::wrapped`]
    /// reproduces one unit at a time instead of one byte at a time.
    #[test]
    fn wrapping_matches_phps_wordwrap() {
        for (subject, width, cut, want) in [
            ("one two three four", 9, false, "one two\nthree\nfour"),
            (
                "The quick brown fox sat over the lazy dog",
                15,
                false,
                "The quick brown\nfox sat over\nthe lazy dog",
            ),
            (
                "A very looooooooooooong word.",
                8,
                false,
                "A very\nlooooooooooooong\nword.",
            ),
            (
                "A very looooooooooooong word.",
                8,
                true,
                "A very\nlooooooo\noooooong\nword.",
            ),
            // A break already in the subject restarts the measurement.
            (
                "already\nwrapped text here",
                9,
                false,
                "already\nwrapped\ntext here",
            ),
            ("", 5, false, ""),
            // No space to break at, and no cutting: the subject comes back
            // whole even at a width of zero.
            ("abc", 0, false, "abc"),
        ] {
            assert_eq!(
                super::wrapped(subject, width, "\n", cut),
                want,
                "wrap({subject:?}, {width}, cut = {cut})"
            );
        }
    }

    /// The width counts characters, not bytes — the one thing `wrap` does not
    /// inherit from PHP, and the reason a wrapped column of accented text lines
    /// up here and does not there.
    #[test]
    fn wrapping_measures_the_same_unit_length_counts() {
        // Each word is four characters and six bytes, so a byte-counting wrap
        // would break after the first one.
        assert_eq!(
            super::wrapped("a\u{301}a\u{301} b\u{301}b\u{301}", 9, "\n", false),
            "a\u{301}a\u{301} b\u{301}b\u{301}"
        );
    }

    /// An empty needle matches at every position, so a scan that advanced by
    /// the match's own length would never terminate — [`super::after_match`] is
    /// what keeps both scans finite, and this is the shape that would hang.
    #[test]
    fn an_empty_needle_terminates_every_scan() {
        let last = run(
            super::nvs_core_str_last_index_of,
            &[
                s("caf\u{e9}"),
                s(""),
                Value::int(i64::MAX),
                Value::bool(false),
            ],
        )
        .expect("no failure");
        assert_eq!(last.as_uint(), Some(4));
        // `countOf` has no answer for it at all, so it throws rather than
        // reporting the character count.
        let status = run(super::nvs_core_str_count_of, &[s("abc"), s("")])
            .expect_err("an empty needle is refused");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    /// Padding measures in the same unit as `length`, so a target of 3 over a
    /// one-character emoji adds two pads rather than the twenty-one bytes a
    /// byte-counting `str_pad` would.
    #[test]
    fn padding_measures_the_same_unit_length_counts() {
        assert_eq!(
            taken(
                run(
                    super::nvs_core_str_pad_start,
                    &[s("\u{1f1e6}\u{1f1f9}"), Value::uint(3), s(".")]
                )
                .expect("no failure")
            ),
            "..\u{1f1e6}\u{1f1f9}"
        );
    }

    /// Every row here was checked against `php -r` while it was written.
    /// `{natural: true}` is a port of `strnatcmp` rather than a fresh reading
    /// of "sort numbers as numbers", and its three quirks are exactly where a
    /// fresh reading would disagree with the program being migrated: the
    /// leading-zero skip applies at the start of a subject and nowhere else, a
    /// run that begins with `0` compares as a fraction, and a whitespace run
    /// is insignificant everywhere except where it ends one subject before the
    /// other.
    #[test]
    fn natural_order_answers_what_strnatcmp_answers() {
        let rows: &[(&str, &str, i64)] = &[
            ("img12", "img2", 1),
            ("01", "1", 0),
            ("a01", "a1", -1),
            ("a0010", "a10", -1),
            ("1.5", "1.10", -1),
            (" 1", "1", 0),
            ("a b", "a  b", 0),
            ("1 2", "1  2", 0),
            ("x", "x ", -1),
            ("1\t", "1 ", -1),
            ("a\t", "a ", 0),
            (" ", "", 1),
            ("9", "10 ", -1),
            ("1a", "1 a", 1),
            ("v1.0", "v1.0.0", -1),
        ];
        for &(left, right, want) in rows {
            let answer = run(
                super::nvs_core_str_compare,
                &[s(left), s(right), Value::bool(false), Value::bool(true)],
            )
            .expect("no failure");
            assert_eq!(answer.as_int(), Some(want), "compare({left:?}, {right:?})");
        }
    }
}
