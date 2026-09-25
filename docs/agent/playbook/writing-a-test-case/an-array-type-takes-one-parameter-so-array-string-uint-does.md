- **An array type takes one parameter, so `array<string, uint>` does not parse.** The key type is
  never written — a string-keyed array of counts is `array<uint>` and the literal supplies the keys
  — so the comma is read as call syntax and what comes back is an `E0101`/`E0102` pair pointing at
  the `<`, saying nothing about types. Declare the value type alone and key the literal:
  `array<uint> $seen = ["get" => 0];`. [until: gone docs/rules/types/arrays.md:one parameter, not two]
