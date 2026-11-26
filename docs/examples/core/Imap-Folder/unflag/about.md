Removes a flag from one message. It is the opposite of `flag`.

Pass the UID of the message (the number the server gives each message in a folder) and the flag.
The flag is a name such as `\Flagged`, or a keyword of your own such as `$Todo`. Only that flag is
removed, and the other flags of the message do not change. If the message does not have the flag,
nothing changes and no error is thrown.

`unflag` takes the same flags as `flag`: `\Answered`, `\Flagged`, `\Draft`, `\Seen`, or a keyword.
To remove `\Seen`, you can also call `markUnseen`.

**Good to know:** as with `flag`, `\Deleted` is not allowed and throws a `RuntimeError`. If no message
has that UID, `unflag` throws an `IOError`.
