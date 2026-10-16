A received frame's payload arrives `tainted`. It is untrusted input off a network, exactly like a
request body, and `Core\Validate` is the only way to launder it — a payload that reaches a sink
unvalidated fails to compile rather than failing at run time.

The qualifier cannot be shed by rebuilding the value, because a `Core\Socket\Message` cannot be
constructed: the only way to hold one is to have received it, from the peer or from the bus.

The same rule runs outward at the same boundary. A topic name refuses `tainted`, for the reason a
metric label does — a name derived from user input is how one tenant subscribes to another's stream,
so a name is built from checked values or it does not compile — and a `secret` may never be passed
through an upgrade's arguments or published to a topic.
