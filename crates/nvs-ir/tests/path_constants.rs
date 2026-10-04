//! `rule:programs/relative-paths-resolve-from-their-file`'s `Core\Path::thisFile`
//! and `thisDir`, lowered: each call is the string constant the checker folded
//! it to, and no instruction calls either member's symbol.
//!
//! The symbols exist only for a source with no file behind it, and their bodies
//! throw, so a call that slipped past the fold would still compile and fail at
//! run time. What this asserts is therefore the *absence* of the call beside
//! the presence of the constant.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{InstKind, Program};

/// Parse, resolve, check, lower — `nvs-cli`'s own `front_end` order, panicking
/// on the first phase that reports. The file needs a path, so the checker has a
/// folder to fold the two members to. An overlay gives it one, and nothing is
/// written to the disk.
fn compile(src: &str) -> Program {
    let mut map = SourceMap::new();
    let path = if cfg!(windows) {
        r"C:\shop\shop.nvs"
    } else {
        "/shop/shop.nvs"
    };
    map.overlay(path, src);
    let id = map.load(path).expect("the overlay answers for the path");
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

/// Every instruction the program lowered to, across every function.
fn insts(program: &Program) -> impl Iterator<Item = &InstKind> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .map(|inst| &inst.kind)
}

/// Both members, with and without a join, become absolute string constants
/// naming this file, its folder and a path under it, and nothing calls either
/// member's symbol.
// covers: Core\Path::thisFile
// covers: Core\Path::thisDir
#[test]
fn this_file_and_this_dir_lower_to_a_string_constant() {
    let program = compile(
        "<?nvs\necho Core\\Path::thisFile(), \"\\n\";\necho Core\\Path::thisDir(), \"\\n\";\n\
         echo Core\\Path::thisDir('data/rates.json'), \"\\n\";\n",
    );
    let calls: Vec<&str> = insts(&program)
        .filter_map(|kind| match kind {
            InstKind::CoreCall { symbol, .. } => Some(*symbol),
            _ => None,
        })
        .filter(|symbol| symbol.starts_with("nvs_core_path_this"))
        .collect();
    assert!(calls.is_empty(), "{calls:?}");

    let paths: Vec<&str> = insts(&program)
        .filter_map(|kind| match kind {
            InstKind::ConstStr(text) if std::path::Path::new(text).has_root() => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 3, "{paths:?}");
    let sep = std::path::MAIN_SEPARATOR_STR;
    assert!(paths[0].ends_with(&format!("{sep}shop.nvs")), "{paths:?}");
    assert_eq!(
        std::path::Path::new(paths[0]).parent(),
        Some(std::path::Path::new(paths[1])),
        "{paths:?}"
    );
    assert!(
        paths[2].ends_with(&format!("{sep}data{sep}rates.json")),
        "{paths:?}"
    );
}
