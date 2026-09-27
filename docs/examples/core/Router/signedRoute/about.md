Checks that a request arrived with a signed link that your program made.
`Core\Router::urlSigned` makes the link, and `Core\Router::signedRoute` checks it when the request
arrives. You give it the same keys, newest first. It returns the route that the request matched,
with its values.

The check covers the route name, every value and the time when the link stops working. If
somebody changes a value, removes the `_sig` parameter or adds a second one, `signedRoute` throws a
`RuntimeError`. A link that is too old also throws a `RuntimeError`, with its own message.

Nothing checks a signature for you. Call `signedRoute` at the start of each handler that needs a
signed link, and decide there what the visitor sees when the check fails.

The examples show a valid download link, a link with a changed value, and an unsubscribe link that
was signed with an old key.
