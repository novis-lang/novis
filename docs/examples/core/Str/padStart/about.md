Adds text before a string until it has the length you ask for.

`Core\Str::padStart` takes a string, a length and a padding. It adds copies of the padding before
the string until the result has that many characters. The default padding is one space. When the
padding has more than one character and does not fit a whole number of times, the last copy is cut
short: `padStart("5", 4, "ab")` gives `"aba5"`.

The length counts characters as a person sees them, so `é` or an emoji counts as one. If the
string already has the length or more, it is returned unchanged. It is never cut.

An empty padding throws a `RuntimeError` when the string is too short.

`Core\Str::padEnd` adds the padding after the string instead.

**Good to know:** this replaces PHP's `str_pad` with `STR_PAD_LEFT`.
