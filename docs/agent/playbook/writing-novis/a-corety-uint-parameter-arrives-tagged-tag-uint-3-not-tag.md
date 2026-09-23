- **A `CoreTy::Uint` parameter arrives tagged `Tag::Uint` (3), not `Tag::Int` (2), so
  `Value::as_int()` on one answers `None`.** `uint` is a tag of its own by `rule:types/arithmetic`,
  and the failure is neither a compile error nor a wrong number but the member's own "expected an
  int, got tag 3" fatal, which reads as a caller bug and is not one. `Value::as_uint()` is the
  reader, `Value::uint(…)` is what a `-p nvs-stdlib` test hands such a member, and
  `crates/nvs-stdlib/src/arr.rs` has the shape to copy. [until: reviewed 2026-09-06]
