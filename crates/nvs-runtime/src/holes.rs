//! `rule:types/string-holes`'s scanner: the one reading of a `string<K>`'s
//! text into its text pieces and its holes.
//!
//! A hole is `{`, a non-empty name with no brace in it, and `}`. `{{` is a
//! literal `{` and `}}` a literal `}`. An empty hole, a `{` with no `}` after
//! it and a `}` that closes nothing are errors, each at its brace.
//!
//! The checker reads a literal with it while compiling, through `nvs_stdlib`,
//! the run-time conversion reads a converted string with it, and
//! `Core\Str::parts` splits a value with it, so the three agree byte for byte.
//! It allocates nothing: [`Holes`] yields byte ranges into the text.

use std::ops::Range;

/// One piece of a string with holes, as a byte range into its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Text between holes, as written: a doubled brace is still two bytes.
    Text(Range<usize>),
    /// A hole's name, without its braces.
    Hole(Range<usize>),
}

/// Why a text is not a string with holes, with the byte offset of the brace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleError {
    /// `{}`.
    Empty(usize),
    /// A `{` with no `}` after it.
    Unclosed(usize),
    /// A `}` that is not doubled and closes no hole.
    LoneClose(usize),
}

impl HoleError {
    /// The byte offset of the brace the error is at.
    #[must_use]
    pub fn at(self) -> usize {
        match self {
            Self::Empty(at) | Self::Unclosed(at) | Self::LoneClose(at) => at,
        }
    }

    /// What is wrong, in the words an error message uses.
    #[must_use]
    pub fn describe(self) -> &'static str {
        match self {
            Self::Empty(_) => "a hole `{}` has no name in it",
            Self::Unclosed(_) => "a `{` has no `}` after it",
            Self::LoneClose(_) => "a `}` closes no hole",
        }
    }
}

/// The pieces of `text` in order, stopping after the first error.
#[derive(Debug)]
pub struct Holes<'a> {
    text: &'a [u8],
    at: usize,
    failed: bool,
}

/// Scans `text` as a string with holes.
#[must_use]
pub fn holes(text: &[u8]) -> Holes<'_> {
    Holes {
        text,
        at: 0,
        failed: false,
    }
}

impl Iterator for Holes<'_> {
    type Item = Result<Piece, HoleError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.at >= self.text.len() {
            return None;
        }
        let text = self.text;
        let start = self.at;
        if text[start] == b'{' && text.get(start + 1) != Some(&b'{') {
            let name = start + 1;
            let Some(len) = text[name..].iter().position(|&b| b == b'}' || b == b'{') else {
                self.failed = true;
                return Some(Err(HoleError::Unclosed(start)));
            };
            if text[name + len] == b'{' {
                self.failed = true;
                return Some(Err(HoleError::Unclosed(start)));
            }
            if len == 0 {
                self.failed = true;
                return Some(Err(HoleError::Empty(start)));
            }
            self.at = name + len + 1;
            return Some(Ok(Piece::Hole(name..name + len)));
        }
        let mut end = start;
        while end < text.len() {
            match (text[end], text.get(end + 1)) {
                (b'{', Some(b'{')) | (b'}', Some(b'}')) => end += 2,
                (b'{', _) => break,
                (b'}', _) => {
                    self.failed = true;
                    return Some(Err(HoleError::LoneClose(end)));
                }
                _ => end += 1,
            }
        }
        self.at = end;
        Some(Ok(Piece::Text(start..end)))
    }
}

#[cfg(test)]
mod tests {
    use super::{HoleError, Piece, holes};

    fn scan(text: &str) -> Result<Vec<Piece>, HoleError> {
        holes(text.as_bytes()).collect()
    }

    #[test]
    fn text_and_holes_come_in_order_and_a_doubled_brace_stays_text() {
        assert_eq!(
            scan("{a} = {{x}}{b.c}"),
            Ok(vec![
                Piece::Hole(1..2),
                Piece::Text(3..11),
                Piece::Hole(12..15),
            ])
        );
    }

    #[test]
    fn an_empty_hole_an_unclosed_brace_and_a_lone_close_are_errors_at_the_brace() {
        assert_eq!(scan("x {} y"), Err(HoleError::Empty(2)));
        assert_eq!(scan("x {a"), Err(HoleError::Unclosed(2)));
        assert_eq!(scan("{a{b}"), Err(HoleError::Unclosed(0)));
        assert_eq!(scan("a } b"), Err(HoleError::LoneClose(2)));
        assert_eq!(scan(""), Ok(vec![]));
    }
}
