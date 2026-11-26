Adds a flag to one message. A flag is a label on the message that every mail program can read. You
name the message by its UID (the number the server gives each message in a folder).

The standard flags are `\Flagged` (a star or a red flag in most mail programs), `\Answered`,
`\Draft` and `\Seen`. You can also use your own label, a keyword such as `$Paid` or `Invoice`. A keyword may start with
`$`, but it does not have to. A
keyword has no space and none of the characters `(){%*"\]`. It is at most 255 bytes long. If the message has the flag already,
nothing changes. `unflag` removes a flag again.

**Good to know:** `\Deleted` is not allowed, and `flag` throws a `RuntimeError` for it. To delete
a message, call `delete`. `delete` deletes only the message you name. Any other flag that starts with
`\` also throws a `RuntimeError`.
