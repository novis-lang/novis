Puts a `\` before every character that has a special meaning in a pattern, such as `.`, `+` or
`(`. The result is a pattern that matches the original text exactly, and nothing else. A text with
no special characters is returned unchanged.

Use it when a pattern must contain a text that you did not write yourself, such as a search that a
person typed. A text from a request is `tainted` (it came from outside the program), and a tainted
text does not compile as a pattern. The result of `Core\Regex::quote` is allowed as a pattern.
This replaces PHP's `preg_quote`, but the escaped characters are not exactly the same.

The examples show a text matched exactly, a quoted text inside a bigger pattern, and a search for
what a person typed.
