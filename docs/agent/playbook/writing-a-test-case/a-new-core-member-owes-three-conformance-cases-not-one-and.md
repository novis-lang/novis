- **A new `Core` member owes three conformance cases, not one, and the second gate says so late.**
  `every_part_one_member_has_a_conformance_case` wants one;
  `every_core_class_has_a_conformance_floor_of_three` counts distinct case files per member, and a
  repeated question does not. Budget three shapes, run `cargo test --test
  conformance_coverage` first, and never add to `BELOW_THE_FLOOR`.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const FLOOR: usize = 3]
