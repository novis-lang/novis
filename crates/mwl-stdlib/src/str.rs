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
//! # The one thing ADR 0009 has not settled yet
//!
//! `padStart`/`padEnd` measure their `$length` argument in **code points**.
//! That is the same open question ADR 0009 parks for `length`/`at`/`slice` —
//! code point versus grapheme — and the spec's § 1 preamble says as much. None
//! of those three is registered yet, so nothing here can disagree with them;
//! when that ADR lands, this module follows it, in these two functions and
//! nowhere else. Every other member is granularity-independent.
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
/// repeated and then cut to exactly the shortfall, in code points (see this
/// module's docs for why that unit, and what settles it).
///
/// Empty padding throws rather than looping: it can never close a shortfall,
/// and PHP's own `str_pad` refuses it too.
fn padding_run(subject: &str, length: usize, padding: &str, member: &str) -> Result<String, Fault> {
    let have = subject.chars().count();
    if have >= length {
        return Ok(String::new());
    }
    if padding.is_empty() {
        return Err(Fault::thrown(format!(
            "Core\\Str::{member}: the padding is empty, so it can never reach the requested \
             length"
        )));
    }
    Ok(padding
        .chars()
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
}
