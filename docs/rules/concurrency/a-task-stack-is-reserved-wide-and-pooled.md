A task's stack is 1 MiB of **reserved** address space with a guard page. Reserved is not committed:
a page a task never touches is never resident on any of the three platforms, so a shallow handler
costs the handful of pages its frames actually wrote. A hundred thousand concurrent tasks is 100 GiB
of address space — unremarkable in a 64-bit process — against a resident cost measured in what the
handlers touched.

The reservation is charged to the request that owns the task, and it is the *reservation* that enters
a worker's admission arithmetic, because address space is the resource an admission decision can count
ahead of time. Resident growth is bounded by the recursion limit that already bounds how deep a task
may go (`rule:concurrency/a-tasks-recursion-limit-comes-from-its-own-stack`).

Stacks are pooled per worker. A stack is an `mmap` or `VirtualAlloc` pair, which is the wrong thing
to do per request on a hot path, so a freed stack goes back to the pool and is handed to the next
task. The pool never holds more stacks than the worker has had tasks in flight at once. A serving
core also keeps at most its share of `[server] max_in_flight` idle, the number divided by the worker
count and rounded up, read each time a stack comes back so a reload moves it. A stack freed past that
is handed back to the operating system, so one burst does not leave its stacks resident for the life
of the worker. That is the O(in-flight) shape `rule:programs/memory-priority` asks for.

A scheduler that is not a serving core, such as `nvs run`'s or a queue worker's, has no admission
number and keeps its high-water mark. Its tasks are already bounded by `max_tasks` per program, and
`nvs run`'s scheduler ends with its one program, so a second compiled-in number would bound nothing
the work does not.

A parse whose native recursion depth is set by its input, such as `Core\Json::decode` at its 1024-level
ceiling, runs on the worker thread's one **spare stack**, 8 MiB reserved, and never on the task's
own. What is left of a task's stack depends on how deep the program already is and on the build's frame
sizes, so the task's stack cannot promise room for such a parse. The spare stack is one reservation per
worker thread, O(workers), and the pages the deepest parse touched stay resident for the life of the
thread. Code on it never calls compiled Novis code, whose recursion limit is armed for the task's stack.

- **Memory:** each task in flight reserves 1 MiB of address space, charged to its request, and keeps
  resident only the pages its frames touched. So a request whose memory limit is 256 MB runs at most
  about 250 tasks at once. A serving core also holds up to its share of
  `max_in_flight` in idle reservations between tasks, and each worker thread one 8 MiB spare stack.
