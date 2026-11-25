Returns the text that `Core\Uuid::parse` reads, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. The
text is always 36 characters long and in lower case. `format` returns the same text as `toString`.

`Core\Uuid` implements the `Parses` interface, and `format` is the method that interface needs.
`Core\Router::url` calls it to write a UUID into a link, in the path or in the query string.
`Core\Uri::buildQuery` calls it too. The text has only digits, the letters `a` to `f` and hyphens, so
a URL never escapes it. A route that declares a `Core\Uuid` parameter reads the same UUID from the
link.

**The examples below** read the text back with `parse`, write a UUID into a link to a route, and
write one into a query string.
