- **A floor check's ledger line carries the summary and never the case that failed, and the driver's
  own console log carries both.** `vscode (headless) [1 floor]: exit 1 -- headless: 247 passing, 2
  failing` names no suite and no test, and running the command by hand printed `249 passing, 0
  failing`, which reads exactly like a check that fixed itself rather than an intermittent one. Before
  concluding a red floor check is stale, `grep -n failing .loop/logs/<run>-console.log` and read the
  run's own output — the per-suite lines, the failing test names and the assertion are all in it.
  [until: gone tools/nv/driver/accept.ts:firstErrLine]
