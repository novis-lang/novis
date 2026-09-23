- **A `# Known gaps` item's claims about the rest of the tree go stale, and an attribution pass
  reads them as current.** `crates/nvs-stdlib/src/uuid.rs`'s gap 1 waited on a
  `nvs_runtime::Tag::Bytes` variant that is live (`crates/nvs-runtime/src/value.rs:307`), and
  `docs/agent/playbook.md` spelled out `json.rs`'s gap *number*, which a moved item silently
  re-points. Grep the blocker a gap names, and the module's own path, before tagging or renumbering
  one. [until: reviewed 2026-09-10]
