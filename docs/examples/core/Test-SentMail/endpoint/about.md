`endpoint()` returns the name of the mail server block that a message was sent through. In a test,
`Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and `endpoint()` is one
of its methods.

The name is the first argument that your program gave to `Core\Mail::send`. It is the `<name>` of a
`[mail.<name>]` block in `nvs.toml`, such as `site` or `alerts`.

Use `endpoint()` when your program has more than one mail server. For example, mail to customers
can use one server, and alerts for your team can use another. The test checks that each message
uses the right one.

**The examples below** show an alert that goes through the `alerts` block, a newsletter that uses
only the `site` block, and a request that fails and sends an alert to the team.
