- **A check naming a *build-time* join against the registry cannot be all in `build.rs`.** A build
  script is compiled and run before the crate it belongs to, so `nvs_stdlib::registry::class` is
  unreachable from `crates/nvs-stdlib/build.rs` and only the inputs that are *documents* can be
  joined there. Split it the way the layer splits — the documents in the build script, the registry
  half in the module the generated table lands in, and the refusal in a `-p <crate>` test, which
  under `nv verify`'s stop-at-the-first-failure is the build that does not finish.
  [until: gone crates/nvs-stdlib/build.rs:build.rs]
