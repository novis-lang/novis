Finds where a piece of text first appears in a string.

`Core\Str::indexOf` searches `$haystack` for `$needle` and returns the position of the first match.
The first character is position `0`. When the needle is not in the string, the result is `null`.
Searching `"The quick brown fox"` for `"quick"` gives `4`.

The position counts characters the way a person sees them, so an accented letter or an emoji counts
as one. You can give the position to `Core\Str::slice` to cut the text at the match.

Two options change the search. `from` starts the search at a later position. A negative `from`
counts back from the end of the string. `caseInsensitive: true` treats upper-case and lower-case
letters as the same.

This replaces PHP's `strpos`, `stripos`, `mb_strpos` and `mb_stripos`. They return `false` when
nothing is found, and `false` is easy to mix up with position `0`.

**Good to know:** to check only whether the text is in the string, use `Core\Str::contains`. To get
the text before or after a match, use `Core\Str::before` or `Core\Str::after`.

related: Core\Str::contains, Core\Str::lastIndexOf, Core\Str::before, Core\Str::after
