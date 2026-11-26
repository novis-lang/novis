Returns `true` when the delivery to this recipient failed for good, and `false` when the problem may go away.

The result is `true` when the status code starts with `5`, such as `5.1.1` for an address that does
not exist. The server will not deliver to that address if you send the mail again. A code that
starts with `4`, such as `4.2.2` for a full mailbox, is temporary, and the result is `false`. A
recipient with no status code also gives `false`.

**In plain words:** a permanent failure means "stop sending to this address". A temporary one means
"try again later".

**Good to know:** use this to decide when to stop mailing an address. Many mail providers block a
sender that keeps sending to addresses that do not exist.
