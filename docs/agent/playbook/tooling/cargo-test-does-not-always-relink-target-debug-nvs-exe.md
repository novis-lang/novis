- **`cargo test` does not always relink `target/debug/nvs.exe`.** A stale binary reports a member
  you just registered as `mixed`, which reads as a registry bug. Run `cargo build` before
  running a fixture or a `.nvst` case by hand. [until: gone AGENTS.md:cargo build]
