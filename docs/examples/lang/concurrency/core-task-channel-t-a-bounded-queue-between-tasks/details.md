The capacity is a `uint`. `-1` and an `int` variable are `E0401`. A capacity of `0` stops the program
with `FATAL: Core\Task\Channel's constructor expected a capacity of at least 1, got 0` and exit
status 1.

You must write the value type. `new Core\Task\Channel(1)` is `E0442`. A `send` of another type is
`E0401`, and so is a `foreach` variable of another type. `foreach ($chan as var $v)` gives `$v` the
channel's value type. A key, as in `foreach ($chan as $k => int $v)`, is `E0444`.

A channel has the methods `send` and `close`. There is no `receive` method (`E0405`). A `foreach` loop
is the way to take values out.

A `send` on a closed channel stops the program with `FATAL: Core\Task\Channel::send was called on a
closed channel` and exit status 1. No `catch` block and no `finally` block runs.

Nothing checks that another task will read or send. A `send` on a full channel with no reader waits
forever. So does a `foreach` on an empty channel that nobody closes. This is true inside a task group
and outside one. A `deadline` on the group stops that wait.

A second `foreach` over a channel that is closed and empty runs nothing.

An object you send is the same object in the reading task. A change the reader makes to it is seen by
the sender.

You can pass a channel to a method. The parameter type is `Core\Task\Channel<int>`, with the value
type.
