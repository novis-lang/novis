- **A `.nvst` run case whose program succeeds must not carry an empty `--EXPECTF-ERROR--` section.**
  The runner reads the section's *presence* as "this run is expected to fail", so a case whose tests
  all pass fails the suite with `expected the run to fail, and it succeeded` — the neighbouring case
  that has the section is one whose tests deliberately fail, which is why copying its skeleton
  misleads. Write the section only when the program's exit status is non-zero, and check with
  `target/debug/nvs.exe test <case>.nvst` before the full sweep. [until: reviewed 2026-10-20]
