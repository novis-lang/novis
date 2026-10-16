`Core\Task::all` and `Core\Task::map` both take a second argument with two fields, `limit` and
`deadline`. Both fields are optional, and they are the only two bounds a group of tasks has.

`limit` says how many tasks may run at the same time. The others wait for a free place, and none of
them is refused. Without a `limit`, every task starts at once. A `limit` of `0` throws `LogicError`.

`deadline` is a time limit for the whole call, not for each task. When it passes, every task still
running is stopped. The call waits for those tasks to stop, and then throws `TimeoutError`. A
stopped task runs no more of its own code: not its `catch`, and not the line after the one it was
waiting on.

**The examples below** show a deadline for a whole call, two tasks running at a time, and a price
list that is polite to a slow service.
