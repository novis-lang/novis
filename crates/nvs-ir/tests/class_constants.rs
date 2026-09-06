//! ADR 0011's class constant on a **user-declared** class, lowered: a read of
//! one is the constant instruction its value folded to, with no storage, no
//! descriptor and no call.
//!
//! The `Core` half of that rule has been true since `nvs_stdlib::registry`
//! stated `Core\Math::PI`, and an enum case's since `rule:enums/no-class-machinery`. This is the
//! third spelling, and it is the one that used to reach
//! `nvs_ir::lower::expr`'s `ClassConstAccess` arm with nothing recorded and
//! panic — so what this asserts is a *value*, per row, rather than the absence
//! of a panic: an arm that emitted a plausible zero would pass a
//! did-it-compile test.
//!
//! The `uint` row is the reason the value is placed in the declared type
//! rather than in the folded literal's own (`nvs_types::signatures::ConstSig`):
//! `const uint WIDE = 5;` folds to an `int` 5 and has to arrive as
//! `ConstUint`, or the representation the instruction produces disagrees with
//! the one the binding declared.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_ir::ir::{InstKind, Program};

/// Parse, resolve, check, lower — `nvs-cli`'s own `front_end` order, panicking
/// on the first phase that reports, since every fixture here is meant to
/// compile clean.
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

/// Every constant instruction the program lowered to, rendered — what a read
/// of a constant leaves behind, and the only thing it is allowed to leave
/// behind.
///
/// Rendered rather than compared as values because `InstKind` is deliberately
/// not [`PartialEq`]: it holds an `f64`, and an instruction is compared for
/// what it *is* rather than for a key, exactly as
/// `nvs_types::consts::ConstValue` is.
fn constants(program: &Program) -> Vec<String> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .map(|inst| &inst.kind)
        .filter(|kind| {
            matches!(
                kind,
                InstKind::ConstInt(_)
                    | InstKind::ConstUint(_)
                    | InstKind::ConstBool(_)
                    | InstKind::ConstFloat(_)
                    | InstKind::ConstStr(_)
            )
        })
        .map(|kind| format!("{kind:?}"))
        .collect()
}

/// The reads themselves, and beside each the literal the declaration wrote —
/// the twin fixture below is this table with the right-hand column
/// substituted in.
const READS: &[(&str, &str, &str)] = &[
    ("int", "$max", "Limits::MAX"),
    ("string", "$name", "Limits::NAME"),
    ("uint", "$wide", "Limits::WIDE"),
    ("bool", "$on", "Limits::ON"),
    ("float", "$ratio", "Limits::RATIO"),
    // Declared on the interface, read through two `extends`/`implements` hops.
    ("int", "$ceiling", "Tighter::CEILING"),
];

/// The same six, written as the literals the declarations hold.
const LITERALS: &[&str] = &["3", "\"limits\"", "5", "true", "0.5", "9"];

/// The declarations every fixture here shares, so the twin below differs from
/// the original in the *reads* alone.
const DECLARATIONS: &str = "<?nvs
interface HasLimit {
    public const int CEILING = 9;
}
class Limits implements HasLimit {
    public const int MAX = 3;
    public const string NAME = \"limits\";
    public const uint WIDE = 5;
    public const bool ON = true;
    public const float RATIO = 0.5;
}
class Tighter extends Limits {}
";

/// [`DECLARATIONS`] plus one top-level binding per row, initialized from
/// `values` — the constant reads for one call, the literals for the other.
fn fixture(values: &[&str]) -> String {
    let mut src = DECLARATIONS.to_owned();
    for ((ty, name, _), value) in READS.iter().zip(values) {
        src.push_str(&format!(
            "{ty} {name} = {value};
"
        ));
    }
    src
}

#[test]
fn a_user_declared_class_constant_lowers_to_its_value() {
    let read: Vec<&str> = READS.iter().map(|(_, _, read)| *read).collect();
    let mut found = constants(&compile(&fixture(&read)));

    // Each row's own value, so a failure names the constant rather than the
    // multiset — the `uint` row being the one that says which *representation*
    // the value arrived in.
    let expected = [
        InstKind::ConstInt(3),
        InstKind::ConstStr("limits".to_owned()),
        InstKind::ConstUint(5),
        InstKind::ConstBool(true),
        InstKind::ConstFloat(0.5),
        InstKind::ConstInt(9),
    ];
    for want in &expected {
        let want = format!("{want:?}");
        assert!(
            found.contains(&want),
            "no `{want}` among the lowered constants {found:?}"
        );
    }

    // And the whole program's constants are the ones the same program written
    // with literals produces — which is what "inlined at every use site" means
    // and is the half a membership check cannot make: a read that also emitted
    // a descriptor name, a slot load's key or a second constant would show up
    // here as an extra with no twin.
    let mut written = constants(&compile(&fixture(LITERALS)));
    found.sort();
    written.sort();
    assert_eq!(
        found, written,
        "a constant read must lower to exactly what its literal does"
    );
}
