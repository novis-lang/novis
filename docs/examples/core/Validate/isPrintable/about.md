Checks whether a text contains no control characters. A control character has no visible form. It
tells a terminal or a file what to do, for example start a new line. The method returns `true` or
`false`.

A tab, a new line, a carriage return, the null byte and `DEL` are control characters, so the result
is `false`. Letters from any language are printable, such as `é`, `ß` or `日`. Emoji are printable
too. The empty text contains no control character, so the result is `true`.

Use it for a short text from a user that must stay on one line, such as a name or a title. A new
line in a name can make one entry in a log look like two.

Some invisible characters are not control characters, such as the zero-width joiner. They give
`true`, so this method does not find text that hides what it really says.

**The examples below** check a few texts, show how `isPrintable` differs from
`Core\Validate::isAscii`, and check a display name before it is saved.
