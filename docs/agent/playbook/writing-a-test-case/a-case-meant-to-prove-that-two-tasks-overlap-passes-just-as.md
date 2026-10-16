- **A case meant to prove that two tasks overlap passes just as well when they run one after the
  other, if it shows the order with `echo`.** Each task writes into its own output buffer
  (`Ctx::child`'s fresh buffer), and the buffers reach the response one task at a time, so an
  `echo` inside a task tells nothing about when it ran. Append the events to an array property of
  an object the tasks share, and print the array after `Core\Task::all` returns, as
  `late-init-a-second-task-waits-for-the-first.nvst` does. See
  `crates/nvs-runtime/src/ctx/isolate.rs`. `until: gone crates/nvs-runtime/src/ctx/isolate.rs:OutputSink::Buffer(Vec::new())`
