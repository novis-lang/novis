- **`Core\Attributes::get<S>(C::m(...))?->field` throws on a `null` answer instead of
  short-circuiting.** The member folds to a compiled-in constant, and a folded `null` does not reach
  the nullsafe test, so the program ends with `attempt to read property … on null` — while the same
  value bound to a `?S` local first, and a userland method declared `: ?S`, both short-circuit
  correctly. Bind the retrieval to a local before reading it — `open_nullsafe` in
  `crates/nvs-ir/src/lower/expr.rs` builds no guard at all unless the lowered receiver is
  `Ty::Tagged`, which a folded constant is not.
  [until: gone crates/nvs-ir/src/lower/expr.rs:open_nullsafe]
