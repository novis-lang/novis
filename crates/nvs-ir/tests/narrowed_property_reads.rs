//! A narrowed read of a `readonly` property keeps a run-time null check, and a
//! narrowed read of a variable does not — the goal's standing decision, and
//! `nvs_ir::lower::expr`'s `checked_narrowed_read` is what holds it.
//!
//! The variable is trusted because every write to it is visible to the
//! checker. The property's check is there so that a write path the checker
//! missed throws a `LogicError` naming the property, and never reads a `null`
//! as an object.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{Function, InstKind, Program};

/// Parse, resolve, check, lower — `nvs-cli`'s own `front_end` order,
/// panicking on the first phase that reports, since every fixture here is
/// meant to compile clean.
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

const FIXTURE: &str = "<?nvs
final class Node {
    public function label(): string { return \"n\"; }
}
final class Holder {
    public function constructor(public readonly ?Node $node) {}
}
final class Probe {
    public static function viaProperty(Holder $h): string {
        if ($h->node != null) {
            return $h->node->label();
        }
        return \"\";
    }
    public static function viaVariable(?Node $n): string {
        if ($n != null) {
            return $n->label();
        }
        return \"\";
    }
}
";

fn function<'p>(program: &'p Program, suffix: &str) -> &'p Function {
    program
        .functions
        .iter()
        .find(|f| f.name.ends_with(suffix))
        .unwrap_or_else(|| panic!("no function named `…{suffix}`"))
}

fn insts(function: &Function) -> impl Iterator<Item = &InstKind> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.insts)
        .map(|inst| &inst.kind)
}

fn null_tests(function: &Function) -> usize {
    insts(function)
        .filter(|kind| matches!(kind, InstKind::IsNull { .. }))
        .count()
}

fn names_the_property(function: &Function) -> bool {
    insts(function).any(|kind| {
        matches!(kind, InstKind::ConstStr(s) if s.contains("`readonly` property `$node` is `null`"))
    })
}

fn untags(function: &Function) -> usize {
    insts(function)
        .filter(|kind| matches!(kind, InstKind::Untag { .. }))
        .count()
}

/// The condition tests the slot once, and the narrowed read tests it again
/// before it untags, with a message that names the property.
#[test]
fn a_narrowed_property_read_keeps_its_null_check() {
    let program = compile(FIXTURE);
    let read = function(&program, "viaProperty");
    assert_eq!(null_tests(read), 2, "{read:#?}");
    assert!(names_the_property(read), "{read:#?}");
    assert!(untags(read) >= 1, "{read:#?}");
}

/// The condition is the only null test: the narrowed variable read untags
/// with no check of its own.
#[test]
fn a_narrowed_variable_read_stays_unchecked() {
    let program = compile(FIXTURE);
    let read = function(&program, "viaVariable");
    assert_eq!(null_tests(read), 1, "{read:#?}");
    assert!(!names_the_property(read), "{read:#?}");
    assert!(untags(read) >= 1, "{read:#?}");
}
