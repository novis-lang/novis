- **`as ?string` over a `mixed` holding `null` answers `''`, so a null test written through that
  cast never fires.** `rule:expressions/nullable-conversion-availability` refuses `null as ?string`
  outright because the conversion cannot fail, and the same conversion reached through `mixed`
  converts the `null` instead of passing it along — a `Core\Db\Row` example meant to skip an empty
  column printed `note: ` for it and blessed cleanly. Test a SQL NULL with `$row->get($name) ==
  null`, which stays `mixed`, or read the column with the typed reader `$row->string($name)`, which
  answers `?string`. [until: gone crates/nvs-stdlib/src/db/row.rs:Core\Db\Row]
