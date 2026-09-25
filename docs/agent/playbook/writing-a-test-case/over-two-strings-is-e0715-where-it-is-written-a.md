- **`<`/`<=`/`>`/`>=`/`<=>` over two `string`s is `E0715` where it is written, and the diagnostic
  names the member that says what was meant.** `Core\Str::compare` is the ordering two strings have;
  the same code refuses a `bytes`, an `array<T>`, a `callable`, an enum case (order `as int`
  instead) and `null`, while the object family keeps `E0411`. A case asserting text ordering asserts
  `Core\Str::compare`, and one wanting a fixed slice still uses `==`; a `Core` member answering
  `uint` (`Core\Str::length`) in `$int + …` is `E0407`, not a widening. [until: gone crates/nvs-diagnostics/src/lib.rs:E0715]
