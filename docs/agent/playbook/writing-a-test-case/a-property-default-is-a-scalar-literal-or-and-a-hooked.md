- **A property default is a scalar literal or `[]`, and a *hooked* property takes none at all.**
  `public array<string> $rows = ["a"];` is `E0472`, and a `{ get => …; set { … } }` block after a
  default is a *parse* error that cascades, so the hook reads as broken syntax rather than as the
  illegal default before it. Seed an `array<T>` property in `constructor` from a typed local
  (`array<int> $seed = […]; $this->counts = $seed;`), as
  `tests/conformance/lang/every-write-spelling-agrees-on-a-refused-element-target.nvst` does.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0472]
