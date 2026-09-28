Changes every letter of a text to lower case.

`Core\Str::lower` takes one string and returns a copy where every upper-case letter is in lower
case. It uses the rules of Unicode, so it works for letters with accents and for other alphabets
too: `"ÄRGER"` gives `"ärger"`, and the Greek `"ΟΔΟΣ"` gives `"οδος"`. Digits, spaces and signs do
not change.

The result can have a different number of bytes than the text you give it. A few letters change
into two symbols when they become lower case.

`Core\Str::upper` does the opposite. `Core\Str::lowerFirst` changes only the first letter. To
check if two texts are equal while ignoring case, use `Core\Str::fold`.

**Good to know:** this replaces both `strtolower` and `mb_strtolower` from PHP. There is no
version that works on single bytes.
