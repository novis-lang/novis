- **`nvs_runtime::call` hands a test back `Result<Value, i32>`, not the helper's `Fault`; the
  exception is on the `Ctx`.** Matching on the `Err` for what a member threw is `expected i32, found
  Fault`; read `ctx.pending_class()` (which falls back to `ThrownClass::name` when no class table is
  installed, as in every unit test) and `ctx.pending()` for the message. `ctx.take_pending()` *is* a
  `catch (Throwable)`, so a case proving something survives a catch performs one rather than
  describing one. [until: gone crates/nvs-runtime/src/ctx/error.rs:fn pending_class]
