- **A keyed array's type names the value alone — `array<V>` — and `array<string, string>` does not
  parse.** A map literal `["EUR" => "Euro"]` is an `array<string>`, so writing the key type too
  stops the parser at the comma and cascades: one such declaration in a two-line example produced
  64 diagnostics, almost all of them about later lines. Write `array<int> $prices = ["cup" => 450];`
  and read only the first diagnostic. [until: gone crates/nvs-diagnostics/src/lib.rs:E0101]
