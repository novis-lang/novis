//! *Write the type every caller passes*, the action that rewrites a parameter
//! declared bare `callable` or `mixed` to the one type every call of its
//! method passes there
//! (`rule:ide/a-code-action-writes-only-what-is-already-determined`).
//!
//! It is offered with the cursor on the parameter's type, under
//! `refactor.rewrite`, and never under `quickfix` or `source.fixAll.nvs`. No
//! diagnostic is behind it, because a deliberate `mixed` is not wrong
//! (`rule:ide/narrow-an-annotation-to-its-literal` declines the hint for the
//! same reason). The edit replaces the type and nothing else.
//!
//! # Decision: the callers are the index's, and each type is the checker's
//!
//! The reference index lists every call of the method
//! ([`SymbolIndex::occurrences`]). The checker records, at each call, the
//! type of the argument it passes to a bare `callable` or `mixed` parameter
//! ([`nvs_types::expr_table::ExprTypeTable::bare_argument`]). So this module
//! types nothing itself: it analyses each caller's file and reads the
//! record. A call with no record, such as one that leaves the parameter to
//! its default, ends the offer.
//!
//! # Decision: nothing is offered unless every caller is known
//!
//! The action is offered only when the index has no entry waiting, the method
//! is taken as a callable value nowhere ([`SymbolIndex::unfollowed`]), it
//! overrides nothing and nothing overrides it, and it has at least one call.
//! A wider parameter in an override would stop compiling, and a call through a
//! callable value passes a type nobody recorded. A variadic or `inout`
//! parameter is not offered.
//!
//! # Decision: the edit is checked
//!
//! The document with the edit applied must report exactly the diagnostics it
//! reports now, as [`crate::narrow`]'s does. A narrower type can refuse a
//! write to the parameter in the method's own body.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. Per
//! `textDocument/codeAction` with the cursor on such a type, one analysis of
//! each file that calls the method and one of the edited document, dropped
//! with the answer. The checker keeps one type for each argument at such a
//! parameter, for the compile.

use std::path::{Path, PathBuf};

use nvs_diagnostics::{BytePos, PositionEncoding, Span, canonical_key};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, MethodMember, Param, Stmt, StmtKind};
use nvs_syntax::walk;
use nvs_types::Ty;
use rustc_hash::FxHashMap;

use crate::document::{Analysed, Documents, analyse_file, analyse_rewritten};
use crate::imports::{entry_stmts, written_at};
use crate::index::SymbolIndex;
use crate::position::range_at;
use crate::render::Action;

/// The kind the action is filed under, which [`crate::split_join`] declares.
pub const KIND: &str = crate::split_join::KIND;

/// Whether the cursor at `offset` is on the type of a parameter declared bare
/// `callable` or `mixed`, which is when [`at`] needs an index. The `.lspt`
/// runner builds one only then.
#[must_use]
pub fn wanted(analysed: &Analysed, offset: BytePos) -> bool {
    parameter_at(analysed, offset).is_some()
}

/// The action at `offset` of the entry document, if one is offered.
#[must_use]
pub fn at(
    analysed: &Analysed,
    documents: &Documents,
    index: &SymbolIndex,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Action> {
    let (method, position, param) = parameter_at(analysed, offset)?;
    let ty = param.ty.as_ref()?;
    if !index.pending().is_empty() {
        return None;
    }
    let (symbol, _) = crate::index::declared_at(analysed, method.name.start)?;
    if index.unfollowed(&symbol)
        || !index.overriders(&symbol).is_empty()
        || !index.overridden(&symbol).is_empty()
    {
        return None;
    }
    let calls = index.occurrences(&symbol);
    if calls.is_empty() {
        return None;
    }

    let file = analysed.map.file(analysed.entry);
    let here = canonical_key(file.path()?);
    let mut others: FxHashMap<PathBuf, Option<Analysed>> = FxHashMap::default();
    let mut passed: Option<String> = None;
    for call in calls {
        let typed = if call.site.path == here {
            passed_at(
                analysed,
                &call.site.path,
                call.site.start,
                call.site.end,
                position,
            )
        } else {
            others
                .entry(call.site.path.clone())
                .or_insert_with(|| analyse_file(documents, &call.site.path, 0))
                .as_ref()
                .and_then(|caller| {
                    passed_at(
                        caller,
                        &call.site.path,
                        call.site.start,
                        call.site.end,
                        position,
                    )
                })
        }?;
        match &passed {
            Some(seen) if *seen != typed => return None,
            Some(_) => {}
            None => passed = Some(typed),
        }
    }
    let written = written_at(analysed, &passed?, None, ty.span.start);

    let mut rewritten = file.text().to_owned();
    rewritten.replace_range(ty.span.range(), &written);
    let checked = analyse_rewritten(documents, analysed, rewritten)?;
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
        title: format!("Write the type every caller passes: `{written}`"),
        kind: KIND.to_owned(),
        range: range_at(file, ty.span, encoding),
        replacement: written,
        also: Vec::new(),
    })
}

/// The type the call whose method name is written at `start..end` of `path`
/// passes to parameter `position`, as an annotation writes it.
fn passed_at(
    analysed: &Analysed,
    path: &Path,
    start: BytePos,
    end: BytePos,
    position: usize,
) -> Option<String> {
    let loaded = analysed.loaded.iter().find(|loaded| {
        analysed
            .map
            .file(loaded.id)
            .path()
            .is_some_and(|at| canonical_key(at) == path)
    })?;
    let call = walk::of_stmts(&loaded.stmts).into_iter().find_map(|root| {
        std::iter::once(&root)
            .chain(root.descendants())
            .find(|node| {
                let name = crate::index::named(node);
                name.file == loaded.id && name.start == start && name.end == end
            })
            .map(|node| node.span)
    })?;
    let ty = analysed.exprs.bare_argument(call, position)?;
    nvs_types::narrow::parameter(&analysed.interner, ty)
}

/// The method and the parameter whose type `offset` is on, with the
/// parameter's position, where that type is bare `callable` or `mixed` and
/// the parameter is neither variadic nor `inout`.
fn parameter_at(analysed: &Analysed, offset: BytePos) -> Option<(&MethodMember, usize, &Param)> {
    let found = stmts_at(entry_stmts(analysed)?, offset)?;
    let (_, _, param) = found;
    let ty = param.ty.as_ref()?;
    if param.variadic || param.inout || !covers(ty.span, offset) {
        return None;
    }
    let declared = analysed.exprs.declared_ty(ty.span)?;
    matches!(analysed.interner.get(declared), Ty::Callable | Ty::Mixed).then_some(found)
}

/// Whether `offset` is inside `span` or touches either end of it.
fn covers(span: Span, offset: BytePos) -> bool {
    span.start <= offset && offset <= span.end
}

/// The method parameter whose type `offset` is on, among the classes `stmts`
/// declare, inside a namespace's block or not.
fn stmts_at(stmts: &[Stmt], offset: BytePos) -> Option<(&MethodMember, usize, &Param)> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::ClassDecl(decl) => members_at(&decl.members, offset),
        StmtKind::EnumDecl(decl) => members_at(&decl.members, offset),
        StmtKind::NamespaceDecl(decl) => decl
            .body
            .as_ref()
            .and_then(|block| stmts_at(&block.stmts, offset)),
        _ => None,
    })
}

/// [`stmts_at`] for the methods of one class or enum that have a body.
fn members_at(members: &[ClassMember], offset: BytePos) -> Option<(&MethodMember, usize, &Param)> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) if method.body.is_some() => method
            .params
            .iter()
            .enumerate()
            .find(|(_, param)| param.ty.as_ref().is_some_and(|ty| covers(ty.span, offset)))
            .map(|(position, param)| (method, position, param)),
        _ => None,
    })
}
