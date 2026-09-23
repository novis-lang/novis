- **A field added to `nvs_runtime::Ctx` can stop every coroutine from starting, and the panic names
  neither the field nor the crate.** `corosensei` refuses an entry closure over 1024 bytes
  (`allocate_obj_on_stack`, `type is too big to transfer`), and the breach surfaces as unrelated `-p
  nvs-stdlib` tests panicking inside a cargo registry path. `nvs_host::scheduler::start` boxes the
  context across and asserts `size_of_val(&entry) <= CORO_TRANSFER_LIMIT`; keep anything new out of
  the entry closure, and note `Finished` is over the limit and never crosses.
  [until: gone crates/nvs-host/src/scheduler.rs:CORO_TRANSFER_LIMIT]
