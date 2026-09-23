- **Making an operator throw is four edits, not one, and the codegen guard is the last of them.** A
  guard added to `emit_binop` alone dies at run time with `internal error: an arithmetic throw with
  no error edge`, because `nvs-ir`'s `lower/operator.rs` `fallible` match decides whether the
  instruction has an `Inst::on_error` edge to leave on. The roster is the `fallible` arm, the
  codegen guard, the same rule in `nvs_runtime::helpers` for the tagged path a `mixed` operand
  takes, and every `.nvst` that used the old answer as an *instrument* — the math cases that read
  the sign of a zero with `1.0 / $z` had to move to `Core\Math::fdiv` — so grep for the shape (`1.0
  /`, `/ 0.0`), not the feature. [until: reviewed 2026-09-06]
