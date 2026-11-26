`unsubscribe()` returns the unsubscribe URL of a message. In a test, `Core\Test::sentMail()` returns
one `Core\Test\SentMail` for each message, and `unsubscribe()` is one of its methods.

The result is the `unsubscribe` option that your program gave to `Core\Mail::send`. When the
program gave no `unsubscribe` option, the result is `null`.

With this option, mail programs show an unsubscribe button. When the reader clicks it, the mail
program sends a POST request to the URL. The reader does not see a page. Your program must handle
that request and stop the mail for this reader. Large mail providers expect this link in every
newsletter.

The URL must start with `https://`. `Core\Mail::send` throws an error for any other URL, and for a
URL with a space or a line break.

Use `unsubscribe()` to check that a newsletter has the link, and that each reader gets their own.

**The examples below** show a newsletter with the link, an order mail with no link, and a request
that sends each reader their own link.
