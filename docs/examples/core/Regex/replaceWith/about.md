Finds every match of a pattern in a text and replaces it with a text that your own function
returns. It returns the new text. If there is no match, it returns the same text and the function
is not called.

The function gets one `Core\Regex\Match` for each match, from left to right. It can read the whole
match and every group, and it returns a `string`. That text is inserted exactly as it is, so `$1`
in it is two characters. The `limit` option replaces only the first matches. This replaces PHP's
`preg_replace_callback`.

**Good to know:** if the replacement is the same text for every match, `Core\Regex::replace` is
shorter.

The examples show a simple function and `limit`, reading a group of each match, and filling the
placeholders in an email text.
