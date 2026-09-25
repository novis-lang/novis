- **`await` is contextual, so `await 5` does not parse and a bare `await $handle;` statement is not
  an `await` either.** A literal after it leaves `await` an identifier and the parse fails with
  `E0101 expected ';'` past the literal; in statement position it reads as a type name, so the case
  reports "`$handle` is already declared". Bind either side: `int $n = 5;` then `await $n`, and `var
  $ignored = await $handle;`; `spawn script …;` as a statement is fine. [until: gone crates/nvs-syntax/src/parser/expr.rs:at_await_operand]
