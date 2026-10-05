//! *Put on separate lines* and *Join onto one line*: the edit that adds or
//! removes the line breaks at the own level of a list, a call chain or an
//! operator chain, written the way `nvs fmt` lays it out afterwards
//! (`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`).
//!
//! The construct is the innermost one around the cursor that [`crate::list`]
//! or [`crate::chain`] lays out. A list on one line is split: each item starts
//! a line one level in from the opener's line, a trailing comma follows the
//! last, and the closer starts a line of its own. A chain on one line is split
//! at every seam [`chain::seams_on`] names: each arrow or operator starts a
//! line one level in, and a condition's `(` ends its line and its `)` starts
//! one. Broken, either is joined: every line break at its own level goes, a
//! list's trailing comma goes, and a block comment at that level stays beside
//! the code it was written next to. A construct with a line comment at its own
//! level is offered no join, because the code after the comment would become
//! comment text. A line break inside an item or an operand belongs to it and
//! is never touched.
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
//! inside a list or a chain, all dropped with the answer. This runs in an editor request,
//! never on the request path or in the runtime.

use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, SourceMap};
use nvs_syntax::{IndexNode, Trivia, TriviaKind};

use crate::chain::{self, SeamKind, Seams};
use crate::indent::UNIT;
use crate::list::{List, code_before, writes_a_list};

/// Which way an edit changes a construct's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// The construct is on one line, and the edit puts one part on each line.
    Split,
    /// The construct is broken, and the edit puts it on one line.
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
    /// Which way the construct's layout changes.
    pub toggle: Toggle,
    /// The first byte replaced.
    pub start: usize,
    /// One past the last byte replaced.
    pub end: usize,
    /// What is written in their place.
    pub replacement: String,
}

/// The split or join offered with the cursor at `offset`, or [`None`] where no
/// list or chain is around it, the file does not parse, or a join would
/// swallow code into a line comment.
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
    let path = index.at(BytePos::try_from(offset).ok()?);
    let nodes = path.nodes();
    let construct = nodes.iter().enumerate().find_map(|(at, &node)| {
        let list = writes_a_list(node.kind)
            .then(|| List::of(index, text, trivia, node))
            .flatten()
            .filter(|list| (list.opener..=list.closer).contains(&offset));
        match list {
            Some(list) => Some(Construct::List(node, list)),
            None => chain::seams_on(index, text, trivia, nodes, at).map(Construct::Chain),
        }
    })?;

    // `from..to` is where formatting may change whitespace and commas, and
    // `first..last` is the construct's own extent.
    let (toggle, edited, from, to, first, last) = match &construct {
        Construct::List(node, list) => {
            let (toggle, written) = if list.broken {
                (Toggle::Join, joined(text, trivia, list)?)
            } else {
                (Toggle::Split, split(text, list))
            };
            let edited = format!(
                "{}{written}{}",
                &text[..list.opener],
                &text[list.closer + 1..]
            );
            let from = node.span.start as usize;
            (
                toggle,
                edited,
                from,
                node.span.end as usize,
                list.opener,
                list.closer + 1,
            )
        }
        Construct::Chain(seams) => {
            let toggle = if seams.broken {
                Toggle::Join
            } else {
                Toggle::Split
            };
            let edited = relaid(text, trivia, seams, toggle)?;
            // A broken condition's `)` moves too, and it sits on the line the
            // chain ends on.
            let to = text[seams.end..]
                .find('\n')
                .map_or(text.len(), |at| seams.end + at);
            let (_, first, _) = *seams.seams.first()?;
            let (_, _, last) = *seams.seams.last()?;
            (toggle, edited, seams.anchor, to, first, last)
        }
    };
    let line_start = text[..from].rfind('\n').map_or(0, |at| at + 1);
    let mut map = SourceMap::new();
    let id = map.add(file.name(), edited.as_str());
    let result = crate::format(map.file(id))
        .ok()
        .filter(|formatted| only_layout(text, formatted, line_start, to))
        .unwrap_or(edited);

    let (start, end) = changed(text, &result);
    let start = start.min(first);
    let end = end.max(last);
    let replacement = result[start..result.len() - (text.len() - end)].to_owned();
    Some(Relayout {
        toggle,
        start,
        end,
        replacement,
    })
}

/// The construct around the cursor.
enum Construct {
    /// A list, and the node that writes it.
    List(IndexNode, List),
    /// A call chain or an operator chain.
    Chain(Seams),
}

/// The line break `text` uses.
fn line_break_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// The indentation of the line `offset` is on.
fn base_of(text: &str, offset: usize) -> String {
    let line_start = text[..offset].rfind('\n').map_or(0, |at| at + 1);
    text[line_start..]
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect()
}

/// The whole of `text` with every seam of a chain split or joined, or [`None`]
/// where a join would swallow code into a line comment.
fn relaid(text: &str, trivia: &[Trivia], seams: &Seams, toggle: Toggle) -> Option<String> {
    let line_break = line_break_of(text);
    let base = base_of(text, seams.anchor);
    let mut out = String::with_capacity(text.len() + 64);
    let mut cursor = 0;
    for &(kind, from, to) in &seams.seams {
        let found = comments(text, trivia, from, to)?;
        out.push_str(&text[cursor..from]);
        if kind == SeamKind::Operand {
            out.push(' ');
            for comment in found {
                out.push_str(comment);
                out.push(' ');
            }
        } else {
            for comment in found {
                out.push(' ');
                out.push_str(comment);
            }
            match toggle {
                Toggle::Split => {
                    out.push_str(line_break);
                    out.push_str(&base);
                    if kind != SeamKind::Close {
                        out.push_str(UNIT);
                    }
                }
                Toggle::Join if kind == SeamKind::Operator => out.push(' '),
                Toggle::Join => {}
            }
        }
        cursor = to;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

/// The block comments in the trivia `from..to`, or [`None`] where a line
/// comment sits there.
fn comments<'a>(text: &'a str, trivia: &[Trivia], from: usize, to: usize) -> Option<Vec<&'a str>> {
    let mut found = Vec::new();
    for trivium in trivia
        .iter()
        .filter(|trivium| from <= trivium.span.start as usize && trivium.span.end as usize <= to)
    {
        match trivium.kind {
            TriviaKind::Whitespace => {}
            TriviaKind::BlockComment => {
                found.push(&text[trivium.span.start as usize..trivium.span.end as usize]);
            }
            TriviaKind::LineComment | TriviaKind::DocComment => return None,
        }
    }
    Some(found)
}

/// The list from its opener to its closer, one item per line.
fn split(text: &str, list: &List) -> String {
    let line_break = line_break_of(text);
    let base = base_of(text, list.opener);
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
    let comments = |from: usize, to: usize| comments(text, trivia, from, to);
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
    fn a_call_chain_on_one_line_splits_one_call_per_line_and_joins_back() {
        assert_eq!(
            relayout("<?nvs\n$rows = $query->from('o')-<|>>where('a')->fetch();\n"),
            Some((
                Toggle::Split,
                "<?nvs\n$rows = $query\n    ->from('o')\n    ->where('a')\n    ->fetch();\n"
                    .to_owned()
            ))
        );
        assert_eq!(
            relayout("<?nvs\n$rows = $query->from('o')\n    -<|>>where('a')->fetch();\n"),
            Some((
                Toggle::Join,
                "<?nvs\n$rows = $query->from('o')->where('a')->fetch();\n".to_owned()
            ))
        );
    }

    #[test]
    fn a_cursor_in_a_calls_arguments_toggles_the_arguments_not_the_chain() {
        assert_eq!(
            relayout("<?nvs\n$q->a(1, <|>2)->b();\n"),
            Some((
                Toggle::Split,
                "<?nvs\n$q->a(\n    1,\n    2,\n)->b();\n".to_owned()
            ))
        );
    }

    #[test]
    fn an_operator_chain_splits_operator_first_and_joins_back() {
        assert_eq!(
            relayout("<?nvs\n$ok = $a &<|>& $b && $c;\n"),
            Some((
                Toggle::Split,
                "<?nvs\n$ok = $a\n    && $b\n    && $c;\n".to_owned()
            ))
        );
        assert_eq!(
            relayout("<?nvs\n$ok = $a &&\n    $b <|>&& $c;\n"),
            Some((Toggle::Join, "<?nvs\n$ok = $a && $b && $c;\n".to_owned()))
        );
    }

    #[test]
    fn a_condition_splits_with_its_parentheses_on_lines_of_their_own() {
        let broken = "<?nvs\nif (\n    $a\n    && $b\n) {\n    echo 1;\n}\n";
        assert_eq!(
            relayout("<?nvs\nif ($a <|>&& $b) {\n    echo 1;\n}\n"),
            Some((Toggle::Split, broken.to_owned()))
        );
        assert_eq!(
            relayout(&broken.replacen("&& $b", "&& <|>$b", 1)),
            Some((
                Toggle::Join,
                "<?nvs\nif ($a && $b) {\n    echo 1;\n}\n".to_owned()
            ))
        );
    }

    #[test]
    fn a_line_comment_in_a_chain_is_offered_no_join() {
        assert_eq!(relayout("<?nvs\n$ok = $a // first\n    &<|>& $b;\n"), None);
    }

    #[test]
    fn a_cursor_outside_every_list_is_offered_nothing() {
        assert_eq!(relayout("<?nvs\n$a <|>= 1;\n"), None);
    }
}
