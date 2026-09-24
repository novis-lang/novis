- **`cargo fmt` after a run of Edit-tool changes floods the session with full-file diffs.** The harness
  reports every file a command touched that you had previously read, and a tree-wide `cargo fmt`
  touches all of them at once — one call cost about 12k of context here, more than the edits it was
  tidying. Format the crates you actually changed (`cargo fmt -p nvs-types -p nvs-ir`) or let
  `nv verify`'s fmt leg report instead, and reach for the tree-wide one only when the wrap is already
  written. [until: reviewed 2026-09-17]
