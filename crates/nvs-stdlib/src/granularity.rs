//! `rule:types/string-is-utf8`'s granularity, in one place: what unit a `string`'s length,
//! indexing and slicing count in.
//!
//! `rule:types/string-is-utf8` is settled
//! here — [`DEFAULT`] is the answer, and that rule states it.
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
//! `rule:types/string-is-utf8` says this out loud: what counts as one character
//! is pinned to whichever Unicode version this crate embeds, and can move
//! across a dependency bump. That is [`unicode_segmentation::UNICODE_VERSION`],
//! and `the_embedded_unicode_version_is_recorded` prints it, so a bump that
//! changes it is visible in a test run rather than in a user's diff.
//!
//! # Where the count comes from
//!
//! A `string` carries its own grapheme count once anything has asked for one —
//! `nvs_runtime::StrHeader`'s fourth word, filled lazily and corrected at a
//! concatenation's seam, which is `rule:types/string-is-utf8`'s cached count. Reaching it
//! needs the *value*, not the payload, so [`Unit::length_of`] is what a member
//! calls and [`Unit::length`] is what answers when there is no allocation to
//! ask (a `&str` this crate built, a `bytes` read as text). The UAX #29
//! primitives both go through live in `nvs_runtime::graphemes`, beside the
//! header that caches them.
//!
//! # What an index costs
//!
//! A member that takes or returns a position — `Core\Str::at`, `slice`,
//! `replaceRange`, `indexOf`, `lastIndexOf`, `Core\Regex::match` — reaches it
//! through an `_of` method that has the value in hand. Those compare the cached
//! count with the byte length first: equal means every unit is one byte, so a
//! position *is* a byte offset and the call is O(1) once the count is cached.
//! A program indexing every character of an ASCII string is therefore linear.
//!
//! **Without a cached count, a position never asks for the total.** A
//! non-negative one walks forward to its unit and a negative one walks backward
//! from the end, so `Core\Str::slice($s, 0, 10)` and `slice($s, 0, -1)` on a
//! long text read only what they cut at. A one-byte walk past `COUNT_PAST`
//! bytes fills the count, so a loop over every position stays linear.
//!
//! **A subject with wider units walks to the position on every call**, and
//! that is a stated bound rather than a gap: the byte offset of the `i`-th
//! cluster is stored nowhere, and storing it would be an index of one word per
//! cluster on every string that is ever indexed. A program that visits every
//! character of such a string splits it once with `Core\Str::graphemes`.

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
/// test and `rule:types/string-is-utf8` records the outcome.
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
            Self::Grapheme if one_byte_per_cluster(subject) => Pieces::Bytes {
                subject,
                next: 0,
                end: subject.len(),
            },
            Self::Grapheme => Pieces::Graphemes(subject.graphemes(true)),
        }
    }

    /// [`Unit::pieces`] for `value`, whose payload is `subject`, with the
    /// one-byte check answered by [`Unit::length_of`] instead of a scan.
    ///
    /// Every unit is at least one byte, so a count equal to the byte length
    /// says every unit is exactly one, and that is [`Pieces::Bytes`]. The count
    /// is the string's cached one, so a member called once per index over one
    /// subject pays the scan on the first call and an O(1) check on every
    /// later one. That is what keeps a loop over every index of an ASCII
    /// string linear rather than quadratic.
    #[must_use]
    pub fn pieces_of<'a>(self, value: &Value, subject: &'a str) -> Pieces<'a> {
        if self.length_of(value, subject) == subject.len() {
            return Pieces::Bytes {
                subject,
                next: 0,
                end: subject.len(),
            };
        }
        match self {
            Self::CodePoint => Pieces::CodePoints {
                subject,
                indices: subject.char_indices(),
            },
            Self::Grapheme => Pieces::Graphemes(subject.graphemes(true)),
        }
    }

    /// The `index`-th unit of `subject`, counting from the end when `index` is
    /// negative, or `None` when it addresses nothing.
    ///
    /// Negative-from-the-end is the same rule
    /// `rule:core-api/shape-rules` R8 fixes
    /// for every range in `Core`; `at` is a range of one.
    #[must_use]
    pub fn at(self, subject: &str, index: i64) -> Option<&str> {
        self.at_in(None, subject, index)
    }

    /// [`Unit::at`] for `value`, whose payload is `subject`. A cached count
    /// equal to the byte length makes every call O(1), and
    /// [`Unit::one_byte_over`] says when the walk fills that count.
    ///
    /// A non-negative index walks forward to its unit and a negative one walks
    /// backward from the end, so neither reads the units on its far side. A
    /// subject with a wider unit walks on every call, because the byte offset
    /// of the `index`-th cluster is not stored anywhere. That is the module
    /// docs' *What an index costs*.
    #[must_use]
    pub fn at_of<'a>(self, value: &Value, subject: &'a str, index: i64) -> Option<&'a str> {
        self.at_in(Some(value), subject, index)
    }

    /// [`Unit::at`] and [`Unit::at_of`], with the value when there is one.
    fn at_in<'a>(self, value: Option<&Value>, subject: &'a str, index: i64) -> Option<&'a str> {
        if index < 0 {
            let back = usize::try_from(index.unsigned_abs()).ok()?;
            return self
                .walk_backward(value, subject, back)
                .map(|(_, piece)| piece);
        }
        let at = self.walk_forward(value, subject, usize::try_from(index).ok()?);
        if at == subject.len() {
            return None;
        }
        self.first_piece(&subject[at..])
    }

    /// The first unit of `rest`, which starts at a unit boundary, read without
    /// segmenting more than the unit and the character after it.
    fn first_piece(self, rest: &str) -> Option<&str> {
        match self {
            Self::CodePoint => rest.chars().next().map(|first| &rest[..first.len_utf8()]),
            Self::Grapheme => {
                // Two one-byte clusters side by side have a boundary between
                // them, so the first byte is a unit of its own.
                let head = rest.get(..rest.len().min(2));
                if head.is_some_and(|head| !head.is_empty() && one_byte_per_cluster(head)) {
                    return Some(&rest[..1]);
                }
                rest.graphemes(true).next()
            }
        }
    }

    /// Whether every unit of `part`, a slice of `subject`, is one byte, so a
    /// position inside `part` is its byte offset.
    ///
    /// A cached count equal to `subject`'s byte length says yes in O(1).
    /// Otherwise `part` is scanned, and nothing outside it. A one-byte part of
    /// [`COUNT_PAST`] bytes or more also fills `value`'s cached count. So a
    /// loop that reads every position of a long one-byte subject scans the
    /// subject once in total, and a position near either end reads only the
    /// bytes up to it. Filling the count is one scan of the subject, once.
    fn one_byte_over(self, value: Option<&Value>, subject: &str, part: &str) -> bool {
        if self == Self::Grapheme
            && value.and_then(Value::known_grapheme_count) == Some(subject.len())
        {
            return true;
        }
        let one_byte = match self {
            Self::CodePoint => part.is_ascii(),
            Self::Grapheme => one_byte_per_cluster(part),
        };
        if one_byte
            && self == Self::Grapheme
            && part.len() >= COUNT_PAST
            && let Some(value) = value
        {
            // Filled for the next call. This call already has its answer.
            let _ = value.grapheme_count();
        }
        one_byte
    }

    /// The byte offset unit `index` starts at, walking forward from the start,
    /// or `subject`'s length for an index at or past its end.
    fn walk_forward(self, value: Option<&Value>, subject: &str, index: usize) -> usize {
        // Every unit is at least one byte, so an index at or past the byte
        // length is past the end, and a default of "no bound" walks nothing.
        if index >= subject.len() {
            return subject.len();
        }
        // Unit `index` starts at byte `index` when every byte up to and
        // including it is a unit of its own. Only that prefix is read, and the
        // walks below are lazy, so the cost follows the index and never the
        // subject's length.
        if subject
            .get(..=index)
            .is_some_and(|part| self.one_byte_over(value, subject, part))
        {
            return index;
        }
        match self {
            Self::CodePoint => subject
                .char_indices()
                .nth(index)
                .map_or(subject.len(), |(at, _)| at),
            Self::Grapheme => subject
                .grapheme_indices(true)
                .nth(index)
                .map_or(subject.len(), |(at, _)| at),
        }
    }

    /// The unit `back` units before the end of `subject` (`back` 1 is the
    /// last), as its byte offset and its text, or `None` when `subject` has
    /// fewer than `back` units.
    ///
    /// The walk goes backward from the end and reads only the units it
    /// passes, so a position near the end never reads the start.
    fn walk_backward<'a>(
        self,
        value: Option<&Value>,
        subject: &'a str,
        back: usize,
    ) -> Option<(usize, &'a str)> {
        // Every unit is at least one byte, so `back` units need `back` bytes.
        let cut = subject.len().checked_sub(back)?;
        if back == 0 {
            return None;
        }
        // The last `back` bytes are `back` units when they and the byte before
        // them are one-byte units: two such bytes always have a boundary
        // between them.
        if subject
            .get(cut.saturating_sub(1)..)
            .is_some_and(|part| self.one_byte_over(value, subject, part))
        {
            return Some((cut, &subject[cut..=cut]));
        }
        match self {
            Self::CodePoint => subject
                .char_indices()
                .rev()
                .nth(back - 1)
                .map(|(at, found)| (at, &subject[at..at + found.len_utf8()])),
            Self::Grapheme => subject.grapheme_indices(true).rev().nth(back - 1),
        }
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

    /// [`Unit::index_of_byte`] for `value`, whose payload is `subject`: on a
    /// prefix of one-byte units the index is the byte offset. A cached count
    /// says that without reading the prefix, and nothing after `byte` is read.
    ///
    /// # Panics
    ///
    /// As [`Unit::index_of_byte`] does.
    #[must_use]
    pub fn index_of_byte_of(self, value: &Value, subject: &str, byte: usize) -> usize {
        if self.one_byte_over(Some(value), subject, &subject[..byte]) {
            return byte;
        }
        self.index_of_byte(subject, byte)
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

    /// [`Unit::cursor`] for `value`, whose payload is `subject`, over
    /// [`Unit::pieces_of`] — so a subject of one-byte units starts the cursor
    /// on its no-walk arm without scanning the subject first.
    #[must_use]
    pub fn cursor_of<'a>(self, value: &Value, subject: &'a str) -> Cursor<'a> {
        Cursor {
            pieces: self.pieces_of(value, subject),
            start: 0,
            counted: 0,
            held: None,
        }
    }

    /// The byte offset a **signed** unit index names in `value`, whose payload
    /// is `subject`, with a negative one counting from the end and either end
    /// saturating.
    ///
    /// [`Self::byte_of_index`] with
    /// `rule:core-api/shape-rules` R8's sign
    /// rule applied first, so every member that takes a *position* — the `from`
    /// of `Core\Regex::match` and `Core\Str::indexOf`, the `before` of
    /// `lastIndexOf`, the `offset` of `slice` — reads it the same way. It
    /// saturates rather than answering `None` for the reason
    /// [`Self::byte_of_index`] does: a search starting past the end finds
    /// nothing, which composes with a loop where a throw would not.
    ///
    /// A non-negative index walks forward from the start and a negative one
    /// walks backward from the end, so neither needs the total. A cached count
    /// equal to the byte length makes the index the byte offset, and
    /// [`Unit::one_byte_over`] fills that count once a walk is long, so a
    /// find-next loop over a subject of one-byte units pays no walk per call.
    #[must_use]
    pub fn byte_of_signed_index_of(self, value: &Value, subject: &str, index: i64) -> usize {
        if index < 0 {
            let back = usize::try_from(index.unsigned_abs()).unwrap_or(usize::MAX);
            return self
                .walk_backward(Some(value), subject, back)
                .map_or(0, |(at, _)| at);
        }
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        self.walk_forward(Some(value), subject, index)
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
        self.walk_forward(None, subject, index)
    }
}

/// How many bytes a one-byte walk reads before it also fills the subject's
/// cached count: [`Unit::one_byte_over`]'s bound.
///
/// A walk shorter than this reads only up to its position. A loop over every
/// position of a long subject reads at most this many bytes per call before
/// the count is filled, and nothing after. **What it spends:** no memory. A
/// position past this bound on a long one-byte subject counts the whole
/// subject once, and that count is the one `Core\Str::length` keeps.
const COUNT_PAST: usize = 4096;

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
        /// The byte offset the last item not yet taken from the back ends at.
        end: usize,
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
            Self::Bytes { subject, next, end } => {
                let start = *next;
                if start >= *end {
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

/// The same pieces, last first — which is how `Core\Str::reverse` writes its
/// result without first collecting every piece of the subject.
impl<'a> DoubleEndedIterator for Pieces<'a> {
    fn next_back(&mut self) -> Option<&'a str> {
        match self {
            Self::CodePoints { subject, indices } => {
                let (start, found) = indices.next_back()?;
                Some(&subject[start..start + found.len_utf8()])
            }
            Self::Bytes { subject, next, end } => {
                if *end <= *next {
                    return None;
                }
                *end -= 1;
                // Every byte offset is a boundary here, as in `next`.
                Some(&subject[*end..*end + 1])
            }
            Self::Graphemes(graphemes) => graphemes.next_back(),
        }
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

    /// Walking the pieces from the back gives the forward pieces in reverse,
    /// on all three arms, and the two ends meet without a piece taken twice.
    // covers: Core\Str::reverse
    #[test]
    fn the_pieces_walk_from_the_back_as_they_walk_from_the_front() {
        for subject in ["", "plain ascii", "line\r\nline", "cafe\u{301} 👍🏽 🇩🇪", "ß"] {
            for unit in [Unit::Grapheme, Unit::CodePoint] {
                let mut forward: Vec<&str> = unit.pieces(subject).collect();
                forward.reverse();
                assert_eq!(
                    unit.pieces(subject).rev().collect::<Vec<&str>>(),
                    forward,
                    "{subject:?} walked back differently"
                );
                let mut both = unit.pieces(subject);
                let mut taken = 0;
                while both.next().is_some() {
                    taken += 1;
                    if both.next_back().is_some() {
                        taken += 1;
                    }
                }
                assert_eq!(taken, forward.len(), "{subject:?} met twice");
            }
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
        // The later subjects begin with plain ASCII and turn into something
        // else, so `byte_of_index` meets both its prefix answer and its walk.
        for subject in [
            "",
            "ascii",
            "cafe\u{301}",
            "a\u{1f1e6}\u{1f1f9}b",
            "日本語",
            "ab\u{301}cd",
            "x\r\ny",
            "ok \u{1f1e9}\u{1f1ea}!",
        ] {
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

    /// The `_of` methods read the cached count and take the one-byte shortcut
    /// when it equals the byte length. Every answer they give matches the
    /// `&str` method that walks, for every index and every sign, on subjects
    /// that take the shortcut and on subjects that do not.
    #[test]
    fn the_cached_count_shortcut_answers_what_the_walk_answers() {
        for subject in [
            "",
            "ascii",
            "x\r\ny",
            "cafe\u{301}",
            "日本語",
            "ok \u{1f1e9}\u{1f1ea}!",
        ] {
            let value = Value::text(subject);
            for unit in [Unit::CodePoint, Unit::Grapheme] {
                let total = i64::try_from(unit.length(subject)).unwrap_or(i64::MAX);
                for index in -total - 2..=total + 2 {
                    assert_eq!(
                        unit.at_of(&value, subject, index),
                        unit.at(subject, index),
                        "{subject:?} at {index}"
                    );
                    let from_start = if index < 0 {
                        (total + index).max(0)
                    } else {
                        index
                    };
                    let walked =
                        unit.byte_of_index(subject, usize::try_from(from_start).unwrap_or(0));
                    assert_eq!(
                        unit.byte_of_signed_index_of(&value, subject, index),
                        walked,
                        "{subject:?} byte of {index}"
                    );
                }
                let mut cursor = unit.cursor_of(&value, subject);
                for (byte, _) in subject.char_indices().chain([(subject.len(), ' ')]) {
                    let counted = unit.index_of_byte(subject, byte);
                    assert_eq!(unit.index_of_byte_of(&value, subject, byte), counted);
                    assert_eq!(cursor.index_of_byte(byte), counted, "{subject:?} at {byte}");
                }
            }
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built for the \
                          subject, and every method above borrowed it"
            )]
            unsafe {
                value.release();
            }
        }
    }

    /// Releases the one reference a test built with [`Value::text`].
    fn release(value: Value) {
        #[expect(
            unsafe_code,
            reason = "each test owns the one reference it built for its \
                      subject, and every method it called borrowed it"
        )]
        unsafe {
            value.release();
        }
    }

    /// A long text with wide units everywhere, and one that is all one-byte.
    /// Under Miri each is a few hundred bytes: a count of the whole text still
    /// shows in the cached count, and the full length took over 300 s there.
    fn long_texts() -> [String; 2] {
        let repeats = if cfg!(miri) { 20 } else { 5_000 };
        [
            "e\u{301}\u{1f44b}\u{1f3fd}".repeat(repeats),
            "ab".repeat(repeats * 10),
        ]
    }

    /// A position from the start, on a text nothing has counted, walks to the
    /// position and leaves the cached count empty. A count would have read
    /// the whole text.
    #[test]
    fn a_forward_position_never_counts_the_whole_string() {
        for subject in long_texts() {
            let value = Value::text(&subject);
            assert_eq!(value.known_grapheme_count(), None);
            let unit = DEFAULT;
            let third = unit.byte_of_index(&subject, 3);
            assert_eq!(unit.byte_of_signed_index_of(&value, &subject, 3), third);
            assert_eq!(unit.at_of(&value, &subject, 3), unit.at(&subject, 3));
            assert_eq!(unit.index_of_byte_of(&value, &subject, third), 3);
            let (start, end) =
                crate::str::window(&value, &subject, &Value::int(2), &Value::int(4), "slice")
                    .expect("an int offset and length");
            assert_eq!(start, unit.byte_of_index(&subject, 2));
            assert_eq!(end, unit.byte_of_index(&subject, 6));
            assert_eq!(
                value.known_grapheme_count(),
                None,
                "a position near the start counted the whole text"
            );
            release(value);
        }
    }

    /// A negative position walks back from the end, on a text nothing has
    /// counted, and leaves the cached count empty.
    #[test]
    fn a_backward_position_walks_from_the_end() {
        for subject in long_texts() {
            let value = Value::text(&subject);
            let unit = DEFAULT;
            let total = unit.length(&subject);
            let last = unit.byte_of_index(&subject, total - 1);
            assert_eq!(unit.byte_of_signed_index_of(&value, &subject, -1), last);
            assert_eq!(
                unit.byte_of_signed_index_of(&value, &subject, -3),
                unit.byte_of_index(&subject, total - 3)
            );
            assert_eq!(unit.at_of(&value, &subject, -1), Some(&subject[last..]));
            let (start, end) =
                crate::str::window(&value, &subject, &Value::int(-4), &Value::int(-1), "slice")
                    .expect("an int offset and length");
            assert_eq!(start, unit.byte_of_index(&subject, total - 4));
            assert_eq!(end, last);
            assert_eq!(
                value.known_grapheme_count(),
                None,
                "a position near the end counted the whole text"
            );
            release(value);
        }
    }

    /// The window `slice` reads, against the walk that counted the whole
    /// subject first, for every offset and length from -12 to 12 (-4 to 4
    /// under Miri) on texts
    /// with combining marks, emoji, flags and a CRLF.
    #[test]
    fn both_walks_agree_on_every_window() {
        fn counted(unit: Unit, subject: &str, offset: i64, length: Option<i64>) -> (usize, usize) {
            let total = i64::try_from(unit.length(subject)).unwrap_or(i64::MAX);
            let at = |index: i64| {
                let index = if index < 0 {
                    (total + index).max(0)
                } else {
                    index
                };
                unit.byte_of_index(subject, usize::try_from(index).unwrap_or(usize::MAX))
            };
            let start = at(offset);
            let end = match length {
                None => subject.len(),
                Some(length) if length < 0 => at((total + length).max(0)),
                Some(length) => {
                    let first = unit.index_of_byte(subject, start);
                    at(i64::try_from(first).unwrap_or(i64::MAX) + length)
                }
            };
            (start, end.max(start))
        }
        let texts = [
            "",
            "a",
            "ascii text",
            "x\r\ny\r\n",
            "cafe\u{301} cre\u{300}me",
            "\u{1f44b}\u{1f3fd}\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}ab",
            "\u{1f1e9}\u{1f1ea}\u{1f1eb}\u{1f1f7}\u{1f1ee}",
            "Gru\u{308}\u{df}e \u{1f44b}\u{1f3fd}\n",
        ];
        // Under Miri the reach is 2, still past both ends of `""` and `"a"`:
        // 12 and then 4 each took over 300 s there.
        let reach: i64 = if cfg!(miri) { 2 } else { 12 };
        for subject in texts {
            for offset in -reach..=reach {
                for length in (-reach..=reach).map(Some).chain([None]) {
                    let value = Value::text(subject);
                    let length_value = length.map_or_else(Value::null, Value::int);
                    let walked = crate::str::window(
                        &value,
                        subject,
                        &Value::int(offset),
                        &length_value,
                        "slice",
                    )
                    .expect("an int offset and length");
                    assert_eq!(
                        walked,
                        counted(DEFAULT, subject, offset, length),
                        "{subject:?} at {offset} for {length:?}"
                    );
                    for index in [offset, length.unwrap_or(0)] {
                        assert_eq!(
                            DEFAULT.at_of(&value, subject, index),
                            DEFAULT.pieces(subject).nth({
                                let total = i64::try_from(DEFAULT.length(subject)).unwrap_or(0);
                                let from_start = if index < 0 { total + index } else { index };
                                usize::try_from(from_start).unwrap_or(usize::MAX)
                            }),
                            "{subject:?} at {index}"
                        );
                    }
                    release(value);
                }
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
