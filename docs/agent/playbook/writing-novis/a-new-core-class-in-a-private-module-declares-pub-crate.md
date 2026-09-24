- **A new `Core` class in a private module declares `pub(crate) const CLASS`, not `pub`.**
  `conventions.md`'s worked example is `crates/nvs-stdlib/src/json.rs`, a `pub mod`, so copying its
  `pub const CLASS` into a private `mod` warns `unreachable pub item`, which the workspace lint
  policy makes a `nv verify` failure. Every private domain module (`env.rs`, `cap.rs`, `out.rs`)
  spells `CLASS` and `address` `pub(crate)`.
  [until: gone crates/nvs-stdlib/src/json.rs:pub const CLASS: CoreClass]
