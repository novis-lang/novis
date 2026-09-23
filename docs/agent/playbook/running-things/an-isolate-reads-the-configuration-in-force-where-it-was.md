- **An isolate reads the configuration in force where it was *spawned*, so an `[[app]]` block keyed
  on the child's own path never reaches it.** `Ctx::isolate` clones the parent's `Request`, and the
  snapshot was resolved once for the entry file, so the block that reaches a child is the one the
  parent matched. A block's sub-tables need no plumbing: `crate::snapshot`'s per-app fold merges
  every other key of the block onto the global table, so grep for the field on
  `nvs_config::tree::App` before assuming a block cannot carry a table — `deny_unknown_fields` is
  all that refuses it. [until: reviewed 2026-09-06]
