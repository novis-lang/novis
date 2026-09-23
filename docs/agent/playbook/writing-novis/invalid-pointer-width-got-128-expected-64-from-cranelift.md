- **"invalid pointer width (got 128, expected 64)" from cranelift, with `i128` as the failing
  operand, is the signature of a missing `InstKind::Untag`.** A tagged value reached an instruction
  that wanted an object: a write through a property has two receivers to untag, and
  `write_back_array`'s property arm (`$m->rows["0"] = "w"` through a narrowed `?T` local) lacked the
  `untag_receiver` call `lower_reassignment`'s arm has. Read it as an untag hole, not a codegen bug
  — the checker is happy and the lowering never panics, so nothing above catches it.
  [until: reviewed 2026-09-06]
