`fromName()` returns the sender name of a message. Mail programs show this name in the list of
messages. In a test, `Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message,
and `fromName()` is one of its methods.

The result is the `fromName` option that your program gave to `Core\Mail::send`. Without that
option, the result is `from_name` of the `[mail.<name>]` block. When neither is set, the result is
`null`. The sender address is always `from` of the block. `fromName` changes only the name.

Use `fromName()` to check that a mail shows the right sender. For example, an answer from a support
agent can show the agent's name.

**The examples below** show the name from the settings, a name that the program gives, and a block
with no name.
