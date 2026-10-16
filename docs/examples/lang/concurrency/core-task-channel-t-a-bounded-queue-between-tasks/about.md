A `Core\Task\Channel<T>` is a queue that one task fills and another empties. You give it a capacity
when you make it, and it holds that many values. `send` puts one value in. If the channel is full,
`send` waits until there is room, so the queue never grows.

You read a channel with a `foreach` loop. It takes the values out one at a time, in the order they
were sent. The loop waits while the channel is empty, and ends once it is empty and closed.

`close` says that no more values are coming. Values already in the queue are still delivered, and
closing twice does nothing. A task that never closes leaves the reading task waiting forever.
Sending into a closed channel stops the program with an error, and so does a capacity of `0`.

**The examples below** show two tasks passing values, what the capacity does, and orders billed
while they are still being read.
