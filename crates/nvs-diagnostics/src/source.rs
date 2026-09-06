//! The source map: every file the compiler has read, addressable by [`SourceId`].

use std::io;
use std::path::{Path, PathBuf};

use crate::span::{BytePos, SourceId, Span};

/// One source file, with a precomputed line index.
#[derive(Debug)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    path: Option<PathBuf>,
    text: String,
    /// Byte offset of the first character of each line. Always starts with 0,
    /// so `line_starts.len()` is the number of lines.
    line_starts: Vec<BytePos>,
}

impl SourceFile {
    /// Both constructors in [`SourceMap`] reject text longer than
    /// [`MAX_SOURCE_LEN`] before calling this, which is what makes the offset
    /// arithmetic below infallible.
    fn new(id: SourceId, name: String, path: Option<PathBuf>, text: String) -> Self {
        debug_assert!(text.len() <= MAX_SOURCE_LEN);
        let mut line_starts = Vec::with_capacity(text.len() / 32 + 1);
        line_starts.push(0);
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "text.len() <= u32::MAX is enforced by both callers"
                )]
                line_starts.push(i as u32 + 1);
            }
        }
        Self {
            id,
            name,
            path,
            text,
            line_starts,
        }
    }

    /// This file's id.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Display name, as it should appear in diagnostics.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The path this file was read from, if it came from disk.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The full text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Number of lines. A file always has at least one line, even when empty.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// The 0-based line containing `pos`.
    ///
    /// An offset at or past the end of the file is clamped to the last line, so
    /// diagnostics about a truncated file still render somewhere sensible.
    #[must_use]
    pub fn line_index(&self, pos: BytePos) -> usize {
        match self.line_starts.binary_search(&pos) {
            Ok(exact) => exact,
            // `Err(i)` is the insertion point, so the containing line is i - 1.
            // i is never 0 because line_starts[0] == 0 and pos >= 0.
            Err(i) => i - 1,
        }
    }

    /// The 0-based line and *character* column of `pos`.
    ///
    /// The column counts `char`s, not bytes, so a diagnostic on a line
    /// containing multi-byte UTF-8 points at the right place.
    #[must_use]
    pub fn line_col(&self, pos: BytePos) -> (usize, usize) {
        let line = self.line_index(pos);
        let line_start = self.line_starts[line] as usize;
        let upto = &self.text[line_start..(pos as usize).min(self.text.len())];
        (line, upto.chars().count())
    }

    /// The text of a 0-based line, without its trailing newline.
    ///
    /// Returns `None` if `line` is out of range.
    #[must_use]
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line)? as usize;
        let end = self
            .line_starts
            .get(line + 1)
            .map_or(self.text.len(), |&e| e as usize);
        Some(self.text[start..end].trim_end_matches(['\n', '\r']))
    }

    /// The byte offset at which a 0-based line begins.
    #[must_use]
    pub fn line_start(&self, line: usize) -> Option<BytePos> {
        self.line_starts.get(line).copied()
    }

    /// The source text a span covers.
    ///
    /// Returns `None` if the span belongs to another file or runs out of bounds.
    #[must_use]
    pub fn span_text(&self, span: Span) -> Option<&str> {
        if span.file != self.id {
            return None;
        }
        self.text.get(span.range())
    }
}

/// Every file the compiler has read.
///
/// Ids are indices, so lookup is a bounds check. The map only grows, which lets
/// spans stay valid for the whole compilation.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

/// Largest source file Novis will load.
///
/// Spans use `u32` offsets; refusing oversized input at the boundary is what
/// makes every later cast infallible.
pub const MAX_SOURCE_LEN: usize = u32::MAX as usize;

impl SourceMap {
    /// An empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file with an explicit display name, for input not read from disk
    /// (a REPL line, a test fixture, generated code).
    ///
    /// # Panics
    ///
    /// Panics if `text` exceeds [`MAX_SOURCE_LEN`].
    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        let text = text.into();
        assert!(text.len() <= MAX_SOURCE_LEN, "source file exceeds 4 GiB");
        let id = SourceId(u32::try_from(self.files.len()).expect("too many source files"));
        self.files
            .push(SourceFile::new(id, name.into(), None, text));
        id
    }

    /// Reads a file from disk and adds it.
    ///
    /// # Errors
    ///
    /// Returns any I/O error from reading the path, and
    /// [`io::ErrorKind::InvalidData`] if the file is not valid UTF-8 or exceeds
    /// [`MAX_SOURCE_LEN`].
    pub fn load(&mut self, path: impl AsRef<Path>) -> io::Result<SourceId> {
        let path = path.as_ref();
        // `rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`: inside a bundled executable the payload is the byte
        // source, and the name a diagnostic prints is the path the *build* saw
        // rather than the synthetic one this process resolves against.
        if let Some((name, text)) = crate::embedded::text(path) {
            let id = SourceId(u32::try_from(self.files.len()).expect("too many source files"));
            self.files.push(SourceFile::new(
                id,
                name.to_owned(),
                Some(path.to_path_buf()),
                text.to_owned(),
            ));
            return Ok(id);
        }
        let text = std::fs::read_to_string(path)?;
        if text.len() > MAX_SOURCE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "source file exceeds 4 GiB",
            ));
        }
        let id = SourceId(u32::try_from(self.files.len()).expect("too many source files"));
        let name = path.display().to_string();
        self.files
            .push(SourceFile::new(id, name, Some(path.to_path_buf()), text));
        Ok(id)
    }

    /// Looks up a file.
    ///
    /// # Panics
    ///
    /// Panics if the id did not come from this map. Ids are only produced by
    /// [`add`](Self::add) and [`load`](Self::load), so this is a bug, not input
    /// validation.
    #[must_use]
    pub fn file(&self, id: SourceId) -> &SourceFile {
        &self.files[id.index()]
    }

    /// Looks up a file, returning `None` for an unknown id.
    #[must_use]
    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.index())
    }

    /// Every file, in insertion order.
    pub fn files(&self) -> impl ExactSizeIterator<Item = &SourceFile> {
        self.files.iter()
    }

    /// How many files have been added.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether no files have been added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_finds_the_containing_line() {
        let mut map = SourceMap::new();
        //                        0123 4567 89
        let id = map.add("t.nvs", "ab\ncd\nef");
        let f = map.file(id);

        assert_eq!(f.line_count(), 3);
        assert_eq!(f.line_index(0), 0);
        assert_eq!(f.line_index(1), 0);
        assert_eq!(
            f.line_index(2),
            0,
            "the newline belongs to the line it ends"
        );
        assert_eq!(f.line_index(3), 1);
        assert_eq!(f.line_index(6), 2);
    }

    #[test]
    fn empty_file_has_one_line() {
        let mut map = SourceMap::new();
        let id = map.add("empty.nvs", "");
        let f = map.file(id);
        assert_eq!(f.line_count(), 1);
        assert_eq!(f.line_text(0), Some(""));
        assert_eq!(f.line_col(0), (0, 0));
    }

    #[test]
    fn trailing_newline_creates_a_final_empty_line() {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", "a\n");
        let f = map.file(id);
        assert_eq!(f.line_count(), 2);
        assert_eq!(f.line_text(1), Some(""));
    }

    #[test]
    fn columns_count_chars_not_bytes() {
        let mut map = SourceMap::new();
        // 'ä' and '€' are 2 and 3 bytes respectively.
        let id = map.add("t.nvs", "ä€x");
        let f = map.file(id);
        assert_eq!(f.line_col(0), (0, 0));
        assert_eq!(f.line_col(2), (0, 1), "after the 2-byte ä");
        assert_eq!(f.line_col(5), (0, 2), "after the 3-byte €");
    }

    #[test]
    fn line_text_strips_crlf() {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", "a\r\nb");
        let f = map.file(id);
        assert_eq!(f.line_text(0), Some("a"));
        assert_eq!(f.line_text(1), Some("b"));
        assert_eq!(f.line_text(2), None);
    }

    #[test]
    fn offset_past_end_clamps_to_last_line() {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", "ab\ncd");
        let f = map.file(id);
        assert_eq!(f.line_index(999), 1);
        assert_eq!(f.line_col(999), (1, 2));
    }

    #[test]
    fn span_text_rejects_a_foreign_span() {
        let mut map = SourceMap::new();
        let a = map.add("a.nvs", "hello");
        let b = map.add("b.nvs", "world");
        let span_in_b = Span::new(b, 0, 5);
        assert_eq!(map.file(b).span_text(span_in_b), Some("world"));
        assert_eq!(map.file(a).span_text(span_in_b), None);
    }
}
