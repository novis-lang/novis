//! The separators a rendered number uses: those of the read's `locale`, or of a section's
//! `[$-407]` prefix. A [`Locale`] is those separators and the locale's month and day names.
//!
//! Each row is CLDR's decimal and group separator for its locale, the ones `Novis\Intl`'s
//! `NumberFormat::decimal` writes. [`of_tag`] finds a tag's row, or its language's, and a tag
//! with no row has English separators. [`Locale::known`] is a read's `locale`: a tag whose
//! language has no names row is unknown, and the read throws. [`of_id`] maps a Windows locale id, the number of a
//! `[$-407]` prefix, to its tag. The upper bits of the prefix name a calendar or a digit
//! system, and are not read. Grouping is by three digits in every locale.

use super::names::{self, Names};

/// The separators and the month and day names of one locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locale {
    pub separators: Separators,
    pub names: &'static Names,
}

impl Locale {
    pub const EN: Self = Self {
        separators: Separators::EN,
        names: &names::ENGLISH,
    };

    /// The locale `tag`, such as `de` or `de-CH`.
    pub fn of_tag(tag: &str) -> Self {
        Self {
            separators: of_tag(tag),
            names: names::of_tag(tag),
        }
    }

    /// The locale `tag`, or `None` when its language has no row: a read's `locale` names a
    /// language the tables have.
    pub fn known(tag: &str) -> Option<Self> {
        names::find(tag).map(|names| Self {
            separators: of_tag(tag),
            names,
        })
    }

    /// The locale of the Windows locale id in a `[$-407]` prefix, or `None` for an id with no
    /// row.
    pub fn of_id(id: u32) -> Option<Self> {
        tag_of_id(id).map(Self::of_tag)
    }
}

/// A decimal separator and a group separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Separators {
    pub decimal: &'static str,
    pub group: &'static str,
}

impl Separators {
    pub const EN: Self = Self {
        decimal: ".",
        group: ",",
    };
}

const COMMA_POINT: Separators = Separators {
    decimal: ",",
    group: ".",
};
const COMMA_SPACE: Separators = Separators {
    decimal: ",",
    group: "\u{a0}",
};
const COMMA_NARROW: Separators = Separators {
    decimal: ",",
    group: "\u{202f}",
};
const POINT_APOSTROPHE: Separators = Separators {
    decimal: ".",
    group: "'",
};
const COMMA_APOSTROPHE: Separators = Separators {
    decimal: ",",
    group: "'",
};

/// The locales with separators other than English's, a region before its language.
const TABLE: &[(&str, Separators)] = &[
    ("de-at", COMMA_SPACE),
    ("de-ch", POINT_APOSTROPHE),
    ("de", COMMA_POINT),
    ("fr-ca", COMMA_SPACE),
    ("fr-ch", COMMA_APOSTROPHE),
    ("fr", COMMA_NARROW),
    ("it-ch", POINT_APOSTROPHE),
    ("it", COMMA_POINT),
    ("es-mx", Separators::EN),
    ("es", COMMA_POINT),
    ("pt-pt", COMMA_SPACE),
    ("pt", COMMA_POINT),
    ("nl", COMMA_POINT),
    ("da", COMMA_POINT),
    ("tr", COMMA_POINT),
    ("el", COMMA_POINT),
    ("ro", COMMA_POINT),
    ("hr", COMMA_POINT),
    ("sl", COMMA_POINT),
    ("id", COMMA_POINT),
    ("vi", COMMA_POINT),
    ("sv", COMMA_SPACE),
    ("nb", COMMA_SPACE),
    ("no", COMMA_SPACE),
    ("fi", COMMA_SPACE),
    ("pl", COMMA_SPACE),
    ("cs", COMMA_SPACE),
    ("sk", COMMA_SPACE),
    ("ru", COMMA_SPACE),
    ("uk", COMMA_SPACE),
    ("hu", COMMA_SPACE),
    ("bg", COMMA_SPACE),
];

/// The separators of the locale `tag`, such as `de` or `de-CH`.
pub fn of_tag(tag: &str) -> Separators {
    let tag = tag.replace('_', "-").to_ascii_lowercase();
    TABLE
        .iter()
        .find(|(name, _)| tag == *name || tag.starts_with(&format!("{name}-")))
        .map_or(Separators::EN, |(_, separators)| *separators)
}

/// The Windows locale ids a prefix names, and their tags.
const IDS: &[(u32, &str)] = &[
    (0x0409, "en-US"),
    (0x0809, "en-GB"),
    (0x0C09, "en-AU"),
    (0x1009, "en-CA"),
    (0x0407, "de-DE"),
    (0x0C07, "de-AT"),
    (0x0807, "de-CH"),
    (0x040C, "fr-FR"),
    (0x080C, "fr-BE"),
    (0x0C0C, "fr-CA"),
    (0x100C, "fr-CH"),
    (0x0410, "it-IT"),
    (0x0810, "it-CH"),
    (0x040A, "es-ES"),
    (0x0C0A, "es-ES"),
    (0x080A, "es-MX"),
    (0x0416, "pt-BR"),
    (0x0816, "pt-PT"),
    (0x0413, "nl-NL"),
    (0x0813, "nl-BE"),
    (0x041D, "sv-SE"),
    (0x0406, "da-DK"),
    (0x0414, "nb-NO"),
    (0x040B, "fi-FI"),
    (0x0415, "pl-PL"),
    (0x0405, "cs-CZ"),
    (0x041B, "sk-SK"),
    (0x0419, "ru-RU"),
    (0x0422, "uk-UA"),
    (0x041F, "tr-TR"),
    (0x040E, "hu-HU"),
    (0x0408, "el-GR"),
    (0x0418, "ro-RO"),
    (0x0402, "bg-BG"),
    (0x041A, "hr-HR"),
    (0x0424, "sl-SI"),
    (0x0421, "id-ID"),
    (0x042A, "vi-VN"),
    (0x0411, "ja-JP"),
    (0x0804, "zh-CN"),
    (0x0404, "zh-TW"),
    (0x0412, "ko-KR"),
];

/// The separators of the Windows locale id in a `[$-407]` prefix, or `None` for an id with no
/// row.
pub fn of_id(id: u32) -> Option<Separators> {
    tag_of_id(id).map(of_tag)
}

/// The tag of the Windows locale id in a `[$-407]` prefix, or `None` for an id with no row.
fn tag_of_id(id: u32) -> Option<&'static str> {
    let id = id & 0xFFFF;
    IDS.iter()
        .find(|(known, _)| *known == id)
        .map(|(_, tag)| *tag)
}
