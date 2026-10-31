//! Format codes as ods number styles, the other way round from `read::ods_styles`' table.
//!
//! [`write`] lexes a code with `crate::format` and writes each section as the number style that
//! shows the same text. A section that shows a date is a `number:date-style`, one that shows only
//! a time a `number:time-style`, one with `@` a `number:text-style`, one with `%` a
//! `number:percentage-style`, and any other a `number:number-style`. A section with an elapsed
//! part, such as `[h]`, sets `number:truncate-on-overflow="false"`.
//!
//! **Sections are maps.** The first sections are styles of their own, named after the main style
//! with `P0` and `P1`, and the last is the main style, which maps to them: two sections map
//! `value()>=0` to the first, and three map `value()>0` and `value()<0` to the first two. A
//! section with a condition such as `[>5]` maps that condition. A text section after three
//! numeric ones has no ods form, and is left out. So are a colour, a locale prefix and a literal
//! inside a number's digits.

use std::fmt::Write as _;

use crate::format::{self, Op, Section, Token};
use crate::xml;

/// Appends the number styles for `code`, the main one named `name`.
pub fn write(out: &mut String, name: &str, code: &str) {
    let mut sections: Vec<Section> = format::split(code).into_iter().map(format::lex).collect();
    if sections.len() > 3 {
        sections.truncate(3);
    }
    if sections.len() > 1
        && sections
            .last()
            .is_some_and(|last| last.tokens.contains(&Token::At))
    {
        sections.pop();
    }
    let last = sections.len() - 1;
    let mut maps = Vec::new();
    for (at, section) in sections.iter().enumerate().take(last) {
        let mapped = format!("{name}P{at}");
        let condition = match (section.condition, last, at) {
            (Some(condition), ..) => condition_text(condition),
            (None, 1, _) => "value()>=0".to_owned(),
            (None, _, 0) => "value()>0".to_owned(),
            (None, ..) => "value()<0".to_owned(),
        };
        style(out, &mapped, section, &[]);
        maps.push((condition, mapped));
    }
    style(out, name, &sections[last], &maps);
}

fn condition_text(condition: format::Condition) -> String {
    let op = match condition.op {
        Op::Less => "<",
        Op::LessEqual => "<=",
        Op::Equal => "=",
        Op::NotEqual => "!=",
        Op::GreaterEqual => ">=",
        Op::Greater => ">",
    };
    let mut value = String::new();
    crate::cells::number(&mut value, condition.value);
    format!("value(){op}{value}")
}

/// The kind of style a section is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Number,
    Percentage,
    Text,
    Date,
    Time,
}

fn kind(section: &Section, months: &[bool]) -> Kind {
    if section.tokens.contains(&Token::At) {
        return Kind::Text;
    }
    if section.is_date() {
        let dated = section.tokens.iter().zip(months).any(|(token, &month)| {
            matches!(token, Token::Part('y' | 'd' | 'e', _))
                || (month && matches!(token, Token::Part('m', _)))
        });
        return if dated { Kind::Date } else { Kind::Time };
    }
    if section.tokens.contains(&Token::Percent) {
        Kind::Percentage
    } else {
        Kind::Number
    }
}

/// For each token, whether an `m` there is a month: it is a minute after an hour or before a
/// second.
fn months(tokens: &[Token]) -> Vec<bool> {
    let parts: Vec<(usize, char)> = tokens
        .iter()
        .enumerate()
        .filter_map(|(at, token)| match token {
            Token::Part(c, _) | Token::Elapsed(c, _) => Some((at, *c)),
            _ => None,
        })
        .collect();
    let mut out = vec![false; tokens.len()];
    for (index, &(at, c)) in parts.iter().enumerate() {
        if c != 'm' || matches!(tokens[at], Token::Elapsed(..)) {
            continue;
        }
        let after_hour = index > 0 && parts[index - 1].1 == 'h';
        let before_second = parts.get(index + 1).is_some_and(|&(_, next)| next == 's');
        out[at] = !(after_hour || before_second);
    }
    out
}

fn style(out: &mut String, name: &str, section: &Section, maps: &[(String, String)]) {
    let months = months(&section.tokens);
    let kind = kind(section, &months);
    let tag = match kind {
        Kind::Number => "number:number-style",
        Kind::Percentage => "number:percentage-style",
        Kind::Text => "number:text-style",
        Kind::Date => "number:date-style",
        Kind::Time => "number:time-style",
    };
    let _ = write!(out, "<{tag} style:name=\"{name}\"");
    if section
        .tokens
        .iter()
        .any(|token| matches!(token, Token::Elapsed(..)))
    {
        out.push_str(" number:truncate-on-overflow=\"false\"");
    }
    out.push('>');
    match kind {
        Kind::Date | Kind::Time => dated(out, &section.tokens, &months),
        Kind::Text => {
            for token in &section.tokens {
                match token {
                    Token::At => out.push_str("<number:text-content/>"),
                    token => text(out, &shown(token)),
                }
            }
        }
        Kind::Number | Kind::Percentage => numeric(out, &section.tokens),
    }
    for (condition, mapped) in maps {
        let _ = write!(
            out,
            "<style:map style:condition=\"{}\" style:apply-style-name=\"{mapped}\"/>",
            condition.replace('<', "&lt;").replace('>', "&gt;")
        );
    }
    let _ = write!(out, "</{tag}>");
}

/// The text a token shows when it is not part of a number or a date.
fn shown(token: &Token) -> String {
    match token {
        Token::Literal(text) => text.clone(),
        Token::Digit(c) => c.to_string(),
        Token::Point => ".".to_owned(),
        Token::Comma => ",".to_owned(),
        Token::Percent => "%".to_owned(),
        Token::Slash => "/".to_owned(),
        Token::Exponent(_) | Token::At | Token::General => String::new(),
        Token::Part(c, n) | Token::Elapsed(c, n) => c.to_string().repeat(*n),
        Token::Meridiem(_) => "AM/PM".to_owned(),
    }
}

fn text(out: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    out.push_str("<number:text>");
    xml::odf(out, text, false);
    out.push_str("</number:text>");
}

fn long(n: usize) -> &'static str {
    if n >= 2 { " number:style=\"long\"" } else { "" }
}

fn dated(out: &mut String, tokens: &[Token], months: &[bool]) {
    let mut literal = String::new();
    let mut at = 0;
    while at < tokens.len() {
        let token = &tokens[at];
        at += 1;
        let element = match token {
            Token::Part('y' | 'e', n) => {
                format!(
                    "<number:year{}/>",
                    if *n >= 3 {
                        " number:style=\"long\""
                    } else {
                        ""
                    }
                )
            }
            Token::Part('m', n) if months[at - 1] => match n {
                1 => "<number:month/>".to_owned(),
                2 => "<number:month number:style=\"long\"/>".to_owned(),
                3 => "<number:month number:textual=\"true\"/>".to_owned(),
                _ => "<number:month number:style=\"long\" number:textual=\"true\"/>".to_owned(),
            },
            Token::Part('d', n) => match n {
                1 => "<number:day/>".to_owned(),
                2 => "<number:day number:style=\"long\"/>".to_owned(),
                3 => "<number:day-of-week/>".to_owned(),
                _ => "<number:day-of-week number:style=\"long\"/>".to_owned(),
            },
            Token::Part('h', n) | Token::Elapsed('h', n) => format!("<number:hours{}/>", long(*n)),
            Token::Part('m', n) | Token::Elapsed('m', n) => {
                format!("<number:minutes{}/>", long(*n))
            }
            Token::Part('s', n) | Token::Elapsed('s', n) => {
                // A second's decimals are `.0`, `.00` or `.000` right after it.
                let mut places = 0;
                if tokens.get(at) == Some(&Token::Point) {
                    while tokens.get(at + 1 + places) == Some(&Token::Digit('0')) {
                        places += 1;
                    }
                    if places > 0 {
                        at += 1 + places;
                    }
                }
                let decimals = if places > 0 {
                    format!(" number:decimal-places=\"{places}\"")
                } else {
                    String::new()
                };
                format!("<number:seconds{}{decimals}/>", long(*n))
            }
            Token::Meridiem(_) => "<number:am-pm/>".to_owned(),
            token => {
                literal.push_str(&shown(token));
                continue;
            }
        };
        text(out, &std::mem::take(&mut literal));
        out.push_str(&element);
    }
    text(out, &literal);
}

fn is_numeric(token: &Token) -> bool {
    matches!(
        token,
        Token::Digit(_)
            | Token::Point
            | Token::Comma
            | Token::Exponent(_)
            | Token::Slash
            | Token::General
    )
}

fn numeric(out: &mut String, tokens: &[Token]) {
    let first = tokens.iter().position(is_numeric);
    let last = tokens.iter().rposition(is_numeric);
    let (Some(first), Some(last)) = (first, last) else {
        let all: String = tokens.iter().map(shown).collect();
        text(out, &all);
        return;
    };
    let before: String = tokens[..first].iter().map(shown).collect();
    let after: String = tokens[last + 1..].iter().map(shown).collect();
    text(out, &before);
    number(out, &tokens[first..=last]);
    text(out, &after);
}

/// Counts the digit placeholders of `tokens`, and the `0`s among them.
fn digits(tokens: &[Token]) -> (usize, usize) {
    let mut all = 0;
    let mut zeros = 0;
    for token in tokens {
        if let Token::Digit(c) = token {
            all += 1;
            zeros += usize::from(*c == '0');
        }
    }
    (all, zeros)
}

fn number(out: &mut String, run: &[Token]) {
    if run.contains(&Token::General) {
        out.push_str("<number:number/>");
        return;
    }
    if let Some(slash) = run.iter().position(|token| *token == Token::Slash) {
        let left = &run[..slash];
        let right = &run[slash + 1..];
        let (whole, numerator) = match left.iter().rposition(|t| matches!(t, Token::Literal(_))) {
            Some(gap) => (&left[..gap], &left[gap + 1..]),
            None => (&left[..0], left),
        };
        let (_, whole_zeros) = digits(whole);
        let (numerator, _) = digits(numerator);
        let _ = write!(
            out,
            "<number:fraction number:min-integer-digits=\"{whole_zeros}\" number:min-numerator-digits=\"{}\"",
            numerator.max(1)
        );
        let fixed: String = right
            .iter()
            .filter_map(|t| match t {
                Token::Literal(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        match fixed.trim().parse::<u32>() {
            Ok(value) if value > 0 => {
                let _ = write!(out, " number:denominator-value=\"{value}\"");
            }
            _ => {
                let (denominator, _) = digits(right);
                let _ = write!(
                    out,
                    " number:min-denominator-digits=\"{}\"",
                    denominator.max(1)
                );
            }
        }
        out.push_str("/>");
        return;
    }
    let exponent = run.iter().position(|t| matches!(t, Token::Exponent(_)));
    let mantissa = &run[..exponent.unwrap_or(run.len())];
    let point = mantissa.iter().position(|t| *t == Token::Point);
    let integer = &mantissa[..point.unwrap_or(mantissa.len())];
    let fraction = point.map_or(&mantissa[..0], |at| &mantissa[at + 1..]);
    let (_, integer_zeros) = digits(integer);
    let (places, fraction_zeros) = digits(fraction);
    // A comma between two digit placeholders groups thousands. One after the last scales.
    let last_digit = integer.iter().rposition(|t| matches!(t, Token::Digit(_)));
    let grouping = last_digit.is_some_and(|last| integer[..last].contains(&Token::Comma));
    match exponent {
        Some(at) => {
            let (exponent_digits, _) = digits(&run[at + 1..]);
            let _ = write!(
                out,
                "<number:scientific-number number:decimal-places=\"{places}\" number:min-decimal-places=\"{fraction_zeros}\" number:min-integer-digits=\"{integer_zeros}\" number:min-exponent-digits=\"{}\"",
                exponent_digits.max(1)
            );
        }
        None => {
            let _ = write!(
                out,
                "<number:number number:decimal-places=\"{places}\" number:min-decimal-places=\"{fraction_zeros}\" number:min-integer-digits=\"{integer_zeros}\""
            );
        }
    }
    if grouping {
        out.push_str(" number:grouping=\"true\"");
    }
    out.push_str("/>");
}
