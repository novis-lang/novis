//! Fixtures shared by `mwl-codegen`'s end-to-end tests.
//!
//! Each `tests/*.rs` file is its own binary, so the four helpers these tests
//! were written against live here and are reached through `mod common;`. They
//! moved out of the single `compile_and_run.rs` unchanged except for becoming
//! `pub(crate)`.
//!
//! Every test that uses them goes through the *real* pipeline — `mwl-syntax`,
//! `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-codegen` — rather than hand-building
//! IR, because the property worth guarding is that the five crates agree. That
//! is also why the assertions are on output bytes and statuses rather than on
//! generated instructions: what `mwl run` prints is the observable contract,
//! and `benches/abi-probe` is where instruction-level costs are held.

#![allow(
    dead_code,
    unused_imports,
    reason = "each test binary compiles all of this module but uses only the \
              helpers and re-exports its own area needs; which ones those are \
              differs per file"
)]

use mwl_diagnostics::{Diagnostics, SourceMap};

/// The runtime surface every area's tests reach for, re-exported so a test
/// file needs one `use common::*;` rather than an import line per area that
/// drifts from what that area actually asserts on.
pub(crate) use mwl_runtime::{
    Ctx, DebugFlags, FATAL, FaultSite, OK, SafepointFlags, THROWN, Value, call,
};

/// Compiles a whole file, returning the unit or the first thing that refused
/// it.
///
/// Front-end diagnostics are a panic rather than an error: every fixture below
/// is meant to type-check, so a diagnostic is a broken fixture, not an outcome
/// under test.
pub(crate) fn compile(source: &str) -> Result<mwl_codegen::Unit, mwl_codegen::CodegenError> {
    mwl_codegen::compile(&lower(source))
}

/// Runs the whole front end over `source` and lowers it — every class method
/// plus the script frame — without compiling it.
pub(crate) fn lower(source: &str) -> mwl_ir::Program {
    let mut map = SourceMap::new();
    let id = map.add("test.mwl", source);
    let src = map.file(id);

    let mut diags = Diagnostics::new();
    let stmts = mwl_syntax::parse_file(src, &mut diags);
    let module = mwl_hir::resolve_file(&stmts, src, &mut diags);
    let mut interner = mwl_types::TypeInterner::new();
    let mut exprs = mwl_types::ExprTypeTable::new();
    let files = [mwl_types::ProgramFile { src, stmts: &stmts }];
    mwl_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    assert!(
        !diags.has_errors(),
        "the fixture does not type-check: {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );

    let layouts = mwl_types::build_class_layouts(&files, &module.graph);
    mwl_ir::lower::lower_file("<script>", &stmts, src, &exprs, &interner, &layouts)
}

/// Compiles and runs `source` against `ctx`, returning the compiled status.
pub(crate) fn run_with(ctx: &mut Ctx, source: &str) -> Result<Value, i32> {
    let unit = compile(source).expect("the fixture compiles");
    let entry = unit
        .function("<script>")
        .expect("the script frame was compiled");
    call(entry, ctx, &[])
}

/// Compiles, runs, and returns whatever the script echoed.
pub(crate) fn output_of(source: &str) -> String {
    let mut ctx = Ctx::buffered();
    run_with(&mut ctx, source).expect("the script ran to completion");
    String::from_utf8(ctx.take_buffered_output().expect("a buffered context"))
        .expect("the script echoed UTF-8")
}
