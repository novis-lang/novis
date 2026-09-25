- **A `?T` row that can carry a fault must be emitted with `emit_fallible`, or a `catch` round the
  conversion never sees the throw.** `Lowering::convert_or_null` emitted every row with a plain
  `emit` because "the helper answers `null` where the throwing row would throw" — false once `as
  ?string` could run the operand's own `toString()` — and a non-fallible `HelperCall` discards the
  status word, so the program prints `Uncaught Exception` *with* a `catch (Throwable $e)` around it.
  When a new row makes a uniform emit site fallible, the scratch file to write throws from inside
  the new row and catches it. [until: gone crates/nvs-ir/src/lower/mod.rs:emit_fallible]
