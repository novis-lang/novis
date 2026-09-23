- **A `--force` beside `--record-perf` widens the sweep to every feature, `--id` and all.** One
  call meant to re-judge a single member's bench appended 305 records to
  `docs/perf/members.ndjson` and took several minutes, because `--force` drops the currency test
  that `--id` narrows rather than narrowing with it. Re-measure one feature by making its record
  stale instead — edit the bench, then `--record-perf --id '<feature>'` with no `--force` — and if a
  `--force` run has already landed, `git checkout -- docs/perf/members.ndjson` and record the one
  feature again. [until: reviewed 2026-09-21]
