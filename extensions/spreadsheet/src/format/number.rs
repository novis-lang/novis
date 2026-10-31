//! A number under a section with no date part, and under `General`.
//!
//! [`render`] reads the section's placeholders in three places: before the point, after it and
//! after `E+`. A `,` between two placeholders before the point groups the digits by three, and
//! each `,` after the last placeholder divides the number by 1,000. Each `%` multiplies it by
//! 100. Before the point, a `0` shows a digit or `0`, a `?` a digit or a space and a `#` a digit
//! or nothing, and the first placeholder takes every digit the others have no room for. After
//! the point, the number is rounded to as many digits as there are placeholders, and a trailing
//! `0` digit under `#` is dropped and under `?` is a space. With `E+`, the exponent is a
//! multiple of the number of placeholders before the point when there are two or more, so
//! `##0.0E+0` shows `12.3E+3`.
//!
//! A section with a `/` after a placeholder is a fraction ([`fraction`]). Placeholders before the
//! numerator's own show the whole part, and without them the fraction is improper. The
//! denominator is the closest one with as many digits as its placeholders, or the number written
//! there, as in `# ?/8`.

use super::Token;
use super::locale::Separators;

/// The first 15 significant digits of a number: the value is `0.d1d2d3… × 10^point`. No digit
/// is a trailing zero, and zero has no digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digits {
    digits: Vec<u8>,
    point: i32,
}

impl Digits {
    /// The digits of `|v|`, which must be finite.
    pub fn of(v: f64) -> Self {
        let v = v.abs();
        if v == 0.0 {
            return Self {
                digits: Vec::new(),
                point: 0,
            };
        }
        let text = format!("{v:.14e}");
        let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
        let exponent: i32 = exponent.parse().unwrap_or(0);
        let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
        while digits.last() == Some(&b'0') {
            digits.pop();
        }
        Self {
            digits,
            point: exponent + 1,
        }
    }

    fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// Rounds to `decimals` digits after the point, half away from zero.
    fn round(&mut self, decimals: i32) {
        let keep = self.point + decimals;
        if keep < 0 {
            self.digits.clear();
            return;
        }
        let keep = keep as usize;
        if keep >= self.digits.len() {
            return;
        }
        let up = self.digits[keep] >= b'5';
        self.digits.truncate(keep);
        if up {
            loop {
                match self.digits.last_mut() {
                    Some(b'9') => {
                        self.digits.pop();
                    }
                    Some(digit) => {
                        *digit += 1;
                        break;
                    }
                    None => {
                        self.digits.push(b'1');
                        self.point += 1;
                        break;
                    }
                }
            }
        }
        while self.digits.last() == Some(&b'0') {
            self.digits.pop();
        }
        if self.digits.is_empty() {
            self.point = 0;
        }
    }

    /// The digits before the point, with no leading zero: empty below 1.
    fn whole(&self) -> String {
        if self.point <= 0 {
            return String::new();
        }
        let point = self.point as usize;
        let mut out: String = self.digits[..point.min(self.digits.len())]
            .iter()
            .map(|&d| char::from(d))
            .collect();
        out.extend(std::iter::repeat_n(
            '0',
            point.saturating_sub(self.digits.len()),
        ));
        out
    }

    /// The first `n` digits after the point.
    fn fraction(&self, n: usize) -> String {
        (0..n)
            .map(|k| {
                let at = self.point + k as i32;
                if at < 0 {
                    '0'
                } else {
                    self.digits.get(at as usize).map_or('0', |&d| char::from(d))
                }
            })
            .collect()
    }
}

/// `|v|` with at most `decimals` digits after the point, its trailing zeros dropped.
fn fixed(v: f64, decimals: i32, separators: Separators) -> String {
    let mut digits = Digits::of(v);
    digits.round(decimals);
    let whole = digits.whole();
    let mut out = if whole.is_empty() {
        "0".to_owned()
    } else {
        whole
    };
    let fraction = digits.fraction(decimals.max(0) as usize);
    let fraction = fraction.trim_end_matches('0');
    if !fraction.is_empty() {
        out.push_str(separators.decimal);
        out.push_str(fraction);
    }
    out
}

/// `|v|` in scientific form with at most `decimals` digits after the point, such as `1.23457E+12`.
fn scientific(v: f64, decimals: i32, separators: Separators) -> String {
    let digits = Digits::of(v);
    let mut exponent = digits.point - 1;
    let mut mantissa = Digits {
        digits: digits.digits,
        point: 1,
    };
    mantissa.round(decimals);
    if mantissa.point > 1 {
        exponent += 1;
        mantissa.point -= 1;
    }
    let mut out = mantissa.whole();
    let fraction = mantissa.fraction(decimals as usize);
    let fraction = fraction.trim_end_matches('0');
    if !fraction.is_empty() {
        out.push_str(separators.decimal);
        out.push_str(fraction);
    }
    out.push('E');
    out.push(if exponent < 0 { '-' } else { '+' });
    if exponent.abs() < 10 {
        out.push('0');
    }
    out.push_str(&exponent.abs().to_string());
    out
}

/// `v` under `General`: as many digits as fit in 11 characters, the sign not counted, and the
/// scientific form for a number too large or too small for that.
pub fn general(v: f64, separators: Separators) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    const WIDTH: usize = 11;
    let a = v.abs();
    let exponent = Digits::of(a).point - 1;
    let text = if (-4..=-1).contains(&exponent) {
        fixed(a, 9, separators)
    } else if exponent.abs() <= 9 {
        let all = fixed(a, 12, separators);
        if all.chars().count() <= WIDTH {
            all
        } else {
            let ten = fixed(a, 9 - exponent, separators);
            if ten.chars().count() <= WIDTH {
                ten
            } else {
                scientific(a, 5, separators)
            }
        }
    } else if exponent == 10 {
        fixed(a, 0, separators)
    } else {
        scientific(a, 5, separators)
    };
    if v < 0.0 { format!("-{text}") } else { text }
}

/// `digits` in the placeholders `places`, from the right: the first placeholder takes every
/// digit left over, and a placeholder with no digit shows its own filler.
fn fill(places: &[char], digits: &str) -> Vec<String> {
    let digits: Vec<char> = digits.chars().collect();
    let mut out = vec![String::new(); places.len()];
    for (from_right, place) in places.iter().rev().enumerate() {
        let at = places.len() - 1 - from_right;
        let first = at == 0;
        if from_right < digits.len() {
            let end = digits.len() - from_right;
            let start = if first { 0 } else { end - 1 };
            out[at] = digits[start..end].iter().collect();
        } else {
            out[at] = match place {
                '0' => "0".to_owned(),
                '?' => " ".to_owned(),
                _ => String::new(),
            };
        }
    }
    out
}

/// `text` with the group separator after every third digit from the right.
fn grouped(text: &str, separators: Separators) -> String {
    let count = text.chars().filter(char::is_ascii_digit).count();
    let mut seen = 0;
    let mut out = String::new();
    for c in text.chars() {
        out.push(c);
        if c.is_ascii_digit() {
            seen += 1;
            let left = count - seen;
            if left > 0 && left % 3 == 0 {
                out.push_str(separators.group);
            }
        }
    }
    out
}

/// `v` under the number section `tokens`. `signed` puts a `-` before a negative number.
pub fn render(tokens: &[Token], v: f64, signed: bool, separators: Separators) -> String {
    let sign = if v < 0.0 && signed { "-" } else { "" };
    let mut a = v.abs();
    for token in tokens {
        if *token == Token::Percent {
            a *= 100.0;
        }
    }
    if tokens.contains(&Token::General) {
        let mut out = sign.to_owned();
        for token in tokens {
            match token {
                Token::General => out.push_str(&general(a, separators)),
                Token::Literal(text) => out.push_str(text),
                Token::Percent => out.push('%'),
                _ => {}
            }
        }
        return out;
    }
    if let Some(slash) = tokens.iter().position(|t| *t == Token::Slash)
        && tokens[..slash].iter().any(|t| matches!(t, Token::Digit(_)))
    {
        return format!("{sign}{}", fraction(tokens, slash, a, separators));
    }
    let exponent_at = tokens.iter().position(|t| matches!(t, Token::Exponent(_)));
    let mantissa = &tokens[..exponent_at.unwrap_or(tokens.len())];
    let point_at = mantissa
        .iter()
        .position(|t| *t == Token::Point)
        .unwrap_or(mantissa.len());
    let digit = |t: &Token| matches!(t, Token::Digit(_));
    let last_digit = mantissa.iter().rposition(digit);
    let mut scale = 0;
    let mut grouping = false;
    for (at, token) in mantissa.iter().enumerate() {
        if *token != Token::Comma {
            continue;
        }
        let before = mantissa[..at].iter().any(digit);
        if before && last_digit.is_some_and(|last| at > last) {
            scale += 1;
        } else if before && at < point_at && mantissa[at..point_at].iter().any(digit) {
            grouping = true;
        }
    }
    for _ in 0..scale {
        a /= 1000.0;
    }
    let places = |part: &[Token]| -> Vec<char> {
        part.iter()
            .filter_map(|t| match t {
                Token::Digit(c) => Some(*c),
                _ => None,
            })
            .collect()
    };
    let whole_places = places(&mantissa[..point_at]);
    let fraction_places = places(&mantissa[point_at..]);
    let decimals = fraction_places.len() as i32;
    let mut digits = Digits::of(a);
    let mut exponent = 0;
    if exponent_at.is_some() {
        let group = whole_places.len().max(1) as i32;
        if !digits.is_zero() {
            exponent = (digits.point - 1).div_euclid(group) * group;
        }
        digits.point -= exponent;
        digits.round(decimals);
        if digits.point > group {
            exponent += group;
            digits.point -= group;
        }
    } else {
        digits.round(decimals);
    }
    let whole = fill(&whole_places, &digits.whole());
    let mut fraction: Vec<String> = digits
        .fraction(fraction_places.len())
        .chars()
        .map(String::from)
        .collect();
    for (at, place) in fraction_places.iter().enumerate().rev() {
        if fraction[at] != "0" || *place == '0' {
            break;
        }
        fraction[at] = if *place == '?' {
            " ".to_owned()
        } else {
            String::new()
        };
    }
    // The part before the point, written whole so the group separators can go into it.
    let mut before = String::new();
    let mut out = sign.to_owned();
    let mut placed = 0;
    // With no placeholder before the point, the whole part shows before it; with no
    // placeholder at all, the number does not show.
    if whole_places.is_empty() && !fraction_places.is_empty() {
        before.push_str(&digits.whole());
    }
    for (at, token) in mantissa.iter().enumerate() {
        if at == point_at {
            out.push_str(&if grouping {
                grouped(&before, separators)
            } else {
                before.clone()
            });
            before.clear();
        }
        let target = if at < point_at { &mut before } else { &mut out };
        match token {
            Token::Digit(_) if at < point_at => {
                target.push_str(&whole[placed]);
                placed += 1;
            }
            Token::Digit(_) => {
                target.push_str(&fraction[placed]);
                placed += 1;
            }
            Token::Point => {
                placed = 0;
                target.push_str(separators.decimal);
            }
            Token::Literal(text) => target.push_str(text),
            Token::Percent => target.push('%'),
            Token::At | Token::Slash => target.push(if *token == Token::At { '@' } else { '/' }),
            _ => {}
        }
    }
    if point_at == mantissa.len() {
        out.push_str(&if grouping {
            grouped(&before, separators)
        } else {
            before
        });
    }
    if let Some(at) = exponent_at {
        let Token::Exponent(plus) = tokens[at] else {
            return out;
        };
        out.push('E');
        if exponent < 0 {
            out.push('-');
        } else if plus {
            out.push('+');
        }
        let rest = &tokens[at + 1..];
        let exponent_places = places(rest);
        let shown = fill(&exponent_places, &exponent.unsigned_abs().to_string());
        let mut placed = 0;
        for token in rest {
            match token {
                Token::Digit(_) => {
                    out.push_str(&shown[placed]);
                    placed += 1;
                }
                Token::Literal(text) => out.push_str(text),
                Token::Percent => out.push('%'),
                _ => {}
            }
        }
    }
    out
}

/// The numerator and denominator of the closest fraction to `x` with a denominator of at most
/// `most`, from the continued fraction of `x`.
fn closest(x: f64, most: f64) -> (f64, f64) {
    let mut b = x;
    let (mut p2, mut p1, mut p) = (0.0, 1.0, 0.0);
    let (mut q2, mut q1, mut q) = (1.0, 0.0, 0.0);
    while q1 < most {
        let a = b.floor();
        p = a * p1 + p2;
        q = a * q1 + q2;
        if b - a < 0.000_000_05 {
            break;
        }
        b = 1.0 / (b - a);
        (p2, p1, q2, q1) = (p1, p, q1, q);
    }
    if q > most {
        if q1 > most {
            (p, q) = (p2, q2);
        } else {
            (p, q) = (p1, q1);
        }
    }
    (p, q)
}

/// `a`, not negative, under a fraction section whose first `/` is at `slash`.
fn fraction(tokens: &[Token], slash: usize, a: f64, separators: Separators) -> String {
    let numerator_start = tokens[..slash]
        .iter()
        .rposition(|t| !matches!(t, Token::Digit(_)))
        .map_or(0, |at| at + 1);
    let numerator_places: Vec<char> = tokens[numerator_start..slash]
        .iter()
        .filter_map(|t| match t {
            Token::Digit(c) => Some(*c),
            _ => None,
        })
        .collect();
    let head = &tokens[..numerator_start];
    let mixed = head.iter().any(|t| matches!(t, Token::Digit(_)));
    // The denominator: placeholders, or the digits written after the slash.
    let mut pattern = String::new();
    let mut tail_start = tokens.len();
    let mut tail_text = String::new();
    for (at, token) in tokens.iter().enumerate().skip(slash + 1) {
        match token {
            Token::Digit(c) => pattern.push(*c),
            Token::Literal(text) => {
                let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
                pattern.push_str(&digits);
                if digits.len() < text.len() {
                    tail_text = text[digits.len()..].to_owned();
                    tail_start = at + 1;
                    break;
                }
            }
            _ => {
                tail_start = at;
                break;
            }
        }
    }
    let fixed = pattern
        .chars()
        .any(|c| c.is_ascii_digit() && c != '0')
        .then(|| pattern.replace(['?', '#'], "0").parse::<f64>().ok())
        .flatten();
    let (mut whole, mut numerator, denominator) = match fixed {
        Some(denominator) => {
            if mixed {
                let whole = a.floor();
                ((whole), ((a - whole) * denominator).round(), denominator)
            } else {
                (0.0, (a * denominator).round(), denominator)
            }
        }
        None => {
            let most = 10f64.powi(pattern.chars().count() as i32) - 1.0;
            let (p, q) = closest(a, most.max(1.0));
            if mixed {
                let whole = (p / q).floor();
                (whole, p - whole * q, q)
            } else {
                (0.0, p, q)
            }
        }
    };
    if mixed && numerator >= denominator {
        whole += 1.0;
        numerator -= denominator;
    }
    let whole_places: Vec<char> = head
        .iter()
        .filter_map(|t| match t {
            Token::Digit(c) => Some(*c),
            _ => None,
        })
        .collect();
    let blank = mixed && numerator == 0.0;
    let whole_text = format!("{whole:.0}");
    let whole_shown = if mixed && whole == 0.0 && !blank {
        fill(&whole_places, "")
    } else {
        fill(&whole_places, &whole_text)
    };
    let mut out = String::new();
    let mut placed = 0;
    for token in head {
        match token {
            Token::Digit(_) => {
                out.push_str(&whole_shown[placed]);
                placed += 1;
            }
            Token::Literal(text) => out.push_str(text),
            Token::Point => out.push_str(separators.decimal),
            _ => {}
        }
    }
    if blank {
        let width = numerator_places.len() + 1 + pattern.chars().count();
        out.extend(std::iter::repeat_n(' ', width));
    } else {
        out.push_str(&fill(&numerator_places, &format!("{numerator:.0}")).concat());
        out.push('/');
        if fixed.is_some() {
            out.push_str(&pattern);
        } else {
            let shown = format!("{denominator:.0}");
            out.push_str(&shown);
            for place in pattern.chars().skip(shown.len()) {
                if place == '?' {
                    out.push(' ');
                } else if place == '0' {
                    out.push('0');
                }
            }
        }
    }
    out.push_str(&tail_text);
    for token in &tokens[tail_start.min(tokens.len())..] {
        if let Token::Literal(text) = token {
            out.push_str(text);
        }
    }
    out
}
