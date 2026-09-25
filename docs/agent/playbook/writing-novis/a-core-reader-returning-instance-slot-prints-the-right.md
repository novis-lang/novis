- **A `Core` reader returning `instance::slot` prints the right answer and then corrupts the heap at
  teardown.** That helper *borrows* the slot, so a member handing one back to Novis over a string or
  object exits `-1073740940` after passing every assertion in the case, which `nvs test` reports as
  "expected the run to succeed" with no line number anywhere. Write a reader as
  `crate::instance::read_slot(args, &CLASS, AT, "member")`, which is the pair that borrows and
  retains, and read a crash with no diagnostic as a missing retain in the last member added.
  [until: gone crates/nvs-stdlib/src/instance.rs:fn read_slot]
