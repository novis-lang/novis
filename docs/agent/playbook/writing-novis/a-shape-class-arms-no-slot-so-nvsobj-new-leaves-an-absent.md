- **A shape class arms no slot, so `NvsObj::new` leaves an absent field `null` rather than
  never-written.** `ClassDesc::defaults` is the list that arms the marker and
  `nvs_ir::lower::record_shape_class` writes `defaults: Vec::new()`, so a native decoder filling a
  `{a?: int}` has to write `Value::unset()` into the slot itself — leave it and `{a?: int}` and
  `{a: ?int}` hold the same thing, which is two types that intern apart reading as one. `NvsObj::new`'s
  own doc comment names the never-written marker, which is exactly what makes the omission look
  impossible. [until: reviewed 2026-09-08]
