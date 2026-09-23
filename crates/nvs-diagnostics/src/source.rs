//! The source map: every file the compiler has read, addressable by [`SourceId`].

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::span::{BytePos, SourceId, Span};

/// How a column is counted, which LSP 3.17 calls a `PositionEncodingKind`.
///
/// A server negotiates `Utf8` or `Utf16` and nothing else
/// (`rule:ide/positions-have-one-home`). `Utf32` is here because it is what
/// [`SourceFile::line_col`] already counts, so one conversion inverts every
/// column this crate hands out rather than most of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionEncoding {
    /// Columns are byte offsets into the line.
    Utf8,
    /// Columns are UTF-16 code units, which is what VS Code sends.
    Utf16,
    /// Columns are `char`s — Unicode scalar values.
    Utf32,
}

impl PositionEncoding {
    /// How many columns `ch` occupies in this encoding, which is never zero.
    const fn width(self, ch: char) -> usize {
        match self {
            Self::Utf8 => ch.len_utf8(),
            Self::Utf16 => ch.len_utf16(),
            Self::Utf32 => 1,
        }
    }
}

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
    /// Every constructor in [`SourceMap`] rejects text longer than
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

    /// The 0-based line and column of `pos`, counted in `encoding`.
    ///
    /// The inverse of [`offset_of`](Self::offset_of), and the direction a
    /// language server converts in when it answers with a position rather than
    /// being asked about one. [`line_col`](Self::line_col) and
    /// [`utf16_col`](Self::utf16_col) are this walk with the encoding written
    /// into the name, because a diagnostic asks for one of them by name; a
    /// caller that negotiated its encoding passes it as a value instead
    /// (`rule:ide/positions-have-one-home`).
    #[must_use]
    pub fn line_col_in(&self, pos: BytePos, encoding: PositionEncoding) -> (usize, usize) {
        let line = self.line_index(pos);
        let line_start = self.line_starts[line] as usize;
        let upto = &self.text[line_start..(pos as usize).min(self.text.len())];
        (line, upto.chars().map(|ch| encoding.width(ch)).sum())
    }

    /// The 0-based line and *character* column of `pos`.
    ///
    /// The column counts `char`s, not bytes, so a diagnostic on a line
    /// containing multi-byte UTF-8 points at the right place.
    #[must_use]
    pub fn line_col(&self, pos: BytePos) -> (usize, usize) {
        self.line_col_in(pos, PositionEncoding::Utf32)
    }

    /// The 0-based line and *UTF-16 code unit* column of `pos`, which is the
    /// position an LSP client speaking `utf-16` sends and expects.
    ///
    /// This is not [`line_col`](Self::line_col) with a different name: that one
    /// counts `char`s, so it agrees here on a `ß` — one `char`, one code unit,
    /// two bytes — and disagrees on anything outside the basic multilingual
    /// plane, where one `char` is a surrogate pair and counts twice.
    #[must_use]
    pub fn utf16_col(&self, pos: BytePos) -> (usize, usize) {
        self.line_col_in(pos, PositionEncoding::Utf16)
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

    /// The byte offset of a 0-based `line` and `col`, where `col` is counted in
    /// `encoding` — the inverse of [`line_col`](Self::line_col) under
    /// [`Utf32`](PositionEncoding::Utf32) and of
    /// [`utf16_col`](Self::utf16_col) under [`Utf16`](PositionEncoding::Utf16).
    ///
    /// A position that does not name a boundary is clamped rather than refused,
    /// because it arrives from an editor whose buffer can be a keystroke ahead
    /// of this one: a line past the last is the end of the file, a column past
    /// its line's content is the end of that content — before the line
    /// terminator, so a CRLF document answers what an LF one does — and a
    /// column falling inside a character is that character's start.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "every offset here is within text.len() <= MAX_SOURCE_LEN"
    )]
    pub fn offset_of(&self, line: usize, col: usize, encoding: PositionEncoding) -> BytePos {
        let Some(line_text) = self.line_text(line) else {
            return self.text.len() as BytePos;
        };
        let start = self.line_starts[line] as usize;
        let mut units = 0usize;
        for (i, ch) in line_text.char_indices() {
            // A width is at least 1, so this also catches `units == col`.
            if units + encoding.width(ch) > col {
                return (start + i) as BytePos;
            }
            units += encoding.width(ch);
        }
        (start + line_text.len()) as BytePos
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
    /// Text that stands in for a path's bytes, keyed by [`canonical_key`].
    /// Empty for every batch compilation — see [`SourceMap::overlay`].
    overlays: HashMap<PathBuf, String>,
    /// Every path a load asked for and got no file from, in the order asked —
    /// see [`SourceMap::missed`].
    missed: Vec<PathBuf>,
}

/// The form a path is keyed and compared under.
///
/// `std::fs::canonicalize` where the path resolves, and the path itself where
/// it does not, since a buffer for a file nobody has saved yet still needs a
/// key. One function because two would disagree the first time one was handed
/// `./b.nvs` and the other `/home/x/b.nvs`: `nvs-hir`'s graph walk
/// canonicalizes every `require` target before it loads it, so an overlay
/// registered under the path an editor sent would never be found.
#[must_use]
pub fn canonical_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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

    /// The id the next file pushed will be given.
    fn next_id(&self) -> SourceId {
        SourceId(u32::try_from(self.files.len()).expect("too many source files"))
    }

    /// Overlays `text` on `path`, so every later [`load`](Self::load) of that
    /// file reads this text and never touches the disk.
    ///
    /// This is how an editor's unsaved buffer reaches the compiler
    /// (`rule:ide/an-open-document-is-its-own-entry-point`): the `require`
    /// graph is resolved exactly as `nvs check` resolves it, so a class edited
    /// in one tab and named in another has to be the class the *buffer*
    /// declares. The substitution lives here rather than in the language server
    /// because the walk that reads the graph loads by path and is the only
    /// thing that knows which paths a program reads.
    ///
    /// A batch compilation registers none and pays one emptiness check per
    /// file for the possibility.
    ///
    /// # Panics
    ///
    /// Panics if `text` exceeds [`MAX_SOURCE_LEN`], as [`add`](Self::add) does.
    pub fn overlay(&mut self, path: impl AsRef<Path>, text: impl Into<String>) {
        let text = text.into();
        assert!(text.len() <= MAX_SOURCE_LEN, "source file exceeds 4 GiB");
        self.overlays.insert(canonical_key(path.as_ref()), text);
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
        let id = self.next_id();
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
        // An open buffer stands in for the file's bytes, and stands in front of
        // the embedded payload because the two can never answer for one path: a
        // bundled executable has no editor attached to it.
        if !self.overlays.is_empty()
            && let Some(text) = self.overlays.get(&canonical_key(path)).cloned()
        {
            let id = self.next_id();
            self.files.push(SourceFile::new(
                id,
                path.display().to_string(),
                Some(path.to_path_buf()),
                text,
            ));
            return Ok(id);
        }
        // `rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`: inside a bundled executable the payload is the byte
        // source, and the name a diagnostic prints is the path the *build* saw
        // rather than the synthetic one this process resolves against.
        if let Some((name, text)) = crate::embedded::text(path) {
            let id = self.next_id();
            self.files.push(SourceFile::new(
                id,
                name.to_owned(),
                Some(path.to_path_buf()),
                text.to_owned(),
            ));
            return Ok(id);
        }
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                self.note_missing(path);
                return Err(error);
            }
        };
        if text.len() > MAX_SOURCE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "source file exceeds 4 GiB",
            ));
        }
        let id = self.next_id();
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

    /// Records `path` as a file this compilation looked for and did not find.
    ///
    /// [`load`](Self::load) records its own failures. A caller that decides a
    /// path is missing before it loads anything — `nvs-hir`'s `require` walk,
    /// whose canonicalize fails first — records it here.
    pub fn note_missing(&mut self, path: &Path) {
        self.missed.push(path.to_path_buf());
    }

    /// Every path this compilation looked for and did not find, in the order it
    /// looked.
    ///
    /// A server that keeps a compiled program watches these beside the files
    /// that were read: a file created at one of them changes what the next
    /// compile of the same sources reads, with no file already read having
    /// changed (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
    #[must_use]
    pub fn missed(&self) -> &[PathBuf] {
        &self.missed
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
    fn utf16_col_counts_code_units_not_chars() {
        let mut map = SourceMap::new();
        // 'ß' is 2 bytes and one code unit; '😀' is 4 bytes and a surrogate
        // pair, so it is where the two columns part company.
        let id = map.add("t.nvs", "ß😀x");
        let f = map.file(id);

        assert_eq!(f.utf16_col(0), (0, 0));
        assert_eq!(f.utf16_col(2), (0, 1), "after the 2-byte ß");
        assert_eq!(f.line_col(2), (0, 1), "one char, one code unit");

        assert_eq!(f.utf16_col(6), (0, 3), "the emoji counted twice");
        assert_eq!(f.line_col(6), (0, 2), "the emoji counted once");
    }

    #[test]
    fn offset_of_inverts_line_col_on_a_multibyte_line() {
        let mut map = SourceMap::new();
        //                        a0 b1 \n2 ß3 x5 😀6 \n10 c11 d12
        let id = map.add("t.nvs", "ab\nßx😀\ncd");
        let f = map.file(id);

        // The emoji starts at byte 6: two chars into its line, two code units
        // into it, and three bytes into it.
        assert_eq!(f.line_col(6), (1, 2));
        assert_eq!(f.utf16_col(6), (1, 2));
        assert_eq!(f.offset_of(1, 2, PositionEncoding::Utf32), 6);
        assert_eq!(f.offset_of(1, 2, PositionEncoding::Utf16), 6);
        assert_eq!(
            f.offset_of(1, 2, PositionEncoding::Utf8),
            5,
            "two bytes in is the x"
        );

        assert_eq!(
            f.offset_of(1, 1, PositionEncoding::Utf8),
            3,
            "a byte column inside the ß is the ß"
        );
        assert_eq!(
            f.offset_of(1, 99, PositionEncoding::Utf16),
            10,
            "past the content is the end of the content"
        );
    }

    #[test]
    fn a_position_round_trips_through_utf16_and_utf8() {
        // A BOM is one char, one code unit and three bytes, and it is left in
        // the text: every offset after it still has to land.
        let src = "\u{feff}let $x = \"ß\";\n// 😀 a comment\n$y = 1;\n";
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", src);
        let f = map.file(id);

        for (pos, _) in src.char_indices().chain([(src.len(), ' ')]) {
            let pos = BytePos::try_from(pos).unwrap();
            let (line, units) = f.utf16_col(pos);
            assert_eq!(
                f.offset_of(line, units, PositionEncoding::Utf16),
                pos,
                "utf-16 round trip at {pos}"
            );

            let bytes = pos as usize - f.line_start(line).unwrap() as usize;
            assert_eq!(
                f.offset_of(line, bytes, PositionEncoding::Utf8),
                pos,
                "utf-8 round trip at {pos}"
            );

            let (_, chars) = f.line_col(pos);
            assert_eq!(
                f.offset_of(line, chars, PositionEncoding::Utf32),
                pos,
                "char round trip at {pos}"
            );
        }
    }

    #[test]
    fn a_crlf_documents_columns_match_an_lf_ones() {
        let mut map = SourceMap::new();
        //                    a0 ß1 \n3 b4 😀5 c9
        let lf = map.add("lf.nvs", "aß\nb😀c");
        //                        a0 ß1 \r3 \n4 b5 😀6 c10
        let crlf = map.add("crlf.nvs", "aß\r\nb😀c");

        let (lf, crlf) = (map.file(lf), map.file(crlf));
        assert_eq!(lf.utf16_col(9), (1, 3));
        assert_eq!(crlf.utf16_col(10), (1, 3), "the same c, the same column");
        assert_eq!(lf.offset_of(1, 3, PositionEncoding::Utf16), 9);
        assert_eq!(crlf.offset_of(1, 3, PositionEncoding::Utf16), 10);

        assert_eq!(
            lf.offset_of(0, 99, PositionEncoding::Utf16),
            crlf.offset_of(0, 99, PositionEncoding::Utf16),
            "a column past the content stops before the terminator in both"
        );
    }

    #[test]
    fn an_offset_past_the_last_line_is_clamped_rather_than_panicking() {
        let mut map = SourceMap::new();
        //                        a0 b1 \n2 ß3 😀5, len 9
        let id = map.add("t.nvs", "ab\nß😀");
        let f = map.file(id);

        assert_eq!(f.utf16_col(999), (1, 3), "the end of the last line");
        assert_eq!(
            f.offset_of(9, 0, PositionEncoding::Utf16),
            9,
            "a line past the last is the end of the file"
        );
        assert_eq!(
            f.offset_of(1, 999, PositionEncoding::Utf8),
            9,
            "a column past the last is the end of its line"
        );
        assert_eq!(f.offset_of(999, 999, PositionEncoding::Utf32), 9);
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
