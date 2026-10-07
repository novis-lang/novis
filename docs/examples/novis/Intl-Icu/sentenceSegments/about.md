Splits each string in a list into sentences, by the rules of a language.

`Icu::sentenceSegments` takes a list of strings and a locale tag. It returns one list of sentences for
each string. A sentence keeps the spaces that follow it, so `"One. Two?"` gives `"One. "` and
`"Two?"`. The sentences of a string joined together give the string back.

`Icu::sentenceSegments` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::sentenceSegments` directly. `Segmenter::sentences`
is the same function.
