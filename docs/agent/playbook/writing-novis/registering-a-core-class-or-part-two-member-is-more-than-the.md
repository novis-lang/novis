- **Registering a `Core` class or Part Two member is more than the five edits: two ledgers under
  `crates/nvs-stdlib/tests/` must shrink.** `spec-classes-part-two-outstanding.txt` holds a line per
  unregistered §§ 14-19 class and `spec-members-part-two-outstanding.txt` one per member, and
  `spec_registry_coverage.rs` fails a stale line as loudly as a missing one, only from a full `-p
  nvs-stdlib` run. `grep -n '<Class>'` both files before writing the row; a class's first member
  strikes its class line and the header sentence counting rows too.
  [until: gone crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt]
