- **A `Core` member may not park, because a forced unwind cannot cross its `extern "C"` frame.**
  `nvs_host::Scheduler::tear_down` cancels a parked task with `corosensei`'s `force_unwind`, and a
  member that suspended (`Core\Time::sleep` through `Host::sleep`) is a `nvs_helper!` frame on that
  stack — first `the ForcedUnwind panic was caught and not rethrown`, then, once `run_helper`
  re-raises on `Teardown::in_progress()`, `panic in a function that cannot unwind` naming the
  helper. `extern "C-unwind"` is not the fix, since the JIT frame below has no landing pads; a
  cancelled task dies by `rule:errors/propagation`'s return status at its next safepoint, which is
  what `nvs_safepoint` gives `SafepointFlags::CANCEL`. [until: gone crates/nvs-host/src/scheduler.rs:force_unwind]
