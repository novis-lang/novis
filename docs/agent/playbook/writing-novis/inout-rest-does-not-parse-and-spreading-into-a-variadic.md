- **`inout ...$rest` does not parse, and spreading into a variadic `inout` tail is accepted in
  silence.** `Parser::parse_arg` tests for `...` before it eats `inout`, so the marked spelling is
  `E0714` plus a cascade of `E0101`/`E0102`, while `Adder::many(...$rest)` against `function
  many(inout int ...$xs)` compiles and runs with nothing written back. `E0714`'s "a spread's
  entries" half is reachable only through a fixed `inout` parameter; a case wanting the variadic row
  has to wait for that hole. [until: gone crates/nvs-diagnostics/src/lib.rs:E0714]
