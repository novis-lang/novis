//! The `use` block: consecutive imports, written back in path order.
//!
//! `rule:tooling/fmt-sorts-the-use-block` sorts a run of `use` declarations
//! lexicographically by its full path, ascending and case-sensitive, and that
//! is the only reordering `nvs fmt` performs anywhere — an import's position is
//! not observable, and a class member's is
//! (`rule:tooling/fmt-never-reorders-members`). There is no grouped
//! `use A\{B, C};` form in Novis, so a declaration names one path and the sort
//! key is that path's own text.
//!
//! **What moves is the import and its comments.** An import is the
//! declaration, every comment written directly above it on lines of their own
//! with no blank line between, and a comment after its `;` on the same line.
//! The comment describes the import, so it goes wherever the import goes. A
//! run is a permutation of those elements: each one goes out at the place one
//! of the run's own elements was written, and the whitespace between two places
//! stays where it was.
//!
//! An element spans trivia, and the printer applies each edit inside one code
//! run ([`crate::print::one_code_run`]), so the sort is not an edit the printer
//! makes. [`sorted`] writes the file's text with every run in order, and
//! [`crate::format()`] parses that text again and prints it. Every other rule
//! then places the moved lines exactly as it places lines the author wrote,
//! comment lines included.
//!
//! Three things end a run, and a run of one is never sorted:
//!
//! - **Any other statement**, which is what makes the block a block. The
//!   statements come from [`nvs_syntax::Parsed`] rather than from a scan, so an
//!   import written inside a bracketed `namespace` is one this reaches and a
//!   `use` written inside a string literal is not.
//! - **A comment that belongs to no import**: one with a blank line between it
//!   and the import below. It describes the imports around it, or none, so the
//!   imports on each side of it are sorted apart.
//! - **A path this cannot sort by**, which is one with a comment inside it,
//!   and an `as Alias` — an import cannot be renamed
//!   (`rule:statements/nothing-gets-a-second-name`), so a declaration carrying
//!   one is a file on its way to a diagnostic and its bytes are left as its
//!   author typed them.
//!
//! # What it spends
//!
//! Nothing for a file whose runs are already in order. A file with a run out
//! of order costs one more copy of its text and one more parse, both dropped
//! once it is printed.

use nvs_syntax::ast::{Stmt, StmtKind, UseDecl};
use nvs_syntax::{Parsed, Trivia, TriviaKind};

use crate::print;

/// `text` with every run of imports in path order, or [`None`] when every run
/// already is.
///
/// `text` must be `parsed`'s own file: every span is an offset into it.
pub(crate) fn sorted(parsed: &Parsed, text: &str) -> Option<String> {
    let mut moves = Vec::new();
    push_block(&mut moves, &parsed.stmts, &parsed.trivia, text);
    if moves.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (place, belongs) in moves {
        out.push_str(&text[cursor..place.start]);
        out.push_str(&text[belongs.start..belongs.end]);
        cursor = place.end;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

/// The bytes one import covers: the comments it carries and its declaration.
#[derive(Clone, Copy)]
struct Element<'a> {
    start: usize,
    end: usize,
    decl: &'a UseDecl,
}

/// Adds what each run of imports in `stmts` owes, and descends into the
/// namespace bodies that hold runs of their own.
fn push_block<'a>(
    moves: &mut Vec<(Element<'a>, Element<'a>)>,
    stmts: &'a [Stmt],
    trivia: &[Trivia],
    text: &str,
) {
    let mut run: Vec<Element<'a>> = Vec::new();
    for stmt in stmts {
        if let StmtKind::UseDecl(decl) = &stmt.kind {
            let element = element(trivia, text, decl);
            let joined = run
                .last()
                .is_some_and(|above| text[above.end..element.start].trim().is_empty());
            if !joined || !sortable(trivia, decl) {
                push_run(moves, &run, text);
                run.clear();
            }
            if sortable(trivia, decl) {
                run.push(element);
            }
            continue;
        }
        push_run(moves, &run, text);
        run.clear();
        if let StmtKind::NamespaceDecl(decl) = &stmt.kind
            && let Some(body) = &decl.body
        {
            push_block(moves, &body.stmts, trivia, text);
        }
    }
    push_run(moves, &run, text);
}

/// Adds what one run owes, which is nothing when it is already in order.
fn push_run<'a>(moves: &mut Vec<(Element<'a>, Element<'a>)>, run: &[Element<'a>], text: &str) {
    if run.len() < 2 {
        return;
    }
    let mut sorted = run.to_vec();
    sorted.sort_by(|left, right| path(text, left.decl).cmp(path(text, right.decl)));
    if run
        .iter()
        .zip(&sorted)
        .all(|(place, belongs)| place.start == belongs.start)
    {
        return;
    }
    moves.extend(run.iter().copied().zip(sorted));
}

/// `decl` and the comments that belong to it.
///
/// A comment above belongs to it when it opens its own line and only
/// whitespace without a blank line separates it from the declaration or from
/// another comment that belongs to it. A comment after the `;` belongs to it
/// when no line break comes first.
fn element<'a>(trivia: &[Trivia], text: &str, decl: &'a UseDecl) -> Element<'a> {
    let mut start = decl.span.start as usize;
    let mut before = trivia.partition_point(|trivium| (trivium.span.end as usize) <= start);
    while before >= 2 {
        let (gap, comment) = (trivia[before - 1], trivia[before - 2]);
        let gap_text = &text[gap.span.start as usize..gap.span.end as usize];
        let ends_where_it_starts = gap.span.end as usize == start;
        if !ends_where_it_starts
            || gap.kind != TriviaKind::Whitespace
            || comment.kind == TriviaKind::Whitespace
            || gap_text.matches('\n').count() > 1
            || !opens_its_line(text, comment.span.start as usize)
        {
            break;
        }
        start = comment.span.start as usize;
        before -= 2;
    }
    let mut end = decl.span.end as usize;
    let mut after = trivia.partition_point(|trivium| (trivium.span.start as usize) < end);
    if let Some(gap) = trivia.get(after).filter(|gap| {
        gap.span.start as usize == end
            && gap.kind == TriviaKind::Whitespace
            && !text[gap.span.start as usize..gap.span.end as usize].contains('\n')
    }) {
        end = gap.span.end as usize;
        after += 1;
    }
    match trivia.get(after) {
        Some(comment)
            if comment.span.start as usize == end && comment.kind != TriviaKind::Whitespace =>
        {
            end = comment.span.end as usize;
        }
        _ => end = decl.span.end as usize,
    }
    Element { start, end, decl }
}

/// Whether only spaces and tabs come between the start of its line and
/// `offset`.
fn opens_its_line(text: &str, offset: usize) -> bool {
    let line_start = text[..offset].rfind('\n').map_or(0, |brk| brk + 1);
    text[line_start..offset]
        .trim_start_matches([' ', '\t'])
        .is_empty()
}

/// One declaration's sort key: the full path, backslashes and all.
fn path<'t>(text: &'t str, decl: &UseDecl) -> &'t str {
    &text[decl.path.span.start as usize..decl.path.span.end as usize]
}

/// Whether `decl`'s path is one run of code the sort can read whole.
fn sortable(trivia: &[Trivia], decl: &UseDecl) -> bool {
    decl.alias.is_none() && print::one_code_run(trivia, decl.path.span)
}
