- **A cancelled task that is parked is usually torn down where it parked and never sees
  `Resumed::Cancelled`.** `Scheduler::run`'s sweep drops an unwindable parked task's coroutine
  outright, so the line after `suspend_current(Waiting::Parked)` runs only for a stack standing on a
  `nvs_runtime::HelperFrame`, which cannot be unwound and is resumed to die. Write both answers at
  every park site, and hold a `HelperFrame::enter()` guard across the park in a test asserting "the
  wait answered its cancellation". [until: reviewed 2026-09-06]
