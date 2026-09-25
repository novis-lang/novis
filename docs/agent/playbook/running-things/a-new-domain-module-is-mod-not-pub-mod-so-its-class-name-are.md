- **A new domain module is `mod`, not `pub mod`, so its `CLASS`/`NAME` are `pub(crate)`.** The
  workspace warns `unreachable_pub`, and half of `nvs-stdlib`'s modules are `pub mod` while the
  newer half is not, so copying `uuid.rs`'s `pub const NAME` into a privately-declared module is a
  warning at build time. `objmap.rs`'s `NAME` is the shape to copy. [until: gone crates/nvs-stdlib/src/uuid.rs:pub const NAME]
