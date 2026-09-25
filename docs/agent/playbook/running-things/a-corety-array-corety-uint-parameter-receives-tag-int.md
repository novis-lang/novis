- **A `CoreTy::Array(&CoreTy::Uint)` parameter receives `Tag::Int` elements**, so a helper that
  reads each one through `as_uint` alone answers the member's most obvious call site with a fatal. A
  written `[97, 98]` type-checks against `array<uint>` and stays int-tagged into the helper; a
  scalar `uint` parameter has no such problem because the call site materializes the literal at the
  declared type. Read both tags (`str.rs`'s `code_point`), and probe the literal spelling in a
  scratch `.nvs` before writing the case. [until: gone crates/nvs-stdlib/src/str.rs:code_point]
