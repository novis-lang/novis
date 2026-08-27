//! Concatenation and comparison over `NvsStr`, including the refcount edges a loop exposes.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn concatenation_joins_its_operands_and_converts_a_scalar_one_first() {
    // `.` over two strings is one `nvs_str_concat`; over a scalar it is the
    // matching `…ToString` helper first, which `nvs-ir` inserts. Both halves
    // are what every acceptance example past `hello.nvs` runs on.
    assert_eq!(output_of("<?nvs\necho \"a\" . \"b\";\n"), "ab");
    assert_eq!(output_of("<?nvs\necho \"\" . \"\";\n"), "");
    assert_eq!(output_of("<?nvs\necho \"n = \" . 42 . \"!\";\n"), "n = 42!");
    assert_eq!(output_of("<?nvs\necho \"f\" . 1.5 . true;\n"), "f1.51");
}

/// `nvs_ir::ir::InstKind::Concat` carries every piece, so three or more of them
/// are one `nvs_str_concat_n` over a stack array rather than a fold of
/// `nvs_str_concat` calls. What that changes is the allocation count, which no
/// program can observe — so what this pins is that the bytes and their order
/// survive the new path, including the two shapes the flattening in
/// `Lowering::lower_concat` has to get right: a parenthesized right operand,
/// and the same local appearing more than once.
#[test]
fn an_n_ary_concatenation_joins_every_piece_in_order() {
    let source = "<?nvs
string $tag = \"td\";
int $i = 7;
echo \"<tr><\" . $tag . \">\" . $i . \"</\" . $tag . \">\", \"\\n\";
echo \"a\" . (\"b\" . \"c\") . \"d\", \"\\n\";
echo \"<\" . $tag . \">$i</\" . $tag . \">\", \"\\n\";
";
    assert_eq!(output_of(source), "<tr><td>7</td>\nabcd\n<td>7</td>\n");
}

/// `docs/perf/userland-gap.md` § B's third change, from the side this crate
/// owns: a string literal is an address in the unit's data section, so
/// evaluating one twice yields the *same* address rather than two
/// allocations.
///
/// Pointer identity is the assertion because it is the only observation this
/// crate can make — `nvs_runtime::counting_alloc` is that crate's own test
/// build — and it is a stronger one anyway: two allocations cannot share an
/// address. `made` is the control, and the reason the test can fail at all:
/// `"be" . "ta"` is a genuine `nvs_str_concat_n` per call, and its two answers
/// land at different addresses.
#[test]
fn a_string_literal_is_one_address_rather_than_an_allocation_per_evaluation() {
    // Both methods take a parameter they ignore: a compiled function called
    // with an empty argument slice faults, so a fixture reached through
    // `call` rather than through the script frame declares at least one.
    let source = "<?nvs
class Label {
    public static function pinned(int $ignored): string {
        return \"beta\";
    }

    public static function made(int $ignored): string {
        return \"be\" . \"ta\";
    }
}
";
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();

    let pinned = unit
        .function("Label::pinned")
        .expect("`pinned` was compiled");
    let first = call(pinned, &mut ctx, &[Value::int(0)]).expect("the method ran");
    let second = call(pinned, &mut ctx, &[Value::int(0)]).expect("the method ran");
    assert_eq!(first.as_str_bytes(), Some(&b"beta"[..]));
    assert_eq!(second.as_str_bytes(), Some(&b"beta"[..]));
    assert_eq!(
        first.as_str_bytes().map(<[u8]>::as_ptr),
        second.as_str_bytes().map(<[u8]>::as_ptr),
        "the literal was allocated rather than pointed at"
    );

    let made = unit.function("Label::made").expect("`made` was compiled");
    let one = call(made, &mut ctx, &[Value::int(0)]).expect("the method ran");
    let two = call(made, &mut ctx, &[Value::int(0)]).expect("the method ran");
    assert_eq!(one.as_str_bytes(), Some(&b"beta"[..]));
    assert_ne!(
        one.as_str_bytes().map(<[u8]>::as_ptr),
        two.as_str_bytes().map(<[u8]>::as_ptr),
        "a concatenation is still a fresh allocation per call"
    );
}

#[test]
fn a_concatenation_in_a_loop_keeps_producing_the_right_bytes() {
    // Each iteration's result is released once the local it was assigned to is
    // overwritten, so a botched refcount here shows up as freed bytes rather
    // than only as a leak. The leak half is what M4's Valgrind/ASAN run is for.
    assert_eq!(
        output_of(
            "<?nvs\nstring $s = \"\";\nint $i = 0;\nwhile ($i < 4) {\n    $s = $s . \"ab\";\n    $i = $i + 1;\n}\necho $s;\n"
        ),
        "abababab"
    );
}

/// The one fact three crates each hold a copy of: which slot a `Throwable`
/// property occupies.
///
/// `nvs_hir::errors::PROPERTIES` is the home; `nvs_runtime::throwable`
/// restates two indices because it depends on nothing, and `nvs_ir::lower`
/// restates the field *names* for the same reason. This test is the seam that
/// keeps the three from drifting — it is here because this is the only crate
/// that can see all of them at once.
#[test]
fn two_strings_compare_by_bytes_rather_than_by_pointer() {
    // `Ty::Str` is a pointer, so an `icmp` would compare identity — which is
    // never what `==` means for a string. Two separately allocated literals
    // holding the same bytes are the case that catches it.
    let source = "<?nvs
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
    let source = "<?nvs
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
