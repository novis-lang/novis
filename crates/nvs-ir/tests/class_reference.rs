//! ADR 0125 § 2's two ways into a `class<T>`, lowered — the compile-time one
//! and the run-time one, told apart by what each leaves in the IR.
//!
//! § 2 makes `as` a class reference's only source and then splits that source
//! in two: `Dog::class as class<Animal>` is decided where it stands, so "the
//! common case pays nothing at run time", while every other operand is a
//! hierarchy walk that throws. Those are the same *expression* with the same
//! type on both sides, so a test that only asked whether the program compiled
//! would pass on either answer. What is asserted here is therefore the
//! instruction: one `ClassDescConst` and no walk for the folded row, one
//! `ClassDescIn` for the dynamic one.

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

/// The two class-descriptor sources this file is about, rendered — a baked
/// class for the first and the bound it walked for the second.
///
/// Rendered rather than compared as values because `InstKind` is deliberately
/// not [`PartialEq`]; `class_constants.rs`'s own helper says why.
fn descriptor_sources(program: &Program) -> Vec<String> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match &inst.kind {
            InstKind::ClassDescConst { class } => Some(format!("const {class}")),
            InstKind::ClassDescIn { base, .. } => Some(format!("in {base}")),
            _ => None,
        })
        .collect()
}

/// The hierarchy every fixture here shares.
const DECLARATIONS: &str = "<?nvs
class Animal {}
class Dog extends Animal {}
";

#[test]
fn a_folded_class_constant_lowers_to_a_descriptor_constant() {
    let program = compile(&format!(
        "{DECLARATIONS}class<Animal> $cls = Dog::class as class<Animal>;
"
    ));
    // One instruction, and it names `Dog` rather than the bound: § 2 decides a
    // written-out `::class` at compile time, so the descriptor the program ends
    // up holding is baked and nothing is compared at run time. A `Dog` here
    // with a walk beside it would mean the fold ran and was not trusted.
    assert_eq!(descriptor_sources(&program), vec!["const Dog".to_owned()]);
}

#[test]
fn a_name_that_is_not_written_out_lowers_to_a_walk_over_the_bound() {
    let program = compile(&format!(
        "{DECLARATIONS}string $name = \"Dog\";
class<Animal> $cls = $name as class<Animal>;
"
    ));
    // The bound, not the class: what the walk admits is every class this unit
    // declares that is an `Animal`, which is the closed set `nvs-codegen`
    // resolves `ClassDescIn` against and the whole of why § 2 lets a `tainted`
    // string through the conversion.
    assert_eq!(descriptor_sources(&program), vec!["in Animal".to_owned()]);
}

/// Every allocation the program performs, rendered as which of the two `new`
/// instructions it is and what constructor label it carries — the two facts
/// § 4's row is about.
///
/// [`descriptor_sources`]'s reason for rendering rather than comparing values.
fn allocations(program: &Program) -> Vec<String> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match &inst.kind {
            InstKind::New { class, ctor, args } => Some(format!(
                "new {class} via {} with {} arg(s)",
                ctor.as_deref().unwrap_or("<none>"),
                args.len()
            )),
            InstKind::NewDynamic { ctor, args, .. } => Some(format!(
                "new dynamic via {} with {} arg(s)",
                ctor.as_deref().unwrap_or("<none>"),
                args.len()
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn a_new_through_a_class_reference_lowers_to_new_dynamic() {
    let program = compile(
        "<?nvs
class Animal { public function constructor(public int $legs) {} }
class Dog extends Animal {}
class<Animal> $cls = Dog::class as class<Animal>;
Animal $pet = new $cls(4);
",
    );
    // Two claims, and the fixture is built so that either one failing changes
    // this line. **`NewDynamic`, on a descriptor the folded conversion baked**:
    // the class is in the compiler's hand here, so an `InstKind::New { class:
    // "Dog" }` would run this program correctly and still be wrong — the
    // checker typed the site as `Animal`, and the same `$cls` reaching this
    // `new` from a parameter or a walk has no class to fold. **The label is
    // `Animal::constructor`**, the bound's, because it is the lookup's
    // *fallback*: `NewDynamic` asks the allocated class first, so a `Dog` that
    // declared its own would win at run time without this site knowing.
    assert_eq!(
        allocations(&program),
        vec!["new dynamic via Animal::constructor with 1 arg(s)".to_owned()]
    );
}

/// Every call this program makes, rendered as whether it bound to a label or
/// dispatched on a descriptor — the distinction § 4's second row is entirely
/// about, and one a program that printed the right answer would not show.
fn calls(program: &Program) -> Vec<String> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match &inst.kind {
            InstKind::Call { target, .. } => Some(format!("direct {target}")),
            InstKind::CallVirtual {
                method, fallback, ..
            } => Some(format!(
                "virtual {method} over {}",
                fallback.as_deref().unwrap_or("<none>")
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn a_static_call_through_a_class_reference_dispatches_on_the_descriptor() {
    let program = compile(
        "<?nvs
class Animal { public static function noise(): string { return \"...\"; } }
class Dog extends Animal { public static function noise(): string { return \"woof\"; } }
class<Animal> $cls = Dog::class as class<Animal>;
echo $cls::noise(), \"\\n\";
",
    );
    // Virtual, not direct, even though the fold above put `Dog` in the
    // compiler's own hand: the checker resolved `noise` on `Animal`, which is
    // the only roster this site can see, so a direct call would run the base's
    // body — the one wrong answer that still compiles and still prints. The
    // label `Animal::noise` is present as the *fallback*, which is what an
    // implementor declaring no `noise` of its own falls through to.
    assert_eq!(
        calls(&program),
        vec!["virtual noise over Animal::noise".to_owned()]
    );
}
