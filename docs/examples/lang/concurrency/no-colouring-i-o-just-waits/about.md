Novis has no `async`, no `await` on a promise, no `Fiber` and no callback style. A function that
waits is an ordinary function. When it reads a file, sleeps or takes a value from a channel, it
stops the task that called it and returns when the answer is there. The result is the value itself,
with nothing to unwrap, and the lines around the call run in their own order.

The same function works when you call it on its own line and when you call it inside a group of
tasks.

You get concurrency by running several tasks, not by marking functions. While one task waits,
another runs. A task belongs to the call that started it, and that call does not return while any
of its tasks is still running.

**The examples below** show an ordinary function that waits, two tasks taking turns, and a page
that asks for two slow answers at once.
