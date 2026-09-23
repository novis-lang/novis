- **`python tools/dossier.py --record-perf --force` ignores `--id` and re-measures the whole
  tree.** One `--force --id 'Core\Db::quoteIdentifier'` appended 290 records to
  `docs/perf/members.ndjson` and ran every bench in the repository, which is minutes of work and a
  ledger diff nothing in the session's commit explains. Re-measure one feature by editing its bench
  and running `--record-perf --id '<feature>'` without `--force`, since a bench whose text moved is
  stale and is measured anyway; if a `--force` run has already landed, `git checkout --
  docs/perf/members.ndjson` and record again. [until: reviewed 2026-09-21]
