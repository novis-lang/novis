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
        &stmts,
        map.file(file),
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    (diags, exprs)
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
        &stmts,
        map.file(file),
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    diags
}
