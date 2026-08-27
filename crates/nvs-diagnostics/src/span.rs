//! Source positions.
//!
//! A [`Span`] is a half-open byte range within one source file. Byte offsets
//! rather than line/column, because every stage after the lexer wants cheap
//! comparison and merging; line and column are computed only when a diagnostic
//! is actually rendered.

use std::fmt;

/// A byte offset within a single source file.
///
/// `u32` caps a source file at 4 GiB, which is not a limit any real `.nvs` file
/// will meet, and halves the size of every span compared to `usize`.
pub type BytePos = u32;

/// Identifies one file within a [`SourceMap`](crate::SourceMap).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SourceId(pub(crate) u32);

impl SourceId {
    /// The raw index, for use as a key in side tables.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The raw id, for cheap ordering without a `usize` round trip.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A half-open byte range `[start, end)` within a single source file.
///
/// Spans are 12 bytes and `Copy`, so they are passed by value everywhere and
/// stored inline in AST nodes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// The file this span points into.
    pub file: SourceId,
    /// Inclusive start offset.
    pub start: BytePos,
    /// Exclusive end offset.
    pub end: BytePos,
}

impl Span {
    /// Creates a span covering `[start, end)`.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `end < start`, which always indicates a bug in
    /// the caller rather than bad input.
    #[must_use]
    pub const fn new(file: SourceId, start: BytePos, end: BytePos) -> Self {
        debug_assert!(start <= end, "span end precedes start");
        Self { file, start, end }
    }

    /// Creates an empty span at `pos`, used to point *between* two tokens —
    /// for example at the position where an expected token was missing.
    #[must_use]
    pub const fn at(file: SourceId, pos: BytePos) -> Self {
        Self {
            file,
            start: pos,
            end: pos,
        }
    }

    /// Length in bytes.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Whether the span covers no bytes. Empty spans are meaningful: they mark
    /// an insertion point.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The smallest span covering both `self` and `other`.
    ///
    /// This is the workhorse for building a parent node's span from its
    /// children: `lhs.span.to(rhs.span)`.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the spans are in different files, which would
    /// silently produce a meaningless range.
    #[must_use]
    pub fn to(self, other: Self) -> Self {
        debug_assert_eq!(self.file, other.file, "cannot merge spans across files");
        Self {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Whether `pos` falls within the span. Uses the half-open convention, so
    /// an empty span contains nothing.
    #[must_use]
    pub const fn contains(self, pos: BytePos) -> bool {
        self.start <= pos && pos < self.end
    }

    /// Whether the two spans overlap in at least one byte.
    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// An empty span at the start of `self`, for pointing at "just before this".
    #[must_use]
    pub const fn shrink_to_start(self) -> Self {
        Self {
            file: self.file,
            start: self.start,
            end: self.start,
        }
    }

    /// An empty span at the end of `self`, for pointing at "just after this".
    ///
    /// The common use is reporting a missing semicolon at the end of the
    /// preceding expression rather than at the start of the next token.
    #[must_use]
    pub const fn shrink_to_end(self) -> Self {
        Self {
            file: self.file,
            start: self.end,
            end: self.end,
        }
    }

    /// The range as a `usize` pair, for slicing source text.
    #[must_use]
    pub const fn range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compact, because spans appear in every AST snapshot test.
        write!(f, "{}:{}..{}", self.file.0, self.start, self.end)
    }
}

/// Attaches a [`Span`] to a value.
///
/// Used where a node is otherwise just a payload — a token, an identifier, a
/// literal — so the payload type stays free of position bookkeeping.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Spanned<T> {
    /// The value.
    pub value: T,
    /// Where it came from.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Pairs a value with its span.
    pub const fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    /// Applies `f` to the value, keeping the span.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }

    /// Borrows the value while keeping the span.
    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned {
            value: &self.value,
            span: self.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: SourceId = SourceId(0);

    #[test]
    fn merging_takes_the_outer_bounds() {
        let a = Span::new(F, 10, 20);
        let b = Span::new(F, 5, 15);
        assert_eq!(a.to(b), Span::new(F, 5, 20));
        // Merging is commutative, which callers rely on when folding children.
        assert_eq!(b.to(a), a.to(b));
    }

    #[test]
    fn contains_is_half_open() {
        let s = Span::new(F, 3, 5);
        assert!(!s.contains(2));
        assert!(s.contains(3));
        assert!(s.contains(4));
        assert!(!s.contains(5), "end is exclusive");
    }

    #[test]
    fn empty_span_contains_nothing() {
        let s = Span::at(F, 7);
        assert!(s.is_empty());
        assert!(!s.contains(7));
    }

    #[test]
    fn adjacent_spans_do_not_overlap() {
        let a = Span::new(F, 0, 5);
        let b = Span::new(F, 5, 10);
        assert!(!a.overlaps(b));
        assert!(a.overlaps(Span::new(F, 4, 6)));
    }

    #[test]
    fn shrink_gives_insertion_points() {
        let s = Span::new(F, 4, 9);
        assert_eq!(s.shrink_to_start(), Span::at(F, 4));
        assert_eq!(s.shrink_to_end(), Span::at(F, 9));
    }
}
