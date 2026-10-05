//! An author's own line breaks survive, as `rule:tooling/fmt-never-reflows`
//! states it.
//!
//! Its own file rather than a case beside the base style's: that rule is the
//! one this formatter is defined against — no width limit anywhere, no reflow
//! decision to make — and a case proving a wrapped call came back wrapped is
//! about what the tool refuses to do rather than about a layout it imposes.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

#[test]
fn an_authors_line_break_inside_an_expression_is_kept() {
    // `rule:tooling/fmt-never-reflows`: the wide call stays on its line however
    // long it is, and the tall one stays broken. Its layout is
    // `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`'s, one item
    // per line one level in, and `crates/nvs-fmt/tests/list_layout.rs` holds
    // the rest of it.
    let mangled = "\
<?nvs
  var $wide = Core\\Arr::sum([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]);
    var $tall = Core\\Arr::sum([
        1,
    2,
          3,
]);
  echo $wide + $tall;
";
    let canonical = "\
<?nvs
var $wide = Core\\Arr::sum([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]);
var $tall = Core\\Arr::sum([
    1,
    2,
    3,
]);
echo $wide + $tall;
";
    assert_eq!(formatted(mangled), canonical);
}
