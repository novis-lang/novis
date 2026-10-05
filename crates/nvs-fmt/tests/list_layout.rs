//! A list is on one line or has one item per line, and a line break its author
//! wrote at the list's own level is what breaks it
//! (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`).
//!
//! Each case is the smallest file that shows one half of the rule: what counts
//! as a break at the list's own level, what does not, and the one layout a
//! broken list gets. A call's arguments, an array literal and an anonymous
//! object are the lists these cases cover.

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
fn a_break_between_two_arguments_puts_each_argument_on_a_line() {
    formats_to(
        "<?nvs\n$order = Shop::place($customer,\n    $basket, $address);\n",
        "<?nvs\n$order = Shop::place(\n    $customer,\n    $basket,\n    $address,\n);\n",
    );
}

#[test]
fn a_break_after_the_opener_or_before_the_closer_breaks_the_list() {
    let canonical = "<?nvs\nvar $total = Core\\Arr::sum([\n    1,\n    2,\n    3,\n]);\n";
    formats_to(
        "<?nvs\nvar $total = Core\\Arr::sum([\n1, 2, 3]);\n",
        canonical,
    );
    formats_to(
        "<?nvs\nvar $total = Core\\Arr::sum([1, 2, 3\n]);\n",
        canonical,
    );
}

#[test]
fn a_list_with_no_break_at_its_own_level_stays_on_one_line() {
    let wide = "<?nvs\nvar $wide = Core\\Arr::sum([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);\n";
    formats_to(wide, wide);
    let trailing = "<?nvs\nvar $one = Core\\Arr::sum([1, 2, 3,]);\n";
    formats_to(trailing, "<?nvs\nvar $one = Core\\Arr::sum([1, 2, 3]);\n");
}

#[test]
fn a_break_inside_an_item_does_not_break_the_list_around_it() {
    let canonical = "\
<?nvs
var $names = Core\\Arr::map($users, fn(User $user): string => {
    return Core\\Str::upper($user->name);
});
";
    formats_to(canonical, canonical);
}

#[test]
fn a_nested_list_is_judged_on_its_own() {
    formats_to(
        "<?nvs\nvar $grid = [[1, 2],\n[3,\n4]];\n",
        "<?nvs\nvar $grid = [\n    [1, 2],\n    [\n        3,\n        4,\n    ],\n];\n",
    );
}

#[test]
fn an_anonymous_object_and_a_method_call_are_lists_too() {
    formats_to(
        "<?nvs\n$log->write({ level: 'info',\nmessage: 'started' }, true);\n",
        "<?nvs\n$log->write({\n    level: 'info',\n    message: 'started',\n}, true);\n",
    );
}

#[test]
fn a_comment_keeps_the_line_it_was_written_on() {
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
fn a_blank_line_between_two_items_is_kept() {
    let canonical = "<?nvs\nvar $steps = [\n    'build',\n\n    'test',\n];\n";
    formats_to("<?nvs\nvar $steps = ['build',\n\n'test'];\n", canonical);
}
