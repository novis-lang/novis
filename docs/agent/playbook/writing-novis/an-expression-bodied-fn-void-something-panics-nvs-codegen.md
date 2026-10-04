- **An expression-bodied `fn (): void => Something();` panics `nvs-codegen`, and the block-bodied
  form of the same anonymous function does not.** `Core\Out::capture(fn (): void => M::run())` dies with
  `nvs-codegen does not lower an operand used before it is defined`, while `fn (): void => {
  M::run(); }` and `fn (): int => M::n()` both run, so it is the `void` return that is unlowerable.
  Write the braces; the panic names neither the anonymous function nor its return type.
  [until: gone crates/nvs-codegen/src/emit.rs:used before it is defined]
