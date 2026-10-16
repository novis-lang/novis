- **Hand-built IR that needs a `Ty::Tagged` operand emits the bare constant and then
  `InstKind::Tag`; the `Ty` on the instruction is not a cast.** `low.emit(b, Ty::Tagged,
  InstKind::ConstNull)` compiles and lowers, then aborts with `internal error: cranelift rejected
  the code generated for `C::current`: should be implemented in ISLE: inst = `v25, v26 = isplit.i64
  v46``, which names no lowering. `convert`'s `(_, Ty::Tagged)` arm produces the `Tag` for every
  source-level widening, so a synthesized body is the only place it is written by hand; every other
  `InstKind::ConstNull` under `lower/` is `Ty::Null` for this reason. `until: gone crates/nvs-ir/src/lower/convert.rs:Ty::Tagged`
