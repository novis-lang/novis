- **A `Core` member row cannot land without its body and three cases, so a handoff group that splits
  "register the member" from "write the body" has no green state in between.**
  `every_part_one_member_has_a_conformance_case` and `every_core_class_has_a_conformance_floor_of_three`
  both read the registry, and `BELOW_THE_FLOOR` is empty and only shrinks, so the row is red from the
  moment it exists until three cases call it. Cut the slices so the whole member — row, card, helper,
  `address()` arm and cases — lands under one verification, and let the per-slice commits be the
  seam instead. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_core_class_has_a_conformance_floor_of_three]
