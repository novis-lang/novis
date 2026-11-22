//! A call that may run program code, or park its task, keeps the retain of a
//! slot it would otherwise be lent
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! `Shop::$items = M(Shop::$items, …)` lends the static property to `M` with
//! no retain when nothing `M` runs can write the property. Each case here has
//! a hook that does write it: a `toString` reached through `as ?string`, a
//! `compareTo` that `Core\Arr::sort` runs on an element, or a sibling task
//! that runs while `M` waits. The hook empties the property and allocates many
//! strings of the same size, so a lent array is freed while `M` still uses it.
//!
//! A case that runs prints the same text with in-place changes on and off. A
//! case that cannot run here, because a test has no task scheduler, reads the
//! lowered function and finds the retain.

mod common;

use common::*;
use nvs_ir::ir::InstKind;

/// The class every case writes into, with the method that allocates.
const SHOP: &str = r#"
class Shop {
    public static array<string> $items = [];

    public static function fill(): void {
        array<string> $fill = [];
        for (int $i = 0; $i < 256; $i++) { $fill[] = Core\Str::repeat("y", 40); }
    }
}
"#;

/// Whether `Cart::run` keeps a retain of a value it read from a static
/// property. A lent slot has none.
fn retains_the_static(program: &nvs_ir::Program) -> bool {
    let run = program
        .functions
        .iter()
        .find(|function| function.name.ends_with("Cart::run"))
        .expect("Cart::run was lowered");
    let insts = || run.blocks.iter().flat_map(|block| &block.insts);
    let reads: Vec<_> = insts()
        .filter(|inst| matches!(inst.kind, InstKind::StaticGet { .. }))
        .filter_map(|inst| inst.result)
        .collect();
    insts()
        .any(|inst| matches!(inst.kind, InstKind::Retain { operand } if reads.contains(&operand)))
}

/// Runs `source` both ways, and checks both print `expected`.
fn prints_both_ways(case: &str, source: &str, expected: &str) {
    let in_place = common::lower(source);
    let copying = nvs_ir::lower::without_in_place(|| common::lower(source));
    assert_eq!(output_of_program(&in_place), expected, "{case}, in place");
    assert_eq!(
        output_of_program(&copying),
        expected,
        "{case}, not in place"
    );
}

/// The program of the review that found the hole: `as ?string` runs a
/// `toString` that empties the property the caller lent.
#[test]
fn a_to_string_reached_through_as_keeps_the_lent_array() {
    let source = format!(
        r#"<?nvs
{SHOP}
class Cart {{
    public static function add(array<string> $xs, mixed $m): array<string> {{
        $xs[] = Core\Str::length(($m as ?string) ?? "") > 0 ? "t" : "f";
        return $xs;
    }}

    public static function run(): void {{
        Shop::$items = [Core\Str::repeat("a", 40), Core\Str::repeat("b", 40)];
        Shop::$items = Cart::add(Shop::$items, new Post());
        echo Core\Json::encode(Shop::$items), "\n";
    }}
}}
class Post {{
    public function toString(): string {{
        Shop::$items = [];
        Shop::fill();
        return "zz";
    }}
}}
Cart::run();
"#
    );
    let expected = format!("[\"{}\",\"{}\",\"t\"]\n", "a".repeat(40), "b".repeat(40));
    prints_both_ways("toString through as ?string", &source, &expected);
    assert!(
        retains_the_static(&common::lower(&source)),
        "the property was lent to a call that runs toString"
    );
}

/// `Core\Arr::sort` runs `compareTo` on each pair of elements. The hook
/// empties the property the sort was lent.
#[test]
fn a_compare_to_run_by_a_core_member_keeps_the_lent_array() {
    let source = format!(
        r#"<?nvs
{SHOP}
class Item implements Comparable {{
    public int $n = 0;

    public function constructor(int $n) {{
        $this->n = $n;
    }}

    public function compareTo(self $other): int {{
        Cart::$items = [];
        Shop::fill();
        return $this->n <=> $other->n;
    }}
}}
class Cart {{
    public static array<Item> $items = [];

    public static function run(): void {{
        Cart::$items = [new Item(3), new Item(1), new Item(2)];
        Cart::$items = Core\Arr::sort(Cart::$items);
        foreach (Cart::$items as Item $item) {{
            echo $item->n;
        }}
        echo "\n";
    }}
}}
Cart::run();
"#
    );
    prints_both_ways("compareTo run by Core\\Arr::sort", &source, "123\n");
    assert!(
        retains_the_static(&common::lower(&source)),
        "an array of objects was lent to a member that runs compareTo"
    );
}

/// An array of scalars runs no hook, so the sort is still lent the property.
#[test]
fn a_sort_of_scalars_is_still_lent_the_array() {
    let source = r#"<?nvs
class Cart {
    public static array<int> $items = [];

    public static function run(): void {
        Cart::$items = [3, 1, 2];
        Cart::$items = Core\Arr::sort(Cart::$items);
        echo Core\Json::encode(Cart::$items), "\n";
    }
}
Cart::run();
"#;
    prints_both_ways("a sort of integers", source, "[1,2,3]\n");
    assert!(
        !retains_the_static(&common::lower(source)),
        "an array of integers was not lent to the sort"
    );
}

/// A callee that waits parks its task, and a sibling task of a
/// `Core\Task::all` group may write the property meanwhile. The callee is not
/// lent the property.
#[test]
fn a_callee_that_can_park_is_not_lent_the_array() {
    let source = format!(
        r#"<?nvs
{SHOP}
class Cart {{
    public static function waitThenAdd(array<string> $xs): array<string> {{
        Core\Time::sleep(1ms);
        $xs[] = "t";
        return $xs;
    }}

    public static function run(): void {{
        Shop::$items = [Core\Str::repeat("a", 40)];
        Core\Task::all({{
            add: fn(): int => {{
                Shop::$items = Cart::waitThenAdd(Shop::$items);
                return 1;
            }},
            clear: fn(): int => {{
                Shop::$items = [];
                Shop::fill();
                return 2;
            }},
        }});
        Shop::$items = Cart::waitThenAdd(Shop::$items);
        echo Core\Arr::count(Shop::$items), "\n";
    }}
}}
Cart::run();
"#
    );
    assert!(
        retains_the_static(&common::lower(&source)),
        "the property was lent to a callee that can park"
    );
}
