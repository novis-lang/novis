Returns the value that was published to the topic.

A publisher calls `Core\Topic::publish` with a topic and a value. Every stream that subscribed to
that topic receives the value as a `Core\Sse\Message`, and `value()` returns it. Its type is
`mixed`, so the program converts it before it uses it, for example with `as string` or `as ?int`.

The value is a copy. If the program changes it, the publisher and the other streams do not see the
change. A value that was `tainted` when it was published is still `tainted` when it arrives.

**The examples below** print a published text, check whether a value is a number, and add up
ticket sales for a live counter.
