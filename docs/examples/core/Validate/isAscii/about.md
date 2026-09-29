Checks whether every character of a text is an ASCII character. ASCII has 128 characters: the
English letters without accents, the digits, common punctuation such as `.` and `-`, the space and
the control characters. The method returns `true` or `false`.

Some systems accept only ASCII, for example a short code for a product, a file name on an old
server, or a field in a network protocol. `Core\Validate::isAscii` checks the text before you send
it there. A letter with an accent such as `é`, a Chinese character or an emoji is not ASCII, so the
result is `false`. The empty text contains no other character, so the result is `true`.

`isAscii` does not check whether a character is visible. A tab or a newline is ASCII. Use
`Core\Validate::isPrintable` to find those.

**The examples below** check a few words, count the texts in a list that need more than ASCII, and
check a product code before it is saved.
