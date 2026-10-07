//! Negotiation: `negotiate` picks the best offered locale for an `Accept-Language` value, and
//! `resolve-locales` says which locale's data answers for a tag (ADR 0277 § 8 and § 9).
//!
//! The header is a comma-separated list of ranges, each a language tag or `*` with an optional
//! `;q=` weight between 0 and 1, which defaults to 1. Only the first [`MAX_RANGES`] ranges are read,
//! so a hostile header costs the same as a short one. The ranges are tried by weight, highest
//! first, and in header order where weights tie; a range of weight 0 is skipped. A range matches an
//! offered tag when the range itself, or a step of its CLDR fallback chain short of the root
//! (`de-AT` to `de`), is that tag. The first match wins and is returned spelt as `offered` spells
//! it. `*` matches `default`.
//!
//! A header that is empty, has a range that does not parse, or matches nothing returns `default`:
//! the header is request data the program does not control, so it never makes the call fail. A
//! malformed tag in `offered` or `default` is the program's own, and is `Invalid`.
//!
//! `resolve` loads one representative data marker per [`Service`] from that crate's compiled data,
//! with the marker attributes its formatter asks with, and returns the locale the data was found
//! under: the tag itself (less any private use or `-u-` keywords), the step of CLDR's fallback chain
//! that has data (`de-AT` to `de`, `en-GB` to `en-001`), or `und` for the root locale. Segmentation
//! answers with the few locales that tailor word breaks, and `und` for every other.

use icu_locale::fallback::LocaleFallbackConfig;
use icu_locale::LocaleFallbacker;
use icu_provider::prelude::*;

use crate::Error;

/// How many ranges of a header are read.
pub const MAX_RANGES: usize = 32;

/// The best of `offered` for `header`, spelt as `offered` spells it, or `default`.
pub fn negotiate(header: &str, offered: &[String], default: &str) -> Result<String, Error> {
    let offered_locales = offered
        .iter()
        .map(|tag| crate::locale(tag).map(|locale| DataLocale::from(&locale)))
        .collect::<Result<Vec<_>, _>>()?;
    crate::locale(default)?;
    let Some(mut ranges) = ranges(header) else {
        return Ok(default.to_owned());
    };
    ranges.sort_by(|a, b| b.1.total_cmp(&a.1));
    let fallbacker = LocaleFallbacker::new().for_config(LocaleFallbackConfig::default());
    for (range, _) in ranges {
        let Some(range) = range else {
            return Ok(default.to_owned());
        };
        let mut chain = fallbacker.fallback_for(DataLocale::from(&range));
        while !chain.get().is_unknown() {
            if let Some(index) = offered_locales.iter().position(|tag| tag == chain.get()) {
                return Ok(offered[index].clone());
            }
            chain.step();
        }
    }
    Ok(default.to_owned())
}

/// The job whose data [`resolve`] looks up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Collation,
    Numbers,
    Plurals,
    Dates,
    RelativeTime,
    Lists,
    Segmentation,
}

/// For each tag, the locale whose data answers for `service`, or `und` where only the root
/// locale's does.
pub fn resolve(tags: &[String], service: Service) -> Result<Vec<String>, Error> {
    tags.iter()
        .map(|tag| {
            let locale = DataLocale::from(&crate::locale(tag)?);
            Ok(match service {
                Service::Collation => answered::<icu_collator::provider::CollationTailoringV1>(
                    &icu_collator::provider::Baked,
                    locale,
                    "",
                ),
                Service::Numbers => answered::<icu_decimal::provider::DecimalSymbolsV1>(
                    &icu_decimal::provider::Baked,
                    locale,
                    "",
                ),
                Service::Plurals => answered::<icu_plurals::provider::PluralsCardinalV1>(
                    &icu_plurals::provider::Baked,
                    locale,
                    "",
                ),
                Service::Dates => answered::<icu_datetime::provider::names::DatetimeNamesWeekdayV1>(
                    &icu_datetime::provider::Baked,
                    locale,
                    "5",
                ),
                Service::RelativeTime => answered::<
                    icu_experimental::relativetime::provider::LongDayRelativeV1,
                >(&icu_experimental::provider::Baked, locale, ""),
                Service::Lists => answered::<icu_list::provider::ListAndV1>(
                    &icu_list::provider::Baked,
                    locale,
                    "W",
                ),
                Service::Segmentation => answered::<
                    icu_segmenter::provider::SegmenterBreakWordOverrideV1,
                >(&icu_segmenter::provider::Baked, locale, ""),
            })
        })
        .collect()
}

/// The locale `provider`'s `M` data is found under for `locale` and the marker `attributes` the
/// formatter itself asks with: the locale itself, the step of its fallback chain that has data, or
/// `und`.
fn answered<M>(provider: &impl DataProvider<M>, locale: DataLocale, attributes: &str) -> String
where
    M: DataMarker,
{
    let mut request = DataRequest::default();
    request.id = DataIdentifierBorrowed::for_marker_attributes_and_locale(
        DataMarkerAttributes::from_str_or_panic(attributes),
        &locale,
    );
    match provider.load(request) {
        Ok(response) => response.metadata.locale.unwrap_or(locale).to_string(),
        Err(_) => "und".to_owned(),
    }
}

/// The ranges of `header` with a weight above 0, in header order, `None` standing for `*`; or
/// `None` when the header is empty or a range does not parse.
fn ranges(header: &str) -> Option<Vec<(Option<icu_locale::Locale>, f32)>> {
    let mut ranges = Vec::new();
    for item in header.split(',').take(MAX_RANGES) {
        let mut parts = item.split(';');
        let tag = parts.next()?.trim();
        if tag.is_empty() {
            continue;
        }
        let mut weight = 1.0;
        for param in parts {
            let value = param.trim().strip_prefix("q=")?;
            weight = value.parse::<f32>().ok().filter(|q| (0.0..=1.0).contains(q))?;
        }
        if weight == 0.0 {
            continue;
        }
        let range = if tag == "*" {
            None
        } else {
            Some(icu_locale::Locale::try_from_str(tag).ok()?)
        };
        ranges.push((range, weight));
    }
    (!ranges.is_empty()).then_some(ranges)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|tag| tag.to_string()).collect()
    }

    #[test]
    fn the_heaviest_offered_range_wins() {
        let tags = offered(&["en", "de", "fr"]);
        assert_eq!(negotiate("fr;q=0.5, de, en;q=0.8", &tags, "en").unwrap(), "de");
        assert_eq!(negotiate("ja, fr;q=0.4, de;q=0.4", &tags, "en").unwrap(), "fr");
    }

    #[test]
    fn a_regional_range_falls_back_to_its_language() {
        let tags = offered(&["en", "de", "pt-BR"]);
        assert_eq!(negotiate("de-AT", &tags, "en").unwrap(), "de");
        assert_eq!(negotiate("pt-br", &tags, "en").unwrap(), "pt-BR");
        assert_eq!(negotiate("pt-PT", &tags, "en").unwrap(), "en");
    }

    #[test]
    fn an_empty_malformed_or_unmatched_header_returns_the_default() {
        let tags = offered(&["en", "de"]);
        for header in ["", " , ", "de;q=2", "de;x=1", "not a tag", "ja, fr", "de;q=0", "*"] {
            assert_eq!(negotiate(header, &tags, "en").unwrap(), "en", "{header:?}");
        }
    }

    #[test]
    fn only_the_first_ranges_are_read() {
        let header = format!("{}de", "ja, ".repeat(MAX_RANGES));
        assert_eq!(negotiate(&header, &offered(&["de"]), "en").unwrap(), "en");
    }

    #[test]
    fn a_tag_resolves_to_the_locale_with_data() {
        let tags = offered(&["de-AT-x-shop", "en-GB", "zz"]);
        assert_eq!(resolve(&tags, Service::Lists).unwrap(), ["de", "en-001", "und"]);
        assert_eq!(resolve(&tags, Service::Plurals).unwrap(), ["de", "en", "und"]);
        assert_eq!(resolve(&offered(&["de-CH"]), Service::Numbers).unwrap(), ["de-CH"]);
        for service in [Service::Collation, Service::Dates, Service::RelativeTime, Service::Segmentation] {
            assert_eq!(resolve(&offered(&["zz"]), service).unwrap(), ["und"], "{service:?}");
        }
        assert!(matches!(resolve(&offered(&["!"]), Service::Lists), Err(Error::Invalid(_))));
    }

    #[test]
    fn a_malformed_offered_tag_is_invalid() {
        let tags = offered(&["en", "not a tag"]);
        assert!(matches!(negotiate("en", &tags, "en"), Err(Error::Invalid(_))));
        assert!(matches!(negotiate("en", &offered(&["en"]), "!"), Err(Error::Invalid(_))));
    }
}
