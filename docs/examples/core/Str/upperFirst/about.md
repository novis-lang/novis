Changes the first letter of a text to upper case.

`Core\Str::upperFirst` takes one string and returns a copy where only the first character is in
upper case. The rest of the text stays exactly as it is, so `"ärger"` gives `"Ärger"`. It uses the
rules of Unicode, so letters with accents and letters from other alphabets change too.

If the first character is already in upper case, or is not a letter, the text does not change. An
empty string gives `""`. A few letters become two letters in upper case, so `"ß"` at the start
becomes `"SS"`.

`Core\Str::lowerFirst` does the opposite. `Core\Str::upper` changes every letter, not only the
first one.
