- **`failed to load manifest for workspace member` names a directory nobody put a crate in, and it
  fails the whole workspace.** The root `Cargo.toml` has `members = ["crates/*", "benches/*"]`,
  expanded before any crate is read, so a `.nvs` or data tree added under `benches/` breaks every
  build, test and clippy run until it is named in `exclude`. The tell is that nothing you touched is
  in the message. [until: gone Cargo.toml:benches/*]
