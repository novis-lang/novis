//! What compiled code spends on reference counts: which values it retains and
//! releases, and how often it enters a refcount helper.
//!
//! A literal lives in the unit's data section with an immortal count, so a
//! retain or release of it changes nothing and is not emitted. The guards read
//! the Cranelift text, or count helper entries with
//! `nvs_runtime::budget::refcount_calls()` around a run. None reads a clock
//! (`rule:testing/bench-counters`). See `tests/common/mod.rs` for the shared
//! fixtures.

mod common;

use common::*;

/// The six typed refcount helpers. The tagged pair is out of scope: a literal
/// is never a tagged value.
const REFCOUNT_SYMBOLS: [&str; 6] = [
    "nvs_str_retain",
    "nvs_str_release",
    "nvs_object_retain",
    "nvs_object_release",
    "nvs_array_retain",
    "nvs_array_release",
];

/// One function's Cranelift IR out of the text `nvs_codegen::clif` renders,
/// which heads each function with `; <name>`.
fn section<'a>(text: &'a str, name: &str) -> &'a str {
    let head = format!("; {name}\n");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("the unit defines no `{name}`:\n{text}"))
        + head.len();
    let body = &text[start..];
    match body.find("\n; ") {
        Some(end) => &body[..end],
        None => body,
    }
}

/// Each method copies a literal into a second variable and returns it, which
/// the lowering retains and releases. Every counted value is a literal.
const LITERALS: &str = r#"<?nvs
class Lit {
    public static function greeting(): string {
        string $s = "hello";
        string $t = $s;
        echo $t;
        return $s;
    }
    public static function page(): Core\Html\Markup {
        var $m = html`<p>hello</p>`;
        var $n = $m;
        return $n;
    }
    public static function wait(): Core\Time\Duration {
        var $d = 90m;
        var $e = $d;
        return $e;
    }
}
echo Lit::greeting();
"#;

#[test]
fn a_literal_is_never_retained_or_released() {
    let program = lower(LITERALS);
    // The lowering does emit the operations: what is under test is that
    // codegen drops them.
    for name in ["Lit::greeting", "Lit::page", "Lit::wait"] {
        let function = program
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("the unit lowers no `{name}`"));
        let counted = function.blocks.iter().flat_map(|b| &b.insts).any(|inst| {
            matches!(
                inst.kind,
                nvs_ir::ir::InstKind::Retain { .. } | nvs_ir::ir::InstKind::Release { .. }
            )
        });
        assert!(counted, "`{name}` lowers no retain or release to drop");
    }

    let text = nvs_codegen::clif(&program).expect("the unit compiled");
    for name in ["Lit::greeting", "Lit::page", "Lit::wait"] {
        let clif = section(&text, name);
        for symbol in REFCOUNT_SYMBOLS {
            assert!(
                !clif.contains(symbol),
                "`{name}` calls `{symbol}` for a literal:\n{clif}"
            );
        }
    }
}

/// A loop that copies and echoes a string literal and copies an HTML literal
/// once per row, `rows` times.
#[cfg(debug_assertions)]
fn literal_rows(rows: u32) -> String {
    format!(
        r#"<?nvs
var $i = 0;
while ($i < {rows}) {{
    string $s = "row";
    string $t = $s;
    echo $t;
    var $m = html`<br>`;
    var $n = $m;
    $i = $i + 1;
}}
"#
    )
}

/// How many times the **run** of `source` enters a typed refcount helper.
/// Compilation happens outside the window.
#[cfg(debug_assertions)]
fn refcount_calls_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::refcount_calls();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::refcount_calls() - before
}

#[cfg(debug_assertions)]
#[test]
fn a_row_of_literals_enters_no_refcount_helper() {
    let small = refcount_calls_of_run(&literal_rows(2_000));
    let large = refcount_calls_of_run(&literal_rows(4_000));
    assert_eq!(
        small, large,
        "2,000 rows entered a refcount helper {small} times and 4,000 rows {large} times"
    );
}
