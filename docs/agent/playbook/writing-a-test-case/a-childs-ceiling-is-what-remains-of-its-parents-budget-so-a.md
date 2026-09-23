- **A child's ceiling is what *remains* of its parent's budget, so a sub-cap case asserting an exact
  byte count fails by a few kilobytes.** `Ctx::narrow_under` resolves a narrowing against `Remains`
  (`crates/nvs-runtime/src/ctx/isolate.rs:590`), and the shortfall is whatever the run has spent by
  then, which is not a constant. Assert the direction — tighter than the parent's ceiling for a
  narrowing, no wider for a refused widening — reading the parent's own number while it is still
  reachable. [until: gone crates/nvs-runtime/src/ctx/isolate.rs:Remains]
