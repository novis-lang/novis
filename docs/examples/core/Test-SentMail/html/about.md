`html()` returns the HTML body of a message. In a test, `Core\Test::sentMail()` returns one
`Core\Test\SentMail` for each message, and `html()` is one of its methods.

Your program sends HTML when the body it gives to `Core\Mail::send` is a `Core\Html\Markup`.
`html()` returns the HTML that was sent, with the CSS of each `<style>` element already copied into
the elements. When the body was a `string`, `html()` returns `null`. The plain-text body is always
there. Use `text()` to read it.

Use `html()` to check what a person sees in a mail program that shows HTML. For example, a test can
check that a name from a form is escaped, so that it shows as text and not as tags.

**The examples below** show a name that is escaped in the HTML body, a short notice that has no HTML
body, and a request that sends a mail with the same link in both bodies.
