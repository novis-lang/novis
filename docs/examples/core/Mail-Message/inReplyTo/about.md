Returns the message id in the `In-Reply-To` header of a received message, without its angle
brackets.

A mail program writes this header when somebody answers a message. Its value is the message id of
the answered message. If your program kept the id of each mail it sent, `inReplyTo()` tells you
which one the reply is about. A new message has no `In-Reply-To` header, so the result is `null`.
When the header has several ids, the result is the first one.

The result is a `tainted string`. Tainted means that the value comes from outside your program, so
you must check or escape it before you use it. Using it as an array key is allowed.

Anybody can write any id in this header. So the id does not prove who sent the message, or that it
answers your mail.

`references()` returns the ids of the whole conversation, and `answers()` combines both headers.

**The examples below** show how to find the sent mail of a reply, how to tell a reply from a new
message, and a header with two ids.
