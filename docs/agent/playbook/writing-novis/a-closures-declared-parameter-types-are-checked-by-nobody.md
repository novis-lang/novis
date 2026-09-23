- **A closure's declared parameter types are checked by nobody, and a mismatch is an arbitrary
  dereference.** `fn (string $s)` mapped over an `array<int>` dies in `nvs-runtime`'s `string.rs` on
  a misaligned pointer, because `rule:types/closure-literal` gives `callable` no parameter list, so
  `invoke` reads each slot at its declared representation. Spell the element type and the parameter
  type the same, and read a crash with no Novis frame as this first. [until: reviewed 2026-09-06]
