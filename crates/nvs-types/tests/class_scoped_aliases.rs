//! `rule:types/type-alias`'s class-scoped member, in the checker: the two
//! spellings a body's own alias is reached by, and the name left of a `::`
//! when that name is itself an alias.
//!
//! Transparency is what these assert. A shape named `Order::Meta`, named
//! `Meta` inside `Order`, and written at file scope is **one** interned type,
//! so the assertions compare `TypeId`s rather than reading each answer off its
//! own line: two spellings that each look right alone and intern apart is
//! exactly the failure a per-spelling assertion cannot see.

mod common;

use common::*;
use nvs_diagnostics::code;

#[test]
fn owner_name_and_a_bare_name_inside_the_owner_lower_to_the_file_scope_shape() {
    let (diags, declared) = check_src_declared(
        "<?nvs
type Loose = {total: int, note?: string};
class Order {
  type Meta = {total: int, note?: string};
  function inside(Meta $bare): void {}
}
class Reader {
  function outside(Order::Meta $qualified, Loose $file): void {}
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let bare = declared.of("Meta", "$bare");
    let qualified = declared.of("Order::Meta", "$qualified");
    let file = declared.of("Loose", "$file");
    assert_eq!(
        bare, qualified,
        "a bare `Meta` inside `Order` is the alias `Order::Meta` names"
    );
    assert_eq!(
        qualified, file,
        "an alias is its expansion, so the member and the file-scope form are one type"
    );
}

#[test]
fn an_alias_left_of_a_double_colon_expands_before_the_member_is_read() {
    let diags = check_src(
        "<?nvs\ntype Ids = array<int>;\nclass Reader {\n  function m(Ids::Read $x): void {}\n}\n",
    );
    let reported = diags
        .iter()
        .find(|d| d.code == Some(code::E_UNKNOWN_MEMBER))
        .unwrap_or_else(|| panic!("`Ids::Read` reads a member of `array<int>`: {diags:?}"));
    assert!(
        reported.message.contains("expands to") && reported.message.contains("array"),
        "the diagnostic names what the alias expands to: {reported:?}"
    );
}
