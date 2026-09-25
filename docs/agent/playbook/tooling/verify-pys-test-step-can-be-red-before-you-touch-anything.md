- **`nv verify`'s test step can be red before you touch anything, and the failure names `nvs-ir`,
  not the goal switch that caused it.** `crates/nvs-ir/tests/refusals.rs` attributes every lowering
  refusal to an open item of the live goal, so installing a new goal orphans every site the old
  goal's items claimed. `bun nv holes --unattributed` says whether it is yours; the gate stops there
  and hides the `.nvst` trees and clippy, so run those by hand until it is closed, and the test
  rightly refuses its own allowlist as the fix. [until: gone crates/nvs-ir/tests/refusals.rs]
