- **Widening `nvs_runtime::MethodRow` is about thirty edit sites, and a grep for the roster finds
  almost none of them.** It is a positional tuple in `nvs-ir`'s synthesized classes
  (`lower/closure.rs`, `lower/generator.rs`, which no search for `layout.methods` reaches) and a
  struct literal in two dozen test fixtures across five crates. Make the core edits, let `cargo
  build` enumerate the rest, and apply those as one `nv splice --patch` anchored on `param_tags:`
  plus the line under it — `grep -rn "param_tags:"` is the real roster.
  [until: gone crates/nvs-runtime/src/object.rs:pub struct MethodRow]
