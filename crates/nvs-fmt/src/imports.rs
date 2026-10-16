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
//! **What moves is the path, not the declaration.** A run is a permutation of
//! itself, which is [`crate::modifiers`]'s shape one step larger: each path
//! goes out at a place one of the run's own paths was written, and everything
//! else in the run stays exactly where it was put. That is not an
//! optimization — the printer writes a file as trivia and the code runs
//! between them, so an edit has to lie inside one run
//! ([`print::one_code_run`]), and `use Core\Str;` is two of them either side of
//! the space after the keyword. Its path is one, and it is also the only thing
//! that differs between two declarations: the keyword, the spacing and the
//! semicolon are the same bytes in every one.
//!
//! Three things end a run, and a run of one is never sorted:
//!
//! - **Any other statement**, which is what makes the block a block. The
//!   statements come from [`nvs_syntax::Parsed`] rather than from a scan, so an
//!   import written inside a bracketed `namespace` is one this reaches and a
//!   `use` written inside a string literal is not.
//! - **A comment between two of them.** A comment is content the formatter has
//!   no opinion about (`rule:tooling/fmt-quotes`) and it is not part of the
//!   declaration below it, so sorting across one would leave it describing an
//!   import it was not written for. Keeping the run on this side of it is the
//!   direction that cannot be wrong, and it is a known gap in
//!   [`the crate's own doc`](crate) rather than a rule: what closes it is a
//!   comment that belongs to the declaration it precedes.
//! - **A path this cannot move whole**, which is one with a comment inside it,
//!   and an `as Alias` — an import cannot be renamed
//!   (`rule:statements/nothing-gets-a-second-name`), so a declaration carrying
//!   one is a file on its way to a diagnostic and its bytes are left as its
//!   author typed them.

use nvs_diagnostics::BytePos;
use nvs_syntax::ast::{Stmt, StmtKind, UseDecl};
use nvs_syntax::{Parsed, Trivia, TriviaKind};

use crate::print::{self, Rewrite};

/// Every import path `parsed` writes somewhere other than where it was
/// written, in source order and covering no byte twice.
///
/// `text` must be `parsed`'s own file: every span is an offset into it.
pub(crate) fn rewrites<'t>(parsed: &Parsed, text: &'t str) -> Vec<Rewrite<'t>> {
    let mut out = Vec::new();
    push_block(&mut out, &parsed.stmts, &parsed.trivia, text);
    out
}

/// Adds what each run of imports in `stmts` owes, and descends into the
/// namespace bodies that hold runs of their own.
fn push_block<'t>(out: &mut Vec<Rewrite<'t>>, stmts: &[Stmt], trivia: &[Trivia], text: &'t str) {
    let mut run: Vec<&UseDecl> = Vec::new();
    for stmt in stmts {
        if let StmtKind::UseDecl(decl) = &stmt.kind {
            let broken = run
                .last()
                .is_some_and(|above| commented(trivia, above.span.end, decl.span.start));
            if broken || !sortable(trivia, decl) {
                push_run(out, &run, text);
                run.clear();
            }
            if sortable(trivia, decl) {
                run.push(decl);
            }
            continue;
        }
        push_run(out, &run, text);
        run.clear();
        if let StmtKind::NamespaceDecl(decl) = &stmt.kind
            && let Some(body) = &decl.body
        {
            push_block(out, &body.stmts, trivia, text);
        }
    }
    push_run(out, &run, text);
}

/// Adds what one run owes, which is nothing when it is already in order.
fn push_run<'t>(out: &mut Vec<Rewrite<'t>>, run: &[&UseDecl], text: &'t str) {
    if run.len() < 2 {
        return;
    }
    let mut sorted = run.to_vec();
    sorted.sort_by(|left, right| path(text, left).cmp(path(text, right)));
    for (place, belongs) in run.iter().zip(sorted) {
        if belongs.path.span.start == place.path.span.start {
            continue;
        }
        out.push(Rewrite {
            start: place.path.span.start as usize,
            end: place.path.span.end as usize,
            written: path(text, belongs),
        });
    }
}

/// One declaration's sort key, and the bytes that go out at another's place:
/// the full path, backslashes and all.
fn path<'t>(text: &'t str, decl: &UseDecl) -> &'t str {
    &text[decl.path.span.start as usize..decl.path.span.end as usize]
}

/// Whether `decl`'s path is bytes the printer can write somewhere else.
fn sortable(trivia: &[Trivia], decl: &UseDecl) -> bool {
    decl.alias.is_none() && print::one_code_run(trivia, decl.path.span)
}

/// Whether anything but whitespace was written between `from` and `to`, which
/// are the end of one import and the start of the next.
fn commented(trivia: &[Trivia], from: BytePos, to: BytePos) -> bool {
    let first = trivia.partition_point(|trivium| trivium.span.start < from);
    trivia[first..]
        .iter()
        .take_while(|trivium| trivium.span.start < to)
        .any(|trivium| trivium.kind != TriviaKind::Whitespace)
}
