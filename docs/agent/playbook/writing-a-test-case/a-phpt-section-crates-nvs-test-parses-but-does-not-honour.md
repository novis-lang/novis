- **A `.phpt` section `crates/nvs-test` parses but does not honour fails the case outright; a member
  needing one is two slices.** `case.rs`'s `NOT_YET` table reports such a case `unsupported`, and
  each entry names a blocker that may have stopped being true. Before a case that sets up its own
  world, read `NOT_YET`, not the section table in `crates/nvs-test/src/lib.rs`, which lists sections
  that do nothing too. [until: gone crates/nvs-test/src/case.rs:const NOT_YET]
