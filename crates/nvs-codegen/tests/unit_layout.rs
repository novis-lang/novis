//! What a compiled unit contains and how it disassembles.
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
    // `nvs_types::expr_table::ExprTypeTable::method_label`. A namespace is
    // where the two would drift apart if they were spelled twice.
    let unit = compile(
        "<?nvs\nnamespace App;\nclass Math {\n    public static function id(int $n): int {\n        return $n;\n    }\n}\necho App\\Math::id(7);\n",
    )
    .expect("the fixture compiles");
    assert!(unit.function("App\\Math::id").is_some(), "{unit:?}");
    assert!(unit.function("<script>").is_some(), "{unit:?}");
}

#[test]
fn disassembling_names_each_frame_and_shows_the_code_that_would_have_run() {
    // What `nvs run --dump-asm` prints. The two structural claims worth
    // holding: every compiled frame gets a section headed by its Novis name, and
    // the section carries the probe sites this backend emits unconditionally —
    // so a disassembly cannot silently be of some differently-configured
    // second compile.
    let text = nvs_codegen::disassemble(&lower("<?nvs\necho \"Hello, World!\";\n"))
        .expect("the fixture compiles");

    assert!(text.starts_with("; <script>\n"), "{text}");
    assert!(text.contains("block0:"), "{text}");
    // Two `load_ext_name` sites at minimum: the safepoint slow path and the
    // statement probe, both out-of-line calls this backend always emits.
    assert!(text.matches("load_ext_name").count() >= 2, "{text}");
}

// Two tests stood here — `an_unlowered_shape_is_an_error_naming_it_rather_than
// _a_panic` and its disassembly twin — and both used `float` `%` as the shape
// the front end accepted and this backend did not. `E0717` refuses that pair
// where it is written now, so the fixture no longer type-checks and the
// property they guarded has no source-reachable instance left in this area:
// every refusal left in `emit_binop` is an internal-consistency check whose
// roster comment names what subtracts to nothing, which is why each is a
// `CodegenError::Internal` now rather than an `Unsupported`. Hand-building an
// IR to keep them would contradict this file's whole reason for going through
// the real pipeline (see the module doc above), so the guard moved rather than
// being rebuilt: `nvs_types`' `a_float_modulo_is_a_compile_error` and
// `an_arithmetic_operand_with_no_row_is_a_compile_error` hold the rule, and
// `tests/conformance/lang/the-arithmetic-table-is-closed.nvst` holds what a
// program actually sees.
