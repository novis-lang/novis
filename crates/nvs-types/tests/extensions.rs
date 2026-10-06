//! A loaded extension's class in the checker: `rule:packaging/extension-calls-are-statically-typed`.
//!
//! Every test reads the conformance fixture `Shop\Ledger`'s manifest, the one the `.nvst` cases
//! load, so the checker is held to the same declarations the runtime is.

use nvs_diagnostics::{Diagnostics, SourceMap, code};
use nvs_ext::manifest::Manifest;
use nvs_hir::{QName, resolve_file_with_extensions};
use nvs_stdlib::registry::Qual;
use nvs_syntax::parse_file;
use nvs_types::check::check_program_loaded;
use nvs_types::expr_table::ExprTypeTable;
use nvs_types::signatures::SignatureTable;
use nvs_types::ty::{Ty, TypeInterner};

/// The fixture's manifest.
fn ledger() -> Manifest {
    let path = nvs_repo::path("tests/conformance/ext/fixtures/ledger/manifest.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    Manifest::parse(&bytes).expect("the fixture's manifest reads")
}

/// Checks `body` inside a method, with `set` as the loaded extension set.
fn check(body: &str, set: &[Manifest]) -> Diagnostics {
    let src = format!("<?nvs\nclass T {{\n  function m(): void {{\n{body}\n  }}\n}}\n");
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", &src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "the snippet parses: {diags:?}");
    let classes = nvs_types::ext_lib::hir_classes(set);
    let module = resolve_file_with_extensions(&stmts, map.file(file), &classes, &mut diags);
    if diags.has_errors() {
        return diags;
    }
    check_program_loaded(
        &[nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        None,
        set,
        &mut TypeInterner::new(),
        &mut ExprTypeTable::new(),
        &mut diags,
    );
    diags
}

/// The signature table of `set` alone.
fn table(set: &[Manifest]) -> (SignatureTable, TypeInterner) {
    let mut interner = TypeInterner::new();
    let table = nvs_types::ext_lib::signatures(set, &mut interner);
    (table, interner)
}

fn codes(diags: &Diagnostics) -> Vec<&'static str> {
    diags
        .iter()
        .filter_map(|diag| diag.code.map(|code| code.as_str()))
        .collect()
}

#[test]
fn a_loaded_manifest_class_is_in_the_signature_table() {
    let (table, mut interner) = table(&[ledger()]);
    let class = table
        .get(&QName::parse("Shop\\Ledger"))
        .expect("the loaded class is in the table");
    let echo = &class.methods["echoInt"];
    assert!(echo.is_static, "an extension method is static");
    assert_eq!(echo.params, vec![interner.int()]);
    assert_eq!(echo.param_names, vec!["number".to_owned()]);
    assert_eq!(echo.return_ty, interner.int());
    let optional = &class.methods["echoOptional"];
    assert_eq!(
        optional.required(),
        0,
        "a manifest default makes the parameter optional"
    );
    assert_eq!(class.methods["writeLine"].return_ty, interner.void());
    let fetch = class.methods["fetch"].return_ty;
    assert_eq!(
        interner.get(fetch),
        &Ty::TaintedString,
        "a source's result is tainted"
    );
    let calls = check(
        "int $n = Shop\\Ledger::echoInt(7);\n\
         string $s = Shop\\Ledger::echoString(\"seven\");\n\
         ?int $o = Shop\\Ledger::echoOptional();\n\
         array<int> $l = Shop\\Ledger::echoList([1, 2]);\n\
         Shop\\Ledger::writeLine(\"a line\");",
        &[ledger()],
    );
    assert!(!calls.has_errors(), "well-typed calls compile: {calls:?}");
}

#[test]
fn an_extension_method_parameter_carries_its_manifest_qualifier_where_core_puts_one() {
    let (table, _) = table(&[ledger()]);
    let class = table.get(&QName::parse("Shop\\Ledger")).expect("loaded");
    assert_eq!(class.methods["writeLine"].qual_at(0), Some(Qual::Sink));
    assert_eq!(
        class.methods["echoString"].qual_at(0),
        Some(Qual::Contagious)
    );
    assert_eq!(class.methods["fail"].qual_at(1), Some(Qual::Contagious));
}

#[test]
fn an_extension_call_with_a_wrong_argument_type_does_not_compile() {
    let diags = check("int $n = Shop\\Ledger::echoInt(\"seven\");", &[ledger()]);
    assert!(
        diags.has_errors(),
        "a string for an `int` parameter compiles"
    );
    let core = check("int $n = Core\\Str::length(7);", &[]);
    assert_eq!(
        codes(&diags),
        codes(&core),
        "the wrong argument gives the code a `Core` call gives"
    );
    let arity = check("int $n = Shop\\Ledger::echoInt();", &[ledger()]);
    assert!(arity.has_errors(), "a missing required argument compiles");
}

#[test]
fn an_extension_class_constant_has_its_manifest_type() {
    let (table, mut interner) = table(&[ledger()]);
    let class = table.get(&QName::parse("Shop\\Ledger")).expect("loaded");
    assert_eq!(class.constants["CURRENCY"].ty, interner.string());
    let read = check("string $c = Shop\\Ledger::CURRENCY;", &[ledger()]);
    assert!(
        !read.has_errors(),
        "the constant reads as a string: {read:?}"
    );
    let wrong = check("int $c = Shop\\Ledger::CURRENCY;", &[ledger()]);
    assert!(wrong.has_errors(), "a string constant assigns to an `int`");
}

#[test]
fn an_extension_class_is_unknown_when_the_set_does_not_load_it() {
    let diags = check("int $n = Shop\\Ledger::echoInt(7);", &[]);
    assert!(
        codes(&diags).contains(&code::E_UNDEFINED_CLASS.as_str()),
        "a class no extension of the set declares is undeclared: {diags:?}"
    );
}
