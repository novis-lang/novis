//! A position on the wire, and the byte offset under it.
//!
//! `rule:ide/positions-have-one-home` puts every offset conversion in
//! `nvs-diagnostics`, so this module is the boundary rather than a second
//! implementation of one: LSP's `Position` is a 0-based line and a column
//! counted in the encoding this server negotiated, `nvs_diagnostics::SourceFile`
//! answers both directions for all three encodings, and what is written here is
//! the change of type between them. An expression that adds to a line, a column
//! or an offset does not belong in this crate — getting it wrong is invisible
//! on ASCII and puts every answer on the wrong column the moment a file holds a
//! `ß`.
//!
//! A document's bytes are never normalised on the way in, and both of the ways
//! that shows are the source map's own decisions with the source map's own
//! tests: a CRLF document's columns are an LF document's, because a line stops
//! at its terminator, and a leading BOM stays in the text as the one column it
//! occupies in each encoding, so every offset after it still lands.

use lsp_types::{Position, PositionEncodingKind, Range};
use nvs_diagnostics::{BytePos, PositionEncoding, SourceFile, Span};

/// The negotiated encoding, as the crate that owns the arithmetic spells it.
///
/// `PositionEncodingKind` is an open string on the wire and
/// [`crate::negotiate_encoding`] closes it to the two this server offers.
/// Anything else reads as `utf-16`, which is LSP's default and what a client
/// that named no encoding at all means.
#[must_use]
pub fn encoding_of(kind: &PositionEncodingKind) -> PositionEncoding {
    if *kind == PositionEncodingKind::UTF8 {
        PositionEncoding::Utf8
    } else {
        PositionEncoding::Utf16
    }
}

/// The byte offset in `file` that `position` names, counted in `encoding`.
///
/// A position naming no boundary is clamped rather than refused, which is
/// `SourceFile::offset_of`'s own contract and not a decision taken here: the
/// buffer the client is positioning against can be a keystroke ahead of this
/// one, and an answer about the character before the cursor beats no answer.
#[must_use]
pub fn offset_at(file: &SourceFile, position: Position, encoding: PositionEncoding) -> BytePos {
    file.offset_of(widen(position.line), widen(position.character), encoding)
}

/// Where `offset` is in `file`, as the 0-based position the wire carries.
#[must_use]
pub fn position_at(file: &SourceFile, offset: BytePos, encoding: PositionEncoding) -> Position {
    let (line, character) = file.line_col_in(offset, encoding);
    Position {
        line: narrow(line),
        character: narrow(character),
    }
}

/// The range in `file` that `span` covers, as the wire carries one.
///
/// A span is a byte range and a range is a pair of positions, so this is
/// [`position_at`] twice — here rather than in each answer that needs one,
/// because an answer converting its own spans is one more place the conversion
/// can be wrong.
#[must_use]
pub fn range_at(file: &SourceFile, span: Span, encoding: PositionEncoding) -> Range {
    Range {
        start: position_at(file, span.start, encoding),
        end: position_at(file, span.end, encoding),
    }
}

/// A line or column the wire sent, as the source map counts one.
fn widen(units: u32) -> usize {
    usize::try_from(units).unwrap_or(usize::MAX)
}

/// A line or column the source map counted, as the wire carries one.
///
/// A file needing more than four billion of either cannot be loaded at all —
/// `nvs_diagnostics::MAX_SOURCE_LEN` refuses it first — so the saturation is
/// unreachable rather than a policy about huge documents.
fn narrow(units: usize) -> u32 {
    u32::try_from(units).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nvs_diagnostics::SourceMap;

    use super::*;

    /// One document, before it is spelled three ways.
    const LINES: [&str; 3] = ["<?nvs", "$total = \"ßx\";", "echo $total;"];

    /// [`LINES`] as a client sends them: `terminator` between and after, and a
    /// BOM in front when the file on disk had one.
    fn spelled(terminator: &str, bom: bool) -> String {
        let mut text = if bom {
            "\u{feff}".to_owned()
        } else {
            String::new()
        };
        for line in LINES {
            text.push_str(line);
            text.push_str(terminator);
        }
        text
    }

    /// `rule:ide/positions-have-one-home`: a document's bytes are never
    /// normalised, so the three spellings hold different bytes at different
    /// offsets — and a position still names the same character in each.
    ///
    /// The BOM is the one asymmetry, and it is not this crate's to decide: the
    /// source map leaves it in the text, where it is one column of line 0 and
    /// as many units as the encoding gives it, so a column on that line is
    /// shifted by exactly that and every offset after it lands.
    #[test]
    fn a_bom_and_a_crlf_document_answer_the_same_offsets_as_an_lf_one() {
        let mut map = SourceMap::new();
        let lf = map.add("lf.nvs", spelled("\n", false));
        let crlf = map.add("crlf.nvs", spelled("\r\n", false));
        let bom = map.add("bom.nvs", spelled("\n", true));

        // Line, its utf-8 column, its utf-16 column, and the text the offset
        // has to land on. The two columns part company after the `ß`, which is
        // two bytes and one code unit.
        let probes = [
            (0, 1, 1, "?nvs"),
            (1, 0, 0, "$total"),
            (1, 10, 10, "ßx\";"),
            (1, 12, 11, "x\";"),
            (2, 5, 5, "$total;"),
        ];

        for (name, id) in [("lf", lf), ("crlf", crlf), ("bom", bom)] {
            let file = map.file(id);
            for (line, utf8_col, utf16_col, expected) in probes {
                for (encoding, col) in [
                    (PositionEncoding::Utf8, utf8_col),
                    (PositionEncoding::Utf16, utf16_col),
                ] {
                    // A BOM is three bytes and one code unit, and line 0 of the
                    // document that has one starts that far along.
                    let col = match (id == bom && line == 0, encoding) {
                        (true, PositionEncoding::Utf8) => col + 3,
                        (true, _) => col + 1,
                        (false, _) => col,
                    };
                    let at = Position {
                        line,
                        character: col,
                    };

                    let offset = offset_at(file, at, encoding);
                    let landed = &file.text()[offset as usize..];
                    assert!(
                        landed.starts_with(expected),
                        "{name} at {line}:{col} in {encoding:?} landed on \
                         {:?} rather than {expected:?}",
                        landed
                            .chars()
                            .take(expected.chars().count())
                            .collect::<String>()
                    );
                    assert_eq!(
                        position_at(file, offset, encoding),
                        at,
                        "{name} at {line}:{col} in {encoding:?} did not round trip"
                    );
                }
            }
        }

        // The three really are different documents rather than one written
        // out three times: the same position is a different byte in each, which
        // is why no caller may compute one from a line and a column itself.
        let start_of_the_last_line = Position {
            line: 2,
            character: 0,
        };
        let offsets: BTreeSet<BytePos> = [lf, crlf, bom]
            .into_iter()
            .map(|id| {
                offset_at(
                    map.file(id),
                    start_of_the_last_line,
                    PositionEncoding::Utf16,
                )
            })
            .collect();
        assert_eq!(offsets.len(), 3, "{offsets:?}");
    }

    /// The negotiation's two answers, and the default under anything else.
    #[test]
    fn the_negotiated_kind_names_the_encoding_the_arithmetic_takes() {
        assert_eq!(
            encoding_of(&PositionEncodingKind::UTF8),
            PositionEncoding::Utf8
        );
        assert_eq!(
            encoding_of(&PositionEncodingKind::UTF16),
            PositionEncoding::Utf16
        );
        assert_eq!(
            encoding_of(&PositionEncodingKind::UTF32),
            PositionEncoding::Utf16,
            "an encoding this server never offered reads as LSP's default"
        );
    }
}
