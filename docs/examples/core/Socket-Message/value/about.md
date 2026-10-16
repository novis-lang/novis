Returns the value that was published to a topic.

A connection that subscribes to a topic with `Core\Topic::subscribe` receives every value published
to it. `receive()` returns each one as a message, and `value()` returns the value that was
published: a number, a string or an array. For a message from the client, `value()` returns `null`.

The value is a copy. If you change it, the value the publisher has does not change. It comes from
your own application and not from the client, so `value()` does not make it `tainted`.

**The examples below** read a published number, tell a message from the client from a published
value, and send a live score to the client each time it changes.
