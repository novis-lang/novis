Returns the `Message-ID` of the mail that a received message is about, without the angle brackets.

For a bounce, a delay and a spam complaint, the id comes from the copy of your own mail inside the
report. For any other message, the id is the `In-Reply-To` header. When there is none, it is the last
id in `References`. The result is `null` when the message names no mail at all.

`Core\Mail::send` returns the `Message-ID` of each mail it sends. Keep it with the order or the
account. Then you can find that record again when a bounce or an answer arrives.

The result is `tainted`, because the sender of the message wrote it. Tainted means that the value
comes from outside your program, so you must check or escape it before you use it. Escape it with
`Core\Html::escape` before you put it on a page. It cannot be part of a file path or an SQL query
without a check. Using it as an array key is allowed.

**Good to know:** some servers send a bounce without a copy of your mail. Then `answers()` is
`null`. Send each mail with a `bounceTag`. Then `bounceTag()` finds the mail of such a bounce.

**The examples below** show a scheduled script that reads a mailbox, a reply found by its headers,
and a spam complaint that stops a newsletter.
