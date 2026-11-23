Checks whether a string ends with a piece of text.

`Core\Str::endsWith` returns `true` when the last characters of the string are exactly the text you
give it, and `false` otherwise. A text longer than the string is never at its end, so the result is
`false`.

The check is case-sensitive, so `"photo.JPG"` does not end with `".jpg"`. To ignore upper and lower
case, change the string to lower case with `Core\Str::lower` first. An empty text is at the end of
every string, so the result is `true`.

The text must also start at a whole character, the way a person reads it. `"\r\n"` is one line
ending, and it counts as one character. So `Core\Str::endsWith("line\r\n", "\n")` is `false`. Check
for `"\r\n"`, or remove the line break with `Core\Str::trimEnd` first.

`Core\Str::beforeSuffix` does the check and returns the text before the suffix in one call. It
returns `null` when the string does not end with the text. `Core\Str::startsWith` does the same check
at the start of a string.
**The examples below** check a few endings, add a missing `/` to the end of a web address, and
accept only uploads that are images.
