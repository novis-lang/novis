- **A `.nvst` case that opens a database needs its own configuration, written into the case as a
  `--FILE nvs.toml--` section.** The conformance runner grants no capability by default, so a case
  calling `Core\Db::connect` fails with `db.connect for main ... is not granted`, which reads exactly
  like a broken program, and `bun nv try` shows the same refusal because it has no config
  either. Copy the section from
  `tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst`: a
  `[capabilities.db]` table, then the `[db.main]` block the case connects to.
  [until: gone crates/nvs-stdlib/src/db/mod.rs:Core\Db::connect]
