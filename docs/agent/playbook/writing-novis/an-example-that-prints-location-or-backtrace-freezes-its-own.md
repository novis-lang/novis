- **An example that prints `location` or `backtrace` freezes its own line numbers into the blessed
  `.out`.** `nv verify`'s `nvs-fmt` step rewrites proof files in place after you blessed them, and any
  later comment edit above the `throw` moves the number too, so the example goes red with nothing
  wrong in it. Bless such an example last, after `bun nv verify` has formatted the tree, and
  re-run `python tools/dossier.py --run examples --group <group>` after any edit above a `throw`.
  [until: reviewed 2026-09-19]
