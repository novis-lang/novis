Returns the `Message-ID` header of a received message, without its angle brackets.

The program that sent the message writes this id, such as `<1f2e@example.com>`. The result is
`1f2e@example.com`. A message without a `Message-ID` header returns `null`.

A reply names the id of the message it answers. So you can use the id to find the replies to a
message, or to notice a message that you received twice.

**Good to know:** the id is tainted, because the sender wrote it. Two different senders can write
the same id. Use it to find a message, and do not use it as proof of who sent it.
