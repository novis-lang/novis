- **One new row in `nvs_stdlib::registry::CLASSES` owes four gates, three of them outside the crate
  you are editing.** `every_registered_member_has_an_implementation_address` wants a symbol with a
  real address, `every_part_one_member_has_a_conformance_case` wants a `.nvst` under
  `tests/conformance/` whose `--FILE--` writes `Core\X::y(`,
  `every_core_class_has_a_conformance_floor_of_three` wants three, and `BELOW_THE_FLOOR` only
  shrinks. That is not a bar on registering a member whose runtime is a later slice: the coverage
  gate greps the `--FILE--` section and never runs anything, so `--EXPECTF-ERROR--` cases discharge
  it, and the address gate takes a body that says at its own site why it was reached — never a
  placeholder `Fault::`, which `every_error_path_is_asserted_or_declared_unreachable` then wants a
  case or a declaration for. [until: reviewed 2026-09-06]
