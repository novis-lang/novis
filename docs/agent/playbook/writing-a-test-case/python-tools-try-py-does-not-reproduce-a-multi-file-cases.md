- **`python tools/try.py` does not reproduce a multi-file case's working directory, so a
  sibling-file read fails under it.** It copies the `--FILE--` body to `.agent-tmp/` and runs that
  alone. Check a multi-file case with `target/debug/nvs test tests/conformance/<tree>` (a single
  `.nvst` path works too), which is what `verify.py` runs; `try.py` is for the single-file shape.
  [until: reviewed 2026-09-06]
