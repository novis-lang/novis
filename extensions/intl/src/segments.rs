//! Segmentation: `word-segments` and `sentence-segments` over `icu_segmenter`'s word and sentence
//! segmenters (ADR 0277 § 8 and § 9, `rule:core-classes/intl-batch-shape`).
//!
//! A string is cut at every boundary UAX #29 finds, so its segments joined give the string back,
//! and an empty string has no segments. A word segment is word-like when the segmenter's rule
//! status names it a word, a number or a letter run (CJK included), and not when it is spaces or
//! punctuation. A sentence keeps its trailing spaces. The locale is the content locale: it selects
//! the dictionary or model `compiled_data` carries for a language written without spaces. One
//! segmenter is built per call and cuts every string.

use icu_segmenter::options::{SentenceBreakOptions, WordBreakOptions};
use icu_segmenter::{SentenceSegmenter, WordSegmenter};

use crate::Error;

/// One segment of a string, and whether it is a word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub word_like: bool,
}

/// Each string cut into words, spaces and punctuation, in input order.
pub fn words(strings: &[String], tag: &str) -> Result<Vec<Vec<Word>>, Error> {
    let locale = crate::locale(tag)?;
    let mut options = WordBreakOptions::default();
    options.content_locale = Some(&locale.id);
    let segmenter = WordSegmenter::try_new_auto(options)
        .map_err(|err| Error::Runtime(format!("No word break data for `{tag}`: {err}.")))?;
    let segmenter = segmenter.as_borrowed();
    Ok(strings
        .iter()
        .map(|string| {
            let mut start = 0;
            segmenter
                .segment_str(string)
                .iter_with_word_type()
                .filter(|&(end, _)| end > 0)
                .map(|(end, word_type)| {
                    let word = Word {
                        text: string[start..end].to_owned(),
                        word_like: word_type.is_word_like(),
                    };
                    start = end;
                    word
                })
                .collect()
        })
        .collect())
}

/// Each string cut into sentences, in input order.
pub fn sentences(strings: &[String], tag: &str) -> Result<Vec<Vec<String>>, Error> {
    let locale = crate::locale(tag)?;
    let mut options = SentenceBreakOptions::default();
    options.content_locale = Some(&locale.id);
    let segmenter = SentenceSegmenter::try_new(options)
        .map_err(|err| Error::Runtime(format!("No sentence break data for `{tag}`: {err}.")))?;
    let segmenter = segmenter.as_borrowed();
    Ok(strings
        .iter()
        .map(|string| {
            let mut start = 0;
            segmenter
                .segment_str(string)
                .filter(|&end| end > 0)
                .map(|end| {
                    let sentence = string[start..end].to_owned();
                    start = end;
                    sentence
                })
                .collect()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|item| item.to_string()).collect()
    }

    fn texts(words: &[Word]) -> Vec<(&str, bool)> {
        words.iter().map(|word| (word.text.as_str(), word.word_like)).collect()
    }

    #[test]
    fn a_string_is_cut_into_words_and_the_rest() {
        let cut = words(&strings(&["Shop opens at 9.", ""]), "en").unwrap();
        assert_eq!(
            texts(&cut[0]),
            [
                ("Shop", true),
                (" ", false),
                ("opens", true),
                (" ", false),
                ("at", true),
                (" ", false),
                ("9", true),
                (".", false),
            ]
        );
        assert!(cut[1].is_empty());
    }

    #[test]
    fn a_language_without_spaces_is_cut_into_words() {
        let cut = words(&strings(&["こんにちは世界"]), "ja").unwrap();
        assert_eq!(texts(&cut[0]), [("こんにちは", true), ("世界", true)]);
    }

    #[test]
    fn a_sentence_keeps_its_trailing_space() {
        let cut = sentences(&strings(&["The Shop is open. Is the Blog? Yes!", ""]), "en").unwrap();
        assert_eq!(cut[0], ["The Shop is open. ", "Is the Blog? ", "Yes!"]);
        assert!(cut[1].is_empty());
    }

    #[test]
    fn a_malformed_tag_is_invalid() {
        assert!(matches!(words(&[], "not a tag"), Err(Error::Invalid(_))));
        assert!(matches!(sentences(&[], "not a tag"), Err(Error::Invalid(_))));
    }
}
