- **Dropping a suspended `corosensei` coroutine unwinds its stack, and a `catch_unwind` in the way
  aborts the process.** `Coroutine::drop` raises a private `ForcedUnwind` marker and expects it
  back; a `catch_unwind` under the task root that swallows it makes corosensei's own `panic!("the
  ForcedUnwind panic was caught and not rethrown")` fire inside a `Drop` and double-panic to
  `STATUS_STACK_BUFFER_OVERRUN`. `nvs_runtime::Teardown` makes `run_task`
  re-raise instead of contain, and anything else gaining a `catch_unwind` between a coroutine's root
  and its suspension points owes the same guard. [until: gone crates/nvs-runtime/src/lib.rs:catch_unwind]
