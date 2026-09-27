A task's stack is 1 MiB of **reserved** address space with a guard page. Reserved is not committed:
a page a task never touches is never resident on any of the three platforms, so a shallow handler
costs the handful of pages its frames actually wrote. A hundred thousand concurrent tasks is 100 GiB
of address space — unremarkable in a 64-bit process — against a resident cost measured in what the
handlers touched.

The reservation is charged to the request that owns the task, and it is the *reservation* that enters
a worker's admission arithmetic, because address space is the resource an admission decision can count
ahead of time.

Stacks are pooled per worker and bounded by that worker's in-flight cap. A stack is an `mmap` or
`VirtualAlloc` pair, which is the wrong thing to do per request on a hot path, so a freed stack goes
back to the pool and is handed to the next task. The pool can never hold more stacks than the worker
has admitted tasks, which is the O(in-flight) shape `rule:programs/memory-priority` asks for.

A parse whose native recursion depth is set by its input, such as `Core\Json::decode` at its 1024-level
ceiling, runs on the worker thread's one **spare stack**, 8 MiB reserved, and never on the task's
own. What is left of a task's stack depends on how deep the program already is and on the build's frame
sizes, so the task's stack cannot promise room for such a parse. The spare stack is one reservation per
worker thread, O(workers), and the pages the deepest parse touched stay resident for the life of the
thread. Code on it never calls compiled Novis code, whose recursion limit is armed for the task's stack.
