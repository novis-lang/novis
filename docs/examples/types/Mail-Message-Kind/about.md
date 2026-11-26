What a received message is, as `Core\Mail\Message::kind()` returns it.

The six cases are `Bounce`, `Delay`, `Complaint`, `AutoReply`, `Reply` and `Other`. A `Bounce` is a
delivery report where at least one address failed. A `Delay` is a delivery report where a delivery
is late, and the server still tries. A `Complaint` is a report that a reader marked your mail as
spam. An `AutoReply` is a message that a program wrote, such as an out-of-office reply. A `Reply` has
an `In-Reply-To` header. Every other message is `Other`.

**Good to know:** the cases are checked in this order, and the first match is the kind. So an
out-of-office reply with `In-Reply-To` is an `AutoReply`, not a `Reply`. Compare a kind with `==`,
or use `match` to handle each case.
