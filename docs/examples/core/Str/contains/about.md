Checks whether a piece of text appears anywhere in a string.

`Core\Str::contains` returns `true` when the text is found, and `false` when it is not. The position
does not matter: the start, the middle and the end all count.

The search is case-sensitive, so `"Error"` does not find `"error"`. To ignore upper and lower case,
change both strings to lower case with `Core\Str::lower` first. An empty search text is found in
every string, so the result is `true`.

`Core\Str::contains` only tells you whether the text is there. To find where it is, use
`Core\Str::indexOf`. To count how many times it appears, use `Core\Str::countOf`. This replaces
PHP's `str_contains`.

**The examples below** search a short string, search without caring about upper and lower case,
and keep only the lines of a log that report an error.
