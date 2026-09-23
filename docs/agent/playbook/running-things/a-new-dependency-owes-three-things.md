- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying
  why that crate, `cargo deny check`, and `python tools/gen-attribution.py`
  (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). A license identifier
  new to the tree goes into both `deny.toml`'s allow list and `tools/gen-attribution.py`'s
  `PREFERENCE` in the same commit, because that script fails if the two disagree. The dependency
  sweep is a pass the user fires by hand
  (`rule:packaging/a-dependency-break-is-absorbed-never-forwarded`); never start it as a side
  effect. [until: reviewed 2026-09-06]
