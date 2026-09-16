//! `rule:types/string-is-utf8`'s granularity, in one place: what unit a `string`'s length,
//! indexing and slicing count in.
//!
//! `rule:types/string-is-utf8` is settled
//! here — [`DEFAULT`] is the answer, and that ADR's own body states the rule.
//! Every `Core\Str` member that has a unit at all reaches for [`DEFAULT`]
//! rather than choosing one, so there is exactly one place a granularity is
//! decided and no two members can disagree about it.
//!
//! # Why `unicode-segmentation`
//!
//! `rule:packaging/a-c-dependency-answers-two-questions`'s two
//! questions: UAX #29 is an external specification with a mature pure-Rust
//! implementation, so it is a dependency rather than ours to write.
//! `unicode-segmentation` is the crate the Rust ecosystem's own text tooling
//! uses, is `no_std`-capable, has no build script and no C, and is MIT OR
//! Apache-2.0 — so `cargo deny check` needs no exception for it. The
//! alternative considered was `icu_segmenter`: correct too, and carrying a
//! data-provider architecture and a locale story that
//! `rule:core-api/tier-placement` already
//! rules out of `Core`.
//!
//! # The Unicode version is part of the answer
//!
//! `rule:types/bytes`'s *Consequences* says this out loud: what counts as one character
//! is pinned to whichever Unicode version this crate embeds, and can move
//! across a dependency bump. That is [`unicode_segmentation::UNICODE_VERSION`],
//! and `the_embedded_unicode_version_is_recorded` prints it, so a bump that
//! changes it is visible in a test run rather than in a user's diff.
//!
//! # Where the count comes from
//!
//! A `string` carries its own grapheme count once anything has asked for one —
//! `nvs_runtime::StrHeader`'s fourth word, filled lazily and corrected at a
//! concatenation's seam, which is `rule:types/bytes`'s *Consequences* paid. Reaching it
//! needs the *value*, not the payload, so [`Unit::length_of`] is what a member
//! calls and [`Unit::length`] is what answers when there is no allocation to
//! ask (a `&str` this crate built, a `bytes` read as text). The UAX #29
//! primitives both go through live in `nvs_runtime::graphemes`, beside the
//! header that caches them.

use unicode_segmentation::UnicodeSegmentation;

/// The unit a `string` operation counts in.
///
/// **Two units, not three.** `rule:types/string-is-utf8` named byte length as its fallback
/// default, and that fallback is closed off here rather than merely
/// out-measured: `string` is guaranteed valid UTF-8, so a byte-indexed
/// `Core\Str::at` would have to hand back the interior byte of a multi-byte
/// character — a `string` that cannot exist. A byte-length default would
/// therefore leave `length` counting one unit and `at`/`slice` addressing
/// another, which is the exact "does length mean what I think" failure `rule:types/bytes`
/// was opened to remove. Byte length remains reachable, where it means
/// something: on `bytes`, whose whole point is that it has no other unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// One Unicode scalar value — what `Core\Str::codePoints` enumerates.
    ///
    /// Deliberately *not* [`DEFAULT`]: `rule:types/bytes`'s *Alternatives rejected*
    /// turns it down for answering "how many scalar values" rather than "how
    /// many characters a person sees" — a flag emoji is two, an accented
    /// letter built from combining marks is two.
    CodePoint,
    /// One extended grapheme cluster (UAX #29) — what a person reading the
    /// source would call one character.
    Grapheme,
}

/// The unit `Core\Str::length`, `at` and `slice` count in — `rule:types/string-is-utf8`'s
/// decision, stated once.
///
/// The cost that decides it is measured by
/// `a_grapheme_index_costs_more_than_a_code_point_index` in
/// [`benches/abi-probe`](/benches/abi-probe/), which is where the
/// figure and its bound live; `rule:types/bytes`'s *Revisiting* asked for exactly that
/// test and its § 2 records the outcome.
pub const DEFAULT: Unit = Unit::Grapheme;

/// `rule:types/string-is-utf8`'s two primitives, whose home is the runtime.
///
/// The count is cached in a string's own header and corrected at a
/// concatenation's seam, which only the crate owning that header can do — so
/// `nvs_runtime::graphemes` is where the UAX #29 kernel lives and this module
/// is where the *choice* of unit is stated. See that module's docs; a fold
/// this subtle in two crates is one of them being wrong later.
use nvs_runtime::Value;
use nvs_runtime::graphemes::{count, one_byte_per_cluster};

impl Unit {
    /// How many of this unit `subject` holds.
    ///
    /// O(n) in both units and allocation-free in both. [`Unit::length_of`] is
    /// the same question asked of a *value*, which is the one that can be
    /// answered from a header rather than counted.
    #[must_use]
    pub fn length(self, subject: &str) -> usize {
        match self {
            Self::CodePoint => subject.chars().count(),
            Self::Grapheme => count(subject),
        }
    }

    /// How many of this unit `value` holds, given its payload as `subject` —
    /// [`Unit::length`] with the string's own cached grapheme count in front
    /// of it.
    ///
    /// This is the seam `rule:types/bytes`'s *Consequences* asks for: a `string`'s
    /// length is O(n) where PHP's `strlen` is O(1), and a program asking twice
    /// pays once. The word lives in `nvs_runtime::StrHeader` — that module's
    /// § *The cached grapheme count* owns when it is filled, why a
    /// concatenation corrects it rather than summing, and what it spends.
    ///
    /// Every member with a unit goes through here rather than through
    /// [`Unit::length`] on a `&str` it has already unwrapped: the payload
    /// alone cannot say which allocation it came from, so the cache is only
    /// reachable while the [`Value`] is still in hand.
    #[must_use]
    pub fn length_of(self, value: &Value, subject: &str) -> usize {
        match self {
            Self::Grapheme => value.grapheme_count().unwrap_or_else(|| count(subject)),
            Self::CodePoint => self.length(subject),
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
    /// `rule:core-api/shape-rules` R8 fixes
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
    /// `rule:types/string-is-utf8` says every `string` position Novis hands back or takes is in
    /// [`DEFAULT`]'s unit. Converting at that seam is what keeps
    /// `Core\Regex\Match::offset` and `Core\Str::indexOf` answering in one unit.
    ///
    /// **O(`byte`)**, allocation-free: the prefix is counted, not indexed. A
    /// caller converting many offsets over one subject therefore pays per
    /// offset, which is what [`Unit::cursor`] exists to spare the members that
    /// report a whole run of them.
    ///
    /// # Panics
    ///
    /// If `byte` is not a character boundary of `subject`, which is a slicing
    /// bug rather than input: both engines report a match at a boundary.
    #[must_use]
    pub fn index_of_byte(self, subject: &str, byte: usize) -> usize {
        self.length(&subject[..byte])
    }

    /// [`Self::index_of_byte`] for a whole run of **non-decreasing** offsets
    /// over one subject, walking that subject once in total rather than once
    /// per offset.
    ///
    /// The seam a byte-addressed engine reporting many positions needs:
    /// `Core\Regex::matchAll` converts one offset per match, and counting each
    /// prefix from its start costs O(n·k) for *k* matches over *n* bytes where
    /// counting the gap since the previous match costs O(n) for the run. The
    /// matches arrive in increasing order, which is the whole precondition
    /// [`Cursor`] asks for.
    #[must_use]
    pub fn cursor(self, subject: &str) -> Cursor<'_> {
        Cursor {
            pieces: self.pieces(subject),
            start: 0,
            counted: 0,
            held: None,
        }
    }

    /// The byte offset a **signed** unit index names, with a negative one
    /// counting from the end and either end saturating.
    ///
    /// [`Self::byte_of_index`] with
    /// `rule:core-api/shape-rules` R8's sign
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

/// [`Unit::cursor`]'s state: [`Unit::index_of_byte`]'s own answer, for offsets
/// asked in the order the subject runs.
///
/// **Each offset must be at or after the one before it.** That is what an
/// engine reporting its matches in the order they occur already hands over, and
/// it is the only thing that makes one walk enough: the cursor holds its place
/// in the subject and never goes back to the start. An offset behind the last
/// one answers the last one's index, so a member converting positions in an
/// arbitrary order calls [`Unit::index_of_byte`] per offset instead.
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    /// The units not yet counted, the first of which begins at `start`.
    pieces: Pieces<'a>,
    /// The byte offset the first uncounted unit begins at.
    start: usize,
    /// How many units end at or before `start`.
    counted: usize,
    /// The byte length of the unit beginning at `start`, pulled from `pieces`
    /// by an offset that landed inside it and held rather than counted.
    held: Option<usize>,
}

impl Cursor<'_> {
    /// How many units of the subject end at or before `byte` — what
    /// [`Unit::index_of_byte`] answers for the same pair.
    ///
    /// The **partial** unit is the edge that earns the [`Cursor::held`] field:
    /// a cluster can span a match's first byte, and an offset landing inside
    /// one is the index of the unit it reached into rather than of the last one
    /// that ended, which is what counting the prefix on its own would say. Such
    /// a unit is not consumed, so an offset still inside it reads the same index
    /// and the one after it is counted exactly once.
    ///
    /// A subject [`one_byte_per_cluster`] accepted is the whole of
    /// [`Pieces::Bytes`], and there the index *is* the byte offset: no walk, and
    /// no separate flag to carry, since the iterator's own arm is the fact.
    #[must_use]
    pub fn index_of_byte(&mut self, byte: usize) -> usize {
        if matches!(self.pieces, Pieces::Bytes { .. }) {
            return byte;
        }
        while self.start < byte {
            let Some(len) = self
                .held
                .take()
                .or_else(|| self.pieces.next().map(str::len))
            else {
                return self.counted;
            };
            if self.start + len > byte {
                self.held = Some(len);
                return self.counted + 1;
            }
            self.start += len;
            self.counted += 1;
        }
        self.counted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example the whole question turns on: a family emoji built from a
    /// ZWJ sequence is one character to a person, four code points, and 25
    /// bytes. `rule:types/string-is-utf8` picks the first of those three.
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

    /// A [`Cursor`] answers what counting each prefix answers, at **every**
    /// character boundary of the subject rather than at the cluster boundaries
    /// where the two obviously agree.
    ///
    /// Asked as agreement over a sweep, because that is where the cheap
    /// implementation of a cursor differs: counting the gap since the previous
    /// offset double-counts a cluster an offset landed inside, and `"cafe"` +
    /// U+0301, a CR LF pair and a ZWJ sequence are the three everyday shapes of
    /// an offset that can. A cursor drifting by one on any of them fails here
    /// while still looking right on a subject whose matches all start a
    /// cluster.
    #[test]
    fn a_cursor_answers_what_counting_each_prefix_answers() {
        let subjects = [
            "",
            "ascii",
            "cafe\u{301} bar",
            "a\r\nb\r\n",
            "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}x1",
            "a\u{1f1e6}\u{1f1f9}\u{1f1e6}b",
            "日本語",
        ];
        for subject in subjects {
            for unit in [Unit::CodePoint, Unit::Grapheme] {
                let mut cursor = unit.cursor(subject);
                let mut asked = 0;
                for byte in (0..=subject.len()).filter(|byte| subject.is_char_boundary(*byte)) {
                    assert_eq!(
                        cursor.index_of_byte(byte),
                        unit.index_of_byte(subject, byte),
                        "{subject:?} at byte {byte} in {unit:?}"
                    );
                    asked += 1;
                }
                // The sweep is only evidence if it reached the boundaries: an
                // empty subject has the one at its end, every other one here
                // has that and more.
                let least = if subject.is_empty() { 1 } else { 2 };
                assert!(asked >= least, "{subject:?} swept {asked} boundaries");
            }
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
    /// it visible — `rule:types/bytes`'s *Consequences* asks for exactly that.
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
