- **A `// bench:` count added after the figure was recorded is invisible until the bench is
  measured again.** The declaration set is stored in the ledger record rather than read from the
  bench file, so `python tools/dossier.py --id <feature>` keeps printing the old `(declares …)`
  list and a declaration that is wrong is never checked against anything. Add the line and re-run
  `python tools/dossier.py --record-perf --only '<feature>' --force` in the same pass.
  [until: reviewed 2026-09-19]
