Reads one escaped part of a link back to the characters it was written from.

A link can only contain some characters, so the others are written as escapes: a `%` and two hex
digits for each byte. `Core\Uri::decodeComponent("caf%C3%A9%20menu")` returns `café menu`. A `+`
stays a `+`. A `%` that is not followed by two hex digits is kept as it is.

The result is `bytes`, because an escape can give any byte, and some bytes are not valid text. `as string` converts the result to text and throws an error when it is not valid. `as ?string`
gives `null` instead. `Core\Uri::encodeComponent` writes the escapes that this function reads.

**The examples below** read a file name from a link, show that a `+` is kept, and check a file name
from a request before a server uses it.
