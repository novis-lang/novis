//! `rule:attributes/a-deprecation-names-its-replacement-as-code` § *The fix
//! fills the template in* and § *Side effects keep their order and count*:
//! the edit each `W1003` carries, applied and compiled again.

mod common;

use common::{check_program_table, check_src};
use nvs_diagnostics::{Diagnostic, Diagnostics, code};

fn deprecations(diags: &Diagnostics) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::W_DEPRECATED))
        .collect()
}

/// Every safe edit the `W1003`s of `src` carry, applied from the end of the
/// file backwards. Two warnings that need the same import carry the same
/// edit, which is applied once.
fn apply(src: &str, diags: &Diagnostics) -> String {
    let mut edits: Vec<(u32, u32, String)> = deprecations(diags)
        .into_iter()
        .flat_map(|d| &d.suggestions)
        .filter(|s| s.safe)
        .map(|s| (s.span.start, s.span.end, s.replacement.clone()))
        .collect();
    edits.sort();
    edits.dedup();
    let mut out = src.to_owned();
    for (start, end, text) in edits.into_iter().rev() {
        out.replace_range(start as usize..end as usize, &text);
    }
    out
}

/// `src` with every fix applied, checked to compile again with no error and
/// no `W1003`.
fn fixed(src: &str) -> String {
    let diags = check_src(src);
    assert!(!diags.has_errors(), "{diags:?}");
    let out = apply(src, &diags);
    let again = check_src(&out);
    assert!(!again.has_errors(), "{out}\n{again:?}");
    assert!(deprecations(&again).is_empty(), "{out}\n{again:?}");
    out
}

const API: &str = r#"<?nvs
class Api {
  #[Core\Deprecated(since: '2.0', replace: '$this->find($id, limit: $limit)')]
  public function findById(int $id, int $limit = 10): int { return $this->find($id, limit: $limit); }
  public function find(int $id, int $limit): int { return $id + $limit; }
}
"#;

/// A parameter becomes the argument the use passed for it, whether the use
/// passed it by position or by name.
#[test]
fn a_deprecation_fix_fills_arguments_by_position_and_by_name() {
    let out = fixed(&format!(
        "{API}Api $api = new Api();\necho $api->findById(5, 20);\necho $api->findById(limit: 3, id: 7);\n"
    ));
    assert!(out.contains("echo $api->find(5, limit: 20);"), "{out}");
    assert!(out.contains("echo $api->find(7, limit: 3);"), "{out}");
}

/// A parameter the use leaves out becomes its default, as the declaration
/// writes it.
#[test]
fn a_deprecation_fix_writes_the_default_of_an_omitted_argument() {
    let out = fixed(&format!(
        "{API}Api $api = new Api();\necho $api->findById(5);\n"
    ));
    assert!(out.contains("echo $api->find(5, limit: 10);"), "{out}");
}

/// `$this` becomes the receiver, as the use writes it.
#[test]
fn a_deprecation_fix_writes_the_receiver_for_this() {
    let out = fixed(&format!(
        "{API}class Shop {{
  public Api $api;
  function constructor() {{ $this->api = new Api(); }}
  function run(): int {{ return $this->api->findById(1, 2); }}
}}
"
    ));
    assert!(
        out.contains("return $this->api->find(1, limit: 2);"),
        "{out}"
    );
}

/// An argument is parenthesized where it binds more loosely than its place
/// in the template, and nowhere else.
#[test]
fn a_deprecation_fix_parenthesizes_only_a_lower_precedence_argument() {
    let out = fixed(
        r#"<?nvs
class Calc {
  #[Core\Deprecated(replace: '$this->find($id * 2, limit: $limit - 1)')]
  public function twice(int $id, int $limit): int { return $this->find($id * 2, limit: $limit - 1); }
  public function find(int $id, int $limit): int { return $id + $limit; }
}
Calc $c = new Calc();
int $n = 4;
echo $c->twice($n + 1, $n * 3);
echo $c->twice($n * 3, $n - 2);
"#,
    );
    assert!(
        out.contains("echo $c->find(($n + 1) * 2, limit: $n * 3 - 1);"),
        "{out}"
    );
    assert!(
        out.contains("echo $c->find($n * 3 * 2, limit: $n - 2 - 1);"),
        "{out}"
    );
}

const TWICE: &str = r#"<?nvs
class Twice {
  #[Core\Deprecated(replace: '$this->sum($id, $id)')]
  public function double(int $id): int { return $this->sum($id, $id); }
  public function sum(int $a, int $b): int { return $a + $b; }
  public function next(): int { return 3; }
}
"#;

/// An argument with a side effect that the template uses twice is computed
/// once, on a `var` line in front of the statement.
#[test]
fn a_deprecation_fix_moves_an_impure_argument_used_twice_to_a_var_line() {
    let out = fixed(&format!(
        "{TWICE}class Shop {{
  function run(Twice $t): int {{
    return $t->double($t->next());
  }}
}}
"
    ));
    assert!(
        out.contains("    var $id = $t->next();\n    return $t->sum($id, $id);"),
        "{out}"
    );
}

/// Arguments with side effects that the template uses in another order are
/// computed first, in the order the use wrote them.
#[test]
fn a_deprecation_fix_moves_impure_arguments_the_template_reorders() {
    let out = fixed(
        r#"<?nvs
class Pair {
  #[Core\Deprecated(replace: '$this->make($second, $first)')]
  public function swap(int $first, int $second): int { return $this->make($second, $first); }
  public function make(int $a, int $b): int { return $a - $b; }
  public function one(): int { return 1; }
  public function two(): int { return 2; }
}
class Shop {
  function run(Pair $p): int {
    return $p->swap($p->one(), $p->two());
  }
}
"#,
    );
    assert!(
        out.contains(
            "    var $first = $p->one();\n    var $second = $p->two();\n    \
             return $p->make($second, $first);"
        ),
        "{out}"
    );
}

/// An argument with a side effect that the template does not use still runs,
/// on a `var` line of its own.
#[test]
fn a_deprecation_fix_keeps_an_impure_argument_the_template_drops() {
    let out = fixed(
        r#"<?nvs
class Store {
  #[Core\Deprecated(replace: '$this->get($id)')]
  public function fetch(int $id, int $retries): int { return $this->get($id); }
  public function get(int $id): int { return $id; }
  public function count(): int { return 2; }
}
class Shop {
  function run(Store $s): int {
    return $s->fetch(1, $s->count());
  }
}
"#,
    );
    assert!(
        out.contains("    var $retries = $s->count();\n    return $s->get(1);"),
        "{out}"
    );
}

/// Where a `var` line in front of the statement would run at another time
/// than the use, there is no fix, and the help names the template.
#[test]
fn a_deprecation_fix_is_not_offered_where_a_var_line_would_change_when_the_code_runs() {
    let diags = check_src(&format!(
        "{TWICE}class Shop {{
  function a(Twice $t, bool $ok): bool {{ return $ok && $t->double($t->next()) > 0; }}
  function b(Twice $t): void {{ while ($t->double($t->next()) > 9) {{ }} }}
  function c(Twice $t, bool $ok): int {{ return $ok ? 0 : $t->double($t->next()); }}
  function d(Twice $t): int {{ return $t->double($t->next()); }}
}}
"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let found = deprecations(&diags);
    assert_eq!(found.len(), 4, "{found:#?}");
    let fixed: Vec<_> = found.iter().filter(|d| !d.suggestions.is_empty()).collect();
    assert_eq!(fixed.len(), 1, "{found:#?}");
    for d in found.iter().filter(|d| d.suggestions.is_empty()) {
        assert!(
            d.notes.iter().any(|n| n.contains("`$this->sum($id, $id)`")),
            "{d:#?}"
        );
    }
}

/// A class the template names is written as the shortest name the use's file
/// resolves to it, with a `use` line where that name needs one.
#[test]
fn a_deprecation_fix_writes_a_name_the_use_site_resolves_and_adds_its_import() {
    const SHOP: &str = r#"<?nvs
namespace Shop;
class Money { public static function of(int $c): int { return $c; } }
class Api {
  #[Core\Deprecated(replace: 'Money::of($cents)')]
  public static function cents(int $cents): int { return Money::of($cents); }
}
class Till { public function total(): int { return Api::cents(1); } }
"#;
    const PAGE: &str = r#"<?nvs
namespace Blog;
require './shop.nvs';
use Shop\Api;
class Page { public function total(): int { return Api::cents(5); } }
"#;
    let (diags, _) = check_program_table(&[("page.nvs", PAGE), ("shop.nvs", SHOP)]);
    assert!(!diags.has_errors(), "{diags:?}");
    let found = deprecations(&diags);
    assert_eq!(found.len(), 2, "{found:#?}");
    let page = found
        .iter()
        .find(|d| d.suggestions.len() == 2)
        .expect("the use in another namespace carries an import");
    assert_eq!(page.suggestions[0].replacement, "Money::of(5)");
    assert!(
        page.suggestions[1]
            .replacement
            .ends_with("use Shop\\Money;"),
        "{page:#?}"
    );
    let till = found
        .iter()
        .find(|d| d.suggestions.len() == 1)
        .expect("the use in the same namespace needs no import");
    assert_eq!(till.suggestions[0].replacement, "Money::of(1)");

    let entry = page.suggestions[0].span.file;
    let mut in_page = Diagnostics::new();
    in_page.extend(
        found
            .into_iter()
            .filter(|d| d.suggestions[0].span.file == entry)
            .cloned(),
    );
    let out = apply(PAGE, &in_page);
    let (again, _) = check_program_table(&[("page.nvs", &out), ("shop.nvs", SHOP)]);
    assert!(!again.has_errors(), "{out}\n{again:?}");
    assert_eq!(deprecations(&again).len(), 1, "{out}\n{again:?}");
}

/// A deprecated class is renamed to its replacement wherever a use writes it:
/// a type, a static call, a constant and a `new`.
#[test]
fn a_deprecation_fix_renames_a_deprecated_class_in_every_position() {
    let out = fixed(
        r#"<?nvs
class Cart {
  const int MAX = 3;
  public static function make(): Cart { return new Cart(); }
}
#[Core\Deprecated(since: '2.0', replace: 'Cart')]
class Basket {
  const int MAX = 3;
  public static function make(): Cart { return new Cart(); }
}
class Shop {
  function take(Basket $b): int { return Basket::MAX; }
  function build(): Cart { return Basket::make(); }
  function fresh(): Basket { return new Basket(); }
}
"#,
    );
    let shop = &out[out.find("class Shop").expect("the class is still there")..];
    assert!(!shop.contains("Basket"), "{out}");
    assert!(
        shop.contains("function take(Cart $b): int { return Cart::MAX; }"),
        "{out}"
    );
    assert!(
        shop.contains("function fresh(): Cart { return new Cart(); }"),
        "{out}"
    );
}

/// Every fix applied together compiles with no `W1003` left, and fixing the
/// result again changes nothing.
#[test]
fn every_deprecation_fix_compiles_without_w1003_and_a_second_fix_changes_nothing() {
    let src = format!(
        "{API}{}class Shop {{
  public Api $api;
  function constructor() {{ $this->api = new Api(); }}
  function run(Twice $t): int {{
    int $n = $this->api->findById($t->next(), limit: 2);
    return $t->double($t->next()) + $this->api->findById($n);
  }}
}}
",
        TWICE.trim_start_matches("<?nvs\n")
    );
    let once = fixed(&src);
    assert_ne!(once, src);
    let diags = check_src(&once);
    assert_eq!(apply(&once, &diags), once);
}
