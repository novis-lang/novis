- **A `lang:` feature's perf figure is keyed to the reference chapter the feature is documented in,
  so editing one sentence of that chapter makes every figure in it stale.** `dossier.py --verify`
  then reports `perf: stale: docs/reference/lang/60-iteration.md changed since it was last
  measured` for features the session never opened, which reads like unfinished work rather than a
  re-measure. Run `python tools/dossier.py --record-perf --group <group>` after any edit to a
  reference chapter and before the gate. [until: reviewed 2026-09-19]
