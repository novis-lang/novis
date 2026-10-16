Reads one value of a form back to the characters it was written from.

A form value, such as a value in a query string or in the body of a POST request, is written with
escapes. A `+` is a space, and a `%` and two hex digits give one byte.
`Core\Uri::decodeFormValue("red+shoes+%26+bags")` returns `red shoes & bags`. `%2B` gives a real
`+`. A `%` that is not followed by two hex digits is kept as it is.

The result is `bytes`, because an escape can give any byte, and some bytes are not valid text.
`as string` converts the result to text and throws an error when it is not valid. `as ?string`
gives `null` instead. `Core\Uri::encodeFormValue` writes the escapes that this function reads.

**The examples below** read a search text from a link, show how a `+` and `%2B` are read, and read a
saved setting from a cookie.
