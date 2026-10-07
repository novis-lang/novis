//! Lists: `format-lists` over `icu_list`'s `ListFormatter` (ADR 0277 § 1 and § 9,
//! `rule:core-classes/intl-batch-shape`).
//!
//! A list is joined with the locale's words for its [`ListType`]: `And` ("a, b, and c"), `Or`
//! ("a, b, or c") or `Unit` ("3 feet, 7 inches"), at a [`Width`]. One formatter is built per call
//! and joins every list. The strings are copied as they are: a list of one is its one string, and
//! an empty list is the empty string.

use icu_list::options::{ListFormatterOptions, ListLength};
use icu_list::ListFormatter;

use crate::relative::Width;
use crate::Error;

/// Which words join the last two items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListType {
    And,
    Or,
    Unit,
}

/// The options of `format-lists`. An absent field is the default: `And` and `Wide`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub list_type: Option<ListType>,
    pub width: Option<Width>,
}

/// Each list joined in `tag`'s words, in input order.
pub fn format(lists: &[Vec<String>], tag: &str, options: Options) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let length = match options.width.unwrap_or(Width::Wide) {
        Width::Wide => ListLength::Wide,
        Width::Short => ListLength::Short,
        Width::Narrow => ListLength::Narrow,
    };
    let icu_options = ListFormatterOptions::default().with_length(length);
    let prefs = (&locale).into();
    let formatter = match options.list_type.unwrap_or(ListType::And) {
        ListType::And => ListFormatter::try_new_and(prefs, icu_options),
        ListType::Or => ListFormatter::try_new_or(prefs, icu_options),
        ListType::Unit => ListFormatter::try_new_unit(prefs, icu_options),
    }
    .map_err(|err| Error::Runtime(format!("No list data for `{tag}`: {err}.")))?;
    Ok(lists
        .iter()
        .map(|list| formatter.format(list.iter().map(String::as_str)).to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn a_list_takes_its_locale_conjunction() {
        let lists = [strings(&["Shop", "Blog", "Wiki"]), strings(&["Shop", "Blog"])];
        let and = |tag| format(&lists, tag, Options::default()).unwrap();
        assert_eq!(and("en"), ["Shop, Blog, and Wiki", "Shop and Blog"]);
        assert_eq!(and("de"), ["Shop, Blog und Wiki", "Shop und Blog"]);
        let or = Options {
            list_type: Some(ListType::Or),
            width: None,
        };
        assert_eq!(format(&lists, "fr", or).unwrap(), ["Shop, Blog ou Wiki", "Shop ou Blog"]);
    }

    #[test]
    fn a_short_list_is_its_items() {
        let lists = [strings(&["Shop"]), Vec::new()];
        assert_eq!(format(&lists, "en", Options::default()).unwrap(), ["Shop", ""]);
    }
}
