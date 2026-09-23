- **`dossier.py --record-perf --force` ignores `--id` and re-measures the whole tree.** A
  `--record-perf --id '<feature>' --force`, reached for because the unforced run says the figure is
  already current, walked every bench and appended 249 rows to `docs/perf/members.ndjson`. Scope a
  forced re-measure with `--group '<group>'`, which `--force` does honour, and `git checkout --
  docs/perf/members.ndjson` puts the ledger back before the scoped run.
  [until: reviewed 2026-09-21]
