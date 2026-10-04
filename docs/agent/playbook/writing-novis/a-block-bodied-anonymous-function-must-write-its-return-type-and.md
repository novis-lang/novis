- **A block-bodied anonymous function must write its return type, and `fn () => { … }` is `E0450`.** An
  expression-bodied anonymous function infers its type from the expression and a block-bodied one cannot, and
  every `Core` member taking a `callable` whose callback does work rather than computing a value
  meets this. The spelling is `fn (): void => { echo "x"; }`. [until: gone crates/nvs-diagnostics/src/lib.rs:E0450]
