Joins the current WebSocket connection or event stream to a topic, so it receives every value
published to that topic.

A topic is a name, such as `room:lobby` or `order:1042`. After `subscribe`, a value that any script
publishes to the topic arrives at the next `receive()` of this connection. The message's `topic()`
returns the name, so you can tell it apart from a message the client sent. Only a script that
`Core\Socket::upgrade` or `Core\Sse::upgrade` started can subscribe. When a connection ends, it
leaves all its topics without any extra code.

**Good to know:** the topic name may not come from user input. Build it from values your program
checked, such as the id of the signed-in user.

**The examples below** join a chat room, show the error outside a connection, and send the updates
of one order to the customer's page.
