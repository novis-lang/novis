- **`Wake::current()` decides which of two isolate boundaries you just measured**, and a `#[test]`
  or a criterion `b.iter` has no scheduler under it, so it takes the inline one:
  `nvs_host::Isolate::run` outside a task runs the child on the caller's stack and reads several
  times faster than the real path with a stack of its own, and only the second is the boundary a
  program crosses. `benches/abi-probe/shared/isolate.rs` is the shape: build the scheduler and task
  outside the clock, time inside the task body, and hand criterion a batch through `iter_custom`.
  [until: reviewed 2026-09-06]
