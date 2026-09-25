- **`nvs-server` cannot name a `Diagnostic`, so a boot reader that *refuses* cannot live there.**
  `nvs-diagnostics` is a dev-dependency of that crate and `nvs-config` re-exports no diagnostic type,
  so the natural `Connection::from_config(config, origins) -> Result<_, Diagnostic>` does not compile
  however obvious it reads. Put the keys, the parse and the refusal in `nvs-config` and hand the
  server crate the resolved overrides — `nvs_config::server::connection_bounds_for` plus
  `nvs_server::bounds::Connection::configured` is that split, and it also keeps the shipped numbers in
  one crate. [until: gone crates/nvs-server/src/bounds.rs:connection_bounds_for]
