- **A ThreadSanitizer fiber switch written as two calls corrupts the sanitizer's heap.** Its
  instrumentation pushes at a function's entry and pops at its exit from *the current fiber's*
  shadow stack, so a `switch_to()` helper entered on one fiber and returned from on another pops one
  that is still empty, and the process dies in `__tsan_func_entry` far from the cause. Keep each
  switch inside one call that leaves and comes back — `Fiber::around` in
  `crates/nvs-host/src/tsan.rs` — and read a SEGV in `__tsan_func_entry` as a broken annotation
  rather than a race. [until: gone crates/nvs-host/src/tsan.rs:__tsan_switch_to_fiber]
