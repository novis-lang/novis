//! The note a moved `Core` name gets on a failed compile
//! (`rule:core-api/a-moved-name-says-where-it-went`).
//!
//! [`note_moves`] is a pass over the finished diagnostics, in the shape of
//! `nvs_hir::AutoloadMap::note_searched_roots`. It reads
//! `nvs_stdlib::moved::MOVES`, and it lives here because this crate already
//! reads both `nvs-hir` and `nvs-stdlib`. A moved name reaches it in one of two
//! codes. A type and an expression such as `Core\Order::Desc` get `E0303`,
//! which `nvs_hir::hierarchy::undeclared_subject` reads back. A `use` line gets
//! `E0306`, which `nvs_hir::resolve::unresolved_import_subject` reads back. The
//! short name such a `use` brought in is reported nowhere, because the `use`
//! line already is (`rule:types/one-mistake-reports-once`), so the fix on the
//! `use` line rewrites those references too.
//!
//! Each diagnostic gets a help line that names the new place and one safe fix,
//! which `nvs check --fix` applies and an editor offers as one action. The fix
//! writes the new name in full where the old one is written. On a `use` line it
//! deletes the line and writes the new name at every reference to the short
//! name, because a class-scoped enum such as `Core\Arr::Order` cannot be
//! imported. A form the fix cannot rewrite exactly, such as an aliased or a
//! grouped `use`, gets the help line alone.
//!
//! The pass runs on a failed compile, and in the editor on every analysis, where
//! it costs one comparison per `E0303` and `E0306`. A `use` line of a moved name
//! costs one lex of its file.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceMap, Span, Suggestion, code};
use nvs_stdlib::moved::Move;
use nvs_syntax::{Keyword, TokenKind};

/// The row for the name `diagnostic` says is not declared (`E0303`) or does not
/// resolve (`E0306`), if that name moved.
#[must_use]
pub fn moved_subject(diagnostic: &Diagnostic) -> Option<&'static Move> {
    let name = nvs_hir::hierarchy::undeclared_subject(diagnostic)
        .or_else(|| nvs_hir::resolve::unresolved_import_subject(diagnostic))?;
    nvs_stdlib::moved::moved(&name.to_string())
}

/// Adds to every diagnostic about a moved name the help line that names its new
/// place and the fix that writes it. `map` holds the files the diagnostics
/// point into.
pub fn note_moves(diags: &mut Diagnostics, map: &SourceMap) {
    for diagnostic in diags.iter_mut() {
        let Some(row) = moved_subject(diagnostic) else {
            continue;
        };
        let import = diagnostic.code == Some(code::E_UNRESOLVED_IMPORT);
        let fix = diagnostic.primary_span().and_then(|span| {
            let file = map.get(span.file)?;
            if import {
                import_fix(row, span, file)
            } else {
                written_fix(row, span, file).map(|edit| vec![edit])
            }
        });
        let moved_to = match row.since {
            Some(version) => format!("`{}` moved to `{}` in {version}", row.old, row.new),
            None => format!("`{}` moved to `{}`", row.old, row.new),
        };
        let help = if import {
            format!(
                "{moved_to}. A `use` line cannot import it. Delete this line, and write `{}` \
                 where this file writes `{}`",
                row.new,
                short_name(row.old)
            )
        } else {
            format!("{moved_to}. Write `{}` here", row.new)
        };
        diagnostic.notes.push(format!("help: {help}"));
        for (span, replacement) in fix.unwrap_or_default() {
            let message = if replacement.is_empty() {
                format!("delete the `use` line for `{}`", row.old)
            } else {
                "write the new name".to_owned()
            };
            diagnostic.suggestions.push(Suggestion {
                span,
                replacement,
                message,
                safe: true,
                alternative: false,
            });
        }
    }
}

/// The last segment of a qualified name: `Order` for `Core\Order`.
fn short_name(name: &str) -> &str {
    name.rsplit('\\').next().unwrap_or(name)
}

/// The edit that writes `row.new` over the old name at the start of `span`,
/// keeping a leading `\`. The span is the name itself, or a whole static call
/// such as `Core\Order::from(1)`, whose name is followed by `::`. `None` when
/// the text there does not start with the old name, such as a short name,
/// which the fix cannot rewrite exactly.
fn written_fix(row: &Move, span: Span, file: &SourceFile) -> Option<(Span, String)> {
    let text = file.span_text(span)?;
    let lead = if text.starts_with('\\') { "\\" } else { "" };
    let name = text.get(lead.len()..lead.len() + row.old.len())?;
    let rest = &text[lead.len() + row.old.len()..];
    if !name.eq_ignore_ascii_case(row.old) || !(rest.is_empty() || rest.starts_with("::")) {
        return None;
    }
    let end = span.start + u32::try_from(lead.len() + row.old.len()).ok()?;
    Some((
        Span::new(span.file, span.start, end),
        format!("{lead}{}", row.new),
    ))
}

/// The edits that delete `use <old>;` and write `row.new` at every reference to
/// its short name in the file. `None` unless the `use` is the plain form, one
/// name and a `;`, because an alias or a group changes what the short name is.
///
/// A reference is the short name as an identifier of its own: not after `::`,
/// `->` or `\`, not before `\`, and not the name a declaration gives.
fn import_fix(row: &Move, span: Span, file: &SourceFile) -> Option<Vec<(Span, String)>> {
    if !file.span_text(span)?.eq_ignore_ascii_case(row.old) {
        return None;
    }
    let mut lexed = Diagnostics::new();
    let tokens = nvs_syntax::tokenize(file, &mut lexed);
    let at = tokens
        .iter()
        .position(|token| token.span.start == span.start)?;
    let first = tokens[..at]
        .iter()
        .rposition(|token| token.kind == TokenKind::Keyword(Keyword::Use))?;
    let last = at
        + tokens[at..]
            .iter()
            .position(|token| token.kind == TokenKind::Semicolon)?;
    if tokens[first + 1..last]
        .iter()
        .any(|token| token.span.start < span.start || token.span.end > span.end)
    {
        return None;
    }
    let text = file.text();
    let mut end = tokens[last].span.end as usize;
    if let Some(newline) = ["\r\n", "\n"]
        .into_iter()
        .find(|newline| text[end..].starts_with(newline))
    {
        end += newline.len();
    }
    let line = Span::new(
        span.file,
        tokens[first].span.start,
        u32::try_from(end).ok()?,
    );
    let mut edits = vec![(line, String::new())];
    let short = short_name(row.old);
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Ident
            || (first..=last).contains(&index)
            || file.span_text(token.span) != Some(short)
        {
            continue;
        }
        let before = index.checked_sub(1).map(|i| tokens[i].kind);
        let after = tokens.get(index + 1).map(|token| token.kind);
        let not_a_reference = matches!(
            before,
            Some(
                TokenKind::DoubleColon
                    | TokenKind::Arrow
                    | TokenKind::NullsafeArrow
                    | TokenKind::Backslash
                    | TokenKind::Keyword(
                        Keyword::Function
                            | Keyword::Const
                            | Keyword::Case
                            | Keyword::Class
                            | Keyword::Enum
                            | Keyword::Interface
                    )
            )
        ) || after == Some(TokenKind::Backslash);
        if !not_a_reference {
            edits.push((token.span, row.new.to_owned()));
        }
    }
    Some(edits)
}
