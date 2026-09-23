- **A test that asserts memory was released fails on the scheduler, because `nvs_host::Scheduler`
  keeps every finished task's whole `Ctx`.** `CoroutineResult::Return` pushes a `Finished { id, ctx,
  outcome }` onto `self.finished` and only `take_finished` removes one, so a context outlives its
  join and `live_bytes` sees no drop. Read after `run_until_idle` returns, where the `Scheduler` is
  dropped, or drain the list first. [until: reviewed 2026-09-06]
