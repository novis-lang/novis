//! A doc comment's tags, written back in one order.
//!
//! `rule:tooling/fmt-orders-the-doc-tags` puts a `///` run in one layout: the
//! prose, one empty `///` line, then the tags with no empty line between them,
//! `@param` in the order the signature lists the parameters, then `@return`,
//! `@throws`, `@see` and `@example`. Within each of the last three kinds the
//! written order stays, because `nvs meta --json` reads it back. An `@param`
//! that names no parameter goes after the ones that do, in written order. A
//! comment with no prose starts with its first tag.
//!
//! **What moves is a whole line.** A tag moves with its continuation lines, and
//! no line's text changes. Only the order of the lines and the empty lines
//! between them change. A comment with a prose line after its first tag that
//! is not a tag's continuation is left as written: a line there belongs to no
//! tag, and moving prose is not this rule's to do.
//!
//! The signature comes from the tree alone: the parameter list of the method
//! the comment is attached to, by name. Nothing is resolved, so this stays a
//! syntactic pass, and order is never a diagnostic
//! (`rule:tooling/fmt-is-never-a-diagnostic`). A comment on a member of an
//! anonymous class is not reached, because the walk reads declarations and an
//! anonymous class is an expression.
//!
//! As with [`crate::imports`], the moves span trivia, so [`ordered`] writes the
//! whole text and [`crate::format()`] parses it again before it prints.
//!
//! # What it spends
//!
//! Nothing for a file whose comments are already in order. A file with one out
//! of order costs one more copy of its text and one more parse, both dropped
//! once it is printed.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::Parsed;
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, DocComment, DocTagKind, EnumDecl, Stmt, StmtKind,
};

/// Whether `diagnostic` is a doc tag error the parse raises, which leaves the
/// tree whole, so the file carrying it is formatted rather than refused.
///
/// A quick fix inserts an `@param` line with no text yet, and format-on-save
/// must not refuse the file that fix just wrote.
pub(crate) fn lets_through(diagnostic: &Diagnostic) -> bool {
    [
        code::E_DOC_COMMENT_UNKNOWN_TAG,
        code::E_DOC_PARAM_NAME_FIRST,
        code::E_DOC_TAG_HAS_NO_TEXT,
    ]
    .into_iter()
    .any(|known| diagnostic.code == Some(known))
}

/// `text` with every doc comment's lines in the rule's order, or [`None`]
/// when every comment already is.
///
/// `text` must be `parsed`'s own file: every span is an offset into it.
pub(crate) fn ordered(parsed: &Parsed, text: &str) -> Option<String> {
    let mut edits = Vec::new();
    push_stmts(&mut edits, &parsed.stmts, text);
    if edits.is_empty() {
        return None;
    }
    edits.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(text.len() + 16);
    let mut cursor = 0;
    for (start, end, written) in edits {
        out.push_str(&text[cursor..start]);
        out.push_str(&written);
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

/// One comment's replacement: its first byte, its end, and its new text.
type Edit = (usize, usize, String);

fn push_stmts(edits: &mut Vec<Edit>, stmts: &[Stmt], text: &str) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => {
                if let Some(body) = &decl.body {
                    push_stmts(edits, &body.stmts, text);
                }
            }
            StmtKind::ClassDecl(decl) => {
                push(edits, decl.doc.as_ref(), &[], text);
                push_members(edits, &decl.members, text);
            }
            StmtKind::InterfaceDecl(decl) => {
                push(edits, decl.doc.as_ref(), &[], text);
                push_members(edits, &decl.members, text);
            }
            StmtKind::EnumDecl(decl) => push_enum(edits, decl, text),
            StmtKind::TypeAliasDecl(decl) => push(edits, decl.doc.as_ref(), &[], text),
            _ => {}
        }
    }
}

fn push_enum(edits: &mut Vec<Edit>, decl: &EnumDecl, text: &str) {
    push(edits, decl.doc.as_ref(), &[], text);
    for case in &decl.cases {
        push(edits, case.doc.as_ref(), &[], text);
    }
    push_members(edits, &decl.members, text);
}

fn push_members(edits: &mut Vec<Edit>, members: &[ClassMember], text: &str) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(method) => {
                let names: Vec<&str> = method
                    .params
                    .iter()
                    .filter_map(|param| text.get(param.name.range()))
                    .map(|name| name.strip_prefix('$').unwrap_or(name))
                    .collect();
                push(edits, member.doc.as_ref(), &names, text);
            }
            ClassMemberKind::Enum(decl) => {
                push(edits, member.doc.as_ref(), &[], text);
                push_enum(edits, decl, text);
            }
            _ => push(edits, member.doc.as_ref(), &[], text),
        }
    }
}

/// Adds `doc`'s edit, which is nothing when it is already in order or when a
/// prose line or a `/// TODO:` line follows its first tag.
fn push(edits: &mut Vec<Edit>, doc: Option<&DocComment>, params: &[&str], text: &str) {
    let Some(doc) = doc else {
        return;
    };
    if doc.tags.is_empty() || doc.lines.len() < 2 {
        return;
    }
    let line = |at: usize| &text[doc.lines[at].range()];

    // The prose is every line above the first tag, without the blank lines
    // that end it.
    let first_tag = doc.tags[0].span.start;
    let above = doc
        .lines
        .iter()
        .take_while(|span| span.end < first_tag)
        .count();
    let mut prose: Vec<&str> = (0..above).map(line).collect();
    while prose.last().is_some_and(|written| blank(written)) {
        prose.pop();
    }

    // Each tag's lines, and no line after the first tag that belongs to none.
    let mut groups: Vec<(usize, usize, Vec<&str>)> = Vec::new();
    for at in above..doc.lines.len() {
        if let Some(tag) = doc.tag_of_line(doc.lines[at]) {
            let index = doc
                .tags
                .iter()
                .position(|t| std::ptr::eq(t, tag))
                .unwrap_or(0);
            match groups.last_mut() {
                Some((owner, _, lines)) if *owner == index => lines.push(line(at)),
                _ => groups.push((index, rank(doc, index, params, text), vec![line(at)])),
            }
        } else if !blank(line(at)) {
            return;
        }
    }
    groups.sort_by_key(|(at, rank, _)| (*rank, *at));

    let separator = &text[doc.lines[0].end as usize..doc.lines[1].start as usize];
    let mut lines: Vec<&str> = prose;
    if !lines.is_empty() {
        lines.push(nvs_syntax::DOC_MARKER);
    }
    lines.extend(groups.into_iter().flat_map(|(_, _, lines)| lines));
    let written = lines.join(separator);
    let (start, end) = (doc.span.start as usize, doc.span.end as usize);
    if text[start..end] != written {
        edits.push((start, end, written));
    }
}

/// Whether a `///` line has nothing after its marker.
fn blank(written: &str) -> bool {
    let written = written.trim();
    written
        .strip_prefix(nvs_syntax::DOC_MARKER)
        .unwrap_or(written)
        .trim()
        .is_empty()
}

/// Where tag `index` goes: its kind first, then, for an `@param`, its
/// parameter's place in the signature. An unknown parameter goes after every
/// known one, and a sort that keeps written order breaks every tie.
fn rank(doc: &DocComment, index: usize, params: &[&str], text: &str) -> usize {
    let tag = &doc.tags[index];
    let kind = match tag.kind {
        DocTagKind::Param => 0,
        DocTagKind::Return => 1,
        DocTagKind::Throws => 2,
        DocTagKind::See => 3,
        DocTagKind::Example => 4,
    };
    let place = match tag.kind {
        DocTagKind::Param => tag
            .target
            .and_then(|target| text.get(target.range()))
            .map(|name| name.strip_prefix('$').unwrap_or(name))
            .and_then(|name| params.iter().position(|param| *param == name))
            .unwrap_or(params.len()),
        _ => 0,
    };
    kind * (params.len() + 1) + place
}
