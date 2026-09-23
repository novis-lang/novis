- **A bench that indexes an array with a `uint` expression records two allocations per op and misses
  an `allocations 0` declaration.** `crates/nvs-ir/src/lib.rs`'s `# Known gaps` item 21 owns it: a
  `uint` subscript is lowered to a fresh decimal string on every access, where an `int` one takes the
  runtime's integer-key entry point. Write the index as `$keys[($total % 3) as int]`, which is what
  `benches/members/core/Str/length.nvs` already does, rather than reading the miss as the member
  under test allocating.
  [until: gone crates/nvs-ir/src/lib.rs:array subscript is rendered to a decimal string]
