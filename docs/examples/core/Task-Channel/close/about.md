Ends the stream, so that the task reading the channel knows nothing more is coming.

A `foreach` over a channel waits while the channel is empty, because the next value may still be on
its way. `close` is what tells the reader otherwise: the loop takes the values that are already
queued and then ends. A reader waiting on a channel nobody will send to again waits forever, so the
task that sends is the task that closes.

**Good to know:** closing twice changes nothing, so a program may close on every path out of its
sending code. Sending after the close is different. That stops the program with an error, and no
`catch` sees it.

**The examples below** end a reader's loop, show that the values queued before the close still
arrive, and close on every path out of a producer.
