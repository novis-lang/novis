- **The conformance coverage gate counts a static call only when the case writes its class in full.**
  A case with `use Core\Queue\Id;` and `Id::parse($s)` runs the member, yet
  `every_part_one_member_has_a_conformance_case` and `every_core_class_has_a_conformance_floor_of_three`
  count it as 0. They search for the text `Core\Queue\Id::parse(`. Write each static `Core` call in a
  new case as `Core\X\Y::member(`, and keep a `use` only for the type names. See
  `crates/nvs-stdlib/tests/conformance_coverage.rs`.
  `until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:let call = format!("{}::{}{opener}", class.name, method.name);`
