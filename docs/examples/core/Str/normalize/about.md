Changes a text into one fixed way of writing it, so that two texts which look the same are also
equal.

Unicode often has two ways to write the same letter. `é` can be one character, or the letter `e`
followed by an accent mark. Both look the same on the screen, but they are different strings, so
`==` and `Core\Str::contains` treat them as different.

`Core\Str::normalize` takes a string and a form, and returns the string in that form. There are
four forms in `Core\NormalForm`. `Nfc` writes `é` as one character, and it is the form to use when
you store and compare text. `Nfd` writes it as two. `Nfkc` and `Nfkd` also replace characters that
only look like others: full-width `１` becomes `1`, and `ﬁ` becomes `fi`. That change cannot be
undone.

A text of ASCII characters is returned unchanged in every form.

The examples show a search, full-width digits, and file names from different computers.
