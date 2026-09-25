- **A `Scope::Path` capability root in a case's `nvs.toml` has to be absolute; a relative one
  refuses everything while reading like a grant.** `nvs_config::capability`'s matcher canonicalises
  the queried path and asks `path.starts_with(root)` with the root as written, so `read =
  ["scratch.db"]` never matches and the refusal is the ordinary "not granted" sentence. Grant an
  absolute root; `:memory:` is not a path and cannot be granted. [until: gone crates/nvs-config/src/capability.rs:starts_with]
