- **An FTS5 table brings shadow tables that a loop over `sqlite_master` also sees, and a virtual
  table takes no `on conflict`.** `apps/registry`'s backup dumped every table, so `search_data`'s
  blobs made `Json::encode` throw and every backup failed once `search` existed, and its restore's
  `insert ... on conflict do nothing` failed on `search` itself. Skip `<virtual>_*` and insert a
  virtual table's row with `where not exists`, as `Backup::tables` and `Backup::load` do. Also,
  `nvs test apps/<app>` from the root does not read the app's `nvs.toml`: name it with `--config`.
  `until: gone apps/registry/src/Backup.nvs:A full-text table has no key`
