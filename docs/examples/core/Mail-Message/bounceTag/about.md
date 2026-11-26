Returns the tag after `+` in the address a received message was sent to, such as `order-42` for `bounces+order-42@example.com`.

When `Core\Mail::send` has a `bounceTag` option, the mail server sends each bounce to an address with
that tag. So the tag tells you which mail bounced. It also works when the bounce has no copy of
your mail, and `answers()` is `null`.

The tag is read from the first `Delivered-To` header, then `X-Original-To`, then `To`. The first
address with a valid tag gives the result. A valid tag has 1 to 64 letters, digits, `.`, `_` or `-`,
the same rule as for sending. The result is `null` when no address has a valid tag.

**Good to know:** the result is `tainted`, because whoever sent the message wrote these headers.
Tainted means that the value comes from outside your program, so you must check it before you use
it. Find the tag in your own records, such as a list of orders. Then use your own value from there.

**The examples below** find the order of a bounce, show which headers are read, and remove a
newsletter reader whose address bounced.
