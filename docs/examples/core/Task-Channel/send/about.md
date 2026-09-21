Puts one value into a channel, for another task running at the same time to take out.

A channel holds a fixed number of values, chosen when you create it. While it is full, `send` pauses
the task that called it, and that task continues as soon as the other one takes a value out. This is
how a fast producer is kept from running ahead of a slow consumer and filling up memory.

**In plain words:** a conveyor belt with a fixed number of slots. When every slot is taken, the
worker filling them waits until one comes free.

**Good to know:** `send` on a channel that is already closed stops the program with an error, and no
`catch` sees it. Close a channel after the last `send` and never before.

**The examples below** hand values to another task, show the sender waiting for a full channel, and
put a producer and a worker together the way a program does.
