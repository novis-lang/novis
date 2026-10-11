- **A new directive has homes outside Rust.** The key is a field on `tree.rs`'s block struct,
  resolves in that block's module and is transcribed into `default.toml` — but the rule fragment
  printing the block's keys **is** the rule. Edit the fragment, then `bun nv rules --render`, or `nv verify` fails on the stale generated chapter — a sentence
  about a file nobody edits rather than about the key you added. `until: gone crates/nvs-config/src/tree.rs:because`
