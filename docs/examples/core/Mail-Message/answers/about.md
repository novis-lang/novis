Returns the `Message-ID` of the mail that a received message is about, without the angle brackets.

For a bounce, a delay and a spam complaint, the id comes from the copy of your own mail inside the
report. For any other message, the id is the `In-Reply-To` header. When there is none, it is the last
entry of `References`. The result is `null` when the message names no mail at all.

`Core\Mail::send` returns the `Message-ID` of each mail it sends. Keep it with the order or the
account, and you can find that record again when a bounce or an answer arrives.

**Good to know:** some servers send a bounce without a copy of your mail. Then `answers()` is
`null`. Send with a `bounceTag`, and `bounceTag()` finds the mail in that case. The result is
`tainted`, because the sender of the message wrote it.

**The examples below** show a scheduled script that reads a mailbox, a reply found by its headers,
and a spam complaint that stops a newsletter.
