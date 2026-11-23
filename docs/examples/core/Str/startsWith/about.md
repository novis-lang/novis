Checks whether a string starts with a piece of text.

`Core\Str::startsWith` returns `true` when the first characters of the string are exactly the text
you give it, and `false` otherwise. A text longer than the string is never at its start, so the
result is `false`.

The check is case-sensitive, so `"HTTPS://example.com"` does not start with `"https://"`. To ignore
upper and lower case, change the string to lower case with `Core\Str::lower` first. An empty text is
at the start of every string, so the result is `true`.

The text must also end after a whole character, the way a person reads it. `"e\u{301}"` is an "e"
with a combining accent, and a person reads it as one character: "é". So
`Core\Str::startsWith("e\u{301}tude", "e")` is `false`.

`Core\Str::afterPrefix` does the check and returns the rest of the string in one call. It returns
`null` when the string does not start with the text. `Core\Str::endsWith` does the same check at the
end of a string.

**The examples below** check a few beginnings, sort commands from plain text in a chat message, and
send each request path to the right part of a web application.
