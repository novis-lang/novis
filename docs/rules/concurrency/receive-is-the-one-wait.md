`receive()` is the **one** wait a connection takes, and it answers whichever of two sources arrives
first: a frame from the peer, or a value published to a topic this connection subscribed to
(`rule:core-classes/topic`). A peer frame carries its payload; a delivery carries the copied value
and the topic's name, and a non-`null` topic is how the loop tells the two apart.

A connection is the one place a program must wait on two sources at once, so the select lives inside
the member every connection script already calls rather than in a construct every script would have
to spell. The alternative — a second member drained by a sibling task — would make every connection
write the same two-task scaffold to get one loop's worth of behaviour.

There is nothing for such a second member to drain in any case: the subscriber queue belongs to the
connection, and `receive()` is the only thing that empties it. A queue that overflows closes *that*
subscriber rather than blocking the publisher, and the overflow is asked before the wait.
