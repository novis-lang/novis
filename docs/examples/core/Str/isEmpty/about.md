Checks whether a string is empty, which means it has no characters at all.

`Core\Str::isEmpty` returns `true` for the empty string `""` and `false` for every other string. A
string that has only spaces is not empty, and neither is `"0"`. If spaces should count as empty, for
example in a form field, call `Core\Str::trim` first and check its result.

The check takes the same short time for a long string and a short one, because it does not count
the characters.

**Good to know:** PHP's `empty("0")` returns `true`. `Core\Str::isEmpty("0")` returns `false`,
because `"0"` has one character.
