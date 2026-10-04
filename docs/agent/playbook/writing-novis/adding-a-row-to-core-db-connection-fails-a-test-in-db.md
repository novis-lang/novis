- **Adding a row to `Core\Db\Connection` fails a test in `db::transaction`, and the failure names
  neither the class nor the member.** `a_transaction_is_a_callable_and_transaction_is_a_queryable`
  sweeps every `CONNECTION.instance` row against `TRANSACTION`'s as debug-printed `Vec<String>`s, so
  a connection-only member arrives as a buried element diff. Add the name to
  `crates/nvs-stdlib/src/db/registry.rs`'s `BEYOND_QUERYABLE`, which the test checks from both ends;
  a "two rosters are identical" guard needs its exception set spelled as data the guard also checks.
  [until: gone crates/nvs-stdlib/src/db/registry.rs:BEYOND_QUERYABLE]
