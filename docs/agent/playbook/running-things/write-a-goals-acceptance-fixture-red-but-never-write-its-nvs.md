- **Write a goal's acceptance fixture red, but never write its `nvs.toml` block red.** A `.nvs`
  fixture naming a missing member fails at `E0405` and costs only itself, but `deny_unknown_fields`
  sits on every struct in `crates/nvs-config/src/tree.rs`, so an unrecognised table fails at boot
  for every program in the repository. The config block belongs to the slice that adds the struct
  that reads it. [until: gone crates/nvs-config/src/tree.rs:deny_unknown_fields]
