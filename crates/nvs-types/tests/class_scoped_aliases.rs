//! `rule:types/type-alias`'s class-scoped member, in the checker: the two
//! spellings a body's own alias is reached by, the name left of a `::` when
//! that name is itself an alias, and the casing an alias's own name is held
//! to at both sites one can be written.
//!
//! The casing case reaches a different pass from the rest — `rule:core-api/identifier-casing` is
//! answered off the bare AST, before resolution — so it calls
//! `common::check_declarations_only` where the others call `check_src`.
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

/// Both sides of the bound, in one source: the file-scope form and the member
/// form are one production, so a name refused at one is refused at the other
/// and a `PascalCase` name at either is accepted. The suggestion is asserted
/// because it is what the diagnostic offers as the rename.
#[test]
fn a_type_alias_that_is_not_pascal_case_is_refused() {
    let diags = check_declarations_only(
        "<?nvs
type user_id = uint;
type Rows = array<int>;
class Order {
  type meta_row = {total: int};
  type Line = {sku: string};
}
",
    );
    let refused: Vec<_> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_BAD_TYPE_CASING))
        .collect();
    assert_eq!(
        refused.len(),
        2,
        "`user_id` and `meta_row`, and neither `Rows` nor `Line`: {diags:?}"
    );
    assert!(
        refused[0].message.contains("UserId") && refused[1].message.contains("MetaRow"),
        "each refusal names the PascalCase rename: {refused:?}"
    );
}
