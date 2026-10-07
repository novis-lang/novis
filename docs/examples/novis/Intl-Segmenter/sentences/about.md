Splits text into sentences, the way a language does.

`Segmenter::sentences` takes a list of strings and returns one list of sentences for each string, in
the same order. A sentence ends after a ".", "?" or "!" and the spaces that follow it. Each sentence
keeps those spaces, so all sentences of a string, joined in order, give the string again. An empty
string gives an empty list.

The rules know common cases. A number such as "3.5" does not end a sentence. Some short words do end
one, so "Mr. Smith is here." gives two sentences: "Mr. " and "Smith is here.". Check the result when
your text has many abbreviations.

`Segmenter::sentences` throws a `LogicError` when the locale tag is not valid.

**Good to know:** to split a text into words, use `Segmenter::words`.
