Counts how many times a piece of text appears in a string.

`Core\Str::countOf` returns a whole number. When the text does not appear at all, the result is `0`.
Each match starts after the end of the previous one, so matches never overlap. In `"aaa"`, the
text `"aa"` is counted once, and the result is `1`.

The search is case-sensitive, so `"error"` does not count `"Error"`. To ignore upper and lower case,
change both strings to lower case with `Core\Str::lower` first. The text to count must not be
empty. An empty text throws a `RuntimeError`.

To check only whether the text appears, use `Core\Str::contains`.

**The examples below** count the commas in a line, show that matches do not overlap, and check that
each placeholder appears exactly once in an email template.
