Sends one email through a mail server that the server settings name.

A `[mail.<name>]` block in `nvs.toml` holds the mail server's address, its port, the password and the
sender address. `Core\Mail::send` uses the block with the name your program writes. Your program
never writes a host or a sender, so it cannot send mail as somebody else.

The call checks every address before it connects. An
address with a line break, a comma or no `@` throws a `RuntimeError`, and nothing is sent. A line
break in the subject becomes part of the subject text, so it cannot add a header.

The options add copy recipients (`cc`), hidden recipients (`bcc`), a reply address (`replyTo`) and an
HTML version of the text (`html`). A mail server that cannot be reached throws an `IOError`.

**Good to know:** the operator decides which blocks a program may use. `[app.capabilities.mail] send`
in `nvs.toml` lists the names.
