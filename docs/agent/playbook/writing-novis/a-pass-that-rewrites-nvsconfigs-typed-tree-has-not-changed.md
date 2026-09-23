- **A pass that rewrites `nvs_config`'s typed tree has not changed what a request sees; the half
  that reaches the runtime is the merged table.** `Snapshot::retype` deserializes `Config` from
  `Snapshot::table` afresh, so in-place edits to `resolved.config` survive only for passes that run
  before the snapshot is built, and `app::canonicalize` is a misleading model because
  `Snapshot::build` reads `resolved.config.app` directly. Run `nvs config dump` in a scratch
  directory: a value still spelled as the operator typed it says the rewrite did not land.
  [until: gone crates/nvs-config/src/snapshot.rs:fn retype]
