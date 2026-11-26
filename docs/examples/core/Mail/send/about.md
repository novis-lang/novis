Sends one email and returns its Message-ID.

The first argument is the name of a `[mail.<name>]` block in `nvs.toml`. The block has the
server, the password and the sender.

A `string` body is sent as plain text. A `Core\Html\Markup` body is sent as HTML, with a
plain-text version made from it. A `tainted` string (text from outside the program, such as a
request) is always sent as plain text.

Options add `cc`, `bcc`, `replyTo`, your own plain text (`text`), `attachments`, `inReplyTo`,
`references`, an `unsubscribe` link and more.

Every address is checked before anything is sent. An address with a line break, a space or a
second `@` throws a `RuntimeError`. A line break in the subject or a name cannot add a header line.
An unreachable server throws an `IOError`.

The Message-ID is random. Save it to match a reply or a bounce to the message.

## Styling a mail

The CSS of a `<style>` element is copied into the `style` attribute of each element it matches.
A `<link rel="stylesheet">` throws a `RuntimeError`. `Core\Mail::preview` shows the result.

Under `nvs test`, nothing is sent, and `Core\Test::sentMail` returns each message.
