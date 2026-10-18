- **A gate's name records the goal that wrote it, not the set it walks; read the test body before
  believing the name.** `every_part_one_member_has_a_conformance_case` iterates `registry::CLASSES`
  whole with no filter for the pure classes, because the registry held those and nothing else when
  it was written, and the floor gate beside it is the same. A twin for the other classes is a
  direction rather than a
  hole, and one `sed -n` over the named test's body settles it faster than a chain of inferences
  from its name. See `crates/nvs-stdlib/tests/conformance_coverage.rs`. `until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_part_one_member_has_a_conformance_case`
