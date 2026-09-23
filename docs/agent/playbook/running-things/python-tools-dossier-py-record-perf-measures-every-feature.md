- **`python tools/dossier.py --record-perf` measures every feature whose figure is stale, and
  `--id <feature>` does not narrow it.** A run naming one new bench measured seven features and
  appended seven records to `docs/perf/members.ndjson`, six of them for members the slice never
  touched. Expect the extra rows and name them in the commit rather than reverting them — they
  are honest measurements from the same machine and the ledger is append-only.
  [until: reviewed 2026-09-21]
