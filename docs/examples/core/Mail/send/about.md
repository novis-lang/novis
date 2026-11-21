Sends one email through a mail server that the server settings name.

A `[mail.<name>]` block in `nvs.toml` holds the mail server's address, its port, the password and the
sender address. `Core\Mail::send` uses the block with the name your program writes. Your program
never writes a host or a sender, so it cannot send mail as somebody else.

The call checks every address before it connects. An address that is not allowed throws a
`RuntimeError`, and nothing is sent. An address is not allowed when it has a line break, a space or
no `@`. It is also not allowed when it has one of `< > , ; : ( ) " \ %`, or is longer than 254 bytes.

A line break in the subject is encoded as part of the subject text. So a subject cannot add a
header to the message. A long subject is split over several header lines, and the mail program
joins them again.

The options add copy recipients (`cc`), hidden recipients (`bcc`), a reply address (`replyTo`) and an
HTML version of the text (`html`). A mail server that cannot be reached throws an `IOError`.

**Good to know:** the person who runs the server decides which blocks a program may use. `[app.capabilities.mail] send`
in `nvs.toml` lists the names. Under `nvs test`, `send` makes the same checks but sends nothing and
opens no connection. `Core\Test::sentMail` returns the message.
