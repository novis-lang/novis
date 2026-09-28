Checks whether a string starts with a piece of text.

`Core\Str::startsWith` returns `true` when the first characters of the string are exactly the text
you give it, and `false` otherwise. A text longer than the string is never at its start, so the
result is `false`.

The check is case-sensitive, so `"HTTPS://example.com"` does not start with `"https://"`. To ignore
upper and lower case, change the string to lower case with `Core\Str::lower` first. An empty text is
at the start of every string, so the result is `true`.

`Core\Str::endsWith` does the same check at the end of a string. This replaces PHP's
`str_starts_with`.

**The examples below** check a few beginnings, sort commands from plain text in a chat message, and
send each request path to the right part of a web application.
