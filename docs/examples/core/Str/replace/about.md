Replaces a piece of text everywhere it appears in a string.

`Core\Str::replace` takes a string, the text to find and the text to put in its place. It returns a
new string: `replace("a cat and a cat", "cat", "dog")` gives `"a dog and a dog"`. If the text is
not found, the string is returned unchanged. An empty search text matches nothing.

Matches are found from left to right and do not overlap. The new text is not searched again, so
`replace("aaa", "aa", "a")` gives `"aa"`.

Two options change the search. `caseInsensitive: true` lets upper-case and lower-case letters match
each other, so `"Draft"` also finds `"DRAFT"`. `limit` sets the most matches to replace, counted
from the left. `limit: 0` replaces nothing.

To replace several different texts in one call, use `Core\Str::replaceAll`.
