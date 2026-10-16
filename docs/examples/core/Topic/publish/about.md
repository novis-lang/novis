Sends a value to every connection that subscribed to a topic, and returns how many connections it
was sent to.

Any script can publish. An ordinary web request, a command-line job or a connection can all tell the
subscribers that something changed. Each subscriber gets its own copy of the value, so a change one
of them makes is not seen by the others. `publish` never waits for a subscriber. A subscriber with
too many unread messages is skipped, and its connection is closed.

**Good to know:** a value that contains a `secret` cannot be published, and `publish` throws a
`LogicError`. This happens even when nobody subscribed to the topic.

**The examples below** publish to a topic nobody joined, try to publish a secret, and tell the open
staff dashboards about a new order.
