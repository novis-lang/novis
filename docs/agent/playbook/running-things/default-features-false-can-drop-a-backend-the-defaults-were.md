- **`default-features = false` can drop a backend the defaults were choosing, and the failure is a
  `compile_error!` in a crate you never named.** `mysql_common`'s defaults pull `flate2/zlib`, which
  puts C in the graph, and turning them off leaves `flate2` with no compression backend at all. The
  fix is a direct workspace entry for `flate2` with `features = ["rust_backend"]`; read a vendored
  crate's `[features] default` before assuming `default-features = false` is only a slimming.
  [until: reviewed 2026-09-06]
