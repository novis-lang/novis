//! The one refactor this server computes: a string, or a `.` chain of
//! strings and values, rewritten as the ``html`…` `` literal that prints the
//! same bytes (`rule:ide/a-string-converts-to-an-html-template`).
//!
//! Every other code action is a diagnostic's own suggestion
//! (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
//! This one has no diagnostic behind it, because nothing about the string is
//! wrong: converting it changes what is printed — a plain string echoed is
//! escaped as text, and an html literal's segments are markup — so it is a
//! rewrite the developer asks for and reviews in the preview, filed under
//! [`KIND`] and never under `source.fixAll.nvs` or `quickfix`.
//!
//! # Decision: the conversion is checked by parsing what it wrote
//!
//! The literal is built by re-escaping each source character for the backtick
//! form ([`render`]), and before it is offered it is parsed back
//! ([`Meaning`]): its segments, cooked, must equal the original's text, and its
//! holes must be the original's expressions, in order. A case the escaper
//! gets wrong is therefore never offered rather than offered wrong, and the
//! comparison uses the lexer's and the cooker's own code, so a later change to
//! the literal's grammar is checked here without this file learning it.
//!
//! # What is converted and what is not
//!
//! A double-quoted string keeps its interpolations and every escape the two
//! grammars share; `\"` is written as `"`, and a `` \` `` or `\{` that the
//! double-quoted grammar passes through as two characters is written so it
//! still prints both. A single-quoted string has no interpolation, so a `$`
//! that would start one is written `\$`. In both, a backtick is `` \` ``, a
//! `{` that would open a hole or draw `W1012` is `\{`, and a `<` that would
//! open `<?=`, `<?nvs` or `<?php` is `\x3C`. A heredoc or nowdoc is not
//! offered: its body is dedented by its closing label, and an html literal
//! has no such rule.
//!
//! In a chain, a variable, or a property or offset read on one, becomes a
//! `{$…}` hole, and any other operand a `<?= … ?>` hole. A chain with a
//! comment between its operands is not offered, since the rewrite would drop
//! it.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. Per request: one parse of
//! the string or chain under the cursor for each `.` above it, one of the
//! literal written, and one per enclosing array or `match` checked for a key
//! position — each the size of the expression, dropped with the answer.

use nvs_diagnostics::{BytePos, Diagnostics, PositionEncoding, SourceFile, SourceMap, Span};
use nvs_syntax::ast::{BinaryOp, Expr, ExprKind, StringPart};
use nvs_syntax::string_lit::{cook_double_quoted_text, cook_markup_text, cook_string_literal};
use nvs_syntax::{IndexNode, parse_expression};

use crate::document::Analysed;
use crate::position::range_at;
use crate::render::Action;

/// The kind the action is filed under: a sub-kind of `refactor.rewrite`, so a
/// client asking for exactly this action by kind gets it and nothing else.
pub const KIND: &str = "refactor.rewrite.htmlLiteral";

/// What the client shows for it.
pub const TITLE: &str = "Convert to html literal";

/// The conversion offered over `[start, end)` of the entry document, if any.
///
/// An empty range is a cursor: the string under it is converted, or, when the
/// string is an operand of a `.` chain, the whole chain, since half of a chain
/// converted is a `Markup` concatenated with a string. A range that is not
/// empty must cover one string or one chain exactly.
#[must_use]
pub fn at(
    analysed: &Analysed,
    start: BytePos,
    end: BytePos,
    encoding: PositionEncoding,
) -> Option<Action> {
    let file = analysed.map.file(analysed.entry);
    let source = file.text();
    let path = analysed.index.at(start);
    let nodes = path.nodes();
    let text_of = |node: &IndexNode| source.get(node.span.start as usize..node.span.end as usize);

    let target = if start == end {
        let base = nodes
            .iter()
            .position(|node| written_string(node.kind, text_of(node)).is_some())
            .unwrap_or(0);
        if nodes
            .get(base)
            .is_some_and(|node| written_string(node.kind, text_of(node)) == Some(false))
        {
            return None;
        }
        let mut top = base;
        while nodes
            .get(top + 1)
            .is_some_and(|parent| parent.kind == "Binary" && text_of(parent).is_some_and(is_concat))
        {
            top += 1;
        }
        top
    } else {
        nodes
            .iter()
            .position(|node| node.span.start == start && node.span.end == end)?
    };

    let node = nodes.get(target)?;
    if !matches!(node.kind, "Str" | "Interpolated" | "Binary") {
        return None;
    }
    let text = text_of(node)?;
    if let Some(parent) = nodes.get(target + 1)
        && !allowed_under(parent, node.span, text_of(parent)?)
    {
        return None;
    }
    let replacement = convert(text)?;
    Some(Action {
        title: TITLE.to_owned(),
        kind: KIND.to_owned(),
        range: range_at(file, node.span, encoding),
        replacement,
    })
}

/// Whether a node is a literal a person wrote with quotes: `Some(true)` for a
/// string this module may convert, `Some(false)` for an html literal, which is
/// converted already, and `None` for anything else — including the bareword
/// offset of `"$row[key]"`, which the parser also records as a string.
fn written_string(kind: &str, text: Option<&str>) -> Option<bool> {
    let text = text?;
    match kind {
        "Markup" => Some(false),
        "Str" | "Interpolated" if text.starts_with(['"', '\'']) || text.starts_with("<<<") => {
            Some(true)
        }
        _ => None,
    }
}

/// Whether `text` is an expression whose outermost operator is `.`.
fn is_concat(text: &str) -> bool {
    Snippet::parse(text).is_some_and(|snippet| {
        matches!(
            snippet.expr.kind,
            ExprKind::Binary {
                op: BinaryOp::Concat,
                ..
            }
        )
    })
}

/// Whether the expression at `target` may become a `Core\Html\Markup` where it
/// stands inside `parent`, as far as the syntax alone says.
///
/// A key, a subscript, a `case` label, a `match` condition, a constant's
/// value and the path of a `require` or `use` are positions a markup value has
/// no meaning in, so the action is not offered there. Everything else is, and
/// whether the result type-checks is the preview's to show.
fn allowed_under(parent: &IndexNode, target: Span, parent_text: &str) -> bool {
    match parent.kind {
        "Const" | "EnumCase" | "Switch" | "Require" | "UseDecl" | "AutoloadDecl" | "Index"
        | "Isset" | "Unset" | "Global" => false,
        "ArrayLiteral" | "Match" => {
            let Some(snippet) = Snippet::parse(parent_text) else {
                return false;
            };
            let relative = (
                target.start - parent.span.start + Snippet::LEAD_LEN,
                target.end - parent.span.start + Snippet::LEAD_LEN,
            );
            let at = |expr: &Expr| (expr.span.start, expr.span.end) == relative;
            match &snippet.expr.kind {
                ExprKind::ArrayLiteral(items) => {
                    !items.iter().any(|item| item.key.as_ref().is_some_and(at))
                }
                ExprKind::Match { subject, arms } => {
                    !at(subject)
                        && !arms
                            .iter()
                            .flat_map(|arm| arm.conditions.iter().flatten())
                            .any(at)
                }
                _ => false,
            }
        }
        _ => true,
    }
}

/// One expression parsed on its own, in a file of its own.
struct Snippet {
    map: SourceMap,
    id: nvs_diagnostics::SourceId,
    expr: Expr,
}

impl Snippet {
    /// What the snippet's file starts with, so the lexer opens in code mode:
    /// a first line beginning `#!` is a comment there
    /// (`rule:tooling/shebang-opens-code-mode`).
    const LEAD: &'static str = "#!\n";
    const LEAD_LEN: u32 = 3;

    /// `text` parsed as one expression, or `None` when it reports an error or
    /// leaves anything unparsed.
    fn parse(text: &str) -> Option<Self> {
        let mut map = SourceMap::new();
        let id = map.add("html-literal.nvs", format!("{}{text}", Self::LEAD));
        let mut diags = Diagnostics::new();
        let expr = parse_expression(map.file(id), &mut diags);
        let whole = u32::try_from(text.len()).ok()? + Self::LEAD_LEN;
        (diags.error_count() == 0 && expr.span.start == Self::LEAD_LEN && expr.span.end == whole)
            .then_some(Self { map, id, expr })
    }

    fn file(&self) -> &SourceFile {
        self.map.file(self.id)
    }

    fn text(&self, span: Span) -> &str {
        self.file().span_text(span).unwrap_or_default()
    }
}

/// The html literal that prints what `text` — a string or a `.` chain —
/// prints, or `None` when one of its parts cannot be converted or the
/// literal written does not parse back to the same text and holes.
#[must_use]
pub fn convert(text: &str) -> Option<String> {
    let snippet = Snippet::parse(text)?;
    let mut parts = Vec::new();
    flatten(&snippet.expr, &mut parts);
    for pair in parts.windows(2) {
        let between = snippet.text(Span::new(snippet.id, pair[0].span.end, pair[1].span.start));
        if between.trim() != "." {
            return None;
        }
    }

    let mut pieces = Vec::new();
    let mut expected = Vec::new();
    let mut literals = 0;
    for (index, part) in parts.iter().enumerate() {
        let followed = index + 1 < parts.len();
        if push_part(&snippet, part, followed, &mut pieces, &mut expected)? {
            literals += 1;
        }
    }
    if literals == 0 {
        return None;
    }

    let written = format!("html`{}`", render(&pieces));
    let back = Snippet::parse(&written)?;
    let ExprKind::Markup(segments) = &back.expr.kind else {
        return None;
    };
    let mut found = Vec::new();
    for segment in segments {
        match segment {
            StringPart::Text(span) => push_text(&mut found, cook_markup_text(back.file(), *span).0),
            StringPart::Expr(hole) => found.push(Meaning::Hole(back.text(hole.span).to_owned())),
        }
    }
    (found == expected).then_some(written)
}

/// The operands of a `.` chain in source order, or the expression alone when
/// it is not one.
fn flatten<'a>(expr: &'a Expr, out: &mut Vec<&'a Expr>) {
    if let ExprKind::Binary {
        op: BinaryOp::Concat,
        lhs,
        rhs,
    } = &expr.kind
    {
        flatten(lhs, out);
        flatten(rhs, out);
    } else {
        out.push(expr);
    }
}

/// What a literal prints, as the parser sees it: runs of cooked text and the
/// source of each hole's expression.
#[derive(Debug, PartialEq, Eq)]
enum Meaning {
    Text(String),
    Hole(String),
}

fn push_text(out: &mut Vec<Meaning>, text: String) {
    if text.is_empty() {
        return;
    }
    if let Some(Meaning::Text(run)) = out.last_mut() {
        run.push_str(&text);
    } else {
        out.push(Meaning::Text(text));
    }
}

/// One unit of the literal being written.
enum Piece {
    /// A character the literal must print as itself.
    Char(char),
    /// An escape both grammars read the same way, copied as written.
    Escape(String),
    /// A hole, written out whole: `$name`, `{$…}` or `<?= … ?>`.
    Hole(String),
}

/// Adds one operand's pieces and meaning, and says whether it was a string
/// literal. `followed` is whether another operand comes after it.
fn push_part(
    snippet: &Snippet,
    part: &Expr,
    followed: bool,
    pieces: &mut Vec<Piece>,
    expected: &mut Vec<Meaning>,
) -> Option<bool> {
    let text = snippet.text(part.span);
    match &part.kind {
        ExprKind::Str(span) => {
            let raw = snippet.text(*span);
            let inner = raw.get(1..raw.len().checked_sub(1)?)?;
            if raw.starts_with('\'') {
                push_single_quoted(inner, pieces);
            } else if raw.starts_with('"') {
                push_double_quoted(inner, pieces)?;
            } else {
                return None;
            }
            push_text(expected, cook_string_literal(snippet.file(), *span));
            Some(true)
        }
        ExprKind::Interpolated(segments) => {
            if !text.starts_with('"') {
                return None;
            }
            let body_end = part.span.end - 1;
            let mut at = part.span.start + 1;
            for segment in segments {
                match segment {
                    StringPart::Text(span) => {
                        if span.start > at {
                            pieces.push(Piece::Hole(
                                snippet
                                    .text(Span::new(snippet.id, at, span.start))
                                    .to_owned(),
                            ));
                        }
                        push_double_quoted(snippet.text(*span), pieces)?;
                        push_text(expected, cook_double_quoted_text(snippet.file(), *span).0);
                        at = span.end;
                    }
                    StringPart::Expr(hole) => {
                        expected.push(Meaning::Hole(snippet.text(hole.span).to_owned()))
                    }
                }
            }
            if body_end > at {
                let mut hole = snippet.text(Span::new(snippet.id, at, body_end)).to_owned();
                // A bare `$name` or `$name->prop` at the end of its string would
                // run on into whatever the next operand starts with, so it is
                // braced; `$name[key]` ends at its `]` and keeps PHP's simple
                // syntax, where a bareword key is a string.
                if followed
                    && let Some(StringPart::Expr(last)) = segments.last()
                    && last.span.end == body_end
                    && !snippet.text(last.span).ends_with(']')
                {
                    hole = format!(
                        "{}{{{}}}",
                        snippet.text(Span::new(snippet.id, at, last.span.start)),
                        snippet.text(last.span)
                    );
                }
                pieces.push(Piece::Hole(hole));
            }
            Some(true)
        }
        _ if reads_a_variable(part) => {
            pieces.push(Piece::Hole(format!("{{{text}}}")));
            expected.push(Meaning::Hole(text.to_owned()));
            Some(false)
        }
        _ => {
            pieces.push(Piece::Hole(format!("<?= {text} ?>")));
            expected.push(Meaning::Hole(text.to_owned()));
            Some(false)
        }
    }
}

/// A variable, or a property or offset read on one — what a `{$…}` hole is
/// written around in a page.
fn reads_a_variable(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Variable(_) => true,
        ExprKind::PropertyAccess { object, .. } => reads_a_variable(object),
        ExprKind::Index {
            base,
            index: Some(_),
        } => reads_a_variable(base),
        _ => false,
    }
}

/// A single-quoted body's characters: `\\` and `\'` are its only escapes.
fn push_single_quoted(inner: &str, pieces: &mut Vec<Piece>) {
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&next) = chars.peek()
            && (next == '\\' || next == '\'')
        {
            chars.next();
            pieces.push(Piece::Char(next));
        } else {
            pieces.push(Piece::Char(c));
        }
    }
}

/// A double-quoted text run's characters and escapes, or `None` for a
/// malformed `\u{…}`, which is already an error where it is written.
fn push_double_quoted(raw: &str, pieces: &mut Vec<Piece>) -> Option<()> {
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match (c, next) {
            ('\\', Some('"')) => pieces.push(Piece::Char('"')),
            // Not an escape in a double-quoted string, so both characters
            // print; the html literal reads each pair as one character.
            ('\\', Some(pair @ ('`' | '{'))) => {
                pieces.push(Piece::Char('\\'));
                pieces.push(Piece::Char(pair));
            }
            ('\\', Some('u')) if chars.get(i + 2) == Some(&'{') => {
                let digits = chars[i + 3..]
                    .iter()
                    .take_while(|d| d.is_ascii_hexdigit())
                    .count();
                let close = i + 3 + digits;
                if digits == 0 || chars.get(close) != Some(&'}') {
                    return None;
                }
                pieces.push(Piece::Escape(chars[i..=close].iter().collect()));
                i = close + 1;
                continue;
            }
            ('\\', Some(other)) => pieces.push(Piece::Escape(format!("\\{other}"))),
            _ => {
                pieces.push(Piece::Char(c));
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    Some(())
}

/// The body of the literal: each piece written so the html literal's lexer
/// and cooker read back exactly the character or hole it stands for.
fn render(pieces: &[Piece]) -> String {
    let mut out = String::new();
    for (index, piece) in pieces.iter().enumerate() {
        let c = match piece {
            Piece::Escape(text) | Piece::Hole(text) => {
                out.push_str(text);
                continue;
            }
            Piece::Char(c) => *c,
        };
        let ahead = ahead(&pieces[index + 1..]);
        let after_brace = index > 0 && matches!(pieces[index - 1], Piece::Char('{'));
        let hole_next =
            matches!(pieces.get(index + 1), Some(Piece::Hole(hole)) if hole.starts_with('$'));
        match c {
            '`' => out.push_str("\\`"),
            '\\' => out.push_str("\\\\"),
            '$' if after_brace
                || ahead.starts_with(|n: char| n == '_' || n.is_ascii_alphabetic()) =>
            {
                out.push_str("\\$");
            }
            '{' if hole_next || before_class_path(&ahead) => out.push_str("\\{"),
            '<' if opens_a_tag(&ahead) => out.push_str("\\x3C"),
            _ => out.push(c),
        }
    }
    out
}

/// The text that follows, as far as any escaping decision looks: a few
/// characters, up to and including the first character of the next hole.
fn ahead(pieces: &[Piece]) -> String {
    let mut text = String::new();
    for piece in pieces {
        match piece {
            Piece::Char(c) => text.push(*c),
            Piece::Escape(escape) => text.push_str(escape),
            Piece::Hole(hole) => {
                text.extend(hole.chars().next());
                break;
            }
        }
        if text.len() >= 64 {
            break;
        }
    }
    text
}

/// Whether a `{` before `rest` is the `{Page::TITLE}` shape the html literal
/// warns about with `W1012`: a name, then `::`.
fn before_class_path(rest: &str) -> bool {
    let name = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'\\')
        .count();
    name > 0
        && rest.as_bytes().first().is_some_and(|b| !b.is_ascii_digit())
        && rest[name..].starts_with("::")
}

/// Whether a `<` before `rest` would open a tag inside an html literal.
fn opens_a_tag(rest: &str) -> bool {
    rest.starts_with("?=")
        || rest
            .get(..4)
            .is_some_and(|tag| tag.eq_ignore_ascii_case("?nvs") || tag.eq_ignore_ascii_case("?php"))
}

#[cfg(test)]
mod tests {
    use super::convert;

    #[test]
    fn a_double_quoted_string_keeps_its_interpolation_and_its_shared_escapes() {
        assert_eq!(
            convert(r#""<b class=\"x\">$name</b>\n""#).as_deref(),
            Some(r#"html`<b class="x">$name</b>\n`"#)
        );
        assert_eq!(
            convert(r#""{$u->name()} `x`""#).as_deref(),
            Some(r"html`{$u->name()} \`x\``")
        );
    }

    #[test]
    fn a_double_quoted_pass_through_escape_still_prints_both_characters() {
        assert_eq!(
            convert(r#""a\`b\{c""#).as_deref(),
            Some(r"html`a\\\`b\\{c`")
        );
        assert_eq!(convert(r#""\{$x}""#).as_deref(), Some(r"html`\\\{$x}`"));
    }

    #[test]
    fn a_single_quoted_dollar_backslash_and_tag_stay_text() {
        assert_eq!(
            convert(r"'$name costs $5 {$x} \\ \' <?= no ?> {Page::TITLE}'").as_deref(),
            Some(r"html`\$name costs $5 {\$x} \\ ' \x3C?= no ?> \{Page::TITLE}`")
        );
        assert_eq!(convert("'C:\\dir'").as_deref(), Some(r"html`C:\\dir`"));
    }

    #[test]
    fn a_chain_turns_reads_into_brace_holes_and_other_values_into_tag_holes() {
        assert_eq!(
            convert(r#""<b>" . $name . "</b>" . count($rows) . '!'"#).as_deref(),
            Some(r"html`<b>{$name}</b><?= count($rows) ?>!`")
        );
        assert_eq!(
            convert(r#""<i>$a" . "bc" . $u->name . $r["k"]"#).as_deref(),
            Some(r#"html`<i>{$a}bc{$u->name}{$r["k"]}`"#)
        );
    }

    #[test]
    fn text_split_across_operands_is_escaped_as_one_run() {
        assert_eq!(
            convert(r"'$' . 'name' . '<?' . '='").as_deref(),
            Some(r"html`\$name\x3C?=`")
        );
        assert_eq!(convert(r#"'{' . "$x""#).as_deref(), Some(r"html`\{$x`"));
    }

    #[test]
    fn what_is_not_a_plain_string_is_not_converted() {
        assert_eq!(convert("html`<b>x</b>`"), None);
        assert_eq!(convert("$a . $b"), None);
        assert_eq!(convert("<<<EOT\n  <b>x</b>\n  EOT"), None);
        assert_eq!(convert("\"a\" . /* why */ $b"), None);
    }
}
