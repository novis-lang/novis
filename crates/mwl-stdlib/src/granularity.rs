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
//! first use — and that is a `mwl_runtime::MwlStr` change, not this module's;
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
/// correct: `is_ascii` is a vectorized scan, so the overwhelmingly common case
/// — a request path full of ASCII — pays a UTF-8-validation-shaped pass rather
/// than a segmentation one. `a_grapheme_index_costs_more_than_a_code_point_index`
/// measures both halves of that claim.
fn one_byte_per_cluster(subject: &str) -> bool {
    subject.is_ascii() && !subject.as_bytes().contains(&b'\r')
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
