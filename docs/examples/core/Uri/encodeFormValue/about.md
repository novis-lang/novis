Escapes a text so that you can put it into a form value, such as a value in a query string or in the
body of a form a browser sends.

A form is written as pairs like `name=value`, joined with `&`. `Core\Uri::encodeFormValue` writes a
space as `+`. It writes every other character as a `%` and two hex digits, except letters, digits and
`-_.`. `Core\Uri::encodeFormValue("salt & pepper")` returns `salt+%26+pepper`. The `&` and `=` in a
value are escaped, so a value cannot start a new pair.

`Core\Uri::decodeFormValue` reads the text back. `Core\Uri::buildQuery` builds a whole query string
from an array. For a path segment, use `Core\Uri::encodeComponent`, which writes a space as `%20`.

**The examples below** escape a value with spaces, show which characters are escaped, and build a
sign-in link that sends the user back to the page they came from.
