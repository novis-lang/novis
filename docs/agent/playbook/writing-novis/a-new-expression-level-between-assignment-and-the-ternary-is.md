- **A new expression level between assignment and the ternary is not one edit — the ternary's `else`
  branch parses at the assignment level and will swallow it.** `rule:expressions/catch-expression`'s
  `catch` slotted into `parse_assignment_inner` in one line, and `f() catch (A) => $y ?: 1 catch (B)
  => 2` still came out with one arm because `parse_ternary`'s else called `parse_assignment`, which
  re-entered `parse_catch`; `parse_ternary_else` is the same body over `parse_ternary`, for the else
  branch only, since the `then` branch is delimited by its own `:`. Any future level added above the
  ternary owes the same check, and a unit test in `crates/nvs-syntax/src/parser/tests/expr.rs` is
  what catches it, not a `.nvst`. [until: reviewed 2026-09-06]
