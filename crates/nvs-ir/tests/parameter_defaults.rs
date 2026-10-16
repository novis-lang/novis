//! An omitted parameter's default, lowered: the constant the call site
//! materializes, and the **type** it materializes it at.
//!
//! `nvs_types::defaults` folds a named constant to a value and loses the name
//! doing it — an enum case becomes the integer `rule:enums/no-class-machinery` says it already is.
//! What the emitter keeps is the position: `nvs_ir::lower::emit_const_arg`
//! takes the parameter's own IR type, so the integer standing for a case is
//! emitted under `nvs_ir::ty::Ty::Enum` and the same integer written at an
//! `int` parameter is not.
//!
//! Asserted on the instruction's `ty` rather than on its kind, because the
//! kind is what both rows share: a fixture that only checked
//! `InstKind::ConstInt(3)` would pass with the case arriving as a plain `int`,
//! which is exactly the thing this pins.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{InstKind, Program};
use nvs_ir::ty::{EnumRepr, Ty};

/// Parse, resolve, check, lower — `nvs-cli`'s own `front_end` order, panicking
/// on the first phase that reports, since the fixture is meant to compile
/// clean.
fn compile(src: &str) -> Program {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();

    let stmts = nvs_syntax::parse_file(map.file(id), &mut diags);
    nvs_syntax::check_declarations(&stmts, map.file(id), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");

    let module = nvs_hir::resolve_file(&stmts, map.file(id), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");

    let files = [nvs_types::ProgramFile {
        src: map.file(id),
        stmts: &stmts,
    }];
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let enums = nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    nvs_ir::lower::lower_program("<script>", &files, &exprs, &interner, &enums, &layouts)
}

/// Every integer constant the program emits, as `(value, representation)` —
/// the pair the two rows of the fixture differ in.
fn int_constants(program: &Program) -> Vec<(i64, Option<Ty>)> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match inst.kind {
            InstKind::ConstInt(value) => Some((value, inst.ty)),
            _ => None,
        })
        .collect()
}

/// One enum case and one class constant, each defaulting a parameter of its
/// own type, and one call that omits both — so every constant below was
/// emitted by the omitting call site and by nothing else.
const FIXTURE: &str = "<?nvs
enum Mode { Off = 0, Fast = 3 }
class Limits { public const int COUNT = 12; }
class Runner {
    public static function run(Mode $m = Mode::Fast, int $n = Limits::COUNT): void {}
}
Runner::run();
";

#[test]
fn an_omitted_enum_default_is_emitted_at_the_parameters_own_type() {
    let found = int_constants(&compile(FIXTURE));

    assert!(
        found.contains(&(3, Some(Ty::Enum(EnumRepr::Int)))),
        "the case's integer arrived as something other than its enum: {found:?}"
    );
    assert!(
        found.contains(&(12, Some(Ty::Int))),
        "the class constant's integer arrived as something other than `int`: {found:?}"
    );
}
