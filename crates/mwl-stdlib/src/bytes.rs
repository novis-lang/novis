//! `Core\Bytes` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 7's second class, over `mwl_runtime`'s `Bytes`-tagged `MwlStr`.
//!
//! Every member here is pure (ADR 0063 R3) and borrows its subject rather than
//! consuming it, exactly as [`crate::str`] does — see [`crate`]'s own docs for
//! why that falls out of being a helper.
//!
//! # What is registered here so far
//!
//! § 7's indexing half — `length`, `at`, `slice`, `indexOf`, `compare` — its
//! three predicates, and two of its three builders: `fill` and `repeat`.
//!
//! **`join` is unwritten, and it is blocked rather than merely unreached.**
//! § 7 gives it as `join(array<bytes> $parts, bytes $separator = "")`, and
//! [`crate::registry::Const`] has no `bytes` variant to state that default
//! with: `Const::Str("")` would materialize a `Str`-tagged value into a
//! `bytes` parameter, which is a type lie the helper would have to `FATAL` on.
//! Adding one is the same missing capability as ADR 0009 § 3's `string as
//! bytes` conversion row, which `mwl_ir::lower::expr` still panics for — both
//! want one constant buffer built at a call site. `pack`/`unpack` are
//! unwritten for the ordinary reason.
//!
//! # The unit is the byte, and that is the whole difference from `Core\Str`
//!
//! § 7 pairs each member below with [`crate::str`]'s member of the same name,
//! and R6 makes the names identical because the *operation* is identical. What
//! differs is the unit: `Core\Str` counts in
//! [`crate::granularity::DEFAULT`]'s grapheme clusters, and this counts in
//! bytes — which [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) § 1
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
//!   ADR 0063 R4/R5's: absence would have to be spelled `?uint` in the type.
//! - **`indexOf` takes `{from?: int}` and no `caseInsensitive`.**
//!   `Core\Str::indexOf`'s second option is a Unicode case folding, and there
//!   is no case in a byte string — a `bytes` carries no charset, which is the
//!   premise of § 7's whole `Core\Encoding`/`Core\Bytes` split. An ASCII-only
//!   folding would be that guess made silently, which ADR 0009 § 3 removes
//!   from the language. A caller who wants one decodes first. `from` survives
//!   unchanged, because a scan resuming where the last one stopped is exactly
//!   what a sniffing loop is.
//! - **`compare` answers `int`, negative/zero/positive, never a `bool`.** It
//!   is `memcmp`'s question and PHP's `strcmp`'s: lexicographic over unsigned
//!   octets. `Core\Str` has no `compare` yet, and when it grows one it takes
//!   this shape over its own unit. This is *not* the `==` operator
//!   ([ADR 0090](../../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)),
//!   which already compares two `bytes` for equality and is what a program
//!   should write when that is the question; `compare` exists for the ordering
//!   `==` does not answer.
//! - **`fill` is `fill(uint $length, uint $byte)`, length first.** It is the
//!   one member with no subject at all — it builds a buffer rather than
//!   answering about one — so ADR 0063 R1 does not order it and `Core\Str`'s
//!   nearest row does: `padStart(string $s, uint $length, string $padding)`
//!   writes the size before the thing repeated into it, and a zero-filled
//!   header reads as `Core\Bytes::fill(16, 0)`. A `$byte` above 255 throws
//!   rather than truncating, which is R4 — the parameter is `uint` because
//!   that is what `at` answers, so the two compose.
//!
//! **What it spends:** nothing per value. A `bytes` is the `MwlStr`
//! allocation it already was (`mwl-runtime`'s module doc § *`bytes` is a tag,
//! not a second heap shape*), `slice` allocates its result and the other four
//! members allocate nothing at all.

use mwl_runtime::{Fault, HelperResult, MwlStr, Tag, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

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
            params: &[CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_bytes_length",
        },
        CoreMethod {
            name: "at",
            params: &[CoreTy::Bytes, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_bytes_at",
        },
        CoreMethod {
            name: "slice",
            params: &[CoreTy::Bytes, CoreTy::Int, CoreTy::Nullable(&CoreTy::Int)],
            defaults: &[Const::Null],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_bytes_slice",
        },
        CoreMethod {
            name: "indexOf",
            params: &[
                CoreTy::Bytes,
                CoreTy::Bytes,
                CoreTy::Options(INDEX_OF_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Uint),
            symbol: "mwl_core_bytes_index_of",
        },
        CoreMethod {
            name: "compare",
            params: &[CoreTy::Bytes, CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_bytes_compare",
        },
        CoreMethod {
            name: "contains",
            params: &[CoreTy::Bytes, CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_bytes_contains",
        },
        CoreMethod {
            name: "startsWith",
            params: &[CoreTy::Bytes, CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_bytes_starts_with",
        },
        CoreMethod {
            name: "endsWith",
            params: &[CoreTy::Bytes, CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_bytes_ends_with",
        },
        CoreMethod {
            name: "fill",
            params: &[CoreTy::Uint, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_bytes_fill",
        },
        CoreMethod {
            name: "repeat",
            params: &[CoreTy::Bytes, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "mwl_core_bytes_repeat",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Bytes::indexOf`'s `{from?: int}` — the module doc owns why
/// `Core\Str::indexOf`'s second option has no counterpart here.
///
/// `from` is a **position**, so it obeys ADR 0063 R8's sign rule and reads
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
        "mwl_core_bytes_length" => (mwl_core_bytes_length as *const ()).cast(),
        "mwl_core_bytes_at" => (mwl_core_bytes_at as *const ()).cast(),
        "mwl_core_bytes_slice" => (mwl_core_bytes_slice as *const ()).cast(),
        "mwl_core_bytes_index_of" => (mwl_core_bytes_index_of as *const ()).cast(),
        "mwl_core_bytes_compare" => (mwl_core_bytes_compare as *const ()).cast(),
        "mwl_core_bytes_contains" => (mwl_core_bytes_contains as *const ()).cast(),
        "mwl_core_bytes_starts_with" => (mwl_core_bytes_starts_with as *const ()).cast(),
        "mwl_core_bytes_ends_with" => (mwl_core_bytes_ends_with as *const ()).cast(),
        "mwl_core_bytes_fill" => (mwl_core_bytes_fill as *const ()).cast(),
        "mwl_core_bytes_repeat" => (mwl_core_bytes_repeat as *const ()).cast(),
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

/// A byte count that is about to become an allocation, refused before it is
/// attempted.
///
/// `Vec`'s own growth panics on capacity overflow, which would be a contained
/// `FATAL` rather than something a program can catch — so the size is checked
/// here and reported as an ordinary throw instead, exactly as
/// `Core\Str::repeat` does.
fn affordable(bytes: Option<usize>, member: &str) -> Result<usize, Fault> {
    bytes
        .filter(|size| isize::try_from(*size).is_ok())
        .ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Bytes::{member}: the requested buffer is larger than any this process \
                 could hold"
            ))
        })
}

/// A signed **position** as a byte offset into a subject of `total` bytes,
/// with a negative one counting from the end and either end saturating.
///
/// [`crate::granularity::Unit::byte_of_signed_index`]'s counterpart for a
/// subject whose unit is already the byte, and it saturates for that method's
/// reason: a search or a slice starting past the end finds nothing, which
/// composes with a loop where a throw would not. ADR 0063 R8 is the sign rule.
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
    Ok(Value::bytes(MwlStr::new(octets)))
}

// ============================================================================
// The members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::length(bytes $b): uint` — how many octets the buffer
    /// holds, replacing PHP's `strlen` used on binary data.
    ///
    /// O(1), and this is the member where the byte unit pays off most
    /// visibly: `Core\Str::length` walks the subject to count grapheme
    /// clusters, and this reads a header field.
    fn mwl_core_bytes_length(_ctx, args: [1]) {
        let subject = raw(&args[0], "length", "the subject")?;
        Ok(Value::uint(counted(subject.len(), "length")?))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::at(bytes $b, int $index): uint` — the one octet at
    /// `$index`, replacing PHP's `ord($s[$i])`.
    ///
    /// The two shapes that separate it from `Core\Str::at` — a `uint` answer,
    /// and an index counted in bytes — are this module's own docs' second and
    /// third headings. A negative index counts from the end (ADR 0063 R8's
    /// range rule applied to a range of one), and an index that addresses
    /// nothing throws rather than answering a sentinel.
    fn mwl_core_bytes_at(_ctx, args: [2]) {
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

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::slice(bytes $b, int $offset, ?int $length = null): bytes`
    /// — replacing PHP's `substr` used on binary data.
    ///
    /// `Core\Str::slice`'s rules, counted in bytes rather than in grapheme
    /// clusters, so both arguments follow ADR 0063 R8's sign rule:
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
    fn mwl_core_bytes_slice(_ctx, args: [3]) {
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

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::indexOf(bytes $haystack, bytes $needle, {from?: int}): ?uint`
    /// — replacing PHP's `strpos` used on binary data.
    ///
    /// **Absence is `null`, not `false`**, which is ADR 0063 R5 and the same
    /// correctness win it is on `Core\Str::indexOf`: a `?uint` has no falsy
    /// member that `0` could be confused with, so PHP's `strpos(…) == false`
    /// bug family cannot be written.
    ///
    /// The answer is a byte offset, so it is directly usable as this class's
    /// own `slice` offset — the property that would break if the two members
    /// counted in different units, which is exactly what `Core\Str` has to work
    /// to preserve and this class gets for free.
    fn mwl_core_bytes_index_of(_ctx, args: [3]) {
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

mwl_runtime::mwl_helper! {
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
    fn mwl_core_bytes_compare(_ctx, args: [2]) {
        let left = raw(&args[0], "compare", "the first buffer")?;
        let right = raw(&args[1], "compare", "the second buffer")?;
        Ok(Value::int(match left.cmp(right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::contains(bytes $haystack, bytes $needle): bool` —
    /// replacing PHP's `str_contains` used on binary data.
    ///
    /// The first of the three predicates R6 says this class exists for: a
    /// magic-byte check is a `bool` question, and routing it through
    /// `indexOf(...) != null` would make the caller handle an absence it does
    /// not care about. An empty needle is contained in every buffer, including
    /// the empty one — Rust's answer and PHP 8's, and `Core\Str::contains`'s.
    fn mwl_core_bytes_contains(_ctx, args: [2]) {
        let haystack = raw(&args[0], "contains", "the subject")?;
        let needle = raw(&args[1], "contains", "the needle")?;
        Ok(Value::bool(find(haystack, needle).is_some()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::startsWith(bytes $b, bytes $prefix): bool` — replacing
    /// PHP's `str_starts_with` used on binary data, and the member a format
    /// sniff actually writes.
    fn mwl_core_bytes_starts_with(_ctx, args: [2]) {
        let subject = raw(&args[0], "startsWith", "the subject")?;
        let prefix = raw(&args[1], "startsWith", "the prefix")?;
        Ok(Value::bool(subject.starts_with(prefix)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::endsWith(bytes $b, bytes $suffix): bool` — replacing PHP's
    /// `str_ends_with` used on binary data.
    fn mwl_core_bytes_ends_with(_ctx, args: [2]) {
        let subject = raw(&args[0], "endsWith", "the subject")?;
        let suffix = raw(&args[1], "endsWith", "the suffix")?;
        Ok(Value::bool(subject.ends_with(suffix)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::fill(uint $length, uint $byte): bytes` — a buffer of
    /// `$length` copies of one octet, replacing PHP's `str_repeat(chr($b), $n)`
    /// idiom.
    ///
    /// The module doc's fourth heading owns the argument order and why `$byte`
    /// is a `uint`. **A value above 255 throws** rather than being truncated to
    /// its low octet, which is ADR 0063 R4: a caller who computed 256 has a
    /// bug, and PHP's `chr()` wrapping it to `"\0"` is the silent-substitution
    /// failure this language does not do.
    ///
    /// Zero length is the empty buffer, as `Core\Str::repeat`'s zero count is
    /// the empty string.
    fn mwl_core_bytes_fill(_ctx, args: [2]) {
        let length = count(&args[0], "fill", "the length")?;
        let byte = unsigned(&args[1], "fill", "the byte")?;
        let octet = u8::try_from(byte).map_err(|_| {
            Fault::thrown(format!(
                "Core\\Bytes::fill: {byte} is not one octet — a byte is 0 to 255"
            ))
        })?;
        affordable(Some(length), "fill")?;
        produced(&vec![octet; length])
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Bytes::repeat(bytes $b, uint $times): bytes` — replacing PHP's
    /// `str_repeat` used on binary data. Zero times is the empty buffer, as in
    /// PHP and as in `Core\Str::repeat`.
    fn mwl_core_bytes_repeat(_ctx, args: [2]) {
        let subject = raw(&args[0], "repeat", "the subject")?;
        let times = count(&args[1], "repeat", "the repeat count")?;
        affordable(subject.len().checked_mul(times), "repeat")?;
        produced(&subject.repeat(times))
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
}
