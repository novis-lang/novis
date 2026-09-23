- **The registry's guards are not all in `registry.rs`; `crates/nvs-stdlib/src/lib.rs`'s `mod tests`
  holds the roster-wide ones.** Every symbol resolving to an address, the symbol count matching rows
  plus row-less constructs, and symbol uniqueness live there, so a grep of `registry.rs` for how
  rows map to symbols finds only the constructor-versus-member check. Grep `lib.rs` too before
  designing around row-to-symbol mapping; `cargo test -p nvs-stdlib --lib` is what catches a
  collision. [until: reviewed 2026-09-06]
