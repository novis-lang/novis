//! UAX #29 grapheme clusters: how many a string holds, the ASCII shortcut
//! that answers most of them without segmenting, and the one question a
//! concatenation asks at its seam.
//!
//! `rule:types/string-is-utf8` decides that a
//! `string`'s length, indexing and iteration count extended grapheme clusters,
//! and `nvs_stdlib::granularity` is where that *choice* is stated — which unit
//! is the default, and what splitting and indexing in it mean. This module is
//! the half the choice cannot live with: the count is cached in
//! [`crate::StrHeader`] and corrected at a concatenation's seam
//! ([`crate::NvsStr::grapheme_count`]), so the primitives it is built from have
//! to be reachable from the crate that owns the header. `granularity` calls
//! these rather than keeping a second copy — a fold this subtle in two crates
//! is one of them being wrong later.
//!
//! Nothing here allocates, and nothing here is a `Core` member: this is the
//! kernel both the cached path and the uncached one read.

#[cfg(test)]
use std::cell::Cell;

use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

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
/// This is what makes `rule:types/string-is-utf8`'s decision affordable rather than merely
/// correct: the two disqualifiers are folded into **one** branchless pass, so
/// the overwhelmingly common case — a request path full of ASCII — pays a
/// single vectorizable scan rather than a segmentation one.
/// `a_grapheme_index_costs_more_than_a_code_point_index` in
/// [`benches/abi-probe`](/benches/abi-probe/) measures both halves of
/// that claim.
///
/// The obvious spelling, `subject.is_ascii() && !bytes.contains(&b'\r')`, is
/// two vectorized passes over the same buffer where one does. Each byte
/// contributes its high bit — set exactly when it is non-ASCII — or the high
/// bit of `(x - 1) & !x` for `x = b ^ b'\r'`, which is the classic zero-byte
/// test and is set exactly when `b` is `CR`. The fold has no early exit, which
/// is what lets it vectorize; the only subject an early exit would have saved
/// work on is one that then goes down the segmentation path, which costs
/// strictly more than the bytes this skips.
#[must_use]
pub fn one_byte_per_cluster(subject: &str) -> bool {
    subject.as_bytes().iter().fold(0_u8, |flags, &byte| {
        let cr = byte ^ b'\r';
        flags | byte | (cr.wrapping_sub(1) & !cr)
    }) & 0x80
        == 0
}

/// How many extended grapheme clusters `subject` holds.
///
/// O(n) and allocation-free either way. This is the scan
/// [`crate::NvsStr::grapheme_count`] caches, and every entry to it is counted
/// under `cfg(test)` so that "answered from the header" is a *measurement*
/// rather than a claim — `scans` below is what a test reads.
#[must_use]
pub fn count(subject: &str) -> usize {
    note_scan();
    if one_byte_per_cluster(subject) {
        subject.len()
    } else {
        subject.graphemes(true).count()
    }
}

/// Whether the clusters on either side of `offset` are one cluster — the
/// question a concatenation asks at its seam, and the whole reason two cached
/// counts do not simply add — or `None` where the join re-groups the text to
/// the right of the seam and **no** correction to a sum of counts is right.
///
/// `text` is the **joined** string and `offset` a byte index into it, which is
/// what makes the boundary answer exact rather than a window heuristic: a
/// cursor given the whole text as one chunk never asks for more context, so a
/// cluster spanning three pieces and a ZWJ chain of any length both answer
/// correctly.
///
/// It is O(1) in the sense that matters: the work is bounded by the clusters
/// *touching* the seam, not by the length of either side. That is the whole of
/// what `rule:types/bytes`'s *Consequences* asks a concatenation to pay, against the
/// full re-segmentation it would otherwise owe.
///
/// The ends never join: a boundary is a boundary at 0 and at `text.len()`, and
/// answering `Some(false)` there keeps the caller's arithmetic uniform over an
/// empty piece.
///
/// # Why one seam is not always one correction
///
/// Every UAX #29 rule looks at the pair of code points around a position plus
/// bounded context inside the cluster it is in — except GB12/GB13, which break
/// a run of `Regional_Indicator`s into *pairs*, so how that run groups depends
/// on how many of them precede it rather than on any one adjacency. A join
/// landing inside such a run re-groups the whole of it: `🇩` is one cluster and
/// `🇩🇪` is one, but `🇩` . `🇩🇪` is **two** — neither the sum nor the sum less
/// a join. No per-seam number describes that, so a seam with a regional
/// indicator on each side answers `None` and the caller leaves the count to
/// the ordinary scan.
///
/// Deliberately the whole run rather than the parities that would decide it
/// case by case: those parities are a property of the *finished* run, so a
/// three-piece concatenation would need the pieces and not just the offsets,
/// and a rule stated over one adjacency is the kind that is right for two
/// pieces and quietly wrong for three. Flags are not a hot path, and this
/// costs them a scan rather than an answer.
#[must_use]
pub fn seam_joins(text: &str, offset: usize) -> Option<bool> {
    if offset == 0 || offset >= text.len() {
        return Some(false);
    }
    if inside_a_regional_indicator_run(text, offset) {
        return None;
    }
    let mut cursor = GraphemeCursor::new(offset, text.len(), true);
    match cursor.is_boundary(text, 0) {
        Ok(boundary) => Some(!boundary),
        // `is_boundary` asks only for context it has not been given, and it has
        // been given all of it. Every other variant belongs to the
        // `next_boundary`/`prev_boundary` half of the cursor's API.
        Err(_) => unreachable!("a grapheme cursor asked for context it already holds"),
    }
}

/// Whether `offset` falls inside a run of regional indicators — [`seam_joins`]'s
/// one `None`, and two range checks rather than a walk.
fn inside_a_regional_indicator_run(text: &str, offset: usize) -> bool {
    /// The Regional_Indicator block, U+1F1E6 REGIONAL INDICATOR SYMBOL LETTER
    /// A through U+1F1FF — a contiguous range, so this needs no table.
    const RANGE: std::ops::RangeInclusive<char> = '\u{1F1E6}'..='\u{1F1FF}';

    let after = text[offset..].chars().next();
    let before = text[..offset].chars().next_back();
    after.is_some_and(|c| RANGE.contains(&c)) && before.is_some_and(|c| RANGE.contains(&c))
}

#[cfg(test)]
thread_local! {
    /// How many times [`count`] has scanned on this thread.
    static SCANS: Cell<usize> = const { Cell::new(0) };
}

/// Counts one scan, under `cfg(test)` only — an atomic-free `Cell` on the
/// thread that did the scanning, which is the only thread that can reach a
/// string it made ([`crate::string`]'s docs).
fn note_scan() {
    #[cfg(test)]
    SCANS.with(|scans| scans.set(scans.get() + 1));
}

/// How many scans [`count`] has performed on this thread, from zero at the
/// last [`forget_scans`].
#[cfg(test)]
pub(crate) fn scans() -> usize {
    SCANS.with(Cell::get)
}

/// Restarts [`scans`] at zero, so a test states a delta rather than a total.
#[cfg(test)]
pub(crate) fn forget_scans() {
    SCANS.with(|scans| scans.set(0));
}

#[cfg(test)]
mod tests {
    use super::{count, one_byte_per_cluster, seam_joins};

    #[test]
    fn the_ascii_shortcut_answers_what_segmentation_would() {
        for subject in [
            "",
            "abc",
            "a\nb",
            "a\r\nb",
            "\r",
            "e\u{301}",
            "\u{1F1E9}\u{1F1EA}",
            "héllo",
        ] {
            let segmented =
                unicode_segmentation::UnicodeSegmentation::graphemes(subject, true).count();
            assert_eq!(count(subject), segmented, "count of {subject:?}");
            if one_byte_per_cluster(subject) {
                assert_eq!(subject.len(), segmented, "the shortcut took {subject:?}");
            }
        }
    }

    #[test]
    fn a_seam_joins_only_where_a_cluster_spans_it() {
        // A base letter and a combining acute are one cluster across the join.
        assert_eq!(seam_joins("e\u{301}", 1), Some(true));
        // Ordinary text breaks everywhere, and so do both ends.
        assert_eq!(seam_joins("ab", 1), Some(false));
        assert_eq!(seam_joins("ab", 0), Some(false));
        assert_eq!(seam_joins("ab", 2), Some(false));
        // GB3 is the one ASCII rule that joins.
        assert_eq!(seam_joins("\r\n", 1), Some(true));
        // A seam with a regional indicator on each side is the one no number
        // describes, however the run happens to group around it.
        assert_eq!(seam_joins("\u{1F1E9}\u{1F1EA}", 4), None);
        assert_eq!(seam_joins("\u{1F1E9}\u{1F1E9}\u{1F1EA}", 4), None);
        assert_eq!(seam_joins("\u{1F1E9}\u{1F1EA}\u{1F1E9}\u{1F1EA}", 8), None);
        // One side is enough to make it an ordinary seam again.
        assert_eq!(seam_joins("\u{1F1E9}ab", 4), Some(false));
        assert_eq!(seam_joins("ab\u{1F1E9}", 2), Some(false));
    }
}
