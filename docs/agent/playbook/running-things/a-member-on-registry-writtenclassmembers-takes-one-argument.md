- **A member on `registry::WRITTEN_CLASS_MEMBERS` takes one argument its row does not declare** —
  the class its call site wrote, in slot 0 — so its helper's `args: [N]` is `params` + 1, plus the
  options bag's flattening. `crates/nvs-stdlib/tests/conformance_coverage.rs` looks for such a
  member spelled `Class::name<`, not `Class::name(`, because that is what every call site writes.
  [until: gone crates/nvs-stdlib/src/registry.rs:WRITTEN_CLASS_MEMBERS]
