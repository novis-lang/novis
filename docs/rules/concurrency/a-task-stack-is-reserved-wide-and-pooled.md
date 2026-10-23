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

A worker keeps **one pool per role**: the server's own work, such as the accept loop and a
connection, takes from one, and compiled Novis code, an isolate's task and every task beneath it,
from the other. A pooled stack keeps the pages its deepest task touched, so one pool let a connection
keep the pages a program grew. Each pool has the cap above, so a serving core holds up to twice its
share idle, which is one connection stack and one program stack per request in flight.

**An idle core gives the pages back.** When a core is about to wait with nothing ready and has taken
no stack for a whole second, each idle stack returned since the last trim gives back all but its top
8 KiB, through `madvise(MADV_DONTNEED)` on Unix and `DiscardVirtualMemory` on Windows. The stack
keeps its mapping and its place in the pool. A trim never runs when a stack comes back, which would
be a system call per request, and never while the core keeps taking stacks. It costs one system call
per idle stack on Unix and a few on Windows, once per quiet period, and up to two wakes of a quiet
core. A trim is not a scrubbing guarantee, and on macOS the old bytes may stay: Novis code cannot
read them because the language is memory-safe.

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
  about 250 tasks at once. A serving core also holds up to twice its share of
  `max_in_flight` in idle reservations between tasks, with all but 8 KiB of each given back once the
  core is idle, and each worker thread one 8 MiB spare stack.
