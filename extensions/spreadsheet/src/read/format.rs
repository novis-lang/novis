//! Which read type a number format gives a number: the date rule of the read table
//! (`rule:core-classes/spreadsheet-value-types`).
//!
//! [`classify`] reads a format code's first section, lexed by the renderer
//! (`crate::format::lex`), so quoted text, an escape, a colour, a condition and the `E` of
//! `E+00` say nothing. `y`, `d` and `e` are a date, `h`, `s` and `AM/PM` a time, and `m` a month,
//! or minutes after an hour or before seconds. A bracketed `[h]`, `[m]` or `[s]` is elapsed time.
//! A format with a date part and a time part is a clock time. [`builtin`] gives the formats Excel
//! numbers and does not write into `styles.xml`.

use std::sync::Arc;

use crate::format::{self, Token, date};

/// The read type a number format gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A number.
    Plain,
    Date,
    Time,
    Clock,
    Duration,
}

/// A cell style's number format: the read type it gives, and its code for the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    pub kind: Kind,
    pub code: Arc<str>,
}

impl Style {
    /// The style of the format `id`, with `custom` its code when the file defines one. An id
    /// with neither is `General`.
    pub fn of(id: u32, custom: Option<&Arc<str>>) -> Self {
        match custom {
            Some(code) => Self {
                kind: classify(code),
                code: Arc::clone(code),
            },
            None => Self {
                kind: builtin(id).unwrap_or(Kind::Plain),
                code: Arc::from(format::builtin(id).unwrap_or("General")),
            },
        }
    }

    /// The style an ods cell of `kind` has: its kind, and the format the writer gives it.
    pub fn of_kind(kind: Kind) -> Self {
        let code = match kind {
            Kind::Plain => "General",
            Kind::Date => "yyyy-mm-dd",
            Kind::Time => "hh:mm:ss",
            Kind::Clock => "yyyy-mm-dd hh:mm",
            Kind::Duration => "[h]:mm:ss",
        };
        Self {
            kind,
            code: Arc::from(code),
        }
    }
}

/// The kind of the built-in format `id`, or `None` for an id `styles.xml` must define.
pub fn builtin(id: u32) -> Option<Kind> {
    Some(match id {
        0..=13 | 37..=44 | 48 | 49 => Kind::Plain,
        14..=17 | 27..=31 | 34..=36 | 50..=58 => Kind::Date,
        22 => Kind::Clock,
        18..=21 | 32 | 33 | 45 | 47 => Kind::Time,
        46 => Kind::Duration,
        _ => return None,
    })
}

/// The kind of the format code `code`.
pub fn classify(code: &str) -> Kind {
    let first = format::split(code).into_iter().next().unwrap_or("");
    let tokens = format::lex(first).tokens;
    let (mut date, mut time, mut elapsed) = (false, false, false);
    for (at, token) in tokens.iter().enumerate() {
        match token {
            Token::Elapsed(..) => elapsed = true,
            Token::Part('m', _) if date::minutes(&tokens, at) => time = true,
            Token::Part('y' | 'd' | 'e' | 'm', _) => date = true,
            Token::Part(..) | Token::Meridiem(_) => time = true,
            _ => {}
        }
    }
    if elapsed {
        return Kind::Duration;
    }
    match (date, time) {
        (true, true) => Kind::Clock,
        (true, false) => Kind::Date,
        (false, true) => Kind::Time,
        (false, false) => Kind::Plain,
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, classify};

    #[test]
    fn format_codes_classify_by_their_parts() {
        for (code, kind) in [
            ("General", Kind::Plain),
            ("0.00", Kind::Plain),
            ("#,##0 \"days\"", Kind::Plain),
            ("yyyy-mm-dd", Kind::Date),
            ("mmm yyyy", Kind::Date),
            ("mmmm", Kind::Date),
            ("hh:mm:ss", Kind::Time),
            ("h:mm AM/PM", Kind::Time),
            ("yyyy-mm-dd hh:mm", Kind::Clock),
            ("[h]:mm:ss", Kind::Duration),
            ("[mm]:ss", Kind::Duration),
            ("[Red]0.00;[Blue]-0.00", Kind::Plain),
            ("[$-409]d-mmm-yy", Kind::Date),
        ] {
            assert_eq!(classify(code), kind, "{code}");
        }
    }
}
