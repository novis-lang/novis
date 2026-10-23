//! The blank lines PER writes between declarations.
//!
//! `rule:tooling/fmt-base-style-is-per` puts exactly one blank line after a
//! `namespace` line, one after a run of imports, and one between two members of
//! a class, an interface or an enum that each have a body. Each of them is the
//! run that ends the line a declaration finishes on, so what this answers is the
//! question [`crate::statement`] answers — "what does the run before this byte
//! have to be" — with two line breaks in place of one.
//!
//! The run is the first one after the declaration that carries a line break,
//! which puts a comment that ends the declaration's own line above the blank
//! line, and a comment that opens a line of its own below it: a comment between
//! two members belongs to the member below it. What lies between that comment
//! and its member, and every blank line between two other members, is the
//! author's.
//!
//! # What it spends
//!
//! One offset and one short string per blank line it writes, kept for the file.
//! Nothing here runs on the request path.

use nvs_syntax::ast::{ClassMember, ClassMemberKind, Stmt, StmtKind};
use nvs_syntax::{Parsed, Trivia, TriviaKind};

use crate::indent::Indent;

/// Everything this rule requires of `indent`'s file: the run that has to
/// precede an offset, by that offset. `parsed` must be the same file's parse.
pub(crate) fn runs(parsed: &Parsed, indent: &Indent<'_>) -> Vec<(usize, String)> {
    let (_, text, trivia) = indent.parse();
    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut ends = Vec::new();
    push_stmts(&mut ends, &parsed.stmts);
    ends.into_iter()
        .filter_map(|end| {
            let at = line_after(trivia, text, end)?;
            let opening = indent.of_line(at)?;
            Some((at, format!("{line_break}{line_break}{opening}")))
        })
        .collect()
}

/// Adds the end of each declaration in `stmts` that a blank line follows, and
/// descends into each body that holds more of them.
fn push_stmts(ends: &mut Vec<usize>, stmts: &[Stmt]) {
    for (at, stmt) in stmts.iter().enumerate() {
        let next = stmts.get(at + 1);
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => match &decl.body {
                Some(body) => push_stmts(ends, &body.stmts),
                None if next.is_some() => ends.push(stmt.span.end as usize),
                None => {}
            },
            StmtKind::UseDecl(_)
                if next.is_some_and(|next| !matches!(next.kind, StmtKind::UseDecl(_))) =>
            {
                ends.push(stmt.span.end as usize);
            }
            StmtKind::ClassDecl(decl) => push_members(ends, &decl.members),
            StmtKind::InterfaceDecl(decl) => push_members(ends, &decl.members),
            StmtKind::EnumDecl(decl) => push_members(ends, &decl.members),
            _ => {}
        }
    }
}

/// Adds the end of each member of one body that is followed by another member,
/// when both have a body, and descends into an enum the body declares.
fn push_members(ends: &mut Vec<usize>, members: &[ClassMember]) {
    for pair in members.windows(2) {
        if has_a_body(&pair[0]) && has_a_body(&pair[1]) {
            ends.push(pair[0].span.end as usize);
        }
    }
    for member in members {
        if let ClassMemberKind::Enum(decl) = &member.kind {
            push_members(ends, &decl.members);
        }
    }
}

/// Whether `member` has a body: a method with a block, a property with hooks,
/// or an enum.
fn has_a_body(member: &ClassMember) -> bool {
    match &member.kind {
        ClassMemberKind::Method(method) => method.body.is_some(),
        ClassMemberKind::Property(property) => property.hooks.is_some(),
        ClassMemberKind::Enum(_) => true,
        _ => false,
    }
}

/// Where the first whitespace run with a line break after `end` stops, when
/// only trivia lies between `end` and that run and code follows it.
///
/// Code on the same line means [`crate::statement`] breaks that line, and this
/// rule leaves it alone. The run that ends the file is the end-of-file rule's.
fn line_after(trivia: &[Trivia], text: &str, end: usize) -> Option<usize> {
    let mut at = end;
    let first = trivia.partition_point(|trivium| (trivium.span.start as usize) < at);
    for trivium in &trivia[first..] {
        let start = trivium.span.start as usize;
        let stop = trivium.span.end as usize;
        if start != at {
            return None;
        }
        if trivium.kind == TriviaKind::Whitespace && text[start..stop].contains('\n') {
            return (stop < text.len()).then_some(stop);
        }
        at = stop;
    }
    None
}
