Returns the name of the topic a message was published to.

A stream's script can subscribe to more than one topic with `Core\Topic::subscribe`. Every value
that arrives through `Core\Sse->receive` is a `Core\Sse\Message`, and `topic()` tells you which of
those topics it came from. It is a `string` and never `null`, because every message on an event
stream comes from a topic.

The name is the one the script subscribed to, so the program already knows every name it can see.
A value published to a topic the script did not subscribe to never arrives.

**The examples below** print the topic of each message, use the topic as the event name, and
separate messages for one user from messages for everybody.
