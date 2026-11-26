`bounceTag()` returns the bounce tag of a message. In a test, `Core\Test::sentMail()` returns one
`Core\Test\SentMail` for each message, and `bounceTag()` is one of its methods.

A bounce is the error message that a mail server sends back when it cannot deliver a message. It
goes to `return_path` of the `[mail.<name>]` block. The bounce tag is added to that address after a
`+`. With `return_path = "bounces@example.com"` and the tag `order-42`, the bounce goes to
`bounces+order-42@example.com`. So your program can read which message failed.

The result is the `bounceTag` option that your program gave to `Core\Mail::send`. When the program
gave no `bounceTag` option, the result is `null`. A tag has 1 to 64 letters, digits, dots,
underscores or hyphens. `Core\Mail::send` throws an error for any other tag.

Use `bounceTag()` to check that each message has a tag that names its delivery.

**The examples below** show a tag with an order number, a mail with no tag, and a request that
gives each reader of a newsletter their own tag.
