- **`peek.py --window` is a global flag, not a per-target one, so a second `--window` later in the
  same argv is an argparse error that discards the whole call.** `peek.py A.rs:re:x --window 40
  B.rs:re:y --window 8` exits 2 with `unrecognized arguments`, having read nothing. Pick the one
  window the widest target needs and let the narrow ones overshoot; splitting into two calls is the
  wrong reflex. [until: reviewed 2026-09-06]
