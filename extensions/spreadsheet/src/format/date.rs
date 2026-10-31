//! A serial under a section with a date or a time part.
//!
//! The serial is first rounded to the precision the section shows: to the second, or to the
//! tenth, hundredth or thousandth of one when a `.0`, `.00` or `.000` follows the seconds. Its
//! whole part is the day, counted as `rule:core-classes/spreadsheet-value-types` says: in the
//! 1900 system serial 1 is 1900-01-01, serial 60 is the 1900-02-29 that Excel counts and the
//! calendar does not have, and serial 0 shows as 1900-01-00. In the 1904 system serial 0 is
//! 1904-01-01. The weekday is Excel's own, which counts 1900-01-01 as a Sunday.
//!
//! `m` and `mm` are minutes after an hour part or before a seconds part, and the month
//! otherwise. `[h]`, `[mm]` and `[ss]` are the whole hours, minutes or seconds the serial is,
//! and a negative serial shows with its sign under them. Under any other date part a negative
//! serial or one past 9999-12-31 has no date, and [`render`] returns `None`.
//!
//! `mmm` is the locale's abbreviated month name, `mmmm` its full name and `mmmmm` the full
//! name's first letter. `ddd` and `dddd` are the abbreviated and full day names
//! (`super::names`).

use super::locale::Locale;
use super::{Section, Token};
use crate::read::{civil, days_from_civil};

/// The serial of 9999-12-31 in the 1900 system.
const LAST: i64 = 2_958_465;

/// The year, month and day of the whole serial `day`, and its weekday from 0 for Sunday.
fn calendar(day: i64, date1904: bool) -> (i64, i64, i64, usize) {
    if date1904 {
        let date = civil(day + days_from_civil(1904, 1, 1));
        return (date.year, date.month, date.day, ((day + 5) % 7) as usize);
    }
    let weekday = ((day + 6) % 7) as usize;
    match day {
        0 => (1900, 1, 0, weekday),
        60 => (1900, 2, 29, weekday),
        _ => {
            let base = if day < 60 {
                days_from_civil(1899, 12, 31)
            } else {
                days_from_civil(1899, 12, 30)
            };
            let date = civil(day + base);
            (date.year, date.month, date.day, weekday)
        }
    }
}

/// Whether the date part `at` of `tokens`, an `m`, is minutes.
pub(crate) fn minutes(tokens: &[Token], at: usize) -> bool {
    let letter = |t: &Token| match t {
        Token::Part(c, _) | Token::Elapsed(c, _) => Some(*c),
        _ => None,
    };
    let before = tokens[..at].iter().rev().find_map(letter);
    let after = tokens[at + 1..].iter().find_map(letter);
    before == Some('h') || after == Some('s')
}

fn padded(value: i64, width: usize) -> String {
    format!("{value:0width$}")
}

/// `serial` under the date section `section`, or `None` for a serial with no date.
pub fn render(section: &Section, serial: f64, date1904: bool, locale: Locale) -> Option<String> {
    let names = locale.names;
    let tokens = &section.tokens;
    let elapsed = tokens.iter().any(|t| matches!(t, Token::Elapsed(..)));
    if serial < 0.0 && !elapsed {
        return None;
    }
    // The digits after the seconds: the longest run of `0` after a point.
    let mut precision = 0;
    for (at, token) in tokens.iter().enumerate() {
        if *token == Token::Point {
            let run = tokens[at + 1..]
                .iter()
                .take_while(|t| **t == Token::Digit('0'))
                .count();
            precision = precision.max(run.min(3));
        }
    }
    let per_second = 10i64.pow(precision as u32);
    let scaled = (serial.abs() * 86_400.0 * per_second as f64).round();
    if scaled > 1e17 {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "bounded by the check above"
    )]
    let units = scaled as i64;
    let per_day = 86_400 * per_second;
    let day = units / per_day;
    if day > LAST && !elapsed {
        return None;
    }
    let of_day = units % per_day;
    let (hour, minute, second, fraction) = (
        of_day / (3600 * per_second),
        of_day / (60 * per_second) % 60,
        of_day / per_second % 60,
        of_day % per_second,
    );
    let (year, month, date, weekday) = calendar(day, date1904);
    let twelve = tokens.iter().any(|t| matches!(t, Token::Meridiem(_)));
    let shown_hour = if twelve {
        match hour % 12 {
            0 => 12,
            h => h,
        }
    } else {
        hour
    };
    let mut out = String::new();
    if serial < 0.0 {
        out.push('-');
    }
    let mut at = 0;
    while at < tokens.len() {
        match &tokens[at] {
            Token::Literal(text) => out.push_str(text),
            Token::Part('y', n) if *n <= 2 => out.push_str(&padded(year % 100, 2)),
            Token::Part('y' | 'e', _) => out.push_str(&padded(year, 4)),
            Token::Part('m', n) if minutes(tokens, at) => {
                out.push_str(&padded(minute, (*n).min(2)));
            }
            Token::Part('m', n) => {
                let at = (month - 1) as usize;
                match n {
                    1 | 2 => out.push_str(&padded(month, *n)),
                    3 => out.push_str(names.months_short[at]),
                    5 => out.extend(names.months[at].chars().next()),
                    _ => out.push_str(names.months[at]),
                }
            }
            Token::Part('d', n) => match n {
                1 | 2 => out.push_str(&padded(date, *n)),
                3 => out.push_str(names.days_short[weekday]),
                _ => out.push_str(names.days[weekday]),
            },
            Token::Part('h', n) => out.push_str(&padded(shown_hour, (*n).min(2))),
            Token::Part('s', n) => out.push_str(&padded(second, (*n).min(2))),
            Token::Elapsed(c, n) => {
                let whole = match c {
                    'h' => units / (3600 * per_second),
                    'm' => units / (60 * per_second),
                    _ => units / per_second,
                };
                out.push_str(&padded(whole, *n));
            }
            Token::Meridiem(None) => out.push_str(if hour >= 12 { "PM" } else { "AM" }),
            Token::Meridiem(Some((am, pm))) => out.push(if hour >= 12 { *pm } else { *am }),
            Token::Point if precision > 0 && tokens.get(at + 1) == Some(&Token::Digit('0')) => {
                let run = tokens[at + 1..]
                    .iter()
                    .take_while(|t| **t == Token::Digit('0'))
                    .count();
                let digits = padded(fraction, precision);
                out.push_str(locale.separators.decimal);
                out.push_str(&digits[..run.min(precision)]);
                at += run;
            }
            Token::Point => out.push('.'),
            Token::Comma => out.push(','),
            Token::Slash => out.push('/'),
            Token::Percent => out.push('%'),
            Token::Digit(c) => out.push(*c),
            _ => {}
        }
        at += 1;
    }
    Some(out)
}
