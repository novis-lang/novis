- **A new Core member owes three conformance cases, not the one the five-edit recipe names.**
  `conformance_coverage.rs`'s `every_core_class_has_a_conformance_floor_of_three` counts cases per
  member and fails with "asked by 1 case(s)", and a second case asking the same question does not
  count. Spend the three on the conventions' depth shapes while the member is fresh; one file can
  ask for two members landed together.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_core_class_has_a_conformance]
