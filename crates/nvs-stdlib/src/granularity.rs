//! ADR 0009 § 2's granularity, in one place: what unit a `string`'s length,
//! indexing and slicing count in.
//!
//! [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) § 2 is settled
//! here — [`DEFAULT`] is the answer, and that ADR's own body states the rule.
//! Every `Core\Str` member that has a unit at all reaches for [`DEFAULT`]
//! rather than choosing one, so there is exactly one place a granularity is
//! decided and no two members can disagree about it.
//!
//! # Why `unicode-segmentation`
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4's two
//! questions: UAX #29 is an external specification with a mature pure-Rust
//! implementation, so it is a dependency rather than ours to write.
//! `unicode-segmentation` is the crate the Rust ecosystem's own text tooling
//! uses, is `no_std`-capable, has no build script and no C, and is MIT OR
//! Apache-2.0 — so `cargo deny check` needs no exception for it. The
//! alternative considered was `icu_segmenter`: correct too, and carrying a
//! data-provider architecture and a locale story that
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) already
//! rules out of `Core`.
//!
//! # The Unicode version is part of the answer
//!
//! ADR 0009's *Consequences* says this out loud: what counts as one character
//! is pinned to whichever Unicode version this crate embeds, and can move
//! across a dependency bump. That is [`unicode_segmentation::UNICODE_VERSION`],
//! and `the_embedded_unicode_version_is_recorded` prints it, so a bump that
//! changes it is visible in a test run rather than in a user's diff.
//!
//! # Known gap
//!
//! A grapheme count is recomputed on every call. ADR 0009's *Consequences*
//! names the fix — cache the count in the string's header, computed lazily on
//! first use — and that is a `nvs_runtime::NvsStr` change, not this module's;
//! `docs/implementation-plan.md`'s M4S paragraph carries it. Nothing here
//! changes when it lands: the seam is already the only caller.

use unicode_segmentation::UnicodeSegmentation;

/// The unit a `string` operation counts in.
///
/// **Two units, not three.** ADR 0009 § 2 named byte length as its fallback
/// default, and that fallback is closed off here rather than merely
/// out-measured: `string` is guaranteed valid UTF-8, so a byte-indexed
/// `Core\Str::at` would have to hand back the interior byte of a multi-byte
/// character — a `string` that cannot exist. A byte-length default would
/// therefore leave `length` counting one unit and `at`/`slice` addressing
/// another, which is the exact "does length mean what I think" failure ADR 0009
/// was opened to remove. Byte length remains reachable, where it means
/// something: on `bytes`, whose whole point is that it has no other unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// One Unicode scalar value — what `Core\Str::codePoints` enumerates.
    ///
    /// Deliberately *not* [`DEFAULT`]: ADR 0009's *Alternatives rejected*
    /// turns it down for answering "how many scalar values" rather than "how
    /// many characters a person sees" — a flag emoji is two, an accented
    /// letter built from combining marks is two.
    CodePoint,
    /// One extended grapheme cluster (UAX #29) — what a person reading the
    /// source would call one character.
    Grapheme,
}

/// The unit `Core\Str::length`, `at` and `slice` count in — ADR 0009 § 2's
/// decision, stated once.
///
/// The cost that decides it is measured by
/// `a_grapheme_index_costs_more_than_a_code_point_index` in
/// [`benches/abi-probe`](../../../../benches/abi-probe/), which is where the
/// figure and its bound live; ADR 0009's *Revisiting* asked for exactly that
/// test and its § 2 records the outcome.
pub const DEFAULT: Unit = Unit::Grapheme;

/// Whether one byte is one grapheme cluster throughout `subject`.
///
/// True for ASCII with no carriage return, and this is exact rather than a
/// heuristic. Inside U+0000..U+007F, UAX #29 assigns every code point the
/// `Control`, `CR`, `LF` or `Other` property — there is no `Extend`,
/// `Prepend`, `SpacingMark`, `ZWJ` or `Regional_Indicator` in ASCII — and the
/// only rule that joins two of them is GB3, `CR × LF`. Exclude `CR` and every
/// remaining rule breaks between every pair, so the cluster count is the byte
/// count and the k-th cluster is the k-th byte.
///
/// This is what makes ADR 0009 § 2's decision affordable rather than merely
/// correct: the two disqualifiers are folded into **one** branchless pass, so
/// the overwhelmingly common case — a request path full of ASCII — pays a
/// single vectorizable scan rather than a segmentation one.
/// `a_grapheme_index_costs_more_than_a_code_point_index` measures both halves
/// of that claim.
///
/// The obvious spelling, `subject.is_ascii() && !bytes.contains(&b'\r')`, is
/// two vectorized passes over the same buffer where one does. Each byte
/// contributes its high bit — set exactly when it is non-ASCII — or the high
/// bit of `(x - 1) & !x` for `x = b ^ b'\r'`, which is the classic zero-byte
/// test and is set exactly when `b` is `CR`. The fold has no early exit, which
/// is what lets it vectorize; the only subject an early exit would have saved
/// work on is one that then goes down the segmentation path, which costs
/// strictly more than the bytes this skips.
fn one_byte_per_cluster(subject: &str) -> bool {
    subject.as_bytes().iter().fold(0_u8, |flags, &byte| {
        let cr = byte ^ b'\r';
        flags | byte | (cr.wrapping_sub(1) & !cr)
    }) & 0x80
        == 0
}

impl Unit {
    /// How many of this unit `subject` holds.
    ///
    /// O(n) in both units and allocation-free in both.
    #[must_use]
    pub fn length(self, subject: &str) -> usize {
        match self {
            Self::CodePoint => subject.chars().count(),
            Self::Grapheme if one_byte_per_cluster(subject) => subject.len(),
            Self::Grapheme => subject.graphemes(true).count(),
        }
    }

    /// `subject` split into one `&str` per unit, in order.
    ///
    /// One iterator type rather than a boxed `dyn Iterator`: a `Core` member
    /// calls this on the request path, and the box would be an allocation per
    /// call for a choice known at the branch.
    #[must_use]
    pub fn pieces(self, subject: &str) -> Pieces<'_> {
        match self {
            Self::CodePoint => Pieces::CodePoints {
                subject,
                indices: subject.char_indices(),
            },
            Self::Grapheme if one_byte_per_cluster(subject) => Pieces::Bytes { subject, next: 0 },
            Self::Grapheme => Pieces::Graphemes(subject.graphemes(true)),
        }
    }

    /// The `index`-th unit of `subject`, counting from the end when `index` is
    /// negative, or `None` when it addresses nothing.
    ///
    /// Negative-from-the-end is the same rule
    /// [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R8 fixes
    /// for every range in the spec; `at` is a range of one.
    #[must_use]
    pub fn at(self, subject: &str, index: i64) -> Option<&str> {
        let from_start = if index < 0 {
            let total = i64::try_from(self.length(subject)).ok()?;
            total.checked_add(index)?
        } else {
            index
        };
        usize::try_from(from_start)
            .ok()
            .and_then(|offset| self.pieces(subject).nth(offset))
    }

    /// How many units of `subject` end at or before byte offset `byte` — the
    /// index a caller who counts in this unit would give the character starting
    /// there.
    ///
    /// The bridge every member that talks to a **byte**-addressed engine needs:
    /// `Core\Regex` runs on two crates that report a match in bytes, while
    /// ADR 0009 § 2 says every `string` position Novis hands back or takes is in
    /// [`DEFAULT`]'s unit. Converting at that seam is what keeps
    /// `Core\Regex\Match::offset` and `Core\Str::indexOf` answering in one unit.
    ///
    /// **O(`byte`)**, allocation-free: the prefix is counted, not indexed. A
    /// caller converting many offsets over one subject therefore pays per
    /// offset — see [`crate::regex`]'s own gap note, which owns that cost for
    /// the one member that converts more than one.
    ///
    /// # Panics
    ///
    /// If `byte` is not a character boundary of `subject`, which is a slicing
    /// bug rather than input: both engines report a match at a boundary.
    #[must_use]
    pub fn index_of_byte(self, subject: &str, byte: usize) -> usize {
        self.length(&subject[..byte])
    }

    /// The byte offset a **signed** unit index names, with a negative one
    /// counting from the end and either end saturating.
    ///
    /// [`Self::byte_of_index`] with
    /// [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R8's sign
    /// rule applied first, so every member that takes a *position* — the `from`
    /// of `Core\Regex::match` and `Core\Str::indexOf`, the `before` of
    /// `lastIndexOf`, the `offset` of `slice` — reads it the same way. It
    /// saturates rather than answering `None` for the reason
    /// [`Self::byte_of_index`] does: a search starting past the end finds
    /// nothing, which composes with a loop where a throw would not.
    #[must_use]
    pub fn byte_of_signed_index(self, subject: &str, index: i64) -> usize {
        let from_start = if index < 0 {
            let total = i64::try_from(self.length(subject)).unwrap_or(i64::MAX);
            total.saturating_add(index).max(0)
        } else {
            index
        };
        self.byte_of_index(subject, usize::try_from(from_start).unwrap_or(usize::MAX))
    }

    /// The byte offset unit `index` of `subject` starts at, or the subject's
    /// whole length for an index at or past its end.
    ///
    /// [`Self::index_of_byte`]'s inverse, and the direction an *incoming*
    /// position converts in — `Core\Regex::match`'s `from` option is a
    /// [`DEFAULT`]-unit index the engine has to be given in bytes. Saturating
    /// at the end rather than answering `None` is what makes a search from past
    /// the end find nothing instead of throwing, which is the answer that
    /// composes with a loop.
    #[must_use]
    pub fn byte_of_index(self, subject: &str, index: usize) -> usize {
        let mut consumed = 0;
        for (seen, piece) in self.pieces(subject).enumerate() {
            if seen == index {
                return consumed;
            }
            consumed += piece.len();
        }
        consumed
    }
}

/// [`Unit::pieces`]'s iterator — one `&str` per unit of the subject.
#[derive(Clone, Debug)]
pub enum Pieces<'a> {
    /// [`Unit::CodePoint`]: each scalar value, re-borrowed from the subject so
    /// the item type matches the grapheme arm's without allocating a `String`.
    CodePoints {
        /// The whole subject, which each item is a sub-slice of.
        subject: &'a str,
        /// Rust's own code-point walk, which supplies the byte offset the
        /// sub-slice is cut at.
        indices: std::str::CharIndices<'a>,
    },
    /// [`Unit::Grapheme`] over a subject [`one_byte_per_cluster`] accepted:
    /// one byte per item, with no segmentation run at all.
    Bytes {
        /// The whole subject, which each item is a one-byte sub-slice of.
        subject: &'a str,
        /// The byte offset the next item starts at.
        next: usize,
    },
    /// [`Unit::Grapheme`]: UAX #29's extended clusters, `unicode-segmentation`'s
    /// own iterator, which already yields `&str`.
    Graphemes(unicode_segmentation::Graphemes<'a>),
}

impl<'a> Iterator for Pieces<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        match self {
            Self::CodePoints { subject, indices } => {
                let (start, found) = indices.next()?;
                Some(&subject[start..start + found.len_utf8()])
            }
            Self::Bytes { subject, next } => {
                let start = *next;
                if start >= subject.len() {
                    return None;
                }
                *next = start + 1;
                // The subject passed `one_byte_per_cluster`, so every byte
                // offset in it is a character boundary and this cannot panic.
                Some(&subject[start..start + 1])
            }
            Self::Graphemes(graphemes) => graphemes.next(),
        }
    }

    /// O(1) for the one-byte-per-cluster arm, which is what keeps
    /// `Core\Str::at` on an ASCII subject from walking the whole string.
    fn nth(&mut self, n: usize) -> Option<&'a str> {
        if let Self::Bytes { next, .. } = self {
            *next = next.checked_add(n)?;
        } else {
            for _ in 0..n {
                self.next()?;
            }
        }
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example the whole question turns on: a family emoji built from a
    /// ZWJ sequence is one character to a person, four code points, and 25
    /// bytes. ADR 0009 § 2 picks the first of those three.
    #[test]
    fn a_zwj_sequence_is_one_grapheme_and_several_code_points() {
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}";
        assert_eq!(Unit::Grapheme.length(family), 1);
        assert_eq!(Unit::CodePoint.length(family), 5);
        assert_eq!(family.len(), 18);
        assert_eq!(DEFAULT.length(family), 1);
    }

    /// A combining accent is the everyday case, not an emoji curiosity:
    /// `e` + U+0301 is one character and two code points.
    #[test]
    fn a_combining_mark_joins_its_base_letter() {
        let cafe = "cafe\u{301}";
        assert_eq!(Unit::Grapheme.length(cafe), 4);
        assert_eq!(Unit::CodePoint.length(cafe), 5);
        assert_eq!(Unit::Grapheme.at(cafe, 3), Some("e\u{301}"));
        assert_eq!(Unit::CodePoint.at(cafe, 3), Some("e"));
    }

    /// `pieces` reassembles the subject exactly, in both units — the property
    /// that makes a slice built out of them a `string` rather than a byte
    /// range that happens to work.
    #[test]
    fn every_unit_partitions_the_subject() {
        for subject in [
            "",
            "ascii",
            "one\r\ntwo",
            "cafe\u{301}",
            "a\u{1f1e6}\u{1f1f9}b",
            "日本語",
        ] {
            for unit in [Unit::CodePoint, Unit::Grapheme] {
                let pieces: Vec<&str> = unit.pieces(subject).collect();
                assert_eq!(pieces.concat(), subject, "{unit:?} lost bytes");
                assert_eq!(pieces.len(), unit.length(subject), "{unit:?} miscounted");
            }
        }
    }

    /// A negative index counts from the end, and one past either end is
    /// nothing — what `Core\Str::at` turns into a throw.
    #[test]
    fn a_negative_index_counts_from_the_end() {
        let word = "cafe\u{301}";
        assert_eq!(DEFAULT.at(word, -1), Some("e\u{301}"));
        assert_eq!(DEFAULT.at(word, -4), Some("c"));
        assert_eq!(DEFAULT.at(word, -5), None);
        assert_eq!(DEFAULT.at(word, 4), None);
        assert_eq!(DEFAULT.at("", 0), None);
    }

    /// [`one_byte_per_cluster`]'s fast path answers exactly what UAX #29 would
    /// have — including `\r\n`, which is the whole reason for its second
    /// condition, and which the fast path must therefore decline.
    #[test]
    fn the_one_byte_fast_path_agrees_with_the_segmenter() {
        for subject in [
            "",
            "plain ascii",
            "line\nline",
            "line\r\nline",
            "\r",
            "\r\r\n",
            "tab\there",
            "\u{7f}\u{0}!",
        ] {
            let reference: Vec<&str> = subject.graphemes(true).collect();
            assert_eq!(
                Unit::Grapheme.length(subject),
                reference.len(),
                "{subject:?} counted differently from the segmenter"
            );
            assert_eq!(
                Unit::Grapheme.pieces(subject).collect::<Vec<&str>>(),
                reference,
                "{subject:?} split differently from the segmenter"
            );
        }
    }

    /// The fused scan answers exactly what the two-pass spelling it replaced
    /// did, over every byte a `str` can carry. The bit trick is the whole of
    /// the fast path's correctness, so it is checked against the obvious
    /// spelling rather than against a handful of examples.
    #[test]
    fn the_fused_scan_agrees_with_the_two_pass_spelling() {
        let mut subjects: Vec<String> = (0..=0x7f_u32)
            .map(|byte| {
                format!(
                    "a{}z",
                    char::from_u32(byte).expect("every ASCII byte is a scalar value")
                )
            })
            .collect();
        subjects.extend([
            String::new(),
            "\u{7f}".to_owned(),
            "caf\u{e9}".to_owned(),
            "日本語".to_owned(),
            "\u{1f1e6}\u{1f1f9}".to_owned(),
        ]);
        for subject in subjects {
            let reference = subject.is_ascii() && !subject.as_bytes().contains(&b'\r');
            assert_eq!(
                one_byte_per_cluster(&subject),
                reference,
                "{subject:?} classified differently from the two-pass spelling"
            );
        }
    }

    /// The two conversions are inverses at every boundary of the subject, in
    /// both units — the property `Core\Regex` depends on when it hands an
    /// engine a byte offset built from a caller's unit index and reports the
    /// resulting match back in that unit.
    #[test]
    fn a_byte_offset_and_a_unit_index_convert_both_ways() {
        for subject in ["", "ascii", "cafe\u{301}", "a\u{1f1e6}\u{1f1f9}b", "日本語"] {
            for unit in [Unit::CodePoint, Unit::Grapheme] {
                let mut byte = 0;
                for (index, piece) in unit.pieces(subject).enumerate() {
                    assert_eq!(unit.index_of_byte(subject, byte), index, "{subject:?}");
                    assert_eq!(unit.byte_of_index(subject, index), byte, "{subject:?}");
                    byte += piece.len();
                }
                // One past the last unit is the subject's own length, in both
                // directions — an index nothing starts at still has an answer.
                assert_eq!(
                    unit.index_of_byte(subject, subject.len()),
                    unit.length(subject)
                );
                assert_eq!(
                    unit.byte_of_index(subject, unit.length(subject)),
                    subject.len()
                );
                assert_eq!(
                    unit.byte_of_index(subject, unit.length(subject) + 9),
                    subject.len()
                );
            }
        }
    }

    /// The Unicode version this build answers for, recorded where a bump makes
    /// it visible — ADR 0009's *Consequences* asks for exactly that.
    #[test]
    fn the_embedded_unicode_version_is_recorded() {
        let (major, minor, patch) = unicode_segmentation::UNICODE_VERSION;
        assert!(
            major >= 16,
            "grapheme boundaries are answered against Unicode {major}.{minor}.{patch}, older \
             than the 16.0 this build was measured against"
        );
    }
}
