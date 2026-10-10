- **Widening what the checker accepts for an integer literal opens a hole in `nvs-ir` one crate
  down.** `lower_int_literal` decides `ConstInt` versus `ConstUint` from the `expected: Option<Ty>`
  its caller threads, not from anything the checker recorded, so a literal newly placed at `uint`
  still panics with *"nvs-ir: integer literal `…` doesn't fit an `int`"* wherever the position hands
  no `Ty` down — `lower_binary` passes `Some(lty)` to its right operand but only the whole
  expression's `expected` to its left. Both crates must make the same placement. See `crates/nvs-ir/src/lower/expr.rs`. `until: gone crates/nvs-ir/src/lower/expr.rs:lower_int_literal`
