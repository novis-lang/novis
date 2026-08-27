//! `echo` and what reaches the output buffer — the acceptance program, operand order, conversions and escapes.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn the_acceptance_program_prints_hello_world() {
    assert_eq!(
        output_of("<?nvs\necho \"Hello, World!\";\n"),
        "Hello, World!"
    );
}

#[test]
fn echo_writes_its_operands_in_order_with_no_separator() {
    assert_eq!(output_of("<?nvs\necho \"a\", \"b\";\necho \"c\";\n"), "abc");
}

#[test]
fn a_scalar_operand_reaches_the_matching_conversion_helper() {
    // `echo` of a non-string goes through `Helper::IntToString` and friends,
    // which is `nvs-codegen`'s helper-symbol table under test as much as the
    // conversion itself.
    assert_eq!(output_of("<?nvs\necho 42;\n"), "42");
    assert_eq!(output_of("<?nvs\necho -7;\n"), "-7");
    assert_eq!(output_of("<?nvs\necho true;\n"), "1");
    assert_eq!(output_of("<?nvs\necho false;\n"), "");
    assert_eq!(output_of("<?nvs\necho 1.5;\n"), "1.5");
}

#[test]
fn an_escape_reaches_the_output_as_the_byte_it_names() {
    assert_eq!(output_of("<?nvs\necho \"a\\tb\\n\";\n"), "a\tb\n");
}
