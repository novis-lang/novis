- **A task cannot reach the `&mut Scheduler` that is resuming it, and every obvious design for the
  task tree dies on that.** `Scheduler::run` holds `&mut self` for the whole turn, so a running task
  can call neither `spawn` nor `cancel`; the part a task needs — id counter, parent links, cancel
  flags, pending children — lives in an `Rc<RefCell<..>>` the scheduler publishes in a thread-local
  for the length of its turn (`scheduler.rs`, the shape `crate::reactor` already uses). Never call
  `Coroutine::force_unwind` from a task's own stack: teardown belongs on the scheduler's stack, so
  cancellation marks and the next turn unwinds. [until: gone crates/nvs-host/src/scheduler.rs:force_unwind]
