//! `rule:iteration/generators`: a generator body that builds or reads a shape declares that
//! shape's class to the unit, as an ordinary method body does.
//!
//! A generator's body lowers into its synthesized advance method, and that method's `Lowering`
//! records each shape class the body names. Each fixture names its shape inside the generator
//! alone, so the class reaches `Program::classes` only if the advance method passes it out.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::Program;

/// Parse, resolve, check and lower one file against the manifests in `set`, panicking on the
/// first phase that reports, since every fixture here compiles clean.
fn compile(src: &str, set: &[nvs_ext::manifest::Manifest]) -> Program {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();

    let stmts = nvs_syntax::parse_file(map.file(id), &mut diags);
    nvs_syntax::check_declarations(&stmts, map.file(id), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");

    let classes = nvs_types::ext_lib::hir_classes(set);
    let module =
        nvs_hir::resolve_file_with_extensions(&stmts, map.file(id), &classes, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");

    let files = [nvs_types::ProgramFile {
        src: map.file(id),
        stmts: &stmts,
    }];
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let enums = nvs_types::check::check_program_loaded(
        &files,
        &module,
        None,
        nvs_types::Backtracking::Allow,
        &nvs_types::Grants::default(),
        None,
        set,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    nvs_ir::lower::lower_program("<script>", &files, &exprs, &interner, &enums, &layouts)
}

/// The labels of every shape class the unit declares.
fn shape_classes(program: &Program) -> Vec<&str> {
    program
        .classes
        .iter()
        .map(|class| class.label.as_str())
        .filter(|label| label.starts_with("$shape"))
        .collect()
}

#[test]
fn a_generator_body_that_builds_a_shape_declares_its_class() {
    let program = compile(
        "<?nvs\n\
         class Counter {\n\
             public function each(): Iterator<int> {\n\
                 var $point = { across: 1, down: 2 };\n\
                 yield $point->across + $point->down;\n\
             }\n\
         }\n\
         foreach ((new Counter())->each() as int $n) {\n\
             echo $n;\n\
         }\n",
        &[],
    );
    assert_eq!(shape_classes(&program), ["$shape{across,down}"]);
}

/// The set is the conformance fixture `Shop\Ledger`'s, whose `echoItem` returns a shape.
#[test]
fn a_generator_body_that_reads_an_extension_shape_declares_its_class() {
    let path = nvs_repo::path("tests/conformance/ext/fixtures/ledger/manifest.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let set = [nvs_ext::manifest::Manifest::parse(&bytes).expect("the fixture's manifest reads")];
    let program = compile(
        "<?nvs\n\
         class Names {\n\
             public function each(): Iterator<string> {\n\
                 var $item = Shop\\Ledger::echoItem({ name: \"pen\" });\n\
                 yield $item->name;\n\
             }\n\
         }\n\
         foreach ((new Names())->each() as string $name) {\n\
             echo $name;\n\
         }\n",
        &set,
    );
    assert!(
        shape_classes(&program).contains(&"$shape{name,note}"),
        "{:?}",
        shape_classes(&program)
    );
}
