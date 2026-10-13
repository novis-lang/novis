//! Which read type a number format gives a number: the date rule of the read table
//! (`rule:core-classes/spreadsheet-value-types`).
//!
//! [`classify`] reads a format code's first section. Quoted text, an escaped character, the
//! characters after `_` and `*`, and a bracket such as a colour or a condition say nothing. `y`,
//! `d` and `e` are a date, `h`, `s` and `AM/PM` a time, and `m` a month beside a date part or
//! alone, and minutes beside a time part. A bracketed `[h]`, `[m]` or `[s]` is elapsed time. A
//! format with a date part and a time part is a clock time. [`builtin`] gives the formats Excel
//! numbers and does not write into `styles.xml`.

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
    let lower = code.to_ascii_lowercase();
    let mut chars = lower.chars().peekable();
    let (mut date, mut time, mut month, mut elapsed) = (false, false, false, false);
    while let Some(c) = chars.next() {
        match c {
            ';' => break,
            '"' => {
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                }
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            '[' => {
                let mut inner = String::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                    inner.push(c);
                }
                if !inner.is_empty() && inner.chars().all(|c| matches!(c, 'h' | 'm' | 's')) {
                    elapsed = true;
                }
            }
            'g' if lower[..].contains("general") => return Kind::Plain,
            'y' | 'd' | 'e' => date = true,
            'h' | 's' => time = true,
            'm' => month = true,
            'a' => {
                let rest: String = chars.clone().take(4).collect();
                if rest.starts_with("m/pm") || rest.starts_with("/p") {
                    time = true;
                    for _ in 0..if rest.starts_with("m/pm") { 4 } else { 2 } {
                        chars.next();
                    }
                }
            }
            _ => {}
        }
    }
    if elapsed {
        return Kind::Duration;
    }
    let date = date || (month && !time);
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
