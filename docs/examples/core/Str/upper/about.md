Changes every letter of a text to upper case.

`Core\Str::upper` takes one string and returns a copy where every lower-case letter is in upper
case. It uses the rules of Unicode, so it works for letters with accents and for other alphabets
too: `"école"` gives `"ÉCOLE"`, and the Greek `"οδος"` gives `"ΟΔΟΣ"`. Digits, spaces and signs do
not change.

The result can be longer than the text you give it. A few letters change into two or three
letters when they become upper case. For example, the German `"ß"` becomes `"SS"`, so `"straße"`
gives `"STRASSE"`.

`Core\Str::lower` does the opposite. `Core\Str::upperFirst` changes only the first letter. To
check if two texts are equal while ignoring case, use `Core\Str::fold`.

**Good to know:** there is no version of this method that works on single bytes.
