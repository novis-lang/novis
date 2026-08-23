//! What a compiled unit contains and how it disassembles, including an unlowered shape reported rather than panicked on.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn every_lowered_method_is_compiled_under_its_class_qualified_name() {
    // The label a call's target is rendered from and the name its callee is
    // compiled under are the same string by construction — see
    // `mwl_types::expr_table::ExprTypeTable::method_label`. A namespace is
    // where the two would drift apart if they were spelled twice.
    let unit = compile(
        "<?mwl\nnamespace App;\nclass Math {\n    public static function id(int $n): int {\n        return $n;\n    }\n}\necho \\App\\Math::id(7);\n",
    )
    .expect("the fixture compiles");
    assert!(unit.function("App\\Math::id").is_some(), "{unit:?}");
    assert!(unit.function("<script>").is_some(), "{unit:?}");
}

#[test]
fn disassembling_names_each_frame_and_shows_the_code_that_would_have_run() {
    // What `mwl run --dump-asm` prints. The two structural claims worth
    // holding: every compiled frame gets a section headed by its MWL name, and
    // the section carries the probe sites this backend emits unconditionally —
    // so a disassembly cannot silently be of some differently-configured
    // second compile.
    let text = mwl_codegen::disassemble(&lower("<?mwl\necho \"Hello, World!\";\n"))
        .expect("the fixture compiles");

    assert!(text.starts_with("; <script>\n"), "{text}");
    assert!(text.contains("block0:"), "{text}");
    // Two `load_ext_name` sites at minimum: the safepoint slow path and the
    // statement probe, both out-of-line calls this backend always emits.
    assert!(text.matches("load_ext_name").count() >= 2, "{text}");
}

#[test]
fn disassembling_an_unlowered_shape_reports_it_rather_than_printing_half_a_unit() {
    let error = mwl_codegen::disassemble(&lower(
        "<?mwl\nfloat $a = 7.0;\nfloat $b = 2.0;\nfloat $q = $a % $b;\n",
    ))
    .unwrap_err();
    assert!(error.to_string().contains("Mod"), "{error}");
}

#[test]
fn an_unlowered_shape_is_an_error_naming_it_rather_than_a_panic() {
    // `%` over two floats has no lowering: ADR 0007 § 4's arithmetic table
    // gives `%` an `int`/`uint` row and no `float` one, and PHP reaches it by
    // converting both operands first — a conversion nothing inserts here. What
    // matters is that the backend *says so* instead of panicking or, worse,
    // emitting something.
    let error =
        compile("<?mwl\nfloat $a = 7.0;\nfloat $b = 2.0;\nfloat $q = $a % $b;\n").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("Mod"), "{message}");
}
