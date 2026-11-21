Sends one email through a mail server named in the server settings.

A `[mail.<name>]` block in `nvs.toml` has the server's address, port, password and sender, and
your program names the block. It never writes a host or a sender, so it cannot send mail as
somebody else.

Every address is checked before the call connects. An address with a line break, a space or no `@`
throws a `RuntimeError`, and nothing is sent. An address with any of `< > , ; : ( ) " \ %`, or
over 254 bytes, also throws. A line break in the subject cannot add a header line to the message. An
unreachable mail server throws an `IOError`.

Options add copy recipients (`cc`), hidden recipients (`bcc`), a reply address (`replyTo`) and an
HTML version (`html`).

**Good to know:** the server's owner lists the allowed blocks in `[app.capabilities.mail] send`.
Under `nvs test`, `send` makes the same checks but sends nothing.
`Core\Test::sentMail` returns the message.
