- **A goal document or a manifest comment can state a dependency's graph as fact, and only `cargo
  tree -i` knows.** A crate taken with `default-features = false` can still pull a dependency
  unconditionally for one type, and the only place this shows is the `Adding <crate>` line in the
  first `cargo check`'s resolution list. `cargo tree -i <dep> -e normal` names the parent and
  `~/.cargo/registry/src/*/<crate>-<version>/Cargo.toml` says whether that edge is `optional` or
  feature-gated — do both *before* writing the comment that claims the graph, because the comment is
  what the next reader trusts. [until: gone Cargo.toml:default-features]
