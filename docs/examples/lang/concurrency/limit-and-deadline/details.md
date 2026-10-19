The options must be written at the call as `{…}`. Options kept in a variable are `E0453`. A field
that is not `limit` or `deadline` is `E0454`, and its help lists the two.

`limit` is a `uint`. `limit: -1` and an `int` variable are `E0401`. A `limit` larger than the
number of tasks is allowed. The waiting tasks start in the order they were written.

`deadline` is a `Core\Time\Duration`, such as `20ms`. A number such as `20` is `E0401`.

The `TimeoutError` message is `Core\Task::map: the deadline expired before every child returned`,
with `Core\Task::all` for that call. `TimeoutError` is a `RuntimeError`. The `LogicError` message
for `limit: 0` starts with `Core\Task::map:` or `Core\Task::all:` too.

`deadline: 0ms` throws `TimeoutError`, even when no task waits.

A stopped task does not run its `finally` blocks either.

A task can start its own group with a shorter deadline, and catch that group's `TimeoutError`. The
outer group then goes on. When the outer deadline passes first, it stops the inner groups too, and
the outer call throws its own `TimeoutError`.
