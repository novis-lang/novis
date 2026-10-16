Returns the name of the topic a message was published to.

A connection receives messages from two places: from the client, and from the topics it subscribed
to with `Core\Topic::subscribe`. `topic()` tells them apart. For a message from a topic, it returns
the name of the topic. For a message from the client, it returns `null`.

A connection that subscribes to more than one topic uses the name to decide what to do with each
message. The name is not `tainted`, because your program chose it when it subscribed.

**The examples below** tell a message from the client from a message from a topic, handle two
topics in one loop, and forward the messages of two chat rooms to the client.
