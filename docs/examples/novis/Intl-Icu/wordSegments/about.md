Splits each string in a list into words, spaces and punctuation, by the rules of a language.

`Icu::wordSegments` takes a list of strings and a locale tag. It returns one list for each string.
Each item of that list is a shape with `text` and `wordLike`. `wordLike` is `true` for a word or a
number, and `false` for a space or punctuation. The pieces of a string joined together give the
string back.

`Icu::wordSegments` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::wordSegments` directly. `Segmenter::words` is the
same function.
