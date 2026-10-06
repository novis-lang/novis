Changes a string so that text which differs only in upper and lower case becomes the same.

`Core\Str::fold` changes every letter to one fixed form. `"HELLO"`, `"Hello"` and `"hello"` all give
`"hello"`. Some letters change to more than one letter: `"ß"` becomes `"ss"`, so `"Straße"` and
`"STRASSE"` give the same result.

The result is a key for comparing and looking up text. It is not meant to be shown to a person. To
show text in lower case, use `Core\Str::lower`, which keeps `"ß"` as it is.

**In plain words:** it is like writing every name in one style on an index card, so the cards sort
and match no matter how each name was typed.

**Good to know:** `Core\Str::compare` with `{caseInsensitive: true}` does not treat `"ß"` and `"SS"`
as equal. Fold both strings first when that matters.

**The examples below** compare two words without case, show how folding differs from `lower`, and
find a customer by email address however it was typed.
