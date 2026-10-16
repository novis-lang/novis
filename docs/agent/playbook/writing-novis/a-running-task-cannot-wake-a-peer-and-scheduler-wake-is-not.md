- **A running task cannot wake a peer, and `Scheduler::wake` is not the route.** It takes `&mut
  Scheduler`, so anything that unblocks another task goes
  through `nvs_host::Wake`: a handle taken while the waiting task is running, which queues a
  `TaskId` on the task tree and is drained into the run queue by `Scheduler::run`. Two things are
  load-bearing — it holds the tree rather than reading the `TREE` thread-local, and the drain
  runs at the top of `run` as well as after every resume, or a wake issued between turns leaves the
  parked task asleep with no error anywhere. See `crates/nvs-host/src/lib.rs`. `until: gone crates/nvs-host/src/lib.rs:Scheduler::wake`
