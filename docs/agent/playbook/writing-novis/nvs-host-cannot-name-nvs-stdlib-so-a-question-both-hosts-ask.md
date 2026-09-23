- **`nvs-host` cannot name `nvs-stdlib`, so a question both hosts ask belongs in `nvs-runtime`.**
  `nvs-stdlib` depends on `nvs-host` for `Core\Http\Client`'s socket, so a handoff item or comment
  spelling `nvs_stdlib::…` is unreachable from `crates/nvs-host/src/`, and the error reads like a
  missing manifest line rather than like the cycle it would be. Move the predicate and its constant
  down to `nvs-runtime`, and leave a `pub use` in the stdlib module so callers outside the host keep
  their spelling. [until: gone crates/nvs-stdlib/Cargo.toml:nvs-host.workspace]
