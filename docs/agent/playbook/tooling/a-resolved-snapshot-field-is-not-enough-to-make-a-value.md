- **A `Resolved`/`Snapshot` field is not enough to make a value reach a reader: `Snapshot::retype`
  rebuilds the typed tree from the merged *table*.** Anything the resolver puts on
  `Resolved::config` and nowhere else is dropped at the snapshot boundary, and a reload retypes
  again, so a value put back once at build is lost on the next carry. Carry the value beside the
  table and re-apply it inside `retype`; before believing a value is lost in the resolver, check
  `Snapshot::table` — `nvs config dump --toml` prints exactly that.
  [until: gone crates/nvs-config/src/snapshot.rs:fn retype]
