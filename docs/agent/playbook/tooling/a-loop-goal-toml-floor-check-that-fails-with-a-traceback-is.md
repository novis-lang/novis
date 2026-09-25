- **A `loop-goal.toml` floor check that fails with a `Traceback` is a bug in the tool the check
  runs, not work the tree still owes.** `tools/bench.py --serve-vs-fpm` died on `'NoneType' object
  cannot be interpreted as an integer` because `--reps` grew a `None` default that only `main`
  resolves, while the serve leg went on reading `args.reps` — a sentinel added for one leg that no
  other leg was read against. Reproduce a red check's own argv by hand before budgeting it as a
  slice, and when a flag's default becomes a sentinel, `grep -n 'args\.<name>'` for every leg that
  reads it rather than the one being changed. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
