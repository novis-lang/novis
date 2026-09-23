- **Putting a member on the intrinsic list breaks every conformance case that made it throw from a
  *literal*.** Once the checker reads it (`rule:expressions/intrinsic-list-is-closed`), the runtime
  throw the case caught becomes `E0769`/`E0770`, reported as `standard output does not match /
  actual: <empty>`. Bind the pattern to a `string $name = "…";` and pass that, and `grep` the
  member's spelling in `tests/conformance/` *before* adding the arm. [until: reviewed 2026-09-06]
