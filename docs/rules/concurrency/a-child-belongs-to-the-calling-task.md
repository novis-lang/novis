Every task starts as a child of the task that started it. There is no unparented task and no way to
write one: a group's callables run as children of the calling task, a served connection is a child of
the task accepting on that core, a scheduled fire is a child of the ticker's task, and a `spawn
script` isolate is a child of the frame that spawned it.

Parentage is what carries accounting. A child shares its request's memory, CPU and capability
grants rather than opening an account of its own, so a tree's cost is attributable to one request
and bounded by that request's limits with nothing added. It is also what carries death: a parent
that ends cancels what it left running, so the tree cannot outlive it and there are no orphans.

A host with no calling task therefore has no place to put children, which is why every entry point
that runs a program makes a task first even where one buys nothing else.
