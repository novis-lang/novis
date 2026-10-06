Changes the first letter of a text to lower case.

`Core\Str::lowerFirst` takes one string and returns a copy where only the first character is in
lower case. The rest of the text stays exactly as it is, so `"ÄRGER"` gives `"äRGER"`. It uses the
rules of Unicode, so letters with accents and letters from other alphabets change too.

If the first character is already in lower case, or is not a letter, the text does not change. An
empty string gives `""`.

`Core\Str::upperFirst` does the opposite. `Core\Str::lower` changes every letter, not only the
first one.
