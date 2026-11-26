Removes a flag from one message. It is the opposite of `flag`.

Pass the UID of the message and the flag, such as `\Flagged` or a keyword of your own such as
`$Todo`. Only that flag is removed, and the other flags of the message stay. If the message does
not have the flag, nothing changes and no error is thrown.

`unflag` takes the same flags as `flag`: `\Answered`, `\Flagged`, `\Draft`, `\Seen`, or a keyword.
To remove `\Seen`, you can also call `markUnseen`.

**Good to know:** `\Deleted` is not allowed here either, and throws a `RuntimeError`. If no message
has that UID, `unflag` throws an `IOError`.
