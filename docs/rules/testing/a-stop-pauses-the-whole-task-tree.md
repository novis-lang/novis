When a task of a debugged request or program reaches an armed stop site, every task of that request's
tree stops at its own next stop site, and the debugger is told once, with each task as a thread. A task
running on another core, started by `spawn worker`, stops the same way
(`rule:concurrency/on-worker-runs-the-child-on-another-core`). Resuming resumes the whole tree.

A stopped task parks on its own stack, the way a task waiting for I/O does, so the other requests on the
same core keep running. Under a development `nvs serve`, a breakpoint stops whichever request reaches it,
and every other request is served as usual.

While the tree is stopped, its wall-time and CPU-time limits are suspended
(`rule:errors/on-limit`): the time spent stopped is added to its wall deadline, and none of it is
charged as CPU time. Without that, any stop longer than the wall limit would end the request the person
is reading. The watchdog does not take a core whose debugged request is stopped for a wedged one
(`rule:http-server/a-wedged-core-is-detected-by-its-deadline`), because the core is still running
other tasks.

Each task is a thread because a thread is how DAP shows concurrent execution, so no protocol extension
is needed (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`).

- **Memory:** a stopped tree holds what it held when it stopped. Each task adds only its frame records
  (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`).

**Declined:**
- **Stopping only the task that reached the breakpoint.** Its siblings would run on and change the state
  being inspected, and a join in the stopped task would find them finished.
- **Stopping the whole process or the whole core.** Other requests would wait behind one debugged
  request, and a development server would stop answering every other tab.
- **Letting the deadlines run while stopped.** A stop longer than the wall limit would end the request
  being debugged.
