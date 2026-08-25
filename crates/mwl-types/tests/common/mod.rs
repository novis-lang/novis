//! Fixtures shared by `mwl-types`'s checker tests.
//!
//! Each `tests/*.rs` file is its own binary, so the four helpers these tests
//! were written against live here and are reached through `mod common;`. They
//! moved out of `check.rs`'s inline `mod tests` unchanged except for becoming
//! `pub` — nothing about what they do is new.

#![allow(
    dead_code,
    reason = "each test binary compiles all of this module but uses only the \
              helpers its own area needs; which ones those are differs per file"
)]

use mwl_diagnostics::{Diagnostics, SourceMap};
use mwl_hir::resolve_file;
use mwl_syntax::parse_file;
use mwl_types::check::check_program;
use mwl_types::expr_table::ExprTypeTable;
use mwl_types::ty::TypeInterner;

/// Wraps `body` inside `class T { function m(): void { ... } }` and
/// checks it — the common shape for a definite-assignment/expression
/// fixture that doesn't need its own class.
pub(crate) fn check_in_method(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?mwl\nclass T {{\n  function m(): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// Like [`check_src`], but hands back the typed-expression table too —
/// what a closure fixture asserts on, since ADR 0031 § 4's `callable`
/// carries none of what was resolved.
pub(crate) fn check_src_table(src: &str) -> (Diagnostics, ExprTypeTable) {
    let mut map = SourceMap::new();
    let file = map.add("t.mwl", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    (diags, exprs)
}

/// Like [`check_src_table`], but hands back the interner too, and a way to name
/// a written annotation's span — what a fixture asserting *which type an atom
/// interned to* needs, since a `TypeId` means nothing without the interner that
/// issued it.
///
/// The span is found by locating the annotation's own source text, which is
/// what [`mwl_types::lower::lower_type`] keys its record by
/// (`ExprTypeTable::declared_ty`). Written out rather than counted, so a
/// fixture says which annotation it means.
pub(crate) fn check_src_declared(src: &str) -> (Diagnostics, DeclaredTypes) {
    let mut map = SourceMap::new();
    let file = map.add("t.mwl", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    (
        diags,
        DeclaredTypes {
            src: src.to_owned(),
            file,
            exprs,
            interner,
        },
    )
}

/// What [`check_src_declared`] hands back: every annotation the run interned,
/// reachable by the text it was written as.
pub(crate) struct DeclaredTypes {
    src: String,
    file: mwl_diagnostics::SourceId,
    exprs: ExprTypeTable,
    pub interner: TypeInterner,
}

impl DeclaredTypes {
    /// The `TypeId` the annotation `ty`, written on the binding `var`,
    /// interned to — `of("Mode", "$whole")`.
    ///
    /// Located by the pair rather than by `ty` alone, because a type's own
    /// text is rarely unique in a fixture (`Mode` appears in the `enum` header
    /// too) while `Mode $whole` is.
    ///
    /// # Panics
    /// Panics if `ty var` appears nowhere in the fixture, or if nothing was
    /// recorded at `ty`'s span — both mean the fixture and the assertion have
    /// drifted apart, which is worth failing loudly for.
    pub(crate) fn of(&self, ty: &str, var: &str) -> mwl_types::ty::TypeId {
        let needle = format!("{ty} {var}");
        let start = self
            .src
            .find(&needle)
            .unwrap_or_else(|| panic!("the fixture does not contain `{needle}`"));
        let start = u32::try_from(start).expect("fixtures are small");
        let span = mwl_diagnostics::Span::new(
            self.file,
            start,
            start + u32::try_from(ty.len()).expect("fixtures are small"),
        );
        self.exprs
            .declared_ty(span)
            .unwrap_or_else(|| panic!("no type was recorded for the annotation `{needle}`"))
    }
}

/// The capture names a fixture's one closure recorded, in order.
pub(crate) fn captures_of(src: &str) -> Vec<String> {
    let (diags, exprs) = check_src_table(src);
    assert!(!diags.has_errors(), "{diags:?}");
    exprs
        .closures()
        .next()
        .expect("the fixture declares one closure")
        .1
        .iter()
        .map(|(name, _)| name.clone())
        .collect()
}

pub(crate) fn check_src(src: &str) -> Diagnostics {
    let mut map = SourceMap::new();
    let file = map.add("t.mwl", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    diags
}
