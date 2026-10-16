//! A list is on one line or has one item per line, and a line break its author
//! wrote at the list's own level is what breaks it
//! (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`).
//!
//! Each case is the smallest file that shows one half of the rule: what counts
//! as a break at the list's own level, what does not, and the one layout a
//! broken list gets. A call's arguments, a parameter list, an array literal,
//! an anonymous object and an enum's cases are the lists these cases cover.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// Asserts `written` formats to `canonical`, and `canonical` to itself.
fn formats_to(written: &str, canonical: &str) {
    assert_eq!(formatted(written), canonical);
    assert_eq!(
        formatted(canonical),
        canonical,
        "formatting is a fixed point"
    );
}

#[test]
fn a_list_on_one_line_stays_on_one_line_however_long() {
    let wide = "<?nvs\nvar $wide = Core\\Arr::sum([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);\n";
    formats_to(wide, wide);
    let trailing = "<?nvs\nvar $one = Core\\Arr::sum([1, 2, 3,]);\n";
    formats_to(trailing, "<?nvs\nvar $one = Core\\Arr::sum([1, 2, 3]);\n");
}

#[test]
fn a_break_between_two_items_puts_every_item_on_its_own_line() {
    formats_to(
        "<?nvs\n$order = Shop::place($customer,\n    $basket, $address);\n",
        "<?nvs\n$order = Shop::place(\n    $customer,\n    $basket,\n    $address,\n);\n",
    );
}

#[test]
fn a_break_after_the_opener_breaks_the_list() {
    formats_to(
        "<?nvs\nvar $total = Core\\Arr::sum([\n1, 2, 3]);\n",
        "<?nvs\nvar $total = Core\\Arr::sum([\n    1,\n    2,\n    3,\n]);\n",
    );
}

#[test]
fn a_break_before_the_closer_breaks_the_list() {
    formats_to(
        "<?nvs\nvar $total = Core\\Arr::sum([1, 2, 3\n]);\n",
        "<?nvs\nvar $total = Core\\Arr::sum([\n    1,\n    2,\n    3,\n]);\n",
    );
}

#[test]
fn a_break_inside_a_function_body_does_not_break_the_call() {
    let canonical = "\
<?nvs
var $names = Core\\Arr::map($users, fn(User $user): string => {
    return Core\\Str::upper($user->name);
});
";
    formats_to(canonical, canonical);
}

#[test]
fn a_break_inside_a_nested_array_does_not_break_the_outer_list() {
    formats_to(
        "<?nvs\nvar $total = Core\\Arr::sum([1,\n2], 3);\n",
        "<?nvs\nvar $total = Core\\Arr::sum([\n    1,\n    2,\n], 3);\n",
    );
    formats_to(
        "<?nvs\nvar $grid = [[1, 2],\n[3,\n4]];\n",
        "<?nvs\nvar $grid = [\n    [1, 2],\n    [\n        3,\n        4,\n    ],\n];\n",
    );
}

#[test]
fn a_multi_line_item_moves_in_with_its_first_line() {
    formats_to(
        "\
<?nvs
var $names = Core\\Arr::map($users,
fn(User $user): string => {
    return $user->name;
});
",
        "\
<?nvs
var $names = Core\\Arr::map(
    $users,
    fn(User $user): string => {
        return $user->name;
    },
);
",
    );
}

#[test]
fn a_parameter_list_follows_the_list_rule() {
    let one_line = "<?nvs\nclass Shop\n{\n    public function place(Customer $customer, { name: string, count: int } $line, array<int> $basket = [], int $count = 1): int\n    {\n        return $count;\n    }\n}\n";
    formats_to(one_line, one_line);
    formats_to(
        "<?nvs\nclass Shop\n{\n    public function place(Customer $customer,\n        array<int> $basket = [1,\n2]): int\n    {\n        return 1;\n    }\n}\n",
        "<?nvs\nclass Shop\n{\n    public function place(\n        Customer $customer,\n        array<int> $basket = [\n            1,\n            2,\n        ],\n    ): int\n    {\n        return 1;\n    }\n}\n",
    );
    formats_to(
        "<?nvs\nvar $add = fn(int $a,\nint $b): int => $a + $b;\n",
        "<?nvs\nvar $add = fn(\n    int $a,\n    int $b,\n): int => $a + $b;\n",
    );
}

#[test]
fn an_anonymous_object_follows_the_list_rule() {
    formats_to(
        "<?nvs\n$log->write({ level: 'info',\nmessage: 'started' }, true);\n",
        "<?nvs\n$log->write({\n    level: 'info',\n    message: 'started',\n}, true);\n",
    );
    let one_line = "<?nvs\n$log->write({ level: 'info', message: 'started' }, true);\n";
    formats_to(one_line, one_line);
}

#[test]
fn an_enum_on_one_line_keeps_its_brace_on_the_enum_line() {
    let canonical = "<?nvs\nenum AxisPosition { Left, Right }\n";
    formats_to("<?nvs\nenum AxisPosition {Left, Right,}\n", canonical);
    formats_to("<?nvs\nenum AxisPosition\n{ Left, Right }\n", canonical);
}

#[test]
fn a_broken_enum_has_an_allman_brace_and_one_case_per_line() {
    formats_to(
        "<?nvs\nenum Permission: uint { Read = 0b001,\nWrite = 0b010 }\n",
        "<?nvs\nenum Permission: uint\n{\n    Read = 0b001,\n    Write = 0b010,\n}\n",
    );
}

#[test]
fn a_line_comment_breaks_the_list_and_stays_with_its_item() {
    let canonical = "\
<?nvs
$order = Shop::place(
    $customer, // who pays
    // what they bought
    $basket,
);
";
    formats_to(
        "<?nvs\n$order = Shop::place($customer, // who pays\n        // what they bought\n  $basket);\n",
        canonical,
    );
}

#[test]
fn a_broken_list_is_a_fixed_point() {
    let canonical = "<?nvs\nvar $steps = [\n    'build',\n\n    'test',\n];\n";
    formats_to("<?nvs\nvar $steps = ['build',\n\n'test'];\n", canonical);
    formats_to(canonical, canonical);
}
