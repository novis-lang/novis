//! `Core\Bytes` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 7's second class, over `nvs_runtime`'s `Bytes`-tagged `NvsStr`.
//!
//! Every member here is pure (`rule:core-api/shape-rules` R3) and borrows its subject rather than
//! consuming it, exactly as [`crate::str`] does — see [`crate`]'s own docs for
//! why that falls out of being a helper.
//!
//! # What is registered here so far
//!
//! § 7's indexing half — `length`, `at`, `slice`, `indexOf`, `compare` — its
//! three predicates, all three of its builders — `fill`, `repeat` and `join` —
//! and `pack`/`unpack`. **§ 7's `Core\Bytes` is complete**, so what is left of
//! that section is nothing.
//!
//! **`join`'s `$separator = ""` is why [`crate::registry::Const`] has a
//! `Bytes` variant.** `Const::Str("")` would materialize a `Str`-tagged value
//! into a `bytes` parameter, which is a type lie this helper would have to
//! `FATAL` on, so the default is written as the octets it means and reaches
//! the call site as `nvs_ir::ir::InstKind::ConstBytes` — the only way a
//! `bytes` constant enters a program, since `rule:types/bytes` gives the language no
//! `bytes` literal.
//!
//! # The unit is the byte, and that is the whole difference from `Core\Str`
//!
//! § 7 pairs each member below with [`crate::str`]'s member of the same name,
//! and R6 makes the names identical because the *operation* is identical. What
//! differs is the unit: `Core\Str` counts in
//! [`crate::granularity::DEFAULT`]'s grapheme clusters, and this counts in
//! bytes — which `rule:types/bytes`
//! says is the only unit `bytes` has to be ambiguous about.
//!
//! So these bodies are *simpler* than `str.rs`'s rather than a copy of them.
//! There is no segmentation pass, no byte-offset-to-unit-index conversion at
//! either end, and no `unicode-segmentation` on this path at all: an offset a
//! caller gives is already the offset the buffer is indexed at. Every member
//! below is therefore O(n) at worst with no allocation except the one its
//! result needs, and `length` is O(1) where `Core\Str::length` is O(n).
//!
//! # Three shapes decided here, because the spec names members and not rows
//!
//! § 7 gives `Core\Bytes` as a prose list of member names, so the signatures
//! are this module's to settle. Each is decided under `Core\Str`'s
//! corresponding row plus what a byte string can actually mean:
//!
//! - **`at` answers a `uint`, not a one-element `bytes`.** `Core\Str::at`
//!   answers a `string` because a character *is* a string and there is no
//!   narrower type to hand back. A byte has one: `uint`. Answering
//!   `bytes` instead would force every caller that wants to compare against a
//!   magic byte — which is what R6's *"the three predicates are what
//!   magic-byte sniffing needs"* says this class is for — through a
//!   one-element buffer allocation and a second call to read the number out of
//!   it. An out-of-range index throws, which is `Core\Str::at`'s answer and
//!   `rule:core-api/shape-rules` R4/R5's: absence would have to be spelled `?uint` in the type.
//!   **That `uint` answer is why `at` is the one member here classified
//!   against the shape of its return type.** `rule:security/unclassified-parameter-refuses-tainted`'s rule — written out
//!   on [`crate::registry::Qual`] — makes a member `Neutral` when its answer
//!   carries no byte of any argument, and every other `uint`-returning member
//!   in this class and in `Core\Str` is one: a length, a position, an
//!   ordering. `at`'s answer *is* a byte of the subject, only spelled as a
//!   number, and `fill` plus `join` reassemble a buffer from those numbers —
//!   so it is `Contagious`, and whoever implements propagation owes the
//!   question of what a qualified `uint` is rather than inheriting a silent
//!   launderer.
//! - **`indexOf` takes `{from?: int}` and no `caseInsensitive`.**
//!   `Core\Str::indexOf`'s second option is a Unicode case folding, and there
//!   is no case in a byte string — a `bytes` carries no charset, which is the
//!   premise of § 7's whole `Core\Encoding`/`Core\Bytes` split. An ASCII-only
//!   folding would be that guess made silently, which `rule:types/conversion` removes
//!   from the language. A caller who wants one decodes first. `from` survives
//!   unchanged, because a scan resuming where the last one stopped is exactly
//!   what a sniffing loop is.
//! - **`compare` answers `int`, negative/zero/positive, never a `bool`.** It
//!   is `memcmp`'s question and PHP's `strcmp`'s: lexicographic over unsigned
//!   octets. `Core\Str` has no `compare` yet, and when it grows one it takes
//!   this shape over its own unit. This is *not* the `==` operator
//!   (`rule:expressions/one-equality-operator`),
//!   which already compares two `bytes` for equality and is what a program
//!   should write when that is the question; `compare` exists for the ordering
//!   `==` does not answer.
//! - **`fill` is `fill(uint $length, uint $byte)`, length first.** It is the
//!   one member with no subject at all — it builds a buffer rather than
//!   answering about one — so `rule:core-api/shape-rules` R1 does not order it and `Core\Str`'s
//!   nearest row does: `padStart(string $s, uint $length, string $padding)`
//!   writes the size before the thing repeated into it, and a zero-filled
//!   header reads as `Core\Bytes::fill(16, 0)`. A `$byte` above 255 throws
//!   rather than truncating, which is R4 — the parameter is `uint` because
//!   that is what `at` answers, so the two compose.
//!
//! # `pack`'s format is a closed grammar, and every code has one meaning
//!
//! § 7 says `pack` replaces PHP's, and makes its format string an `rule:expressions/intrinsic-literals`
//! intrinsic and a sink — but it does not write the code table, so that is
//! settled here. PHP's is taken as the starting point and **narrowed to the
//! codes that name a wire format outright**, because a format string is a
//! description of octets on a wire or on disk and a code that means "whatever
//! this machine does" describes nothing:
//!
//! | code | field |
//! |---|---|
//! | `a` / `A` / `Z` | a buffer, NUL-padded / space-padded / NUL-padded and NUL-terminated |
//! | `c` / `C` | one octet, read back signed / unsigned |
//! | `n` / `v` | 16 bits, most / least significant octet first |
//! | `N` / `V` | 32 bits, most / least significant octet first |
//! | `J` / `P` | 64 bits, most / least significant octet first |
//! | `G` / `g` | binary32, most / least significant octet first |
//! | `E` / `e` | binary64, most / least significant octet first |
//! | `x` | one NUL octet, consuming no argument |
//!
//! A code is optionally followed by a count, or by `*`. On a numeric field
//! that is **how many arguments it takes** (`*` is every one remaining); on a
//! buffer field it is **the field's width in octets** (`*` is the argument's
//! own length). Four decisions inside that:
//!
//! - **A bare code takes the whole argument, where PHP's takes one octet.**
//!   PHP defaults every repeater to 1, so `pack("a", "Hello")` is `"H"` — a
//!   truncation with no diagnostic, which is precisely the failure mode this
//!   member is being rewritten to remove.
//! - **An argument wider than its field throws**, rather than being cut to fit.
//! - **An integer field accepts the union of its width's signed and unsigned
//!   ranges** and writes two's complement, so `-1` and `4294967295` both write
//!   `ffffffff` under `N`. Nothing outside that range wraps — it throws, which
//!   is `rule:core-api/shape-rules` R4, and it is why `c` and `C` write the same octet and differ
//!   only in what `unpack` will read back.
//! - **A `mixed` argument is not converted.** An integer field takes an `int`
//!   or a `uint` and a float field takes a `float`; anything else throws
//!   naming the tag it got, because `mixed` is `rule:types/grammar`'s one unchecked
//!   position and a silent widening there is the language's own rule broken at
//!   a library boundary.
//!
//! `unpack` reads that same table backwards, one field at a time, and two of
//! its own decisions follow from sharing it:
//!
//! - **It answers a positional `array<mixed>`, where PHP's answers a
//!   name→value map.** PHP's `unpack` carries a *second* grammar for the
//!   names, and the property given up — `$fields["length"]` — is bought back
//!   as `unpack(pack($f, ...$v), $f) == $v`, exactly, with one format string
//!   meaning one thing in both directions.
//! - **Octets left over throw.** PHP ignores a tail the format did not
//!   describe; a binary parser that silently ignores what it was not told
//!   about is the failure AGENTS.md's priority 1 exists to refuse. A header
//!   read off a longer buffer is `Core\Bytes::slice` and then `unpack`, or a
//!   trailing `a*`.
//!
//! Ten of PHP's codes are refused, each throwing with the replacement named:
//! `s`/`S`/`i`/`I`/`l`/`L`/`q`/`Q` and `f`/`d` take the machine's width or
//! byte order, `h`/`H` are `Core\Encoding::fromHex` spelled twice, and
//! `X`/`@` move the cursor backwards or to an absolute position, which turns a
//! format into a small assembler for no reach a forward `x` does not have.
//!
//! **What it spends:** nothing per value. A `bytes` is the `NvsStr`
//! allocation it already was (`nvs-runtime`'s module doc § *`bytes` is a tag,
//! not a second heap shape*), `slice` allocates its result and the other four
//! members allocate nothing at all.

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// The class's fully-qualified name, written once.
pub(crate) const NAME: &str = r"Core\Bytes";

/// `Core\Bytes`'s registry rows, in the spec's own order.
///
/// Declared beside the implementations rather than in one flat table, so
/// adding a member touches this file and nothing else — see
/// [`crate::registry::CLASSES`], which grows one line per *class*.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "length",
            names: &["b"],
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_bytes_length",
            doc: Some(&LENGTH_DOC),
        },
        CoreMethod {
            name: "at",
            names: &["b", "index"],
            params: &[CoreTy::Blob(Qual::Contagious), CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_bytes_at",
            doc: Some(&AT_DOC),
        },
        CoreMethod {
            name: "slice",
            names: &["b", "offset", "length"],
            params: &[
                CoreTy::Blob(Qual::Contagious),
                CoreTy::Int,
                CoreTy::Nullable(&CoreTy::Int),
            ],
            defaults: &[Const::Null],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_bytes_slice",
            doc: Some(&SLICE_DOC),
        },
        CoreMethod {
            name: "indexOf",
            names: &["haystack", "needle"],
            params: &[
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Options(INDEX_OF_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "nvs_core_bytes_index_of",
            doc: Some(&INDEX_OF_DOC),
        },
        CoreMethod {
            name: "compare",
            names: &["a", "b"],
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_bytes_compare",
            doc: Some(&COMPARE_DOC),
        },
        CoreMethod {
            name: "contains",
            names: &["haystack", "needle"],
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_bytes_contains",
            doc: Some(&CONTAINS_DOC),
        },
        CoreMethod {
            name: "startsWith",
            names: &["b", "prefix"],
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_bytes_starts_with",
            doc: Some(&STARTS_WITH_DOC),
        },
        CoreMethod {
            name: "endsWith",
            names: &["b", "suffix"],
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_bytes_ends_with",
            doc: Some(&ENDS_WITH_DOC),
        },
        CoreMethod {
            name: "fill",
            names: &["length", "byte"],
            params: &[CoreTy::Uint, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_bytes_fill",
            doc: Some(&FILL_DOC),
        },
        CoreMethod {
            name: "repeat",
            names: &["b", "times"],
            params: &[CoreTy::Blob(Qual::Contagious), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_bytes_repeat",
            doc: Some(&REPEAT_DOC),
        },
        CoreMethod {
            name: "join",
            names: &["parts", "separator"],
            params: &[
                CoreTy::Array(&CoreTy::Bytes),
                CoreTy::Blob(Qual::Contagious),
            ],
            defaults: &[Const::Bytes(b"")],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_bytes_join",
            doc: Some(&JOIN_DOC),
        },
        CoreMethod {
            name: "pack",
            names: &["format", "values"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Variadic(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_bytes_pack",
            doc: Some(&PACK_DOC),
        },
        CoreMethod {
            name: "unpack",
            names: &["b", "format"],
            params: &[CoreTy::Blob(Qual::Contagious), CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "nvs_core_bytes_unpack",
            doc: Some(&UNPACK_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Bytes::length`'s reference card — `rule:core-api/reference-card`.
const LENGTH_DOC: MethodDoc = MethodDoc {
    short: "Counts the octets in `$b`, as `strlen` does on binary data; O(1), where \
            `Core\\Str::length` walks its subject.",
    params: &[ParamDoc {
        name: "b",
        desc: "The buffer to measure.",
        shape: &[],
    }],
    ret: "The byte count; `0` for the empty buffer.",
    errors: &[],
};

/// `Core\Bytes::at`'s reference card — `rule:core-api/reference-card`.
const AT_DOC: MethodDoc = MethodDoc {
    short: "Answers the one octet at `$index` of `$b` as a number, as `ord($s[$i])` does; a \
            negative index counts from the end.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The buffer.",
            shape: &[],
        },
        ParamDoc {
            name: "index",
            desc: "The byte's position, negative to count from the end.",
            shape: &[],
        },
    ],
    ret: "The octet, `0` to `255`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$index` addresses no byte of `$b`.",
    }],
};

/// `Core\Bytes::slice`'s reference card — `rule:core-api/reference-card`.
const SLICE_DOC: MethodDoc = MethodDoc {
    short: "Copies a window of `$b`, as `substr` does on binary data, counted in bytes.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The buffer.",
            shape: &[],
        },
        ParamDoc {
            name: "offset",
            desc: "Where the window starts; negative counts from the end, and one before the \
                   start clamps to it.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "How many bytes to take; negative stops that many short of the end, and \
                   `null`, the default, runs to the end.",
            shape: &[],
        },
    ],
    ret: "The window as a new buffer; the empty buffer for an offset past the end or a window \
          that closes before it opens.",
    errors: &[],
};

/// `Core\Bytes::indexOf`'s reference card — `rule:core-api/reference-card`.
const INDEX_OF_DOC: MethodDoc = MethodDoc {
    short: "Finds where `$needle` first occurs in `$haystack`, as `strpos` does on binary data, \
            as a byte offset `slice` takes directly. There is no case-insensitive option, \
            because a byte string has no case.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The buffer searched.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The bytes looked for; an empty needle is found where the search starts.",
            shape: &[],
        },
        ParamDoc {
            name: "from",
            desc: "The byte offset the search starts at; negative counts from the end, and \
                   the default is `0`.",
            shape: &[],
        },
    ],
    ret: "The byte offset of the first occurrence at or after `from`, or `null` when there is \
          none — never `false`.",
    errors: &[],
};

/// `Core\Bytes::compare`'s reference card — `rule:core-api/reference-card`.
const COMPARE_DOC: MethodDoc = MethodDoc {
    short: "Orders `$a` against `$b` lexicographically over unsigned octets, as `strcmp` does on \
            binary data; a shorter buffer that is a prefix of a longer one sorts first. For \
            equality alone, `==` is the operator.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The first buffer.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The second buffer.",
            shape: &[],
        },
    ],
    ret: "`-1`, `0` or `1` — exactly those three, as PHP 8's `strcmp` answers.",
    errors: &[],
};

/// `Core\Bytes::contains`'s reference card — `rule:core-api/reference-card`.
const CONTAINS_DOC: MethodDoc = MethodDoc {
    short: "Tells whether `$needle` occurs anywhere in `$haystack`, as `str_contains` does on \
            binary data.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The buffer searched.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The bytes looked for.",
            shape: &[],
        },
    ],
    ret: "`true` when it occurs; an empty needle is contained in every buffer, the empty one \
          included.",
    errors: &[],
};

/// `Core\Bytes::startsWith`'s reference card — `rule:core-api/reference-card`.
const STARTS_WITH_DOC: MethodDoc = MethodDoc {
    short: "Tells whether `$b` begins with `$prefix`, as `str_starts_with` does on binary data — \
            the member a magic-byte sniff writes.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The buffer.",
            shape: &[],
        },
        ParamDoc {
            name: "prefix",
            desc: "The bytes it must begin with.",
            shape: &[],
        },
    ],
    ret: "`true` when it does; an empty prefix begins every buffer.",
    errors: &[],
};

/// `Core\Bytes::endsWith`'s reference card — `rule:core-api/reference-card`.
const ENDS_WITH_DOC: MethodDoc = MethodDoc {
    short: "Tells whether `$b` ends with `$suffix`, as `str_ends_with` does on binary data.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The buffer.",
            shape: &[],
        },
        ParamDoc {
            name: "suffix",
            desc: "The bytes it must end with.",
            shape: &[],
        },
    ],
    ret: "`true` when it does; an empty suffix ends every buffer.",
    errors: &[],
};

/// `Core\Bytes::fill`'s reference card — `rule:core-api/reference-card`.
const FILL_DOC: MethodDoc = MethodDoc {
    short: "Builds a buffer of `$length` copies of the octet `$byte`, as the \
            `str_repeat(chr($b), $n)` idiom does; a zero-filled header is `fill(16, 0)`.",
    params: &[
        ParamDoc {
            name: "length",
            desc: "How many octets to write.",
            shape: &[],
        },
        ParamDoc {
            name: "byte",
            desc: "The octet repeated, `0` to `255`.",
            shape: &[],
        },
    ],
    ret: "The new buffer; the empty buffer for a length of `0`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$byte` is above `255`, or `$length` is larger than any buffer this process \
               could hold.",
    }],
};

/// `Core\Bytes::repeat`'s reference card — `rule:core-api/reference-card`.
const REPEAT_DOC: MethodDoc = MethodDoc {
    short: "Builds a buffer of `$b` repeated `$times` times, as `str_repeat` does on binary \
            data.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The buffer repeated.",
            shape: &[],
        },
        ParamDoc {
            name: "times",
            desc: "How many copies to write.",
            shape: &[],
        },
    ],
    ret: "The new buffer; the empty buffer for a count of `0` or an empty subject.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$times`, or the result, is larger than any buffer this process could hold.",
    }],
};

/// `Core\Bytes::join`'s reference card — `rule:core-api/reference-card`.
const JOIN_DOC: MethodDoc = MethodDoc {
    short: "Concatenates every buffer in `$parts` with `$separator` between neighbours, as \
            `implode` does on binary data — `Core\\Str::join`'s row over buffers, and this \
            class's only concatenation. No element is converted.",
    params: &[
        ParamDoc {
            name: "parts",
            desc: "The buffers to join, in order.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "The bytes written between neighbours; the empty buffer by default.",
            shape: &[],
        },
    ],
    ret: "The new buffer; the empty buffer for no parts.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result is larger than any buffer this process could hold.",
    }],
};

/// `Core\Bytes::pack`'s reference card — `rule:core-api/reference-card`.
const PACK_DOC: MethodDoc = MethodDoc {
    short: "Writes `$values` as the octets `$format` describes, as `pack` does, over a closed \
            code table: `a`/`A`/`Z` a buffer NUL-padded, space-padded, or NUL-padded and \
            NUL-terminated; `c`/`C` one octet; `n`/`v`, `N`/`V` and `J`/`P` 16, 32 and 64 bits \
            most or least significant octet first; `G`/`g` binary32 and `E`/`e` binary64 the \
            same way; `x` one NUL octet. A bare code takes the whole argument, never one \
            octet, and an integer field takes the union of its width's signed and unsigned \
            ranges in two's complement.",
    params: &[
        ParamDoc {
            name: "format",
            desc: "The format string — an intrinsic and a sink, so a literal — where each code \
                   may be followed by a count or `*`: how many arguments a numeric field \
                   takes, and how many octets wide a buffer field is.",
            shape: &[],
        },
        ParamDoc {
            name: "values",
            desc: "The values written, one per numeric field and one buffer per buffer field; \
                   an integer field takes an `int` or a `uint`, a float field a `float`, and \
                   nothing is converted.",
            shape: &[],
        },
    ],
    ret: "The packed buffer.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$format` holds a code outside the table — the machine-width codes `s`, `S`, \
               `i`, `I`, `l`, `L`, `q`, `Q`, `f` and `d`, the hex codes `h` and `H`, and the \
               cursor codes `X` and `@` are refused naming the replacement — or `x*`, or a \
               count larger than any buffer; a field's value is missing, of the wrong type, \
               outside its width's range, a `float` outside binary32's range, or a buffer \
               wider than its field; or the format writes fewer values than the call passed.",
    }],
};

/// `Core\Bytes::unpack`'s reference card — `rule:core-api/reference-card`.
const UNPACK_DOC: MethodDoc = MethodDoc {
    short: "Reads `$b` back through `$format`, as `unpack` does, over `pack`'s code table in \
            reverse — so `unpack(pack($f, ...$v), $f)` is `$v`, field for field. Octets the \
            format does not describe throw rather than being ignored.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The buffer read.",
            shape: &[],
        },
        ParamDoc {
            name: "format",
            desc: "The same format string `pack` takes, an intrinsic and a sink; there is no \
                   second grammar for field names.",
            shape: &[],
        },
    ],
    ret: "The fields as a positional list, never PHP's name-keyed map: an `int` for `c`, a \
          `uint` for every other integer code, a `float` for `G`/`g`/`E`/`e`, and a `bytes` \
          for a buffer code with `A`'s and `Z`'s padding taken back off.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$format` holds a code outside the table or a count larger than any buffer, a \
               field reads past the end of `$b`, or octets are left over after the last \
               field.",
    }],
};

/// `Core\Bytes::indexOf`'s `{from?: int}` — the module doc owns why
/// `Core\Str::indexOf`'s second option has no counterpart here.
///
/// `from` is a **position**, so it obeys `rule:core-api/shape-rules` R8's sign rule and reads
/// through [`offset`], the same way every other position in this module does.
/// Its default is the start of the subject, which is a search of the whole of
/// it.
const INDEX_OF_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "from",
    ty: CoreTy::Int,
    default: Const::Int(0),
}];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_bytes_length" => (nvs_core_bytes_length as *const ()).cast(),
        "nvs_core_bytes_at" => (nvs_core_bytes_at as *const ()).cast(),
        "nvs_core_bytes_slice" => (nvs_core_bytes_slice as *const ()).cast(),
        "nvs_core_bytes_index_of" => (nvs_core_bytes_index_of as *const ()).cast(),
        "nvs_core_bytes_compare" => (nvs_core_bytes_compare as *const ()).cast(),
        "nvs_core_bytes_contains" => (nvs_core_bytes_contains as *const ()).cast(),
        "nvs_core_bytes_starts_with" => (nvs_core_bytes_starts_with as *const ()).cast(),
        "nvs_core_bytes_ends_with" => (nvs_core_bytes_ends_with as *const ()).cast(),
        "nvs_core_bytes_fill" => (nvs_core_bytes_fill as *const ()).cast(),
        "nvs_core_bytes_repeat" => (nvs_core_bytes_repeat as *const ()).cast(),
        "nvs_core_bytes_join" => (nvs_core_bytes_join as *const ()).cast(),
        "nvs_core_bytes_pack" => (nvs_core_bytes_pack as *const ()).cast(),
        "nvs_core_bytes_unpack" => (nvs_core_bytes_unpack as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Reading arguments, and the two index rules
// ============================================================================

/// One `bytes` argument's octets.
///
/// A `FATAL` rather than a `THROWN` for the reason [`crate::str`]'s own
/// argument readers are: the checker placed this argument and compiled code
/// wrote the tag, so a mismatch is a runtime-contract violation rather than
/// anything a program can cause.
fn raw<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a [u8], Fault> {
    value.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Bytes::{member} expected {:?} for {position}, got tag {}",
            Tag::Bytes,
            value.tag_byte()
        ))
    })
}

/// One `int` argument, for [`raw`]'s reason.
fn integer(value: &Value, member: &str, position: &str) -> Result<i64, Fault> {
    value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Bytes::{member} expected {:?} for {position}, got tag {}",
            Tag::Int,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument, for [`raw`]'s reason.
fn unsigned(value: &Value, member: &str, position: &str) -> Result<u64, Fault> {
    value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Bytes::{member} expected {:?} for {position}, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument as a `usize`, refusing a count no buffer could reach.
fn count(value: &Value, member: &str, position: &str) -> Result<usize, Fault> {
    let raw = unsigned(value, member, position)?;
    usize::try_from(raw).map_err(|_| {
        Fault::thrown(format!(
            "Core\\Bytes::{member}: {position} is {raw}, which is larger than any buffer this \
             process could hold"
        ))
    })
}

/// [`nvs_runtime::affordable`] with this class's name already on the message.
///
/// That function's own docs own why the check lives there rather than here,
/// and what it will grow into once the M6 arena carries `[limits.hard]`.
fn affordable(bytes: Option<usize>, member: &str) -> Result<usize, Fault> {
    nvs_runtime::affordable(bytes, &format!("Core\\Bytes::{member}"))
}

/// A signed **position** as a byte offset into a subject of `total` bytes,
/// with a negative one counting from the end and either end saturating.
///
/// [`crate::granularity::Unit::byte_of_signed_index`]'s counterpart for a
/// subject whose unit is already the byte, and it saturates for that method's
/// reason: a search or a slice starting past the end finds nothing, which
/// composes with a loop where a throw would not. `rule:core-api/shape-rules` R8 is the sign rule.
fn offset(total: usize, index: i64) -> usize {
    let from_start = if index < 0 {
        i64::try_from(total)
            .unwrap_or(i64::MAX)
            .saturating_add(index)
            .max(0)
    } else {
        index
    };
    usize::try_from(from_start).unwrap_or(usize::MAX).min(total)
}

/// A signed index as the byte it **addresses**, or `None` when it addresses
/// nothing.
///
/// [`offset`]'s strict twin, and the difference is the whole reason both
/// exist: a position may sit one past the end and mean "the empty tail", where
/// an index must name a byte that is there. `Core\Bytes::at` is the one member
/// that asks the strict question, and it turns the `None` into a throw.
fn addressed(total: usize, index: i64) -> Option<usize> {
    let from_start = if index < 0 {
        i64::try_from(total).ok()?.checked_add(index)?
    } else {
        index
    };
    usize::try_from(from_start)
        .ok()
        .filter(|found| *found < total)
}

/// Where `needle` first occurs in `haystack`, in bytes.
///
/// An empty needle occurs at 0, which is Rust's answer and PHP 8's. The scan
/// is the naive one: `Core\Bytes` sniffs magic numbers, so the needle is a
/// handful of bytes and a skip table would cost more to build than the scan it
/// saves — `Core\Regex` is where a subject wants a real engine.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// A byte count or position as the `uint` a member answers with.
fn counted(value: usize, member: &str) -> Result<u64, Fault> {
    // `try_from` rather than `as`: `usize` is no wider than `u64` on any target
    // `deny.toml` builds for, so this cannot lose a digit, and spelling it this
    // way keeps the cast lints this crate denies from needing a silence.
    u64::try_from(value)
        .map_err(|_| Fault::fatal(format!("Core\\Bytes::{member} counted past `uint`")))
}

/// A freshly built `bytes` result.
fn produced(octets: &[u8]) -> HelperResult {
    Ok(Value::bytes(NvsStr::new(octets)))
}

/// [`produced`], answering rather than aborting when the allocator refuses.
///
/// [`NvsStr::new`] aborts, so a member that built its octets fallibly would
/// still die at the copy out — the second allocation is exactly as able to
/// fail as the first was, and at that moment both are live.
fn produced_fallibly(octets: &[u8], member: &str) -> HelperResult {
    NvsStr::try_build(octets.len(), |out| out.push(octets))
        .map(Value::bytes)
        .ok_or_else(|| too_large(member))
}

/// The sentence a result the allocator will not serve refuses with.
///
/// [`affordable`] is the *policy* seam and refuses only a size past
/// `isize::MAX`; this is the allocator itself refusing a size that seam
/// allowed. They answer different questions, and the second is the difference
/// between a throw and an abort that takes every in-flight request with it —
/// `nvs_core_random_bytes` runs the same two for the same reason.
fn too_large(member: &str) -> Fault {
    Fault::thrown(format!(
        "Core\\Bytes::{member}: the result is larger than any buffer this process could hold"
    ))
}

/// Room in `out` for a result of `size` octets, or [`too_large`].
///
/// `try_reserve` and not `try_reserve_exact`: on the empty `Vec` that `fill`
/// and `repeat` start from the two reserve the same thing, while
/// `Core\Bytes::join` reaches here once per part, where an exact reservation
/// would reallocate on every one of them.
fn reserved(out: &mut Vec<u8>, size: usize, member: &str) -> Result<(), Fault> {
    out.try_reserve(size.saturating_sub(out.len()))
        .map_err(|_| too_large(member))
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::length(bytes $b): uint` — how many octets the buffer
    /// holds, replacing PHP's `strlen` used on binary data.
    ///
    /// O(1), and this is the member where the byte unit pays off most
    /// visibly: `Core\Str::length` walks the subject to count grapheme
    /// clusters, and this reads a header field.
    fn nvs_core_bytes_length(_ctx, args: [1]) {
        let subject = raw(&args[0], "length", "the subject")?;
        Ok(Value::uint(counted(subject.len(), "length")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::at(bytes $b, int $index): uint` — the one octet at
    /// `$index`, replacing PHP's `ord($s[$i])`.
    ///
    /// The two shapes that separate it from `Core\Str::at` — a `uint` answer,
    /// and an index counted in bytes — are this module's own docs' second and
    /// third headings. A negative index counts from the end (`rule:core-api/shape-rules` R8's
    /// range rule applied to a range of one), and an index that addresses
    /// nothing throws rather than answering a sentinel.
    fn nvs_core_bytes_at(_ctx, args: [2]) {
        let subject = raw(&args[0], "at", "the subject")?;
        let index = integer(&args[1], "at", "the index")?;
        let found = addressed(subject.len(), index).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Bytes::at: index {index} is outside a buffer of {} bytes",
                subject.len()
            ))
        })?;
        Ok(Value::uint(u64::from(subject[found])))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::slice(bytes $b, int $offset, ?int $length = null): bytes`
    /// — replacing PHP's `substr` used on binary data.
    ///
    /// `Core\Str::slice`'s rules, counted in bytes rather than in grapheme
    /// clusters, so both arguments follow `rule:core-api/shape-rules` R8's sign rule:
    ///
    /// * A **negative offset** counts from the end, and one before the start
    ///   clamps to it.
    /// * A **negative length** stops that many bytes short of the end; a
    ///   window that closes before it opens is the empty buffer.
    /// * An omitted length runs to the end — a `?int` saying so in the type
    ///   rather than through a sentinel, which is R5.
    ///
    /// An offset past the end is the empty buffer rather than a throw, which
    /// is PHP 8's answer and the one that composes with a loop.
    fn nvs_core_bytes_slice(_ctx, args: [3]) {
        let subject = raw(&args[0], "slice", "the subject")?;
        let total = subject.len();
        let start = offset(total, integer(&args[1], "slice", "the offset")?);

        let end = match args[2].tag() {
            Some(Tag::Null) => total,
            _ => {
                let length = integer(&args[2], "slice", "the length")?;
                if length < 0 {
                    // Counted from the *end*, not from the start: this is the
                    // one place R8's sign rule means "stop short of" rather
                    // than "begin at".
                    offset(total, length)
                } else {
                    let span = usize::try_from(length).unwrap_or(usize::MAX);
                    start.saturating_add(span).min(total)
                }
            }
        };

        // `get` rather than an index: a window that closes before it opens is
        // `None` here, and the empty buffer is what R8 says it answers.
        produced(subject.get(start..end).unwrap_or(&[]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::indexOf(bytes $haystack, bytes $needle, {from?: int}): ?uint`
    /// — replacing PHP's `strpos` used on binary data.
    ///
    /// **Absence is `null`, not `false`**, which is `rule:core-api/shape-rules` R5 and the same
    /// correctness win it is on `Core\Str::indexOf`: a `?uint` has no falsy
    /// member that `0` could be confused with, so PHP's `strpos(…) == false`
    /// bug family cannot be written.
    ///
    /// The answer is a byte offset, so it is directly usable as this class's
    /// own `slice` offset — the property that would break if the two members
    /// counted in different units, which is exactly what `Core\Str` has to work
    /// to preserve and this class gets for free.
    fn nvs_core_bytes_index_of(_ctx, args: [3]) {
        let subject = raw(&args[0], "indexOf", "the subject")?;
        let needle = raw(&args[1], "indexOf", "the needle")?;
        let from = integer(&args[2], "indexOf", "the `from` option")?;

        let start = offset(subject.len(), from);
        match subject
            .get(start..)
            .and_then(|rest| find(rest, needle))
            .map(|at| start + at)
        {
            None => Ok(Value::null()),
            Some(at) => Ok(Value::uint(counted(at, "indexOf")?)),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::compare(bytes $a, bytes $b): int` — lexicographic order
    /// over unsigned octets, replacing PHP's `strcmp` used on binary data.
    ///
    /// Negative, zero or positive, and this implementation answers exactly
    /// `-1`, `0` or `1` — PHP 8 narrowed `strcmp` to the same three values, so
    /// there is no magnitude for a ported program to have depended on. A
    /// shorter buffer that is a prefix of a longer one sorts before it, which
    /// is `memcmp` plus the length tiebreak and Rust's own slice ordering.
    ///
    /// This is not the `==` operator: the module doc's third heading owns why
    /// both exist.
    fn nvs_core_bytes_compare(_ctx, args: [2]) {
        let left = raw(&args[0], "compare", "the first buffer")?;
        let right = raw(&args[1], "compare", "the second buffer")?;
        Ok(Value::int(match left.cmp(right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::contains(bytes $haystack, bytes $needle): bool` —
    /// replacing PHP's `str_contains` used on binary data.
    ///
    /// The first of the three predicates R6 says this class exists for: a
    /// magic-byte check is a `bool` question, and routing it through
    /// `indexOf(...) != null` would make the caller handle an absence it does
    /// not care about. An empty needle is contained in every buffer, including
    /// the empty one — Rust's answer and PHP 8's, and `Core\Str::contains`'s.
    fn nvs_core_bytes_contains(_ctx, args: [2]) {
        let haystack = raw(&args[0], "contains", "the subject")?;
        let needle = raw(&args[1], "contains", "the needle")?;
        Ok(Value::bool(find(haystack, needle).is_some()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::startsWith(bytes $b, bytes $prefix): bool` — replacing
    /// PHP's `str_starts_with` used on binary data, and the member a format
    /// sniff actually writes.
    fn nvs_core_bytes_starts_with(_ctx, args: [2]) {
        let subject = raw(&args[0], "startsWith", "the subject")?;
        let prefix = raw(&args[1], "startsWith", "the prefix")?;
        Ok(Value::bool(subject.starts_with(prefix)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::endsWith(bytes $b, bytes $suffix): bool` — replacing PHP's
    /// `str_ends_with` used on binary data.
    fn nvs_core_bytes_ends_with(_ctx, args: [2]) {
        let subject = raw(&args[0], "endsWith", "the subject")?;
        let suffix = raw(&args[1], "endsWith", "the suffix")?;
        Ok(Value::bool(subject.ends_with(suffix)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::fill(uint $length, uint $byte): bytes` — a buffer of
    /// `$length` copies of one octet, replacing PHP's `str_repeat(chr($b), $n)`
    /// idiom.
    ///
    /// The module doc's fourth heading owns the argument order and why `$byte`
    /// is a `uint`. **A value above 255 throws** rather than being truncated to
    /// its low octet, which is `rule:core-api/shape-rules` R4: a caller who computed 256 has a
    /// bug, and PHP's `chr()` wrapping it to `"\0"` is the silent-substitution
    /// failure this language does not do.
    ///
    /// Zero length is the empty buffer, as `Core\Str::repeat`'s zero count is
    /// the empty string.
    fn nvs_core_bytes_fill(_ctx, args: [2]) {
        let length = count(&args[0], "fill", "the length")?;
        let byte = unsigned(&args[1], "fill", "the byte")?;
        let octet = u8::try_from(byte).map_err(|_| {
            Fault::thrown(format!(
                "Core\\Bytes::fill: {byte} is not one octet — a byte is 0 to 255"
            ))
        })?;
        affordable(Some(length), "fill")?;
        let mut octets: Vec<u8> = Vec::new();
        reserved(&mut octets, length, "fill")?;
        octets.resize(length, octet);
        produced_fallibly(&octets, "fill")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::repeat(bytes $b, uint $times): bytes` — replacing PHP's
    /// `str_repeat` used on binary data. Zero times is the empty buffer, as in
    /// PHP and as in `Core\Str::repeat`.
    ///
    /// **An empty subject short-circuits the loop rather than running it.**
    /// Both size checks are about how large the result is, so an empty subject
    /// passes them for every count there is — and the loop would then run a
    /// caller-supplied `uint` of iterations appending nothing, which is an
    /// unbounded spin on the request path for an answer already known.
    /// `Core\Str::repeat` carries the same guard for the same reason.
    fn nvs_core_bytes_repeat(_ctx, args: [2]) {
        let subject = raw(&args[0], "repeat", "the subject")?;
        let times = count(&args[1], "repeat", "the repeat count")?;
        let size = affordable(subject.len().checked_mul(times), "repeat")?;
        let mut octets: Vec<u8> = Vec::new();
        reserved(&mut octets, size, "repeat")?;
        let runs = if subject.is_empty() { 0 } else { times };
        for _ in 0..runs {
            octets.extend_from_slice(subject);
        }
        produced_fallibly(&octets, "repeat")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::join(array<bytes> $parts, bytes $separator = ""): bytes` —
    /// `Core\Str::join`'s row over buffers, which is why it is `join` here
    /// rather than a `concat` of its own: `rule:core-api/shape-rules` R6 pairs this class's
    /// members with `Core\Str`'s by name, and spec § 7 says so outright.
    ///
    /// The separator's default is the empty buffer, materialized at the call
    /// site from `nvs_stdlib::registry::Const::Bytes` exactly as
    /// `Core\Str::join`'s is from `Const::Str` — so this body always receives
    /// two arguments and knows nothing about defaults.
    ///
    /// **No element is ever converted.** `Core\Str::join` renders each element
    /// as text; here every element is already a buffer, so a wrong tag is a
    /// miscompile rather than a conversion this member could perform —
    /// `rule:types/conversion` keeps `bytes` and `string` apart at exactly this boundary.
    fn nvs_core_bytes_join(_ctx, args: [2]) {
        // Unreachable from source: parameter 0 is `array<bytes>` in `CLASS`
        // above, so a non-container subject is `E0401: expected
        // array<bytes>, found mixed` at the checker. Same judgement as
        // `crate::arr`'s `nvs_core_arr_count`, which states it in full.
        let parts = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Bytes::join expected {:?} for the subject, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let separator = raw(&args[1], "join", "the separator")?;

        let mut out: Vec<u8> = Vec::new();
        let mut from = 0usize;
        let mut written = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, value) = unsafe {
                let slot = nvs_runtime::nvs_array_next_slot(parts, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let mut value = Value::null();
                nvs_runtime::nvs_array_value_at(parts, slot, &raw mut value);
                (slot, value)
            };
            from = slot + 1;
            let octets = raw(&value, "join", "an element")?;
            // Counted rather than tested against `out.is_empty()`, because an
            // empty *first* element must still be followed by a separator —
            // the same edge `Core\Str::join` writes against its own cursor.
            let size = out
                .len()
                .checked_add(octets.len())
                .and_then(|size| {
                    size.checked_add(if written == 0 { 0 } else { separator.len() })
                });
            let size = affordable(size, "join")?;
            // The seam allowed it; the allocator is the one that has to serve
            // it, and `extend_from_slice` growing on its own would abort.
            reserved(&mut out, size, "join")?;
            if written > 0 {
                out.extend_from_slice(separator);
            }
            out.extend_from_slice(octets);
            written += 1;
        }
        produced_fallibly(&out, "join")
    }
}

// ============================================================================
// The format grammar `pack` writes and `unpack` reads
// ============================================================================

/// A field's repeater — what follows its code in the format string.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Repeat {
    /// Nothing was written. One value for a numeric field, and the whole of
    /// the argument for a buffer one — **not** PHP's default of 1, which
    /// silently truncates `pack("a", "Hello")` to `"H"`.
    Natural,
    /// An explicit count: how many values a numeric field takes, and how many
    /// octets wide a buffer field is.
    Count(usize),
    /// `*` — every remaining argument for a numeric field, and the whole of
    /// the argument for a buffer one.
    All,
}

/// The repeater following a code, and the rest of the format after it.
fn repeater<'a>(after: &'a str, member: &str) -> Result<(Repeat, &'a str), Fault> {
    if let Some(rest) = after.strip_prefix('*') {
        return Ok((Repeat::All, rest));
    }
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Ok((Repeat::Natural, after));
    }
    let mut count: usize = 0;
    for digit in after.bytes().take(digits) {
        count = count
            .checked_mul(10)
            .and_then(|so_far| so_far.checked_add(usize::from(digit - b'0')))
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "Core\\Bytes::{member}: a repeat count larger than any buffer this process \
                     could hold"
                ))
            })?;
    }
    Ok((Repeat::Count(count), &after[digits..]))
}

/// The width in octets and the byte order of each **integer** code, or `None`
/// for a code that is not one. `true` is most-significant-octet first.
const fn integral(code: char) -> Option<(usize, bool)> {
    Some(match code {
        // One octet has no byte order; the pair differs only in how `unpack`
        // reads it back, which the module doc records.
        'c' | 'C' => (1, true),
        'n' => (2, true),
        'v' => (2, false),
        'N' => (4, true),
        'V' => (4, false),
        'J' => (8, true),
        'P' => (8, false),
        _ => return None,
    })
}

/// [`integral`]'s counterpart for the four **float** codes.
const fn fractional(code: char) -> Option<(usize, bool)> {
    Some(match code {
        'G' => (4, true),
        'g' => (4, false),
        'E' => (8, true),
        'e' => (8, false),
        _ => return None,
    })
}

/// The throw for a code this grammar does not have, naming what to write
/// instead wherever PHP had one — the module doc owns why each is refused.
fn unknown_code(member: &str, code: char) -> Fault {
    let instead = match code {
        's' | 'S' | 'i' | 'I' | 'l' | 'L' | 'q' | 'Q' => Some(
            "it takes the machine's own width and byte order, which is not a wire format — write \
             `n`/`v` for 16 bits, `N`/`V` for 32, or `J`/`P` for 64",
        ),
        'f' | 'd' => Some(
            "it takes the machine's own byte order — write `g`/`G` for a 32-bit float or `e`/`E` \
             for a 64-bit one",
        ),
        'h' | 'H' => Some("write `Core\\Encoding::toHex`/`fromHex`, which is the whole of it"),
        'X' | '@' => Some(
            "a format runs forwards here — `x` covers NUL padding, and nothing moves the cursor \
             backwards or to an absolute position",
        ),
        _ => None,
    };
    Fault::thrown(match instead {
        Some(reason) => format!("Core\\Bytes::{member}: `{code}` is not a format code — {reason}"),
        None => format!("Core\\Bytes::{member}: `{code}` is not a format code"),
    })
}

/// How a `mixed` argument's runtime type is named in a throw.
///
/// A `FATAL` names the [`Tag`] the checker placed; this names the one the
/// *program* placed, because `mixed` is `rule:types/grammar`'s one unchecked position
/// and a wrong type here is the caller's mistake rather than a miscompile.
fn described(value: &Value) -> String {
    value.tag().map_or_else(
        || format!("tag {}", value.tag_byte()),
        |tag| format!("{tag:?}"),
    )
}

/// The next argument a field consumes, or the throw for a format that asks for
/// more than the call wrote.
fn consumed<'a>(arguments: &'a [Value], next: &mut usize, code: char) -> Result<&'a Value, Fault> {
    let found = arguments.get(*next).ok_or_else(|| {
        Fault::thrown(format!(
            "Core\\Bytes::pack: `{code}` wants argument {}, and the call wrote {}",
            *next + 1,
            arguments.len()
        ))
    })?;
    *next += 1;
    Ok(found)
}

/// `field`'s octets, most significant first, appended in the order `big` names.
fn emit(out: &mut Vec<u8>, field: &[u8], big: bool) {
    if big {
        out.extend_from_slice(field);
    } else {
        out.extend(field.iter().rev());
    }
}

/// One integer field: `width` octets of two's complement in `big`'s order.
///
/// The accepted range is the **union** of the width's signed and unsigned
/// ranges, so `-1` and `4294967295` both write `ffffffff` under `N` and
/// neither loses anything; anything outside it throws rather than wrapping,
/// which is `rule:core-api/shape-rules` R4 and the same rule `fill` applies to an octet.
fn integer_field(
    out: &mut Vec<u8>,
    value: i128,
    width: usize,
    big: bool,
    code: char,
) -> Result<(), Fault> {
    let bits = width * 8;
    let modulus: i128 = 1 << bits;
    let low = -(modulus >> 1);
    let high = modulus - 1;
    if value < low || value > high {
        return Err(Fault::thrown(format!(
            "Core\\Bytes::pack: {value} does not fit `{code}`'s {bits} bits, which hold {low} to \
             {high}"
        )));
    }
    // `i128::to_be_bytes` is already the two's complement representation, so
    // its low `width` octets are the field — and the check above is what makes
    // dropping the rest lossless.
    let full = value.to_be_bytes();
    emit(out, &full[full.len() - width..], big);
    Ok(())
}

/// One float field, IEEE 754 binary32 or binary64 in `big`'s order.
///
/// A 32-bit field **rounds** to the nearest `f32` rather than throwing on a
/// value that does not round-trip: unlike an integer field's range, precision
/// is what a caller chose when they wrote a 4-byte float, and refusing `0.1`
/// there would leave `g`/`G` unusable. Overflowing to an infinity is a
/// different thing and does throw — that is a value becoming a different kind
/// of value, not a narrower one.
fn float_field(
    out: &mut Vec<u8>,
    value: f64,
    width: usize,
    big: bool,
    code: char,
) -> Result<(), Fault> {
    if width == 4 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "narrowing to the field's own precision is what this member is for, and \
                      the one case the rounding cannot express is checked on the next line"
        )]
        let narrow = value as f32;
        if value.is_finite() && !narrow.is_finite() {
            return Err(Fault::thrown(format!(
                "Core\\Bytes::pack: {value} is outside `{code}`'s 32-bit range, and rounding it \
                 would answer an infinity"
            )));
        }
        emit(out, &narrow.to_be_bytes(), big);
    } else {
        emit(out, &value.to_be_bytes(), big);
    }
    Ok(())
}

/// One buffer field — `a` NUL-padded, `A` space-padded, `Z` NUL-padded with at
/// least one NUL of its own.
///
/// An argument **longer** than the declared width throws rather than being
/// truncated, which is where this parts company with PHP: a record field too
/// small for its value is a bug, and silently writing the first half of a name
/// is the substitution failure `rule:types/conversion` removes from the language.
fn buffer_field(out: &mut Vec<u8>, data: &[u8], repeat: Repeat, code: char) -> Result<(), Fault> {
    let pad = if code == 'A' { b' ' } else { 0 };
    let needed = data.len() + usize::from(code == 'Z');
    let width = match repeat {
        Repeat::Natural | Repeat::All => needed,
        Repeat::Count(count) => count,
    };
    if width < needed {
        return Err(Fault::thrown(format!(
            "Core\\Bytes::pack: `{code}` was given a {width}-octet field, and the argument needs \
             {needed}"
        )));
    }
    let total = affordable(out.len().checked_add(width), "pack")?;
    out.extend_from_slice(data);
    out.resize(total, pad);
    Ok(())
}

/// The whole of `pack`: walk the format, consuming arguments as fields ask for
/// them, and refuse a call whose two halves do not line up in either direction.
fn packed(format: &str, arguments: &[Value]) -> Result<Vec<u8>, Fault> {
    let mut out: Vec<u8> = Vec::new();
    let mut next = 0usize;
    let mut rest = format;

    while let Some(code) = rest.chars().next() {
        let (repeat, after) = repeater(&rest[code.len_utf8()..], "pack")?;
        rest = after;

        if code == 'x' {
            let width = match repeat {
                Repeat::Natural => 1,
                Repeat::Count(count) => count,
                Repeat::All => {
                    return Err(Fault::thrown(
                        "Core\\Bytes::pack: `x*` has nothing to repeat — `x` consumes no \
                         argument, so write the number of NUL bytes"
                            .to_owned(),
                    ));
                }
            };
            let total = affordable(out.len().checked_add(width), "pack")?;
            out.resize(total, 0);
            continue;
        }

        if matches!(code, 'a' | 'A' | 'Z') {
            let value = consumed(arguments, &mut next, code)?;
            // Either buffer tag: a `string` is the ordinary argument for a
            // text field, and a `bytes` is what a program that already has
            // octets holds. Nothing else has octets to write.
            let data = value.as_bytes().or_else(|| value.as_str_bytes()).ok_or_else(|| {
                Fault::thrown(format!(
                    "Core\\Bytes::pack: `{code}` writes a buffer field, and the argument is a {}",
                    described(value)
                ))
            })?;
            buffer_field(&mut out, data, repeat, code)?;
            continue;
        }

        let Some((width, big)) = integral(code).or_else(|| fractional(code)) else {
            return Err(unknown_code("pack", code));
        };
        let times = match repeat {
            Repeat::Natural => 1,
            Repeat::Count(count) => count,
            Repeat::All => arguments.len().saturating_sub(next),
        };
        for _ in 0..times {
            let value = consumed(arguments, &mut next, code)?;
            affordable(out.len().checked_add(width), "pack")?;
            if integral(code).is_some() {
                let number = match value.tag() {
                    Some(Tag::Int) => value.as_int().map(i128::from),
                    Some(Tag::Uint) => value.as_uint().map(i128::from),
                    _ => None,
                }
                .ok_or_else(|| {
                    Fault::thrown(format!(
                        "Core\\Bytes::pack: `{code}` writes an integer field, and the argument \
                         is a {}",
                        described(value)
                    ))
                })?;
                integer_field(&mut out, number, width, big, code)?;
            } else {
                let number = match value.tag() {
                    Some(Tag::Float) => value.as_float(),
                    _ => None,
                };
                let number = number.ok_or_else(|| {
                    Fault::thrown(format!(
                        "Core\\Bytes::pack: `{code}` writes a float field, and the argument is a \
                         {}",
                        described(value)
                    ))
                })?;
                float_field(&mut out, number, width, big, code)?;
            }
        }
    }

    if next < arguments.len() {
        return Err(Fault::thrown(format!(
            "Core\\Bytes::pack: the format writes {next} of the {} arguments the call wrote",
            arguments.len()
        )));
    }
    Ok(out)
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::pack(string $format, mixed ...$values): bytes` — replacing
    /// PHP's `pack`, and the second `Core` member with a variadic parameter
    /// after `Core\Str::format`.
    ///
    /// So the second argument slot is **one** `Tag::Array` holding every
    /// written value rather than one slot each, built at the call site by
    /// `nvs_ir::lower::lower_variadic_tail` —
    /// [`crate::registry::CoreTy::Variadic`] owns why that shape. A call that
    /// writes no value at all still receives an array here, empty rather than
    /// absent.
    ///
    /// The grammar, the codes it refuses and every throw are this module's
    /// own docs; [`packed`] is the whole of the member.
    ///
    /// **Two classifications are still owed**, both named by spec § 7 and
    /// neither invented here: the format is an
    /// `rule:expressions/intrinsic-literals`
    /// intrinsic, so a *literal* format should have its field count checked
    /// against the argument list at compile time rather than at the call —
    /// exactly as `Core\Str::format`'s template still owes; and it is a
    /// **sink**
    /// (`rule:security/sink-predicate`),
    /// which is that ADR's registry-wide item — no member row anywhere carries
    /// a qualifier classification yet, so half of one here would be a lie
    /// about what is enforced.
    fn nvs_core_bytes_pack(_ctx, args: [2]) {
        // Unreachable from source: parameter 0 is `CoreTy::Str` in `CLASS`
        // above, so a non-string format is `E0401: expected 'string', found
        // 'mixed'` at the checker. A string that is no *format* is the
        // grammar's own throw, which `packed` owns.
        let format = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Bytes::pack expected {:?} for the format, got tag {}",
                Tag::Str,
                args[0].tag_byte()
            ))
        })?;
        // Unreachable for the stronger reason `Core\Path::join` states in
        // full, and so unreachable from source with no diagnostic to name:
        // parameter 1 is `CoreTy::Variadic`, and `nvs_ir::lower::
        // lower_call_args` *builds* the array this slot holds out of every
        // argument from that position on, so no source expression reaches the
        // slot at all.
        let values = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Bytes::pack expected {:?} for the value list, got tag {}",
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
                let slot = nvs_runtime::nvs_array_next_slot(values, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let mut value = Value::null();
                nvs_runtime::nvs_array_value_at(values, slot, &raw mut value);
                (slot, value)
            };
            from = slot + 1;
            collected.push(value);
        }

        produced(&packed(format, &collected)?)
    }
}

// ============================================================================
// `unpack`: the same grammar, read backwards
// ============================================================================

/// One value [`unpacked`] read, before it becomes a [`Value`].
///
/// The intermediate exists so that a format failing part way through a buffer
/// leaves **nothing to release** — every variant here is plain owned Rust, so
/// the error path is `?` rather than a hand-written unwind over the references
/// already built.
#[derive(Clone, Debug, PartialEq)]
enum Field {
    /// `c`, the one code read back signed.
    Signed(i64),
    /// Every other integer code — `C`, `n`, `v`, `N`, `V`, `J`, `P`.
    Unsigned(u64),
    /// The four float codes.
    Fractional(f64),
    /// `a`, `A` or `Z` — octets, and `bytes` rather than `string` because a
    /// field of a binary record carries no charset (`rule:types/bytes`). Validating
    /// it as UTF-8 here would make reading a record throw on data that is
    /// perfectly well-formed for what it is.
    Buffer(Vec<u8>),
}

impl Field {
    /// The value compiled code receives, taking ownership of a buffer field's
    /// octets.
    fn into_value(self) -> Value {
        match self {
            Self::Signed(number) => Value::int(number),
            Self::Unsigned(number) => Value::uint(number),
            Self::Fractional(number) => Value::float(number),
            Self::Buffer(octets) => Value::bytes(NvsStr::new(&octets)),
        }
    }
}

/// The cursor after a field of `width` octets read at `at`, or the throw for a
/// buffer that ends before the format does.
fn advanced(at: usize, width: usize, total: usize, code: char) -> Result<usize, Fault> {
    at.checked_add(width)
        .filter(|end| *end <= total)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Bytes::unpack: `{code}` reads {width} octets at offset {at}, and the \
                 buffer holds {total}"
            ))
        })
}

/// One numeric field's octets as the value they spell.
///
/// The field is normalized to most-significant-octet-first and right-aligned
/// in a `u64` first, so every code below reads the same integer rather than
/// each carrying its own shifting.
fn read_field(field: &[u8], code: char, big: bool) -> Field {
    let mut wide = [0u8; 8];
    let width = field.len().min(wide.len());
    for (index, octet) in field.iter().take(width).enumerate() {
        let significance = if big { width - 1 - index } else { index };
        wide[wide.len() - 1 - significance] = *octet;
    }
    match code {
        // Two's complement, so the sign is restored by *reading* the octet
        // signed — which is the whole of the difference between `c` and `C`.
        'c' => Field::Signed(i64::from(i8::from_be_bytes([wide[7]]))),
        'g' | 'G' => Field::Fractional(f64::from(f32::from_be_bytes([
            wide[4], wide[5], wide[6], wide[7],
        ]))),
        'e' | 'E' => Field::Fractional(f64::from_bits(u64::from_be_bytes(wide))),
        _ => Field::Unsigned(u64::from_be_bytes(wide)),
    }
}

/// A buffer field's octets with whatever its code pads with taken back off.
fn trimmed(raw: &[u8], code: char) -> &[u8] {
    match code {
        // PHP's `A`: the padding this grammar can have written, and nothing
        // else — a trailing octet that happens to be a space is not
        // distinguishable from the padding that produced it either way.
        'A' => {
            let end = raw
                .iter()
                .rposition(|octet| *octet != b' ' && *octet != 0)
                .map_or(0, |last| last + 1);
            &raw[..end]
        }
        // PHP's `Z`: the field ends at its own terminator, and anything after
        // it inside the declared width is padding.
        'Z' => match raw.iter().position(|octet| *octet == 0) {
            Some(nul) => &raw[..nul],
            None => raw,
        },
        _ => raw,
    }
}

/// The whole of `unpack`: walk the same format [`packed`] writes, reading one
/// field at a time, and refuse a buffer that does not end where the format
/// does.
fn unpacked(subject: &[u8], format: &str) -> Result<Vec<Field>, Fault> {
    let mut out = Vec::new();
    let mut at = 0usize;
    let mut rest = format;

    while let Some(code) = rest.chars().next() {
        let (repeat, after) = repeater(&rest[code.len_utf8()..], "unpack")?;
        rest = after;
        let left = subject.len().saturating_sub(at);

        if code == 'x' {
            let width = match repeat {
                Repeat::Natural => 1,
                Repeat::Count(count) => count,
                Repeat::All => left,
            };
            at = advanced(at, width, subject.len(), code)?;
            continue;
        }

        if matches!(code, 'a' | 'A' | 'Z') {
            // A buffer field with no width is the rest of the buffer, which is
            // what makes `pack`'s bare `a` its exact inverse.
            let width = match repeat {
                Repeat::Natural | Repeat::All => left,
                Repeat::Count(count) => count,
            };
            let end = advanced(at, width, subject.len(), code)?;
            out.push(Field::Buffer(trimmed(&subject[at..end], code).to_vec()));
            at = end;
            continue;
        }

        let Some((width, big)) = integral(code).or_else(|| fractional(code)) else {
            return Err(unknown_code("unpack", code));
        };
        let times = match repeat {
            Repeat::Natural => 1,
            Repeat::Count(count) => count,
            // Never a division by zero: every code in either table is at least
            // one octet wide.
            Repeat::All => left / width,
        };
        for _ in 0..times {
            let end = advanced(at, width, subject.len(), code)?;
            out.push(read_field(&subject[at..end], code, big));
            at = end;
        }
    }

    if at < subject.len() {
        return Err(Fault::thrown(format!(
            "Core\\Bytes::unpack: the format reads {at} of the buffer's {} octets — a header read \
             off a longer buffer is `Core\\Bytes::slice` and then this",
            subject.len()
        )));
    }
    Ok(out)
}

nvs_runtime::nvs_helper! {
    /// `Core\Bytes::unpack(bytes $b, string $format): array<mixed>` —
    /// replacing PHP's `unpack`, and [`packed`]'s exact inverse over the same
    /// code table, which is why that table is written once and read twice.
    ///
    /// **The answer is a positional list, where PHP's is a name→value map.**
    /// PHP's `unpack` carries a *second* grammar for the names —
    /// `"Nlength/nport"`, with `/` between fields, an unnamed field silently
    /// called `"1"`, and a repeated name silently overwriting the field before
    /// it. One grammar shared with `pack` costs a caller `$fields[0]` instead
    /// of `$fields["length"]` and buys back the property that makes this pair
    /// worth having: `unpack(pack($f, …$v), $f)` is `$v`, field for field,
    /// with nothing to check about how the two format strings were spelled.
    ///
    /// **Octets left over throw**, rather than being ignored the way PHP
    /// ignores them: a format is a description of the buffer, and a
    /// description that stops short of the data is exactly the bug a binary
    /// parser must not swallow (AGENTS.md's priority 1). A header read off a
    /// longer buffer is `Core\Bytes::slice` and then this, or a trailing `a*`.
    ///
    /// The format is an `rule:expressions/intrinsic-literals` intrinsic and a sink for the same reasons
    /// [`nvs_core_bytes_pack`]'s is, and both classifications are owed there.
    fn nvs_core_bytes_unpack(_ctx, args: [2]) {
        let subject = raw(&args[0], "unpack", "the subject")?;
        // Unreachable from source on `nvs_core_bytes_pack`'s judgement: this
        // parameter is `CoreTy::Str` too, so a non-string format is `E0401:
        // expected 'string', found 'mixed'` at the checker, and a string that
        // is no format is `unpacked`'s throw below.
        let format = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Bytes::unpack expected {:?} for the format, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?;

        let mut out = NvsArray::new();
        for field in unpacked(subject, format)? {
            out.append(field.into_value());
        }
        Ok(Value::array(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`offset`] saturates at both ends where [`addressed`] refuses — the
    /// distinction the two members that use them turn on, pinned so a later
    /// edit cannot quietly merge them.
    #[test]
    fn a_position_saturates_where_an_index_refuses() {
        assert_eq!(offset(4, 0), 0);
        assert_eq!(offset(4, 4), 4);
        assert_eq!(offset(4, 9), 4);
        assert_eq!(offset(4, -1), 3);
        assert_eq!(offset(4, -9), 0);
        assert_eq!(offset(0, -1), 0);

        assert_eq!(addressed(4, 0), Some(0));
        assert_eq!(addressed(4, 3), Some(3));
        assert_eq!(addressed(4, 4), None);
        assert_eq!(addressed(4, -1), Some(3));
        assert_eq!(addressed(4, -4), Some(0));
        assert_eq!(addressed(4, -5), None);
        assert_eq!(addressed(0, 0), None);
    }

    /// `Core\Bytes::length` counts the octets a program can address, and
    /// **agrees** with [`nvs_core_bytes_at`] on where the buffer stops: the last
    /// index the count admits reads, and the first one past it throws. The two
    /// are named together because a count one entry short answers plausibly
    /// against either half alone, and the agreement is counted over the whole
    /// table rather than read off a row. The rows a walk over the octets would
    /// get wrong — an interior `NUL`, a multi-byte character, an octet past
    /// `0x7f` — are what the table is made of.
    // covers: Core\Bytes::length
    #[test]
    fn a_count_is_the_number_of_octets_that_can_be_addressed() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let table: [&[u8]; 7] = [
            b"",
            b"\x00",
            b"\x00\x00\x00\x00",
            b"report.pdf",
            b"Gr\xc3\xb6\xc3\x9fe",
            b"\x89PNG\r\n\x1a\n",
            b"\xff\xfe\xfd",
        ];

        let mut agreed = 0_usize;
        for octets in table {
            let subject = Value::bytes(NvsStr::new(octets));
            let counted = nvs_runtime::call(nvs_core_bytes_length, &mut ctx, &[subject])
                .expect("a count answers")
                .as_uint()
                .expect("a count answers a `uint`");
            let end = i64::try_from(counted).expect("a row of this table fits an `i64`");
            let last_octet_reads = counted == 0
                || nvs_runtime::call(nvs_core_bytes_at, &mut ctx, &[subject, Value::int(end - 1)])
                    .is_ok();
            let past_the_end_throws =
                nvs_runtime::call(nvs_core_bytes_at, &mut ctx, &[subject, Value::int(end)])
                    .is_err()
                    && ctx.take_pending().is_some();
            let real = u64::try_from(octets.len()).expect("a row of this table fits a `u64`");
            if counted == real && last_octet_reads && past_the_end_throws {
                agreed += 1;
            }
            release(vec![subject]);
        }
        assert_eq!(agreed, table.len());
    }

    /// `Core\Bytes::at` reads the octet at each end of a buffer and refuses the
    /// first index past either of them, the two named together so a member that
    /// stops one entry early cannot pass on half the range. A refusal is a throw
    /// carrying the index it was given, because the `uint` return type
    /// (`rule:core-api/shape-rules` R4/R5) leaves no sentinel for it to answer.
    // covers: Core\Bytes::at
    #[test]
    fn an_index_reads_inside_the_buffer_and_throws_outside_it() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let subject = Value::bytes(NvsStr::new(b"\x89PNG"));

        for (index, octet) in [(0_i64, 0x89_u64), (3, 0x47), (-1, 0x47), (-4, 0x89)] {
            let found =
                nvs_runtime::call(nvs_core_bytes_at, &mut ctx, &[subject, Value::int(index)])
                    .expect("an index inside the buffer reads");
            assert_eq!(found.as_uint(), Some(octet), "at index {index}");
        }

        for index in [4_i64, -5, i64::MAX, i64::MIN] {
            nvs_runtime::call(nvs_core_bytes_at, &mut ctx, &[subject, Value::int(index)])
                .expect_err("an index outside the buffer throws");
            assert_eq!(
                ctx.take_pending().map(std::borrow::Cow::into_owned),
                Some(format!(
                    "Core\\Bytes::at: index {index} is outside a buffer of 4 bytes"
                ))
            );
        }

        release(vec![subject]);
    }

    /// `Core\Bytes::slice` copies exactly the window its two positions name,
    /// swept over every offset and every length a four-octet buffer admits:
    /// both signs of both, the `null` that runs to the end, and the pairs that
    /// close the window before it opens. The answer is checked with
    /// [`nvs_core_bytes_compare`] against a window cut by [`offset`], whose own
    /// clamping the test above pins, so the two ends of the window are asserted
    /// together and a member that shifted one of them by one fails here.
    /// Counted over the sweep rather than read off a row, for the reason its
    /// siblings below are.
    // covers: Core\Bytes::slice
    #[test]
    fn a_window_is_the_octets_between_the_two_positions_it_names() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let octets: &[u8] = b"\x00A\xff\x7f";
        let subject = Value::bytes(NvsStr::new(octets));
        let lengths = [
            None,
            Some(-6_i64),
            Some(-3),
            Some(-1),
            Some(0),
            Some(1),
            Some(3),
            Some(6),
        ];

        let mut agreed = 0_usize;
        let mut swept = 0_usize;
        for from in -6_i64..=6 {
            for length in lengths {
                swept += 1;
                let start = offset(octets.len(), from);
                let end = match length {
                    None => octets.len(),
                    Some(span) if span < 0 => offset(octets.len(), span),
                    Some(span) => start
                        .saturating_add(usize::try_from(span).expect("a length of this sweep fits"))
                        .min(octets.len()),
                };
                let want = Value::bytes(NvsStr::new(octets.get(start..end).unwrap_or(&[])));
                let window = nvs_runtime::call(
                    nvs_core_bytes_slice,
                    &mut ctx,
                    &[
                        subject,
                        Value::int(from),
                        length.map_or_else(Value::null, Value::int),
                    ],
                )
                .expect("a window answers");
                let ordering = nvs_runtime::call(nvs_core_bytes_compare, &mut ctx, &[window, want])
                    .expect("two buffers order")
                    .as_int();
                if ordering == Some(0) {
                    agreed += 1;
                }
                release(vec![window, want]);
            }
        }
        assert_eq!(agreed, swept);
        assert_eq!(swept, 13 * lengths.len());

        release(vec![subject]);
    }

    /// `Core\Bytes::compare` answers one of exactly three values, reverses when
    /// its operands do, and agrees with a table written in order by hand —
    /// counted over every ordered pair rather than read off a line, so a pair
    /// that happened to answer plausibly cannot carry the row. The table holds
    /// the three places a length-first or signed comparison would differ: an
    /// empty buffer, a prefix beside the buffer that extends it, and an octet
    /// past `0x7f`.
    // covers: Core\Bytes::compare
    #[test]
    fn an_ordering_reverses_with_its_operands_and_follows_the_octets() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let table: [&[u8]; 7] = [
            b"",
            b"\x00",
            b"car",
            b"cargo",
            b"\x7f",
            b"\xff",
            b"\xff\x00",
        ];
        let built: Vec<Value> = table
            .iter()
            .map(|octets| Value::bytes(NvsStr::new(octets)))
            .collect();

        let ordering = |ctx: &mut nvs_runtime::Ctx, left: Value, right: Value| {
            nvs_runtime::call(nvs_core_bytes_compare, ctx, &[left, right])
                .expect("two buffers order")
                .as_int()
                .expect("an ordering is an int")
        };

        let mut agreed = 0_usize;
        for (i, left) in built.iter().enumerate() {
            for (j, right) in built.iter().enumerate() {
                let expected = match i.cmp(&j) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                if ordering(&mut ctx, *left, *right) == expected
                    && ordering(&mut ctx, *right, *left) == -expected
                {
                    agreed += 1;
                }
            }
        }
        assert_eq!(agreed, table.len() * table.len());

        release(built);
    }

    /// `Core\Bytes::contains` answers a table written by hand, and **agrees**
    /// with the two predicates that answer its ends: a needle at the start or
    /// at the end of the subject is a needle the subject contains. Both are
    /// counted over the whole table rather than read off a line, so a predicate
    /// that grew a scan of its own fails here while still looking right alone.
    /// The rows carry the edges a scan comes apart on — an empty needle, a
    /// needle wider than the subject, an overlapping repeat, and a letter whose
    /// case differs, since a buffer carries no charset to fold it with.
    // covers: Core\Bytes::contains
    #[test]
    fn a_search_answers_its_table_and_agrees_with_both_ends() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let table: [(&[u8], &[u8], bool); 10] = [
            (b"", b"", true),
            (b"abc", b"", true),
            (b"", b"a", false),
            (b"abc", b"abcd", false),
            (b"abcabc", b"bc", true),
            (b"abcabc", b"cab", true),
            (b"\x00\xff\x10", b"\xff\x10", true),
            (b"aaab", b"aab", true),
            (b"aaa", b"aab", false),
            (b"Content-Type", b"content", false),
        ];

        let mut agreed = 0_usize;
        for (haystack, needle, expected) in table {
            let subject = Value::bytes(NvsStr::new(haystack));
            let sought = Value::bytes(NvsStr::new(needle));
            let asked = |ctx: &mut nvs_runtime::Ctx, member: nvs_runtime::NvsFn| {
                nvs_runtime::call(member, ctx, &[subject, sought])
                    .expect("a predicate answers")
                    .as_bool()
                    .expect("a predicate answers a bool")
            };
            let found = asked(&mut ctx, nvs_core_bytes_contains);
            let at_start = asked(&mut ctx, nvs_core_bytes_starts_with);
            let at_end = asked(&mut ctx, nvs_core_bytes_ends_with);
            let ends_imply_it = found || !(at_start || at_end);
            if found == expected && ends_imply_it {
                agreed += 1;
            }
            release(vec![subject, sought]);
        }
        assert_eq!(agreed, table.len());
    }

    /// The empty needle occurs at the start of everything, and a needle longer
    /// than the subject occurs nowhere — the two edges a `windows` scan would
    /// otherwise panic or loop on.
    #[test]
    fn a_scan_answers_at_both_edges() {
        assert_eq!(find(b"", b""), Some(0));
        assert_eq!(find(b"abc", b""), Some(0));
        assert_eq!(find(b"", b"a"), None);
        assert_eq!(find(b"abc", b"abcd"), None);
        assert_eq!(find(b"abcabc", b"bc"), Some(1));
        assert_eq!(find(b"\x00\xff\x10", b"\xff\x10"), Some(1));
    }

    /// One row of the search table below: a subject, a needle, the `from` the
    /// search starts at, and the offset that must come back.
    type SearchRow = (&'static [u8], &'static [u8], i64, Option<u64>);

    /// `Core\Bytes::indexOf` answers the byte offset its table names, resolves
    /// `from` at both ends — a negative one counts back from the end, and one
    /// past either end saturates rather than throwing — and **agrees** with
    /// `contains` on every row that searches the whole buffer: an offset is
    /// found exactly where the buffer contains the needle. Both are counted
    /// over the table rather than read off a line, so a row that answered
    /// plausibly on its own cannot carry the assertion. The `from` option is
    /// filled the way a call site fills it, with the `Const::Int(0)` the
    /// registry declares for it rather than a `Value::null()`.
    // covers: Core\Bytes::indexOf
    #[test]
    fn a_search_answers_an_offset_and_resolves_from_at_both_ends() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let table: [SearchRow; 13] = [
            (b"abcabc", b"bc", 0, Some(1)),
            (b"abcabc", b"bc", 2, Some(4)),
            (b"abcabc", b"bc", -3, Some(4)),
            (b"abcabc", b"bc", -99, Some(1)),
            (b"abcabc", b"bc", 5, None),
            (b"abcabc", b"bc", 99, None),
            (b"abc", b"", 0, Some(0)),
            (b"abc", b"", 99, Some(3)),
            (b"", b"", 0, Some(0)),
            (b"", b"a", 0, None),
            (b"abc", b"abcd", 0, None),
            (b"\x00\xff\x10", b"\xff\x10", 0, Some(1)),
            (b"aaab", b"aab", 0, Some(1)),
        ];

        let mut agreed = 0_usize;
        for (haystack, needle, from, expected) in table {
            let subject = Value::bytes(NvsStr::new(haystack));
            let sought = Value::bytes(NvsStr::new(needle));
            let answer = nvs_runtime::call(
                nvs_core_bytes_index_of,
                &mut ctx,
                &[subject, sought, Value::int(from)],
            )
            .expect("a search answers");
            let found = match answer.tag() {
                Some(Tag::Null) => None,
                _ => Some(answer.as_uint().expect("an offset is a `uint`")),
            };
            let contained =
                nvs_runtime::call(nvs_core_bytes_contains, &mut ctx, &[subject, sought])
                    .expect("a predicate answers")
                    .as_bool()
                    .expect("a predicate answers a bool");
            let agrees_with_contains = from != 0 || contained == found.is_some();
            if found == expected && agrees_with_contains {
                agreed += 1;
            }
            release(vec![subject, sought]);
        }
        assert_eq!(agreed, table.len());
    }

    /// `Core\Bytes::startsWith` answers its table and **agrees** with the
    /// search: a buffer begins with a prefix exactly when `indexOf` finds that
    /// prefix at offset 0. Counted over the whole table, so a predicate that
    /// grew a scan of its own fails here while still reading right on any one
    /// line. The rows carry what a prefix test comes apart on — an empty
    /// prefix, a prefix as wide as the buffer, one octet wider than the buffer,
    /// a prefix that occurs later but not at the start, and a letter whose case
    /// differs, since a buffer carries no charset to fold it with.
    // covers: Core\Bytes::startsWith
    #[test]
    fn a_prefix_is_what_the_search_finds_at_offset_zero() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let table: [(&[u8], &[u8], bool); 9] = [
            (b"", b"", true),
            (b"\x89PNG", b"", true),
            (b"", b"\x89", false),
            (b"\x89PNG", b"\x89PNG", true),
            (b"\x89PNG", b"\x89PNG\r", false),
            (b"\x89PNG", b"\x89P", true),
            (b"\x89PNG", b"PNG", false),
            (b"\x89PNG", b"\x89png", false),
            (b"GIF89a", b"GIF8", true),
        ];

        let mut agreed = 0_usize;
        for (octets, prefix, expected) in table {
            let subject = Value::bytes(NvsStr::new(octets));
            let sought = Value::bytes(NvsStr::new(prefix));
            let begins =
                nvs_runtime::call(nvs_core_bytes_starts_with, &mut ctx, &[subject, sought])
                    .expect("a predicate answers")
                    .as_bool()
                    .expect("a predicate answers a bool");
            let at_zero = nvs_runtime::call(
                nvs_core_bytes_index_of,
                &mut ctx,
                &[subject, sought, Value::int(0)],
            )
            .expect("a search answers")
            .as_uint()
                == Some(0);
            if begins == expected && begins == at_zero {
                agreed += 1;
            }
            release(vec![subject, sought]);
        }
        assert_eq!(agreed, table.len());
    }

    /// `Core\Bytes::endsWith` answers its table and **agrees** with the search:
    /// a buffer ends with a suffix exactly when `indexOf`, started where that
    /// suffix would have to begin, finds it there. A suffix wider than the
    /// buffer has no such position at all, which is the row the subtraction
    /// below reads as `None`. Counted over the whole table, for the reason its
    /// sibling above is.
    // covers: Core\Bytes::endsWith
    #[test]
    fn a_suffix_is_what_the_search_finds_at_the_far_end() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let table: [(&[u8], &[u8], bool); 11] = [
            (b"", b"", true),
            (b"report.pdf", b"", true),
            (b"", b"a", false),
            (b"report.pdf", b".pdf", true),
            (b"report.pdf", b"pdf", true),
            (b"report.pdf", b".PDF", false),
            (b"report.pdf", b"report.pdf", true),
            (b"report.pdf", b"xreport.pdf", false),
            (b"aaa", b"aa", true),
            (b"\x00\xff", b"\xff", true),
            (b"\r\n\r\n", b"\r\n", true),
        ];

        let mut agreed = 0_usize;
        for (octets, suffix, expected) in table {
            let subject = Value::bytes(NvsStr::new(octets));
            let sought = Value::bytes(NvsStr::new(suffix));
            let ends = nvs_runtime::call(nvs_core_bytes_ends_with, &mut ctx, &[subject, sought])
                .expect("a predicate answers")
                .as_bool()
                .expect("a predicate answers a bool");
            let at_far_end = octets.len().checked_sub(suffix.len()).is_some_and(|start| {
                let from = i64::try_from(start).expect("a row of this table fits an `i64`");
                nvs_runtime::call(
                    nvs_core_bytes_index_of,
                    &mut ctx,
                    &[subject, sought, Value::int(from)],
                )
                .expect("a search answers")
                .as_uint()
                    == u64::try_from(start).ok()
            });
            if ends == expected && ends == at_far_end {
                agreed += 1;
            }
            release(vec![subject, sought]);
        }
        assert_eq!(agreed, table.len());
    }

    /// `Core\Bytes::fill` writes the octet it is given as many times as it is
    /// asked for, and the widest value it accepts sits beside the first it
    /// refuses, the two named together so a member that stopped one short of
    /// `0xff` — or took a `256` modulo 256 — would still fail. Each buffer is
    /// checked with [`nvs_core_bytes_compare`] against one built here, so its
    /// length and its content are asserted at once rather than a count being
    /// read off a line.
    // covers: Core\Bytes::fill
    #[test]
    fn a_fill_repeats_one_octet_and_refuses_a_value_wider_than_one() {
        let mut ctx = nvs_runtime::Ctx::buffered();

        let mut agreed = 0_usize;
        let mut swept = 0_usize;
        for length in [0_u64, 1, 2, 7, 4096] {
            for byte in [0_u64, 1, 0x7f, 0xfe, 0xff] {
                swept += 1;
                let octet = u8::try_from(byte).expect("a row of this sweep is one octet");
                let wide = usize::try_from(length).expect("a row of this sweep fits a `usize`");
                let want = Value::bytes(NvsStr::new(&vec![octet; wide]));
                let built = nvs_runtime::call(
                    nvs_core_bytes_fill,
                    &mut ctx,
                    &[Value::uint(length), Value::uint(byte)],
                )
                .expect("a buffer of this size is affordable");
                if nvs_runtime::call(nvs_core_bytes_compare, &mut ctx, &[built, want])
                    .expect("two buffers order")
                    .as_int()
                    == Some(0)
                {
                    agreed += 1;
                }
                release(vec![built, want]);
            }
        }
        assert_eq!(agreed, swept);

        for byte in [256_u64, u64::from(u32::MAX), u64::MAX] {
            nvs_runtime::call(
                nvs_core_bytes_fill,
                &mut ctx,
                &[Value::uint(8), Value::uint(byte)],
            )
            .expect_err("a value wider than one octet throws");
            assert_eq!(
                ctx.take_pending().map(std::borrow::Cow::into_owned),
                Some(format!(
                    "Core\\Bytes::fill: {byte} is not one octet — a byte is 0 to 255"
                ))
            );
        }
    }

    /// `Core\Bytes::repeat` writes the whole buffer on every round, which is
    /// what separates it from [`nvs_core_bytes_fill`] and its single octet, so
    /// the sweep is over buffers whose length is not one. Both degenerate
    /// counts are named beside it: zero copies of any buffer and any number of
    /// copies of an empty one are the empty buffer. The second is also how the
    /// short-circuit is asserted — a member that ran the loop would need
    /// `u64::MAX` rounds to answer that line rather than returning at once.
    /// Each result is checked with [`nvs_core_bytes_compare`] against one built
    /// here, so its length and its content are asserted at once.
    // covers: Core\Bytes::repeat
    #[test]
    fn a_repeat_copies_the_whole_buffer_and_an_empty_one_is_never_looped() {
        let mut ctx = nvs_runtime::Ctx::buffered();

        let subjects: [&[u8]; 5] = [
            b"",
            b"\x00",
            b"ab",
            b"\x89PNG\r\n\x1a\n",
            b"Gr\xc3\xb6\xc3\x9fe",
        ];
        let mut agreed = 0_usize;
        let mut swept = 0_usize;
        for octets in subjects {
            for times in [0_u64, 1, 2, 3, 512] {
                swept += 1;
                let copies = usize::try_from(times).expect("a row of this sweep fits a `usize`");
                let subject = Value::bytes(NvsStr::new(octets));
                let want = Value::bytes(NvsStr::new(&octets.repeat(copies)));
                let built = nvs_runtime::call(
                    nvs_core_bytes_repeat,
                    &mut ctx,
                    &[subject, Value::uint(times)],
                )
                .expect("a buffer of this size is affordable");
                if nvs_runtime::call(nvs_core_bytes_compare, &mut ctx, &[built, want])
                    .expect("two buffers order")
                    .as_int()
                    == Some(0)
                {
                    agreed += 1;
                }
                release(vec![subject, want, built]);
            }
        }
        assert_eq!(agreed, swept);

        let empty = Value::bytes(NvsStr::new(b""));
        let unbounded = nvs_runtime::call(
            nvs_core_bytes_repeat,
            &mut ctx,
            &[empty, Value::uint(u64::MAX)],
        )
        .expect("an empty buffer costs nothing at any count");
        assert_eq!(unbounded.as_bytes(), Some(&b""[..]));
        release(vec![empty, unbounded]);

        let subject = Value::bytes(NvsStr::new(b"\x89PNG"));
        nvs_runtime::call(
            nvs_core_bytes_repeat,
            &mut ctx,
            &[subject, Value::uint(u64::MAX)],
        )
        .expect_err("four octets that many times reaches no allocator");
        assert_eq!(
            ctx.take_pending().map(std::borrow::Cow::into_owned),
            Some(
                "Core\\Bytes::repeat: the requested allocation is larger than any this process \
                 could hold"
                    .to_owned()
            )
        );
        release(vec![subject]);
    }

    /// `Core\Bytes::join` writes the separator between neighbours and nowhere
    /// else, so a list of `n` parts carries `n - 1` of them. The sweep counts
    /// that over every list length from none to six and three separators, each
    /// result compared with [`nvs_core_bytes_compare`] against one built here,
    /// so a member that wrote a leading or a trailing separator fails on
    /// content rather than on a length read off one line. A part with nothing
    /// in it opens the table and closes it: the member counts what it has
    /// written rather than testing whether its buffer is still empty, and an
    /// empty *first* part is the row that tells those two apart.
    // covers: Core\Bytes::join
    #[test]
    fn a_join_writes_one_separator_between_every_pair_of_parts() {
        let mut ctx = nvs_runtime::Ctx::buffered();

        let table: [&[u8]; 6] = [b"", b"alpha", b"", b"\x00\xff", b"b", b""];
        let mut agreed = 0_usize;
        let mut swept = 0_usize;
        for parts in 0..=table.len() {
            for octets in [&b""[..], b"-", b"\r\n\r\n"] {
                swept += 1;
                let mut list = NvsArray::new();
                for part in &table[..parts] {
                    list.append(Value::bytes(NvsStr::new(part)));
                }
                let subject = Value::array(list);
                let separator = Value::bytes(NvsStr::new(octets));
                let want = Value::bytes(NvsStr::new(&table[..parts].join(octets)));
                let built = nvs_runtime::call(nvs_core_bytes_join, &mut ctx, &[subject, separator])
                    .expect("a buffer of this size is affordable");
                if nvs_runtime::call(nvs_core_bytes_compare, &mut ctx, &[built, want])
                    .expect("two buffers order")
                    .as_int()
                    == Some(0)
                {
                    agreed += 1;
                }
                release(vec![subject, separator, want, built]);
            }
        }
        assert_eq!(agreed, swept);
    }

    /// One `string` argument, which the caller still owns — [`packed`] borrows
    /// its octets exactly as the helper convention does.
    fn text(literal: &str) -> Value {
        Value::str(NvsStr::new(literal.as_bytes()))
    }

    /// Gives back every reference [`text`] built.
    fn release(values: Vec<Value>) {
        for value in values {
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built for each \
                          value, and `packed` borrowed rather than consumed it"
            )]
            unsafe {
                value.release();
            }
        }
    }

    /// What a format writes, as hex — the spelling a `.nvst` case asserts in,
    /// since `echo` has no `bytes` row.
    fn hex(format: &str, arguments: &[Value]) -> String {
        packed(format, arguments)
            .expect("the format packs")
            .iter()
            .map(|octet| format!("{octet:02x}"))
            .collect()
    }

    /// The message a format that cannot pack throws.
    fn refusal(format: &str, arguments: &[Value]) -> String {
        match packed(format, arguments) {
            Ok(octets) => panic!(
                "the format packed {} octets rather than throwing",
                octets.len()
            ),
            Err(Fault::Thrown(_, message)) => message.into_owned(),
            Err(_) => panic!("the format failed with something other than a throw"),
        }
    }

    /// Every numeric code's width and byte order, checked against what
    /// `php -r 'echo bin2hex(pack(…));'` answers for the same call.
    // covers: Core\Bytes::pack
    #[test]
    fn each_numeric_code_writes_its_own_width_and_order() {
        assert_eq!(hex("C", &[Value::uint(255)]), "ff");
        assert_eq!(hex("n", &[Value::uint(4660)]), "1234");
        assert_eq!(hex("v", &[Value::uint(4660)]), "3412");
        assert_eq!(hex("N", &[Value::uint(305_419_896)]), "12345678");
        assert_eq!(hex("V", &[Value::uint(305_419_896)]), "78563412");
        assert_eq!(hex("J", &[Value::uint(1)]), "0000000000000001");
        assert_eq!(hex("P", &[Value::uint(1)]), "0100000000000000");
        assert_eq!(hex("G", &[Value::float(1.0)]), "3f800000");
        assert_eq!(hex("g", &[Value::float(1.0)]), "0000803f");
        assert_eq!(hex("E", &[Value::float(1.0)]), "3ff0000000000000");
        assert_eq!(hex("e", &[Value::float(1.0)]), "000000000000f03f");
    }

    /// A repeater counts *arguments* on a numeric field, `*` takes every one
    /// left, and `x` writes NUL octets while consuming none — the three rules
    /// that make a header expressible in one call.
    // covers: Core\Bytes::pack
    #[test]
    fn a_repeater_counts_arguments_and_x_counts_octets() {
        let three = [Value::uint(1), Value::uint(2), Value::uint(3)];
        assert_eq!(hex("n3", &three), "000100020003");
        assert_eq!(hex("n*", &three), "000100020003");
        assert_eq!(hex("Cn", &three[..2]), "010002");
        assert_eq!(hex("x", &[]), "00");
        assert_eq!(hex("x4", &[]), "00000000");
        assert_eq!(hex("n0", &[]), "");
    }

    /// An integer field accepts the union of its width's signed and unsigned
    /// ranges, writes two's complement, and throws outside it rather than
    /// wrapping the way PHP does — `rule:core-api/shape-rules` R4.
    // covers: Core\Bytes::pack
    #[test]
    fn an_integer_field_spans_both_ranges_and_refuses_outside_them() {
        assert_eq!(hex("N", &[Value::int(-1)]), "ffffffff");
        assert_eq!(hex("N", &[Value::uint(4_294_967_295)]), "ffffffff");
        assert_eq!(hex("C", &[Value::int(-128)]), "80");
        assert_eq!(hex("C", &[Value::uint(255)]), "ff");
        assert!(refusal("C", &[Value::uint(256)]).contains("does not fit"));
        assert!(refusal("C", &[Value::int(-129)]).contains("does not fit"));
        assert!(refusal("n", &[Value::int(-32_769)]).contains("does not fit"));
    }

    /// A buffer field pads to its declared width, `Z` keeps one octet for its
    /// own NUL, and an argument that does not fit throws instead of being cut.
    // covers: Core\Bytes::pack
    #[test]
    fn a_buffer_field_pads_and_refuses_to_truncate() {
        let hi = vec![text("Hi")];
        assert_eq!(hex("a4", &hi), "48690000");
        assert_eq!(hex("A4", &hi), "48692020");
        assert_eq!(hex("Z4", &hi), "48690000");
        assert_eq!(hex("a", &hi), "4869");
        assert_eq!(hex("Z", &hi), "486900");
        assert!(refusal("a1", &hi).contains("needs 2"));
        assert!(refusal("Z2", &hi).contains("needs 3"));
        release(hi);
    }

    /// The format and the argument list must line up in **both** directions,
    /// and a `mixed` argument of the wrong runtime type is the caller's
    /// mistake rather than a conversion this member performs.
    // covers: Core\Bytes::pack
    #[test]
    fn a_call_whose_halves_disagree_throws() {
        assert!(refusal("N", &[]).contains("wants argument 1"));
        assert!(refusal("N", &[Value::uint(1), Value::uint(2)]).contains("writes 1 of the 2"));
        assert!(refusal("N", &[Value::float(1.0)]).contains("integer field"));
        assert!(refusal("E", &[Value::int(1)]).contains("float field"));

        let hi = vec![text("Hi")];
        assert!(refusal("N", &hi).contains("integer field"));
        assert!(refusal("a2", &[Value::uint(1)]).contains("buffer field"));
        release(hi);
    }

    /// Every code this grammar refuses names what to write instead, so a
    /// program ported from PHP is told the answer rather than just told no.
    // covers: Core\Bytes::pack
    #[test]
    fn a_refused_code_names_its_replacement() {
        assert!(refusal("l", &[Value::int(1)]).contains("`N`/`V`"));
        assert!(refusal("d", &[Value::float(1.0)]).contains("`e`/`E`"));
        assert!(refusal("H", &[Value::int(1)]).contains("Core\\Encoding::toHex"));
        assert!(refusal("@", &[]).contains("forwards"));
        assert!(refusal("?", &[]).contains("is not a format code"));
        assert!(refusal("x*", &[]).contains("nothing to repeat"));
    }

    /// What a format reads out of a buffer.
    fn read(format: &str, octets: &[u8]) -> Vec<Field> {
        unpacked(octets, format).expect("the format unpacks")
    }

    /// The message a buffer the format does not describe throws.
    fn read_refusal(format: &str, octets: &[u8]) -> String {
        match unpacked(octets, format) {
            Ok(fields) => {
                panic!(
                    "the format read {} fields rather than throwing",
                    fields.len()
                )
            }
            Err(Fault::Thrown(_, message)) => message.into_owned(),
            Err(_) => panic!("the format failed with something other than a throw"),
        }
    }

    /// The property that makes one shared code table worth having: a format
    /// means the same thing read as it does written.
    // covers: Core\Bytes::unpack
    #[test]
    fn unpack_is_packs_inverse_over_the_same_format() {
        let format = "NnCcEg";
        let written = packed(
            format,
            &[
                Value::uint(305_419_896),
                Value::uint(4660),
                Value::uint(255),
                Value::int(-2),
                Value::float(0.5),
                Value::float(0.5),
            ],
        )
        .expect("the format packs");
        assert_eq!(
            read(format, &written),
            vec![
                Field::Unsigned(305_419_896),
                Field::Unsigned(4660),
                Field::Unsigned(255),
                Field::Signed(-2),
                Field::Fractional(0.5),
                Field::Fractional(0.5),
            ]
        );
    }

    /// Each order reads the octets back the way it wrote them, and `c` is the
    /// one code that restores a sign — the whole of its difference from `C`.
    // covers: Core\Bytes::unpack
    #[test]
    fn each_order_reads_back_what_it_wrote() {
        assert_eq!(read("n", b"\x12\x34"), vec![Field::Unsigned(0x1234)]);
        assert_eq!(read("v", b"\x12\x34"), vec![Field::Unsigned(0x3412)]);
        assert_eq!(
            read("N", b"\x12\x34\x56\x78"),
            vec![Field::Unsigned(0x1234_5678)]
        );
        assert_eq!(
            read("V", b"\x12\x34\x56\x78"),
            vec![Field::Unsigned(0x7856_3412)]
        );
        assert_eq!(
            read("J", b"\x00\x00\x00\x00\x00\x00\x00\x01"),
            vec![Field::Unsigned(1)]
        );
        assert_eq!(
            read("P", b"\x01\x00\x00\x00\x00\x00\x00\x00"),
            vec![Field::Unsigned(1)]
        );
        assert_eq!(
            read("cC", b"\xff\xff"),
            vec![Field::Signed(-1), Field::Unsigned(255)]
        );
        assert_eq!(read("xC", b"\x00\x07"), vec![Field::Unsigned(7)]);
    }

    /// A buffer field gives back octets — never a `string`, since a record
    /// field carries no charset — with its own padding taken off where the
    /// code says what the padding was.
    // covers: Core\Bytes::unpack
    #[test]
    fn a_buffer_field_answers_octets_without_its_padding() {
        assert_eq!(
            read("a4", b"Hi\0\0"),
            vec![Field::Buffer(b"Hi\0\0".to_vec())]
        );
        assert_eq!(read("A4", b"Hi  "), vec![Field::Buffer(b"Hi".to_vec())]);
        assert_eq!(read("Z4", b"Hi\0\0"), vec![Field::Buffer(b"Hi".to_vec())]);
        assert_eq!(read("a", b"Hi"), vec![Field::Buffer(b"Hi".to_vec())]);
        assert_eq!(
            read("nZ*", b"\x00\x01ok\0"),
            vec![Field::Unsigned(1), Field::Buffer(b"ok".to_vec())]
        );
    }

    /// The format must describe the whole buffer, in both directions — a tail
    /// PHP would ignore is what a binary parser must not swallow.
    // covers: Core\Bytes::unpack
    #[test]
    fn a_buffer_the_format_does_not_describe_throws() {
        assert_eq!(read("n*", b"\x00\x01\x00\x02").len(), 2);
        assert!(read_refusal("n", b"\x00\x01\x00\x02").contains("reads 2 of the buffer's 4"));
        assert!(read_refusal("n*", b"\x00\x01\x00").contains("reads 2 of the buffer's 3"));
        assert!(read_refusal("N", b"\x00\x01").contains("buffer holds 2"));
        assert!(read_refusal("a4", b"Hi").contains("buffer holds 2"));
        assert!(read_refusal("l", b"").contains("`N`/`V`"));
    }
}
