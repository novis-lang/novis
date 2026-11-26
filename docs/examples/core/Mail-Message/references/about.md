Returns the message ids in the `References` header of a received message, without their angle
brackets.

A mail program writes this header when somebody answers a message. It lists the message ids of the
conversation, the oldest first. The first id is the message that started the conversation. The
last id is usually the message that was answered. A message with no `References` header gives an
empty array.

The sender wrote this header, so each id is a `tainted string`. Tainted means that the value comes
from outside your program. Do not use an id as a proof of who sent the message.

`inReplyTo()` returns only the id of the message that was answered.

**The examples below** show the ids of a conversation, replies grouped by conversation, and a
header that goes on over several lines.
