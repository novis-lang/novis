//! `rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`: the
//! checker records a qualified value's path only on a table the editor
//! switched on, and nothing outside `nvs-lsp` switches it on.

mod common;

use common::check_src_table;
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_hir::resolve_file;
use nvs_syntax::parse_file;
use nvs_types::check::check_program;
use nvs_types::expr_table::ExprTypeTable;
use nvs_types::provenance::Step;
use nvs_types::ty::TypeInterner;

const SRC: &str = "<?nvs
class Greeter {
    public static function greet(tainted string $who): uint {
        tainted string $line = \"Hello \" . $who;
        uint $size = Core\\Str::length($line);
        return $size;
    }
}
";

/// `src` checked on a table that records provenance, as the editor checks it.
fn check_recording(src: &str) -> (Diagnostics, ExprTypeTable) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    exprs.record_provenance();
    check_program(
        &[nvs_types::ProgramFile {
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

#[test]
fn a_check_records_no_path_unless_the_table_asks_for_one() {
    let (_, exprs) = check_src_table(SRC);
    assert!(!exprs.records_provenance());
    assert_eq!(exprs.taint_paths().count(), 0);
}

#[test]
fn a_recording_check_follows_the_qualifier_from_its_parameter_to_the_call_that_ends_it() {
    let (diags, exprs) = check_recording(SRC);
    assert!(!diags.has_errors(), "{diags:?}");
    let longest = exprs
        .taint_paths()
        .map(|(_, path)| path.iter().map(|step| step.step).collect::<Vec<_>>())
        .max_by_key(Vec::len)
        .expect("the tainted parameter's value has a path");
    assert_eq!(
        longest[..4],
        [
            Step::Declared,
            Step::Concatenated,
            Step::Assigned,
            Step::Ended
        ],
        "{longest:?}"
    );
}

#[test]
fn only_the_language_server_switches_the_recording_on() {
    let crates = nvs_repo::path("crates");
    let crates = crates.as_path();
    let mut callers = Vec::new();
    let mut dirs = vec![crates.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("a crate directory reads") {
            let path = entry.expect("a directory entry reads").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "target") {
                    dirs.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.starts_with(crates.join("nvs-types").join("tests"))
                && std::fs::read_to_string(&path)
                    .is_ok_and(|text| text.contains(".record_provenance()"))
            {
                callers.push(path);
            }
        }
    }
    assert!(!callers.is_empty(), "the language server calls it");
    for caller in &callers {
        assert!(
            caller.starts_with(crates.join("nvs-lsp")),
            "{} switches provenance on, and only `nvs-lsp` may",
            caller.display()
        );
    }
}
