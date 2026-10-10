- **One new row in `nvs_stdlib::registry::CLASSES` owes four gates, three of them outside the crate
  you are editing.** `every_registered_member_has_an_implementation_address` wants an address, `every_part_one_member_has_a_conformance_case` wants a case under
  `tests/conformance/` calling it, and `every_core_class_has_a_conformance_floor_of_three` wants
  three. A member whose runtime is a later slice can still register: the coverage gate greps the `--FILE--` section, so `--EXPECTF-ERROR--` cases discharge
  it, and the address gate takes a body saying why it ran, never a placeholder `Fault::`. `until: gone crates/nvs-stdlib/src/lib.rs:every_registered_member_has_an_implementation_address`
