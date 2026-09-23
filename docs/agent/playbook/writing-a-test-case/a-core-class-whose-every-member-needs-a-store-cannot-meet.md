- **A `Core` class whose every member needs a store cannot meet the conformance floor with a happy
  path; the floor is a gate, not a target.** `conformance_coverage.rs` fails `cargo test -p
  nvs-stdlib` below three `.nvst` cases per member, so a `Core\Session` row cannot land before its
  cases. Plan each member with its refusals: no `[session]` block, an unreachable backend, a call
  before `start`. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:BELOW_THE_FLOOR]
