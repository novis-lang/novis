- **A module doc's gap *number* is not a stable anchor, so a comment citing one drifts silently.**
  Striking a gap renumbers every item after it, and the citations elsewhere in the same file keep
  pointing at the old position — `crates/nvs-stdlib/src/json.rs` held five "this module's gap 1" that
  meant three different things. Cite the owning `# ` section by its title instead, and when you do
  strike a gap, grep the file and `docs/agent/goals/` for `gap [0-9]`.
  [until: reviewed 2026-09-16]
