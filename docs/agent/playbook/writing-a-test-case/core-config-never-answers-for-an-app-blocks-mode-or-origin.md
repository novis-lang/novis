- **`Core\Config` never answers for an `[[app]]` block's `mode` or `origin`, so an example built on
  either prints `(unset)` while the setting is in force.** `Snapshot::build` lifts those two onto
  the snapshot's own fields and drops `app` from the folded table
  (`crates/nvs-config/src/snapshot.rs:158`), so a block is readable only through its `[app.limits]`
  and `[app.capabilities]` — as `limits.*` and `capabilities.*`, never under an `app.` prefix. Show
  a block through one of those, or through behaviour the way `examples/routes.nvs` does.
  [until: gone crates/nvs-config/src/snapshot.rs:table.remove("app")]
