//! Concatenation and comparison over `MwlStr`, including the refcount edges a loop exposes.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn concatenation_joins_its_operands_and_converts_a_scalar_one_first() {
    // `.` over two strings is one `mwl_str_concat`; over a scalar it is the
    // matching `…ToString` helper first, which `mwl-ir` inserts. Both halves
    // are what every acceptance example past `hello.mwl` runs on.
    assert_eq!(output_of("<?mwl\necho \"a\" . \"b\";\n"), "ab");
    assert_eq!(output_of("<?mwl\necho \"\" . \"\";\n"), "");
    assert_eq!(output_of("<?mwl\necho \"n = \" . 42 . \"!\";\n"), "n = 42!");
    assert_eq!(output_of("<?mwl\necho \"f\" . 1.5 . true;\n"), "f1.51");
}

/// `mwl_ir::ir::InstKind::Concat` carries every piece, so three or more of them
/// are one `mwl_str_concat_n` over a stack array rather than a fold of
/// `mwl_str_concat` calls. What that changes is the allocation count, which no
/// program can observe — so what this pins is that the bytes and their order
/// survive the new path, including the two shapes the flattening in
/// `Lowering::lower_concat` has to get right: a parenthesized right operand,
/// and the same local appearing more than once.
#[test]
fn an_n_ary_concatenation_joins_every_piece_in_order() {
    let source = "<?mwl
string $tag = \"td\";
int $i = 7;
echo \"<tr><\" . $tag . \">\" . $i . \"</\" . $tag . \">\", \"\\n\";
echo \"a\" . (\"b\" . \"c\") . \"d\", \"\\n\";
echo \"<\" . $tag . \">$i</\" . $tag . \">\", \"\\n\";
";
    assert_eq!(output_of(source), "<tr><td>7</td>\nabcd\n<td>7</td>\n");
}

#[test]
fn a_concatenation_in_a_loop_keeps_producing_the_right_bytes() {
    // Each iteration's result is released once the local it was assigned to is
    // overwritten, so a botched refcount here shows up as freed bytes rather
    // than only as a leak. The leak half is what M4's Valgrind/ASAN run is for.
    assert_eq!(
        output_of(
            "<?mwl\nstring $s = \"\";\nint $i = 0;\nwhile ($i < 4) {\n    $s = $s . \"ab\";\n    $i = $i + 1;\n}\necho $s;\n"
        ),
        "abababab"
    );
}

/// The one fact three crates each hold a copy of: which slot a `Throwable`
/// property occupies.
///
/// `mwl_hir::errors::PROPERTIES` is the home; `mwl_runtime::throwable`
/// restates two indices because it depends on nothing, and `mwl_ir::lower`
/// restates the field *names* for the same reason. This test is the seam that
/// keeps the three from drifting — it is here because this is the only crate
/// that can see all of them at once.
#[test]
fn two_strings_compare_by_bytes_rather_than_by_pointer() {
    // `Ty::Str` is a pointer, so an `icmp` would compare identity — which is
    // never what `==` means for a string. Two separately allocated literals
    // holding the same bytes are the case that catches it.
    let source = "<?mwl
class T {
    public static function same(string $a, string $b): bool { return $a == $b; }
}
if (T::same(\"ab\", \"ab\")) { echo \"eq \"; }
if (T::same(\"ab\", \"ba\")) { echo \"wrong \"; }
if (\"x\" != \"y\") { echo \"ne\"; }
";
    assert_eq!(output_of(source), "eq ne");
}

#[test]
fn comparing_against_a_string_literal_in_a_loop_leaks_nothing() {
    // The literal is a fresh allocation no slot owns, so the comparison has
    // to release it once it has read it — 20_000 of them otherwise grow the
    // heap without bound. Stage 6's valgrind leg is what proves the absence;
    // this proves the program still computes the right answer.
    let source = "<?mwl
int $hits = 0;
int $i = 0;
string $key = \"bad\";
while ($i < 20000) {
    if ($key == \"bad\") { $hits = $hits + 1; }
    $i = $i + 1;
}
echo $hits;
";
    assert_eq!(output_of(source), "20000");
}
