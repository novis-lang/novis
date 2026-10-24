//! *Narrow the type*, the action that rewrites an `array<mixed>` annotation to
//! the type of the array literal that initializes it
//! (`rule:ide/narrow-an-annotation-to-its-literal`).
//!
//! It is offered with the cursor on the annotation of a local declaration
//! whose initializer is an array literal, as in `array<mixed> $rows = [...];`.
//! No diagnostic is behind it, because nothing is wrong. The edit replaces the
//! annotation and nothing else.
//!
//! # Decision: the type is the one `var` would find
//!
//! The document is analysed once more with `var` in place of the annotation,
//! and the type the checker recorded for the literal there
//! ([`nvs_types::expr_table::ExprTypeTable::literal_type`]) is the answer.
//! That is the synthesis `var` runs, so an element that is a variable or a
//! call has the type the checker holds for it, and nothing here types an
//! expression a second time (`rule:ide/no-compile-path-calls-the-synthesis`).
//! [`nvs_types::narrow::narrowest`] then widens it and declines what an
//! annotation cannot state. A literal whose elements are `mixed`, an empty
//! one and an initializer that is not a literal get nothing
//! (`rule:ide/the-action-answers-from-the-literal-or-not-at-all`).
//!
//! # Decision: the edit is checked, and is never a fix-all
//!
//! The document with the edit applied is analysed before the action is
//! offered, and it must report exactly the diagnostics the document reports
//! now. A narrower annotation refuses a write the wider one allowed, and that
//! write is on a line the action does not touch. So an edit that would add an
//! error is not offered. The action is filed under `refactor.rewrite` and is
//! never part of `source.fixAll.nvs`
//! (`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`).
//!
//! A declaration inside a closure, a property's default and a parameter's are
//! not offered: only a statement can be rewritten to `var`.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. Per
//! `textDocument/codeAction` with the cursor on such an annotation, two
//! analyses of the entry's graph, dropped with the answer.

use nvs_diagnostics::{BytePos, PositionEncoding, Span};
use nvs_syntax::ast::{
    Block, ClassMember, ClassMemberKind, ExprKind, ForInit, Stmt, StmtKind, Type,
};
use nvs_types::Ty;

use crate::document::{Analysed, Documents, analyse_rewritten};
use crate::imports::{entry_stmts, written_at};
use crate::position::range_at;
use crate::render::Action;

/// The kind the action is filed under, which [`crate::split_join`] declares.
pub const KIND: &str = crate::split_join::KIND;

/// What the annotation is replaced with to ask the checker for the literal's
/// type.
const INFERRED: &str = "var";

/// The narrowing action at `offset` of the entry document, if one is offered.
#[must_use]
pub fn at(
    analysed: &Analysed,
    documents: &Documents,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Action> {
    let (ty, literal) = declaration_at(entry_stmts(analysed)?, offset)?;
    let declared = analysed.exprs.declared_ty(ty.span)?;
    let Ty::Array(elem) = analysed.interner.get(declared) else {
        return None;
    };
    if !matches!(analysed.interner.get(*elem), Ty::Mixed) {
        return None;
    }
    let file = analysed.map.file(analysed.entry);
    let text = file.text();
    let replaced = |with: &str| {
        let mut rewritten = text.to_owned();
        rewritten.replace_range(ty.span.range(), with);
        rewritten
    };

    let mut probe = analyse_rewritten(documents, analysed, replaced(INFERRED))?;
    let shrunk = ty.span.end - ty.span.start - BytePos::try_from(INFERRED.len()).ok()?;
    let at = Span::new(probe.entry, literal.start - shrunk, literal.end - shrunk);
    let synthesized = probe.exprs.literal_type(at)?;
    let narrowed = nvs_types::narrow::narrowest(&mut probe.interner, synthesized)?;
    let written = written_at(
        analysed,
        &nvs_types::narrow::written(&probe.interner, narrowed),
        None,
        ty.span.start,
    );

    let checked = analyse_rewritten(documents, analysed, replaced(&written))?;
    let reported = |diags: &nvs_diagnostics::Diagnostics| {
        let mut found: Vec<(Option<nvs_diagnostics::Code>, String)> = diags
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
            .collect();
        found.sort_by(|left, right| left.1.cmp(&right.1));
        found
    };
    if reported(&checked.diags) != reported(&analysed.diags) {
        return None;
    }
    Some(Action {
        title: format!("Narrow the type to `{written}`"),
        kind: KIND.to_owned(),
        range: range_at(file, ty.span, encoding),
        replacement: written,
        also: Vec::new(),
    })
}

/// The annotation `offset` is on and the span of the array literal that
/// initializes it, for a local declaration anywhere in `stmts`.
fn declaration_at(stmts: &[Stmt], offset: BytePos) -> Option<(&Type, Span)> {
    stmts.iter().find_map(|stmt| stmt_at(stmt, offset))
}

/// [`declaration_at`] for one statement and the statements inside it.
fn stmt_at(stmt: &Stmt, offset: BytePos) -> Option<(&Type, Span)> {
    if offset < stmt.span.start || stmt.span.end < offset {
        return None;
    }
    match &stmt.kind {
        StmtKind::LocalDecl {
            ty: Some(ty),
            value: Some(value),
            ..
        } if ty.span.start <= offset && offset <= ty.span.end => {
            matches!(value.kind, ExprKind::ArrayLiteral(_)).then_some((ty, value.span))
        }
        StmtKind::Block(block) => block_at(block, offset),
        StmtKind::If { arms, else_ } => arms
            .iter()
            .find_map(|arm| stmt_at(&arm.then, offset))
            .or_else(|| else_.as_deref().and_then(|stmt| stmt_at(stmt, offset))),
        StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } => stmt_at(body, offset),
        StmtKind::For { init, body, .. } => match init {
            ForInit::Decl(decl) => stmt_at(decl, offset),
            ForInit::Exprs(_) => None,
        }
        .or_else(|| stmt_at(body, offset)),
        StmtKind::Foreach { body, .. } => stmt_at(body, offset),
        StmtKind::Switch { cases, .. } => cases
            .iter()
            .find_map(|case| declaration_at(&case.body, offset)),
        StmtKind::Try {
            body,
            catches,
            finally,
        } => block_at(body, offset)
            .or_else(|| {
                catches
                    .iter()
                    .find_map(|catch| block_at(&catch.body, offset))
            })
            .or_else(|| finally.as_ref().and_then(|block| block_at(block, offset))),
        StmtKind::ClassDecl(decl) => members_at(&decl.members, offset),
        StmtKind::InterfaceDecl(decl) => members_at(&decl.members, offset),
        StmtKind::EnumDecl(decl) => members_at(&decl.members, offset),
        StmtKind::NamespaceDecl(decl) => {
            decl.body.as_ref().and_then(|block| block_at(block, offset))
        }
        _ => None,
    }
}

/// [`declaration_at`] for a block's statements.
fn block_at(block: &Block, offset: BytePos) -> Option<(&Type, Span)> {
    declaration_at(&block.stmts, offset)
}

/// [`declaration_at`] for the method bodies of a class, an interface or an
/// enum.
fn members_at(members: &[ClassMember], offset: BytePos) -> Option<(&Type, Span)> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) => {
            method.body.as_ref().and_then(|body| block_at(body, offset))
        }
        _ => None,
    })
}
