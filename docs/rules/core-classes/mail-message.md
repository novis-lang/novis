`Core\Mail` composes the whole message, so every part and every header a program can set is a typed
parameter, and the composer is the only code that writes a `\r\n`. The call is
`Core\Mail::send(string $endpoint, array<string|Core\Mail\Address> $to, string $subject,
string|Core\Html\Markup $body, {options}): string`.

**The body.** A `string` body is sent as `text/plain`. A `Core\Html\Markup` body is sent as HTML with a
plain-text alternative built from it. There is no third way to send HTML, so a `tainted` string,
which never becomes `Markup` unescaped (`rule:core-classes/html-auto-escape`), is always sent as text.
The `text` option replaces the built plain text, and is a `RuntimeError` beside a `string` body. The
plain text is read from the html5ever tree: a paragraph or a heading is a block after a blank line,
`<br>` a line break, a list item a `- ` line, a link `text (url)`, an image its `alt`, a table cell
a tab, `<style>`, `<script>` and `<head>` nothing, an entity its character, and a line wraps at 78
columns.

**The headers.** `send` returns the `Message-ID` it wrote, without angle brackets: 128 random bits as
hex, at the domain of the block's `from`. A program stores it, passes it to `inReplyTo` and matches a
bounce or a reply to it. `From:` is the block's `from` and its `from_name`, and the `fromName` option
replaces the name, never the address. `Core\Mail\Address::of(string $address, string $name)` is a
named address; a plain string is an address only and is never parsed for a name
(`rule:errors/ambiguous-input-refused`). Every name and the subject are RFC 2047 encoded by one path,
so no value adds a header. An address with a second `@` throws, because two readers would split it
at different places. `inReplyTo` and `references` take message ids only. `unsubscribe` takes an
`https://` URL and writes `List-Unsubscribe` and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`
(RFC 8058). `Auto-Submitted: auto-generated` (RFC 3834) is on every message unless `autoSubmitted:
false`, because most mail a web app sends is automatic and the header stops out-of-office replies. A
non-ASCII domain is sent as punycode with the STD3 rules, an ASCII one as written, and a non-ASCII
local part only to a server that offers `SMTPUTF8`; elsewhere it is an `IOError` with nothing sent.

**The parts.** `Core\Mail\Attachment::fromFile`, `fromBytes`, `inlineFile` and `inlineBytes` build an
attachment. A file is read under `fs.read` when the call runs, so a missing or ungranted file throws
on the line that names it. The type defaults to what `Core\Mime` reads from the bytes, then
`application/octet-stream`. An inline attachment's `src()` is its `cid:` URL, random and made when it
is built. Every `cid:` in the HTML needs an inline attachment of the message, and every inline
attachment needs a `cid:` in the HTML; either miss is a `RuntimeError` that names it. The parts nest
`multipart/mixed` > `multipart/related` > `multipart/alternative`, each level only when needed, and
every part is base64. A non-ASCII file name is RFC 2231 encoded, and a control byte in one throws.

**The CSS.** Every HTML body is inlined, in `send` and in `preview`, with no option, because most mail
programs drop a `<style>` element and none loads a stylesheet. The engine is `css-inline` with no
network or file feature, over Servo's `cssparser` and `selectors`. A style rule is copied onto each
element it matches by the cascade (specificity, then order, `!important` above normal, and the
author's `style` attribute above a normal rule), then removed. An at-rule, a rule with a
pseudo-class or a pseudo-element, and a rule that matched nothing stay in the `<style>` element they
were written in, because a mail program may add the attribute such a rule is aimed at. Inlining an
inlined body changes nothing. A `<link rel="stylesheet">` is a `RuntimeError` that says to put the
CSS in a `<style>` element, and nothing is fetched. A styled body deeper than 256 elements throws,
because the engine's writer recurses once per level.

**Preview.** `Core\Mail::preview(string|Core\Html\Markup $body, {text, attachments}):
Core\Mail\Preview` builds the body through the same path as `send`, sends nothing and needs no
block. `html()` is the inlined HTML (`null` for a `string` body), `text()` the plain text,
`keptCss()` the rules left in `<style>`, and `unusedCss()` the kept rules with no `:` outside
brackets, which could have matched and did not. It takes no subject, because nothing it returns
reads one. Its `string` body and its `text` are sinks, because `text()` returns them untainted.

What it spends: the message is composed in memory for the length of the send, about 1.37 times the
attachments' size, under the request's memory limit, and an HTML body with a `<style>` element is
parsed twice. Both are freed when the send ends.

**Declined:**
- **A raw MIME or raw header parameter, for packages or anyone.** It reopens the header injection
  the class exists to close.
- **A `messageId` option.** A random id cannot collide or be guessed, and the returned one serves
  every use.
- **An option that turns inlining off.** One more thing to forget, and inlining an inlined body
  changes nothing. It is reconsidered when a real case needs it.
- **A matcher of our own.** A specificity mistake would be silent.
- **Streaming a file attachment into `DATA`.** Memory is last in the ordering, and one path is
  simpler.
- **A `mailto:` unsubscribe, S/MIME, PGP and a free-form header map.**
- **A background-sending member.** `Core\Task::runAfterResponse` sends after the page is out, and
  `Core\Queue` retries.
