//! `Core\Str` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 1, over `mwl_runtime`'s reference-counted `MwlStr`.
//!
//! Every member here is pure (ADR 0063 R3) and borrows its subject rather than
//! consuming it — see [`crate`]'s own docs for why that falls out of being a
//! helper rather than being a rule this module states.
//!
//! # `string` is valid UTF-8, so this module decodes rather than validating
//!
//! [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) guarantees a
//! `string`'s bytes are valid UTF-8, which is what lets every member below
//! reach for `&str` operations directly. [`text`] is the one place that
//! guarantee is discharged, and it treats a violation as a contained `FATAL`
//! rather than a `panic!`: a broken invariant is a compiler or runtime bug, and
//! ADR 0020's ladder wants it reported, not left to take the process down.
//!
//! # Granularity is decided elsewhere, and read from one place
//!
//! `length`, `at` and `padStart`/`padEnd`'s `$length` all count in
//! [`crate::granularity::DEFAULT`] — ADR 0009 § 2's answer, stated in that
//! module and in that ADR, never restated here. Every other member is
//! granularity-independent.
//!
//! # Case conversion is Unicode's, not PHP's
//!
//! `lower`/`upper`/`upperFirst`/`lowerFirst` use Rust's full Unicode case
//! mappings, so they answer for `straße`/`ÄRGER` what PHP's `mb_strtoupper`
//! answers and *not* what its byte-wise `strtolower`/`ucfirst` do. The spec's
//! **Replaces** column lists both PHP spellings against one MWL member on
//! purpose (R13: "no member takes an encoding argument"), so the byte-wise
//! behaviour has no surviving spelling to be compatible with — a deliberate
//! divergence, and the shape `docs/agent/loop-goal.md`'s `--ORACLE-DIVERGES--`
//! section exists to record in the conformance suite.

use mwl_runtime::{Fault, HelperResult, MwlArray, MwlStr, Tag, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Str`'s registry rows, in the spec's own order.
///
/// Declared beside the implementations rather than in one flat table, so
/// adding a member touches this file and nothing else. [`crate::registry`]'s
/// `CLASSES` lists this const; that list grows one line per *class*, never one
/// per member.
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Str",
    methods: &[
        CoreMethod {
            name: "length",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_str_length",
        },
        CoreMethod {
            name: "at",
            params: &[CoreTy::Str, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_at",
        },
        CoreMethod {
            name: "isEmpty",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_str_is_empty",
        },
        CoreMethod {
            name: "contains",
            params: &[CoreTy::Str, CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_str_contains",
        },
        CoreMethod {
            name: "startsWith",
            params: &[CoreTy::Str, CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_str_starts_with",
        },
        CoreMethod {
            name: "endsWith",
            params: &[CoreTy::Str, CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_str_ends_with",
        },
        CoreMethod {
            name: "join",
            params: &[CoreTy::Array(&CoreTy::Str), CoreTy::Str],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_join",
        },
        CoreMethod {
            name: "split",
            params: &[CoreTy::Str, CoreTy::Str, CoreTy::Options(SPLIT_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "mwl_core_str_split",
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
            symbol: "mwl_core_str_replace",
        },
        CoreMethod {
            name: "padStart",
            params: &[CoreTy::Str, CoreTy::Uint, CoreTy::Str],
            defaults: &[Const::Str(" ")],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_pad_start",
        },
        CoreMethod {
            name: "padEnd",
            params: &[CoreTy::Str, CoreTy::Uint, CoreTy::Str],
            defaults: &[Const::Str(" ")],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_pad_end",
        },
        CoreMethod {
            name: "trim",
            params: &[CoreTy::Str, CoreTy::Options(TRIM_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_trim",
        },
        CoreMethod {
            name: "trimStart",
            params: &[CoreTy::Str, CoreTy::Options(TRIM_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_trim_start",
        },
        CoreMethod {
            name: "trimEnd",
            params: &[CoreTy::Str, CoreTy::Options(TRIM_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_trim_end",
        },
        CoreMethod {
            name: "repeat",
            params: &[CoreTy::Str, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_repeat",
        },
        CoreMethod {
            name: "lower",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_lower",
        },
        CoreMethod {
            name: "upper",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_upper",
        },
        CoreMethod {
            name: "upperFirst",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_upper_first",
        },
        CoreMethod {
            name: "lowerFirst",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_str_lower_first",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Str::split`'s `{limit?: int}` — [`mwl_core_str_split`]'s own docs own
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
    ty: CoreTy::Str,
    default: Const::Str(" \t\n\r\0\u{0b}"),
}];

/// `Core\Str::replace`'s `{caseInsensitive?: bool, limit?: uint}`.
///
/// [`mwl_core_str_replace`]'s own docs own both defaults — in particular why
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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain.
///
/// [`crate::symbols`] chains one of these per domain, so a new class adds an
/// arm here rather than to a single workspace-wide match.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_str_length" => (mwl_core_str_length as *const ()).cast(),
        "mwl_core_str_at" => (mwl_core_str_at as *const ()).cast(),
        "mwl_core_str_is_empty" => (mwl_core_str_is_empty as *const ()).cast(),
        "mwl_core_str_contains" => (mwl_core_str_contains as *const ()).cast(),
        "mwl_core_str_starts_with" => (mwl_core_str_starts_with as *const ()).cast(),
        "mwl_core_str_ends_with" => (mwl_core_str_ends_with as *const ()).cast(),
        "mwl_core_str_join" => (mwl_core_str_join as *const ()).cast(),
        "mwl_core_str_split" => (mwl_core_str_split as *const ()).cast(),
        "mwl_core_str_replace" => (mwl_core_str_replace as *const ()).cast(),
        "mwl_core_str_trim" => (mwl_core_str_trim as *const ()).cast(),
        "mwl_core_str_trim_start" => (mwl_core_str_trim_start as *const ()).cast(),
        "mwl_core_str_trim_end" => (mwl_core_str_trim_end as *const ()).cast(),
        "mwl_core_str_pad_start" => (mwl_core_str_pad_start as *const ()).cast(),
        "mwl_core_str_pad_end" => (mwl_core_str_pad_end as *const ()).cast(),
        "mwl_core_str_repeat" => (mwl_core_str_repeat as *const ()).cast(),
        "mwl_core_str_lower" => (mwl_core_str_lower as *const ()).cast(),
        "mwl_core_str_upper" => (mwl_core_str_upper as *const ()).cast(),
        "mwl_core_str_upper_first" => (mwl_core_str_upper_first as *const ()).cast(),
        "mwl_core_str_lower_first" => (mwl_core_str_lower_first as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument's text.
///
/// Two failures, both `FATAL` rather than `THROWN`: a non-string argument
/// means the checker let a call through it should have refused, and invalid
/// UTF-8 means ADR 0009's own invariant is broken. Neither is something a
/// program can catch its way out of.
fn text<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    let bytes = value.as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Str::{member} expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })?;
    std::str::from_utf8(bytes).map_err(|_| {
        Fault::fatal(format!(
            "Core\\Str::{member} received a `string` that is not valid UTF-8, which ADR 0009 \
             guarantees it cannot be"
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

/// A freshly built `string` result.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(MwlStr::new(text.as_bytes())))
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::length(string $s): uint` — how many characters the string
    /// holds, replacing PHP's `strlen` *and* `mb_strlen` at once.
    ///
    /// The unit is [`crate::granularity::DEFAULT`], which is ADR 0009 § 2's
    /// decision and is stated there and nowhere else. What that means for a
    /// PHP program being ported is the divergence row in that ADR's
    /// *Consequences*: `strlen("café")` is 5 and this is 4.
    ///
    /// O(n) in the string's bytes, and `granularity`'s own known gap owns the
    /// cached-count fix ADR 0009 names.
    fn mwl_core_str_length(_ctx, args: [1]) {
        let subject = text(&args[0], "length", "the subject")?;
        // `try_from` rather than `as`: `usize` is no wider than `u64` on any
        // target `deny.toml` builds for, so this cannot lose a digit, and
        // spelling it this way keeps the cast lints this crate denies from
        // needing a silence.
        let length = u64::try_from(crate::granularity::DEFAULT.length(subject))
            .map_err(|_| Fault::fatal("Core\\Str::length counted past `uint`"))?;
        Ok(Value::uint(length))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::at(string $s, int $index): string` — the one character at
    /// `$index`, replacing PHP's `$s[$i]` and `mb_substr($s, $i, 1)`.
    ///
    /// Two things separate it from PHP's `$s[$i]`, and both follow from rules
    /// already taken rather than being decided here:
    ///
    /// * **The unit is a character, not a byte** — [`crate::granularity`],
    ///   whose own docs record why a byte-indexed `at` cannot exist at all
    ///   under ADR 0009's UTF-8 invariant.
    /// * **An index that addresses nothing throws**, rather than PHP's warning
    ///   plus `""`. The declared return type is `string`, not `?string`, and
    ///   [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R4/R5
    ///   make that the difference between the two: absence would have to be
    ///   spelled in the type.
    ///
    /// A negative index counts from the end, which is R8's range rule applied
    /// to a range of one.
    fn mwl_core_str_at(_ctx, args: [2]) {
        let subject = text(&args[0], "at", "the subject")?;
        let index = integer(&args[1], "at", "the index")?;
        let unit = crate::granularity::DEFAULT;
        let found = unit.at(subject, index).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Str::at: index {index} is outside a string of {} characters",
                unit.length(subject)
            ))
        })?;
        produced(found)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::isEmpty(string $s): bool` — whether the string holds no
    /// characters at all, replacing PHP's `$s === ""` idiom.
    fn mwl_core_str_is_empty(_ctx, args: [1]) {
        Ok(Value::bool(text(&args[0], "isEmpty", "the subject")?.is_empty()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::contains(string $haystack, string $needle): bool` —
    /// replacing PHP's `str_contains` and `strstr` used as a predicate.
    ///
    /// An empty needle is contained in every string, including the empty one,
    /// which is both Rust's and PHP 8's answer.
    fn mwl_core_str_contains(_ctx, args: [2]) {
        let haystack = text(&args[0], "contains", "the subject")?;
        let needle = text(&args[1], "contains", "the needle")?;
        Ok(Value::bool(haystack.contains(needle)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::startsWith(string $s, string $prefix): bool` — replacing
    /// PHP's `str_starts_with`.
    fn mwl_core_str_starts_with(_ctx, args: [2]) {
        let subject = text(&args[0], "startsWith", "the subject")?;
        let prefix = text(&args[1], "startsWith", "the prefix")?;
        Ok(Value::bool(subject.starts_with(prefix)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::endsWith(string $s, string $suffix): bool` — replacing PHP's
    /// `str_ends_with`.
    fn mwl_core_str_ends_with(_ctx, args: [2]) {
        let subject = text(&args[0], "endsWith", "the subject")?;
        let suffix = text(&args[1], "endsWith", "the suffix")?;
        Ok(Value::bool(subject.ends_with(suffix)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::join(array<string> $parts, string $separator = ""): string`
    /// — replacing PHP's `implode`/`join`.
    ///
    /// The **first `Core` member with an optional parameter**: a call that
    /// omits the separator has the empty string materialized at the call site
    /// from `mwl_stdlib::registry::CoreMethod::defaults`, so this body always
    /// receives two arguments and knows nothing about defaults at all — see
    /// `mwl_types::defaults` for why the caller does that work.
    ///
    /// PHP's legacy argument-swapped `implode($glue, $array)` form has no
    /// counterpart: ADR 0063 R1 puts the subject first, and R20 leaves no room
    /// for a second spelling of one operation.
    fn mwl_core_str_join(_ctx, args: [2]) {
        let parts = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Str::join expected {:?} for the subject, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let separator = text(&args[1], "join", "the separator")?;

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
                let slot = mwl_runtime::mwl_array_next_slot(parts, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let mut value = Value::null();
                mwl_runtime::mwl_array_value_at(parts, slot, &raw mut value);
                (slot, value)
            };
            from = slot + 1;
            if !out.is_empty() || from > 1 {
                // Written against the cursor rather than a "first" flag so an
                // empty leading element still gets its separator.
                out.push_str(separator);
            }
            out.push_str(text(&value, "join", "an element")?);
        }
        produced(&out)
    }
}

mwl_runtime::mwl_helper! {
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
    /// the same reasons, as [`mwl_core_str_replace`]'s own `limit`.
    ///
    /// **An empty separator throws**, as PHP's `explode` does: there is no
    /// sensible piece boundary, and returning the subject unsplit would hide
    /// a computed separator that came out empty by mistake.
    fn mwl_core_str_split(_ctx, args: [3]) {
        let subject = text(&args[0], "split", "the subject")?;
        let separator = text(&args[1], "split", "the separator")?;
        let limit = integer(&args[2], "split", "the `limit` option")?;
        if separator.is_empty() {
            return Err(Fault::thrown(
                "Core\\Str::split(): the separator must not be empty",
            ));
        }

        let mut out = MwlArray::new();
        if limit >= 0 {
            // A limit of `0` means one piece, not none — see the docs above.
            let pieces = usize::try_from(limit).unwrap_or(usize::MAX).max(1);
            for piece in subject.splitn(pieces, separator) {
                out.append(Value::str(MwlStr::new(piece.as_bytes())));
            }
        } else {
            let dropped = usize::try_from(limit.unsigned_abs()).unwrap_or(usize::MAX);
            let all: Vec<&str> = subject.split(separator).collect();
            for piece in all.get(..all.len().saturating_sub(dropped)).unwrap_or(&[]) {
                out.append(Value::str(MwlStr::new(piece.as_bytes())));
            }
        }
        Ok(Value::array(out))
    }
}

/// `subject` with every leading and/or trailing character drawn from
/// `characters` removed — the shared body of `trim`/`trimStart`/`trimEnd`.
///
/// Two deliberate divergences from PHP's `trim`, both consequences of ADR 0009
/// making a `string` text rather than bytes, and of ADR 0063 R13 refusing a
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

mwl_runtime::mwl_helper! {
    /// `Core\Str::trim(string $s, {characters?: string}): string` — replacing
    /// PHP's `trim`. [`trimmed`] owns the two divergences from it, and
    /// `crate::registry`'s `TRIM_OPTIONS` owns the default set.
    fn mwl_core_str_trim(_ctx, args: [2]) {
        let subject = text(&args[0], "trim", "the subject")?;
        let characters = text(&args[1], "trim", "the `characters` option")?;
        produced(trimmed(subject, characters, true, true))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::trimStart(string $s, {characters?: string}): string` —
    /// replacing PHP's `ltrim`. See [`mwl_core_str_trim`].
    fn mwl_core_str_trim_start(_ctx, args: [2]) {
        let subject = text(&args[0], "trimStart", "the subject")?;
        let characters = text(&args[1], "trimStart", "the `characters` option")?;
        produced(trimmed(subject, characters, true, false))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::trimEnd(string $s, {characters?: string}): string` —
    /// replacing PHP's `rtrim`/`chop`. See [`mwl_core_str_trim`].
    fn mwl_core_str_trim_end(_ctx, args: [2]) {
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

mwl_runtime::mwl_helper! {
    /// `Core\Str::replace(string $s, string $search, string $replacement, {caseInsensitive?: bool, limit?: uint}): string`
    /// — replacing PHP's `str_replace` **and** `str_ireplace`, which are one
    /// member here because ADR 0063 R13/R20 leave no room for a second
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
    ///   A sentinel `0` would have been a magic value, and `?uint = null` is
    ///   the shape `mwl_types::defaults` cannot state yet
    ///   (`mwl-stdlib`'s known gap 3). A `limit` of `0` therefore means
    ///   exactly what it says: replace nothing.
    /// * **An empty `$search` replaces nothing**, rather than inserting the
    ///   replacement between every character or looping forever. PHP returns
    ///   the subject unchanged too.
    fn mwl_core_str_replace(_ctx, args: [5]) {
        let subject = text(&args[0], "replace", "the subject")?;
        let search = text(&args[1], "replace", "the search string")?;
        let replacement = text(&args[2], "replace", "the replacement")?;
        let case_insensitive = boolean(&args[3], "replace", "the `caseInsensitive` option")?;
        let limit = unsigned(&args[4], "replace", "the `limit` option")?;

        if search.is_empty() || limit == 0 {
            return produced(subject);
        }
        let mut out = String::with_capacity(subject.len());
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
        produced(&out)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::padStart(string $s, uint $length, string $padding = " "): string`
    /// — replacing PHP's `str_pad` with `STR_PAD_LEFT`.
    ///
    /// A subject already at least `$length` long comes back unchanged, and a
    /// padding run that does not divide evenly is truncated at the end nearest
    /// the subject — both PHP's behaviour, verified against 8.5.
    fn mwl_core_str_pad_start(_ctx, args: [3]) {
        let subject = text(&args[0], "padStart", "the subject")?;
        let length = count(&args[1], "padStart", "the target length")?;
        let padding = text(&args[2], "padStart", "the padding")?;
        let fill = padding_run(subject, length, padding, "padStart")?;
        produced(&(fill + subject))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::padEnd(string $s, uint $length, string $padding = " "): string`
    /// — replacing PHP's `str_pad` with `STR_PAD_RIGHT`. See
    /// [`mwl_core_str_pad_start`] for the shared rules.
    fn mwl_core_str_pad_end(_ctx, args: [3]) {
        let subject = text(&args[0], "padEnd", "the subject")?;
        let length = count(&args[1], "padEnd", "the target length")?;
        let padding = text(&args[2], "padEnd", "the padding")?;
        let fill = padding_run(subject, length, padding, "padEnd")?;
        produced(&(subject.to_owned() + &fill))
    }
}

/// The run of padding `padStart`/`padEnd` prepend or append — `padding`
/// repeated and then cut to exactly the shortfall, counted in
/// [`crate::granularity::DEFAULT`].
///
/// Empty padding throws rather than looping: it can never close a shortfall,
/// and PHP's own `str_pad` refuses it too.
fn padding_run(subject: &str, length: usize, padding: &str, member: &str) -> Result<String, Fault> {
    let unit = crate::granularity::DEFAULT;
    let have = unit.length(subject);
    if have >= length {
        return Ok(String::new());
    }
    if padding.is_empty() {
        return Err(Fault::thrown(format!(
            "Core\\Str::{member}: the padding is empty, so it can never reach the requested \
             length"
        )));
    }
    Ok(unit
        .pieces(padding)
        .cycle()
        .take(length - have)
        .collect::<String>())
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::repeat(string $s, uint $times): string` — replacing PHP's
    /// `str_repeat`. Zero times is the empty string, as in PHP.
    fn mwl_core_str_repeat(_ctx, args: [2]) {
        let subject = text(&args[0], "repeat", "the subject")?;
        let times = count(&args[1], "repeat", "the repeat count")?;
        // `str::repeat` panics on capacity overflow, which would be a contained
        // FATAL rather than a catchable failure — so the size is checked first
        // and reported as an ordinary throw instead.
        subject
            .len()
            .checked_mul(times)
            .filter(|bytes| isize::try_from(*bytes).is_ok())
            .ok_or_else(|| {
                Fault::thrown(
                    "Core\\Str::repeat: the requested repetition is larger than any string this \
                     process could hold",
                )
            })?;
        produced(&subject.repeat(times))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::lower(string $s): string` — replacing PHP's `strtolower` and
    /// `mb_strtolower`. Unicode's full lowercase mapping; see this module's
    /// docs for why there is only one of them.
    fn mwl_core_str_lower(_ctx, args: [1]) {
        produced(&text(&args[0], "lower", "the subject")?.to_lowercase())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::upper(string $s): string` — replacing PHP's `strtoupper` and
    /// `mb_strtoupper`.
    fn mwl_core_str_upper(_ctx, args: [1]) {
        produced(&text(&args[0], "upper", "the subject")?.to_uppercase())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::upperFirst(string $s): string` — replacing PHP's `ucfirst`.
    ///
    /// Only the first character changes; the rest is copied through, which is
    /// what separates this from title casing (`ucwords`, which the spec's § 1
    /// note keeps out of `Core` entirely because word segmentation is
    /// locale-dependent).
    fn mwl_core_str_upper_first(_ctx, args: [1]) {
        produced(&map_first(text(&args[0], "upperFirst", "the subject")?, true))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Str::lowerFirst(string $s): string` — replacing PHP's `lcfirst`.
    fn mwl_core_str_lower_first(_ctx, args: [1]) {
        produced(&map_first(text(&args[0], "lowerFirst", "the subject")?, false))
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
    use mwl_runtime::{Ctx, MwlArray, MwlStr, OutputSink, Value, call};

    /// Runs one member through the ADR 0002 boundary compiled code reaches it
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
        Value::str(MwlStr::new(text.as_bytes()))
    }

    #[test]
    fn case_conversion_uses_unicodes_mapping_not_a_byte_wise_one() {
        assert_eq!(
            taken(run(super::mwl_core_str_upper, &[s("straße")]).expect("upper never fails")),
            "STRASSE"
        );
        assert_eq!(
            taken(run(super::mwl_core_str_lower, &[s("ÄRGER")]).expect("lower never fails")),
            "ärger"
        );
        assert_eq!(
            taken(
                run(super::mwl_core_str_upper_first, &[s("ärger")])
                    .expect("upperFirst never fails")
            ),
            "Ärger"
        );
        assert_eq!(
            taken(
                run(super::mwl_core_str_lower_first, &[s("ÄRGER")])
                    .expect("lowerFirst never fails")
            ),
            "äRGER"
        );
    }

    #[test]
    fn case_conversion_of_the_empty_string_is_the_empty_string() {
        assert_eq!(
            taken(run(super::mwl_core_str_upper_first, &[s("")]).expect("no failure")),
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
            assert_eq!(call_it(super::mwl_core_str_contains), contains, "{needle}");
            assert_eq!(call_it(super::mwl_core_str_starts_with), starts, "{needle}");
            assert_eq!(call_it(super::mwl_core_str_ends_with), ends, "{needle}");
        }
    }

    #[test]
    fn join_walks_the_array_in_insertion_order() {
        let mut parts = MwlArray::new();
        parts.set(MwlStr::new(b"0"), s("a"));
        parts.set(MwlStr::new(b"1"), s("b"));
        parts.set(MwlStr::new(b"2"), s("c"));
        assert_eq!(
            taken(
                run(super::mwl_core_str_join, &[Value::array(parts), s("|")]).expect("no failure")
            ),
            "a|b|c"
        );
    }

    /// The pieces `split` produces, read back in order.
    fn split_at(subject: &str, separator: &str, limit: i64) -> Vec<String> {
        let result = run(
            super::mwl_core_str_split,
            &[s(subject), s(separator), Value::int(limit)],
        )
        .expect("a non-empty separator never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the \
                      handle takes over and releases on drop"
        )]
        let array =
            unsafe { MwlArray::from_raw(result.array_ptr().expect("split returns an array")) };
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
        let status = run(super::mwl_core_str_split, &[s("a b"), s(""), Value::int(9)])
            .expect_err("an empty separator is refused");
        assert_eq!(status, mwl_runtime::THROWN);
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
            trim(super::mwl_core_str_trim, " \thi\n ", php_default),
            "hi"
        );
        assert_eq!(
            trim(super::mwl_core_str_trim_start, "  hi  ", php_default),
            "hi  "
        );
        assert_eq!(
            trim(super::mwl_core_str_trim_end, "  hi  ", php_default),
            "  hi"
        );
        assert_eq!(trim(super::mwl_core_str_trim, "xxhixx", "x"), "hi");
        // Only the characters named: a written set replaces the default.
        assert_eq!(trim(super::mwl_core_str_trim, " xhix ", "x"), " xhix ");
        // ADR 0063 R13: `a..z` is three characters, not a range.
        assert_eq!(trim(super::mwl_core_str_trim, "abc", "a..z"), "bc");
    }

    /// `replace` with both options at their defaults, which is what a call
    /// site that writes no bag at all passes. Every row verified against PHP
    /// 8.5's `str_replace`.
    fn replaced(subject: &str, search: &str, replacement: &str) -> String {
        taken(
            run(
                super::mwl_core_str_replace,
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

    /// The `caseInsensitive` option is what makes this member subsume
    /// `str_ireplace` as well, and the match's byte length comes out of the
    /// subject — the whole reason `find_from` returns one.
    #[test]
    fn replace_is_case_insensitive_only_when_the_option_says_so() {
        let run_ci = |ci: bool| {
            taken(
                run(
                    super::mwl_core_str_replace,
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
                    super::mwl_core_str_replace,
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
        let mut parts = MwlArray::new();
        parts.set(MwlStr::new(b"0"), s(""));
        parts.set(MwlStr::new(b"1"), s("b"));
        assert_eq!(
            taken(
                run(super::mwl_core_str_join, &[Value::array(parts), s("-")]).expect("no failure")
            ),
            "-b"
        );
    }

    #[test]
    fn joining_nothing_is_the_empty_string() {
        assert_eq!(
            taken(
                run(
                    super::mwl_core_str_join,
                    &[Value::array(MwlArray::new()), s(",")]
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
                        super::mwl_core_str_pad_start,
                        &[s(subject), Value::uint(length), s(padding)]
                    )
                    .expect("no failure")
                ),
                start
            );
            assert_eq!(
                taken(
                    run(
                        super::mwl_core_str_pad_end,
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
            super::mwl_core_str_pad_start,
            &[s("ab"), Value::uint(5), s("")],
        )
        .expect_err("an empty padding can never reach the length");
        assert_eq!(status, mwl_runtime::THROWN);
    }

    #[test]
    fn repeating_zero_times_is_the_empty_string() {
        assert_eq!(
            taken(run(super::mwl_core_str_repeat, &[s("ab"), Value::uint(0)]).expect("no failure")),
            ""
        );
        assert_eq!(
            taken(run(super::mwl_core_str_repeat, &[s("ab"), Value::uint(3)]).expect("no failure")),
            "ababab"
        );
    }

    /// A repetition too large to allocate is a throw, not the panic
    /// `str::repeat` would raise — contained either way, but only one of the
    /// two is something a program can catch.
    #[test]
    fn an_unrepresentable_repetition_throws() {
        let status = run(
            super::mwl_core_str_repeat,
            &[s("ab"), Value::uint(u64::MAX)],
        )
        .expect_err("no string that long can exist");
        assert_eq!(status, mwl_runtime::THROWN);
    }

    #[test]
    fn a_non_string_argument_is_a_contained_fault() {
        let status =
            run(super::mwl_core_str_upper, &[Value::int(7)]).expect_err("an int is not a string");
        assert_eq!(status, mwl_runtime::FATAL);
    }

    #[test]
    fn is_empty_answers_for_both_shapes() {
        assert_eq!(
            run(super::mwl_core_str_is_empty, &[s("")])
                .expect("no failure")
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            run(super::mwl_core_str_is_empty, &[s("a")])
                .expect("no failure")
                .as_bool(),
            Some(false)
        );
    }

    /// `length` counts characters, which is the answer PHP needs two functions
    /// and a correct `mb_internal_encoding` to reach — and does not reach for
    /// the last row at all, since `strlen` says 25 and `mb_strlen` says 5.
    #[test]
    fn length_counts_characters_not_bytes_or_code_points() {
        for (subject, want) in [
            ("", 0u64),
            ("mwl", 3),
            ("cafe\u{301}", 4),
            ("\u{1f1e6}\u{1f1f9}", 1),
            ("\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}", 1),
        ] {
            assert_eq!(
                run(super::mwl_core_str_length, &[s(subject)])
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
            ("mwl", 0i64, "m"),
            ("mwl", 2, "l"),
            ("mwl", -1, "l"),
            ("cafe\u{301}", 3, "e\u{301}"),
            ("cafe\u{301}", -1, "e\u{301}"),
        ] {
            assert_eq!(
                taken(
                    run(super::mwl_core_str_at, &[s(subject), Value::int(index)])
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
            let status = run(super::mwl_core_str_at, &[s("mwl"), Value::int(index)])
                .expect_err("the index addresses nothing");
            assert_eq!(status, mwl_runtime::THROWN, "at(\"mwl\", {index})");
        }
        let status = run(super::mwl_core_str_at, &[s(""), Value::int(0)])
            .expect_err("the empty string has no characters");
        assert_eq!(status, mwl_runtime::THROWN);
    }

    /// Padding measures in the same unit as `length`, so a target of 3 over a
    /// one-character emoji adds two pads rather than the twenty-one bytes a
    /// byte-counting `str_pad` would.
    #[test]
    fn padding_measures_the_same_unit_length_counts() {
        assert_eq!(
            taken(
                run(
                    super::mwl_core_str_pad_start,
                    &[s("\u{1f1e6}\u{1f1f9}"), Value::uint(3), s(".")]
                )
                .expect("no failure")
            ),
            "..\u{1f1e6}\u{1f1f9}"
        );
    }
}
