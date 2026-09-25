- **A `-p <crate>` check is impossible rather than unwritten when that crate's `Cargo.toml` does not
  name the crate owning the surface.** `crates/nvs-server/Cargo.toml` names no `nvs-stdlib`, so a
  check there about `files()` can never run. Read `crates/<crate>/Cargo.toml`'s `[dependencies]`
  first; adding a dev-dependency to make the filing true puts the test one layer above the rule it
  asserts. [until: gone crates/nvs-server/Cargo.toml:nvs-stdlib]
