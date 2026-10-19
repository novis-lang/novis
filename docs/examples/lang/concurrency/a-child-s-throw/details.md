A task runs until it waits, for example in `Core\Time::sleep` or a `send` on a full channel. The other
tasks are stopped at the place where they wait. A task that throws before it waits for anything is
not stopped in time. When two tasks throw that way, your `catch` gets the first error. The second
error is printed on standard error as `uncaught in a cancelled sibling: second`, and the program goes
on. With `Core\Task::map`, the first error is the one of the first element.

A stopped task runs none of its own `catch` and `finally` blocks. What it printed before it stopped is
still printed.

The error you catch is the same object the task threw. `==` with a copy the task saved is `true`. A
`catch (Throwable $e)` gets it too, and `$e is LogicError` is still `true` for a `LogicError`.

The `backtrace` of the error has one entry: the task's function, at the line of the `throw`.

A task can start its own group. An error in the inner group passes through the outer group to your
`catch`, and the other tasks of the outer group are stopped too.

When nothing catches the error, the program ends with exit status 1 and the usual error report.

When a `deadline` passes before any task throws, the call throws `TimeoutError`, and the later error
is not thrown.
