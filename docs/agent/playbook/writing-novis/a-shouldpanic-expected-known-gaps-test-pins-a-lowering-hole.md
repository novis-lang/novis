- **A `#[should_panic(expected = "known gaps")]` test pins a lowering hole, so closing the hole
  turns it red.** The failure reads like a regression and is the opposite. `grep -n "should_panic"
  crates/<crate>/src` first, rewrite it as a snapshot test of what now happens, and run `cargo insta
  test -p <crate> --lib` (plain `cargo test` stops at the first `.snap.new`), checking `git status
  --short | grep pending-snap` before `cargo insta accept`. [until: reviewed 2026-09-06]
