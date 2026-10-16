Waits for the next value published to a topic that this event stream subscribed to.

A script that `Core\Sse::upgrade` starts keeps a stream open to one client. It calls
`Core\Topic::subscribe` for the topics it wants. Then it calls `receive()` in a loop. Each call waits
until another script publishes a value on one of those topics, and returns it as a
`Core\Sse\Message`. The message has the value and the name of its topic.

`receive()` returns `null` when the stream is over. This happens when the client has closed the
connection, or when the stream fell too far behind and its queue of values was full. A `while` loop
over `receive()` then ends by itself.

`receive()` works only in a script that `Core\Sse::upgrade` started. A request that answers with
`Core\Sse::stream` ends with its own events, so `receive()` throws a `LogicError` there.

**The examples below** forward every update to the client, show that only subscribed topics arrive,
and send the steps of a deployment to a status page.
