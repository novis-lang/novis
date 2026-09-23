- **`Core\Db\Rows::count()` counts rows, and a `select count(*)` statement answers one row.** A proof
  program reading `select count(*) as n` and then `->count()` gets `1` — a number plausible enough to
  bless and wrong. Read the count out of the column with `->first()` and a typed reader, or select the
  rows and count those. [until: reviewed 2026-09-22]
