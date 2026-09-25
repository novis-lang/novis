- **A block inserted directly above an item lands inside the doc comment of the item before it, and
  everything still compiles.** `///` lines attach to whatever item follows them, so a new `struct`
  anchored on the line of an existing one ends up wearing the first half of that item's comment
  while the old item keeps the second — two docs that each read as a non-sequitur, with no warning
  anywhere (`crates/nvs-cli/src/serve.rs`'s `FleetLease` and `Scheduled` were one, repaired here).
  Anchor an insertion on the blank line *above* a doc comment rather than on the item, and read the
  `///` lines on both sides of the seam back afterwards. [until: gone crates/nvs-cli/src/serve.rs:FleetLease]
