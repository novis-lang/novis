Returns the `Message-ID` of the mail that a received message is about, without the angle brackets.

A bounce is a message from a mail server that says your mail could not be delivered. A bounce, a
delay and a spam complaint contain the headers of your mail, and the id comes from there. For any other message, the id is the `In-Reply-To` header. When there is none, it is the last
id in `References`. Otherwise the result is `null`.

`Core\Mail::send` returns the `Message-ID` of each mail it sends. Save it with your record of
the mail, so you can find that record when a bounce or an answer arrives.

The result is `tainted`. Tainted means that the value
comes from outside your program, so you must check or escape it before you use it. Using it as an
array key is allowed.

**Good to know:** some servers send a bounce without a copy of your mail. Then `answers()` is
`null`. Pass the `bounceTag` option to `send`. Then `bounceTag()` returns that tag from the bounce.

**The examples below** show a script that reads a mailbox, a reply found by its headers, and a
spam complaint that stops a newsletter.
