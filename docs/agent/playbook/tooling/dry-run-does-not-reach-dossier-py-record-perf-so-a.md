- **`--dry-run` does not reach `dossier.py --record-perf`, so a measurement meant as a trial appends
  to the ledger anyway.** The flag belongs to `--emit-goals`, and `--record-perf --only
  'Core\Arr::contains' --dry-run` ended with `4 records appended to docs/perf/members.ndjson`. Read
  the run's last line rather than the flag, and measure only once the session's last edit to the
  implementing file has landed — `git diff --stat docs/perf/members.ndjson` says what really went in.
  [until: reviewed 2026-09-20]
