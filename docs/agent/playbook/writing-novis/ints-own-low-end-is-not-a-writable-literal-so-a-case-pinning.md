- **`int`'s own low end is not a writable literal, so a case pinning a 64-bit bound spells it
  `-9223372036854775807 - 1`.** `-9223372036854775808` is `E0429: this number is too large for
  `int`` — the minus applies to an already-overflowed literal. Where a field's range spans `int`
  and `uint`, as for `Core\Bytes::pack`'s `J`/`P`, neither refusal has a writable argument, so
  assert the reach at both ends. [until: gone crates/nvs-diagnostics/src/lib.rs:E0429]
