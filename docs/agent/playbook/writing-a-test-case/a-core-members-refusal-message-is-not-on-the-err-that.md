- **A `Core` member's refusal *message* is not on the `Err` that `nvs_runtime::call` answers with.**
  That `Err` is a status code — `nvs_runtime::THROWN` — so destructuring it as a `Fault` is an
  `E0308` against `&i32`, and there is no message on it to read. The throw is on the context:
  `ctx.pending_class()` is the `catch` name, `ctx.pending()` is the sentence, and `ctx.take_pending()`
  clears it so a later assertion on the same context is not reading the first refusal.
  [until: gone crates/nvs-runtime/src/abi.rs:take_pending]
