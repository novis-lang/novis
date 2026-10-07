Splits text into words, the way a language does, also for languages that write no spaces between
words.

`Segmenter::words` takes a list of strings and returns one list of parts for each string, in the same
order. Each part has a `text` and a `wordLike` flag. `wordLike` is `true` for a word or a number, such
as "Shop", "It's" or "9.50". It is `false` for spaces and punctuation. All parts of a string, joined in
order, give the string again.

Japanese and Chinese have no spaces between words. `Segmenter::words` uses a dictionary for them, so
"東京で買い物" gives the three words "東京", "で" and "買い物". An empty string gives an empty list.

`Segmenter::words` throws a `LogicError` when the locale tag is not valid.

**Good to know:** to count the words of a text, count the parts where `wordLike` is `true`. To split a
text into sentences, use `Segmenter::sentences`.
