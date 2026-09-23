- **The three `Core\Db\Write` member benches swing about twofold between two runs of one binary
  minutes apart, so a clock delta on them is not a regression.** Two `--record-perf --force` runs on
  an idle machine reported `affected` at 71.4 and then 32.2 ns/op, and the `units` column does not
  divide it out, because the calibration program is measured in the same sweep and takes the same
  weather as the bench. Read `statements`, `calls` and `allocations` — which did not move — and
  re-run before believing a clock column on these three. [until: reviewed 2026-09-22]
