//! *Put on separate lines* and *Join onto one line*: the edit that adds or
//! removes the line breaks at a list's own level, written the way `nvs fmt`
//! lays the list out afterwards
//! (`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`).
//!
//! The list is the innermost one around the cursor that [`crate::list`] lays
//! out. On one line it is split: each item starts a line one level in from the
//! opener's line, a trailing comma follows the last, and the closer starts a
//! line of its own. Broken, it is joined: every line break at the list's own
//! level goes, the trailing comma goes, and a comment at that level stays
//! beside the item it was written next to. A list with a line comment at its
//! own level is offered no join, because the code after the comment would
//! become comment text. A line break inside an item belongs to the item and is
//! never touched.
//!
//! # Decision: the formatter writes the final layout
//!
//! The breaks are written first, and the file with them written is formatted.
//! Where formatting changed nothing outside the construct, and changed nothing
//! inside it but whitespace and commas, the formatted construct is the edit:
//! that is how an enum's brace moves and an item's own lines move in with it,
//! with no second copy of those rules here. Where formatting changed anything
//! else, the file was not formatted to begin with, and the edit is the breaks
//! alone. Formatting that file afterwards finishes the layout.
//!
//! # What it spends
//!
//! Two parses and one format of the file for each request whose cursor is
//! inside a list, all dropped with the answer. This runs in an editor request,
//! never on the request path or in the runtime.

use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, SourceMap};
use nvs_syntax::{Trivia, TriviaKind};

use crate::indent::UNIT;
use crate::list::{List, code_before, writes_a_list};

/// Which way an edit changes a list's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// The list is on one line, and the edit puts one item on each line.
    Split,
    /// The list is broken, and the edit puts it on one line.
    Join,
}

impl Toggle {
    /// What an editor shows for the action.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Split => "Put on separate lines",
            Self::Join => "Join onto one line",
        }
    }
}

/// One edit to a file: the bytes `start..end` are replaced by `replacement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relayout {
    /// Which way the list's layout changes.
    pub toggle: Toggle,
    /// The first byte replaced.
    pub start: usize,
    /// One past the last byte replaced.
    pub end: usize,
    /// What is written in their place.
    pub replacement: String,
}

/// The split or join offered with the cursor at `offset`, or [`None`] where no
/// list is around it, the file does not parse, or a join would swallow code
/// into a line comment.
pub(crate) fn at(file: &SourceFile, offset: usize) -> Option<Relayout> {
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(file, &mut diagnostics);
    if diagnostics
        .iter()
        .any(|d| d.is_error() && !crate::tokens::repairs(d))
    {
        return None;
    }
    let text = file.text();
    let (index, trivia) = (&parsed.index, &parsed.trivia);
    let (node, list) = index
        .at(BytePos::try_from(offset).ok()?)
        .nodes()
        .iter()
        .filter(|node| writes_a_list(node.kind))
        .find_map(|&node| {
            List::of(index, text, trivia, node)
                .filter(|list| (list.opener..=list.closer).contains(&offset))
                .map(|list| (node, list))
        })?;

    let (toggle, written) = if list.broken {
        (Toggle::Join, joined(text, trivia, &list)?)
    } else {
        (Toggle::Split, split(text, &list))
    };
    let edited = format!(
        "{}{written}{}",
        &text[..list.opener],
        &text[list.closer + 1..]
    );
    let line_start = text[..node.span.start as usize]
        .rfind('\n')
        .map_or(0, |at| at + 1);
    let mut map = SourceMap::new();
    let id = map.add(file.name(), edited.as_str());
    let result = crate::format(map.file(id))
        .ok()
        .filter(|formatted| only_layout(text, formatted, line_start, node.span.end as usize))
        .unwrap_or(edited);

    let (start, end) = changed(text, &result);
    let start = start.min(list.opener);
    let end = end.max(list.closer + 1);
    let replacement = result[start..result.len() - (text.len() - end)].to_owned();
    Some(Relayout {
        toggle,
        start,
        end,
        replacement,
    })
}

/// The list from its opener to its closer, one item per line.
fn split(text: &str, list: &List) -> String {
    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let line_start = text[..list.opener].rfind('\n').map_or(0, |at| at + 1);
    let base: String = text[line_start..]
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();
    let separators = std::iter::once(list.opener).chain(list.commas.iter().copied());
    let mut out = text[list.opener..=list.opener].to_owned();
    for (index, (separator, &item)) in separators.zip(&list.items).enumerate() {
        let end = list.commas.get(index).copied().unwrap_or(list.closer);
        let before = text[separator + 1..item].trim();
        out.push_str(line_break);
        out.push_str(&base);
        out.push_str(UNIT);
        if !before.is_empty() {
            out.push_str(before);
            out.push(' ');
        }
        out.push_str(text[item..end].trim_end());
        out.push(',');
    }
    out.push_str(line_break);
    out.push_str(&base);
    out.push_str(&text[list.closer..=list.closer]);
    out
}

/// The list from its opener to its closer, on one line, or [`None`] where a
/// line comment sits at its own level.
fn joined(text: &str, trivia: &[Trivia], list: &List) -> Option<String> {
    let comments = |from: usize, to: usize| -> Option<Vec<&str>> {
        let mut found = Vec::new();
        for trivium in trivia.iter().filter(|trivium| {
            from <= trivium.span.start as usize && trivium.span.end as usize <= to
        }) {
            match trivium.kind {
                TriviaKind::Whitespace => {}
                TriviaKind::BlockComment => {
                    found.push(&text[trivium.span.start as usize..trivium.span.end as usize]);
                }
                TriviaKind::LineComment | TriviaKind::DocComment => return None,
            }
        }
        Some(found)
    };
    let separators = std::iter::once(list.opener).chain(list.commas.iter().copied());
    let mut pieces: Vec<String> = Vec::new();
    for (index, (separator, &item)) in separators.zip(&list.items).enumerate() {
        let end = list.commas.get(index).copied().unwrap_or(list.closer);
        let code_end = code_before(trivia, end);
        let mut words = comments(separator + 1, item)?;
        words.push(&text[item..code_end]);
        words.extend(comments(code_end, end)?);
        pieces.push(words.join(" "));
    }
    if let Some(&trailing) = list.commas.get(list.items.len()) {
        let after = comments(trailing + 1, list.closer)?;
        if let Some(last) = pieces.last_mut() {
            for comment in after {
                last.push(' ');
                last.push_str(comment);
            }
        }
    }
    let pad = if text.as_bytes()[list.opener] == b'{' {
        " "
    } else {
        ""
    };
    Some(format!(
        "{}{pad}{}{pad}{}",
        &text[list.opener..=list.opener],
        pieces.join(", "),
        &text[list.closer..=list.closer]
    ))
}

/// Whether `formatted` differs from `text` only inside `from..to`, and only in
/// whitespace and commas there.
fn only_layout(text: &str, formatted: &str, from: usize, to: usize) -> bool {
    let (start, end) = changed(text, formatted);
    let kept = |slice: &str| -> String {
        slice
            .chars()
            .filter(|c| !c.is_whitespace() && *c != ',')
            .collect()
    };
    from <= start
        && end <= to
        && kept(&text[start..end]) == kept(&formatted[start..formatted.len() - (text.len() - end)])
}

/// The bytes of `before` that `after` replaces: everything between the
/// longest common prefix and the longest common suffix, on char boundaries.
fn changed(before: &str, after: &str) -> (usize, usize) {
    let (left, right) = (before.as_bytes(), after.as_bytes());
    let mut start = left.iter().zip(right).take_while(|(a, b)| a == b).count();
    while !before.is_char_boundary(start) {
        start -= 1;
    }
    let room = left.len().min(right.len()) - start;
    let mut suffix = left
        .iter()
        .rev()
        .zip(right.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(left.len() - suffix) {
        suffix -= 1;
    }
    (start, left.len() - suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relayout(source: &str) -> Option<(Toggle, String)> {
        let offset = source.find("<|>").expect("a cursor");
        let source = source.replacen("<|>", "", 1);
        let mut map = SourceMap::new();
        let id = map.add("case.nvs", source.as_str());
        let edit = at(map.file(id), offset)?;
        let mut out = source;
        out.replace_range(edit.start..edit.end, &edit.replacement);
        Some((edit.toggle, out))
    }

    #[test]
    fn a_call_on_one_line_splits_with_a_trailing_comma() {
        assert_eq!(
            relayout("<?nvs\nShop::place($a, $b<|>);\n"),
            Some((
                Toggle::Split,
                "<?nvs\nShop::place(\n    $a,\n    $b,\n);\n".to_owned()
            ))
        );
    }

    #[test]
    fn a_broken_array_joins_and_loses_its_trailing_comma() {
        assert_eq!(
            relayout("<?nvs\n$a = [\n    1,<|>\n    2,\n];\n"),
            Some((Toggle::Join, "<?nvs\n$a = [1, 2];\n".to_owned()))
        );
    }

    #[test]
    fn a_line_comment_at_the_lists_level_is_offered_no_join() {
        assert_eq!(
            relayout("<?nvs\n$a = [\n    1, // one\n    2<|>,\n];\n"),
            None
        );
    }

    #[test]
    fn the_innermost_list_is_the_one_toggled() {
        assert_eq!(
            relayout("<?nvs\nShop::place(\n    [1, <|>2],\n);\n"),
            Some((
                Toggle::Split,
                "<?nvs\nShop::place(\n    [\n        1,\n        2,\n    ],\n);\n".to_owned()
            ))
        );
    }

    #[test]
    fn an_enum_on_one_line_splits_under_an_allman_brace_and_joins_back() {
        let split = relayout("<?nvs\nenum Axis { Left, <|>Right }\n");
        assert_eq!(
            split,
            Some((
                Toggle::Split,
                "<?nvs\nenum Axis\n{\n    Left,\n    Right,\n}\n".to_owned()
            ))
        );
        assert_eq!(
            relayout("<?nvs\nenum Axis\n{\n    Left,<|>\n    Right,\n}\n"),
            Some((
                Toggle::Join,
                "<?nvs\nenum Axis { Left, Right }\n".to_owned()
            ))
        );
    }

    #[test]
    fn a_cursor_outside_every_list_is_offered_nothing() {
        assert_eq!(relayout("<?nvs\n$a <|>= 1;\n"), None);
    }
}
