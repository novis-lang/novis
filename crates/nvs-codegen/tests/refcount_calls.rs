//! What compiled code spends on reference counts: which values it retains and
//! releases, and how often it enters a refcount helper.
//!
//! A literal lives in the unit's data section with an immortal count, so a
//! retain or release of it changes nothing and is not emitted. Any other
//! retain of a string, an object or an array is inline and makes no call, and
//! a release is a call (`nvs_codegen`'s § *The inline retain*). The guards read
//! the Cranelift text, or count helper entries with
//! `nvs_runtime::budget::refcount_calls()` or `retain_calls()` around a run. None reads a clock
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

/// A loop that copies a fresh string, an array and an object into a second
/// variable once per row, `rows` times. Each copy is a retain.
#[cfg(debug_assertions)]
fn copy_rows(rows: u32) -> String {
    format!(
        r#"<?nvs
class Box {{
    public int $n = 0;
}}
var $i = 0;
int $total = 0;
while ($i < {rows}) {{
    string $s = "row {{$i}}";
    string $t = $s;
    array<int> $a = [$i, $i];
    array<int> $b = $a;
    Box $o = new Box();
    Box $p = $o;
    echo $t;
    $total = $total + $b[0] + $p->n;
    $i = $i + 1;
}}
echo $total;
"#
    )
}

/// How many times the **run** of `source` enters a typed retain helper.
#[cfg(debug_assertions)]
fn retain_calls_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::retain_calls();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::retain_calls() - before
}

#[cfg(debug_assertions)]
#[test]
fn a_row_that_retains_enters_no_retain_helper() {
    let program = lower(&copy_rows(1));
    let retains = program
        .functions
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.insts)
        .filter(|inst| matches!(inst.kind, nvs_ir::ir::InstKind::Retain { .. }))
        .count();
    assert!(
        retains >= 3,
        "the row lowers {retains} retains, not one per copy"
    );

    let small = retain_calls_of_run(&copy_rows(2_000));
    let large = retain_calls_of_run(&copy_rows(4_000));
    assert_eq!(
        small, large,
        "2,000 rows entered a retain helper {small} times and 4,000 rows {large} times"
    );
}

/// Each `make` method keeps the value it returns in a static property, so the
/// value has two holders when Rust receives it. Each `keep` method takes the
/// caller's reference and stores the value twice.
const KEEPER: &str = r#"<?nvs
class Box {
    public int $n = 0;
}
class K {
    public static string $text = "";
    public static ?Box $box = null;
    public static array<int> $list = [];
    public static array<string> $texts = [];
    public static array<Box> $boxes = [];
    public static array<array<int>> $lists = [];
    public static function text(int $n): string {
        K::$text = "row {$n}";
        return K::$text;
    }
    public static function literal(): string { return "hello"; }
    public static function box(): Box {
        Box $b = new Box();
        K::$box = $b;
        return $b;
    }
    public static function list(int $n): array<int> {
        K::$list = [$n, $n];
        return K::$list;
    }
    public static function keepText(string $s): void { K::$texts = [$s, $s]; }
    public static function keepBox(Box $b): void { K::$boxes = [$b, $b]; }
    public static function keepList(array<int> $l): void { K::$lists = [$l, $l]; }
    public static function drop(): void {
        K::$texts = [];
        K::$boxes = [];
        K::$lists = [];
    }
}
K::drop();
"#;

/// A method call of `KEEPER`'s `K`, which must succeed.
fn call(unit: &nvs_codegen::Unit, ctx: &mut Ctx, method: &str, args: &[Value]) -> Value {
    unit.call_static(ctx, "K", method, args)
        .unwrap_or_else(|| panic!("`K::{method}` is compiled"))
        .unwrap_or_else(|status| panic!("`K::{method}` failed with status {status}"))
}

#[test]
fn compiled_code_and_the_runtime_agree_on_the_count_word() {
    let unit = compile(KEEPER).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    unit.script()
        .expect("the script frame was compiled")
        .call(&mut ctx)
        .expect("the script ran to completion");

    // For each kind: a static property and Rust hold the value. `keep` takes
    // Rust's reference and stores the value twice, one of them with the inline
    // retain. `drop` releases both copies through the helper, and the static
    // property keeps the value alive for the last read.
    let kinds = [
        ("text", "keepText", vec![Value::int(7)]),
        ("box", "keepBox", vec![]),
        ("list", "keepList", vec![Value::int(7)]),
    ];
    for (make, keep, args) in kinds {
        let value = call(&unit, &mut ctx, make, &args);
        let count = |v: &Value| {
            v.refcount()
                .unwrap_or_else(|| panic!("`{make}` is counted"))
        };
        assert_eq!(
            count(&value),
            2,
            "`{make}` and its static property hold the value"
        );
        call(&unit, &mut ctx, keep, &[value]);
        assert_eq!(count(&value), 3, "`{keep}` stored the `{make}` value twice");
        call(&unit, &mut ctx, "drop", &[]);
        assert_eq!(
            count(&value),
            1,
            "`drop` released both copies of the `{make}` value"
        );
    }

    // A literal's header is never written, by the inline retain or the helper.
    let literal = call(&unit, &mut ctx, "literal", &[]);
    assert_eq!(literal.refcount(), Some(nvs_runtime::IMMORTAL_REFCOUNT));
    call(&unit, &mut ctx, "keepText", &[literal]);
    assert_eq!(literal.refcount(), Some(nvs_runtime::IMMORTAL_REFCOUNT));
    call(&unit, &mut ctx, "drop", &[]);
    assert_eq!(literal.refcount(), Some(nvs_runtime::IMMORTAL_REFCOUNT));
}

#[test]
fn compiled_code_names_no_retain_helper() {
    let program = lower(KEEPER);
    let text = nvs_codegen::clif(&program).expect("the unit compiled");
    for (name, retain) in [
        ("K::keepText", "nvs_str_retain"),
        ("K::keepBox", "nvs_object_retain"),
        ("K::keepList", "nvs_array_retain"),
    ] {
        let function = program
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("the unit lowers no `{name}`"));
        let retains = function
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst.kind, nvs_ir::ir::InstKind::Retain { .. }));
        assert!(retains, "`{name}` lowers no retain to emit inline");
        let clif = section(&text, name);
        assert!(!clif.contains(retain), "`{name}` calls `{retain}`:\n{clif}");
    }
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
