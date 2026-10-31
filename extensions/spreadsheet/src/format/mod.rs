//! The number-format renderer: a cell's value as Excel displays it under its format code
//! (`rule:core-classes/spreadsheet-reads-as-a-stream`'s `text` option,
//! `rule:core-classes/spreadsheet-styles-and-defaults`' auto width).
//!
//! [`render`] splits the code into its sections, at most four, on each `;` outside quotes,
//! escapes and brackets. A number takes the first section when it is positive, the second when
//! it is negative and the third when it is zero, as [`choose`] says; a section with a condition
//! such as `[>100]` is taken when its condition holds. Text takes the fourth section, or the last
//! one when it has `@`, and shows as it is otherwise. A boolean is `TRUE` or `FALSE` and an error
//! its own text, whatever the format.
//!
//! A section is lexed once ([`lex`]) into literals and the parts that show the value. A colour is
//! parsed and dropped, `_x` is one space and `*x` is dropped, because a rendered text has no
//! cell to fill. `[$€-407]` is the literal `€` and the locale `0x407`, whose separators and
//! month and day names the section uses ([`locale`], [`names`]). A section with no prefix uses
//! the read's `locale`. A section with a date or a time part is rendered by [`date`] and any
//! other by [`number`], `General` by [`number::general`].
//!
//! The digits of a number are its first 15 significant digits, rounded half away from zero at
//! the last digit shown, which is how Excel shows `2.675` under `0.00` as `2.68`.
//!
//! What it spends: the lexed section and the text it returns, per cell rendered. Nothing is kept
//! between cells.

pub mod date;
pub mod locale;
pub mod names;
pub mod number;

pub use locale::{Locale, Separators};

/// A value to render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    Number(f64),
    Text(&'a str),
    Boolean(bool),
    /// An error cell, by the text Excel shows for it.
    Error(&'a str),
}

/// What a render needs beside the code and the value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Whether serials count from 1904-01-01 and not from 1900.
    pub date1904: bool,
    /// The separators and names a section with no locale prefix uses.
    pub locale: Locale,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            date1904: false,
            locale: Locale::EN,
        }
    }
}

/// The code of the built-in format `id`, as Excel shows it in English, or `None` for an id
/// `styles.xml` must define. An id kept for a locale's own format gives the English format of its
/// kind.
pub fn builtin(id: u32) -> Option<&'static str> {
    Some(match id {
        0 | 23..=26 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        5 => "\"$\"#,##0_);\\(\"$\"#,##0\\)",
        6 => "\"$\"#,##0_);[Red]\\(\"$\"#,##0\\)",
        7 => "\"$\"#,##0.00_);\\(\"$\"#,##0.00\\)",
        8 => "\"$\"#,##0.00_);[Red]\\(\"$\"#,##0.00\\)",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 | 27..=31 | 34..=36 | 50..=58 => "m/d/yyyy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 | 32 | 33 => "h:mm:ss",
        22 => "m/d/yyyy h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        41 => "_(* #,##0_);_(* \\(#,##0\\);_(* \"-\"_);_(@_)",
        42 => "_(\"$\"* #,##0_);_(\"$\"* \\(#,##0\\);_(\"$\"* \"-\"_);_(@_)",
        43 => "_(* #,##0.00_);_(* \\(#,##0.00\\);_(* \"-\"??_);_(@_)",
        44 => "_(\"$\"* #,##0.00_);_(\"$\"* \\(#,##0.00\\);_(\"$\"* \"-\"??_);_(@_)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

/// One lexed part of a section.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Text shown as it is: quoted, escaped, or a character with no other meaning.
    Literal(String),
    /// A digit placeholder: `0`, `#` or `?`.
    Digit(char),
    /// `.`
    Point,
    /// `,`
    Comma,
    /// `%`
    Percent,
    /// `E+` or `E-`; `true` for `E+`, which shows the sign of a positive exponent too.
    Exponent(bool),
    /// `/`
    Slash,
    /// `@`, the text itself.
    At,
    /// `General`.
    General,
    /// A run of one date or time letter, lowercased, and its length: `y`, `m`, `d`, `h`, `s`,
    /// `e`.
    Part(char, usize),
    /// `AM/PM` (`None`) or `A/P` with the two letters as written.
    Meridiem(Option<(char, char)>),
    /// `[h]`, `[mm]` or `[ss]`: the letter and its length.
    Elapsed(char, usize),
}

/// A condition such as `[>=100]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Condition {
    pub op: Op,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Less,
    LessEqual,
    Equal,
    NotEqual,
    GreaterEqual,
    Greater,
}

impl Condition {
    pub fn holds(self, v: f64) -> bool {
        match self.op {
            Op::Less => v < self.value,
            Op::LessEqual => v <= self.value,
            Op::Equal => v == self.value,
            Op::NotEqual => v != self.value,
            Op::GreaterEqual => v >= self.value,
            Op::Greater => v > self.value,
        }
    }

    fn parse(text: &str) -> Option<Self> {
        let (op, rest) = [
            ("<=", Op::LessEqual),
            (">=", Op::GreaterEqual),
            ("<>", Op::NotEqual),
            ("<", Op::Less),
            (">", Op::Greater),
            ("=", Op::Equal),
        ]
        .into_iter()
        .find_map(|(sign, op)| text.strip_prefix(sign).map(|rest| (op, rest)))?;
        Some(Self {
            op,
            value: rest.trim().parse().ok()?,
        })
    }
}

/// One lexed section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Section {
    pub tokens: Vec<Token>,
    pub condition: Option<Condition>,
    /// The locale id of a `[$-407]` prefix.
    pub locale: Option<u32>,
}

impl Section {
    /// Whether the section shows a date or a time.
    pub fn is_date(&self) -> bool {
        self.tokens
            .iter()
            .any(|t| matches!(t, Token::Part(..) | Token::Meridiem(_) | Token::Elapsed(..)))
    }

    fn has_at(&self) -> bool {
        self.tokens.contains(&Token::At)
    }
}

/// The raw text of each section of `code`.
pub fn split(code: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, mut quoted, mut bracket, mut escaped) = (0, false, false, false);
    for (at, c) in code.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '"' if !bracket => quoted = !quoted,
            '\\' | '_' | '*' if !quoted && !bracket => escaped = true,
            '[' if !quoted => bracket = true,
            ']' if !quoted => bracket = false,
            ';' if !quoted && !bracket => {
                out.push(&code[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(&code[start..]);
    out
}

/// The tokens of one section's text.
pub fn lex(text: &str) -> Section {
    let chars: Vec<char> = text.chars().collect();
    let mut section = Section::default();
    let tokens = &mut section.tokens;
    let literal = |tokens: &mut Vec<Token>, text: &str| {
        if let Some(Token::Literal(last)) = tokens.last_mut() {
            last.push_str(text);
        } else {
            tokens.push(Token::Literal(text.to_owned()));
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let rest = || chars[i..].iter().collect::<String>();
        match c {
            '"' => {
                let end = chars[i + 1..]
                    .iter()
                    .position(|&c| c == '"')
                    .map_or(chars.len(), |at| i + 1 + at);
                let quoted: String = chars[i + 1..end].iter().collect();
                literal(tokens, &quoted);
                i = end + 1;
                continue;
            }
            '\\' => {
                if let Some(&next) = chars.get(i + 1) {
                    literal(tokens, &next.to_string());
                }
                i += 2;
                continue;
            }
            '_' => {
                literal(tokens, " ");
                i += 2;
                continue;
            }
            '*' => {
                i += 2;
                continue;
            }
            '[' => {
                let end = chars[i + 1..]
                    .iter()
                    .position(|&c| c == ']')
                    .map_or(chars.len(), |at| i + 1 + at);
                let inner: String = chars[i + 1..end].iter().collect();
                bracket(&inner, &mut section.condition, &mut section.locale, tokens);
                i = end + 1;
                continue;
            }
            '0' | '#' | '?' => tokens.push(Token::Digit(c)),
            '.' => tokens.push(Token::Point),
            ',' => tokens.push(Token::Comma),
            '%' => tokens.push(Token::Percent),
            '/' => tokens.push(Token::Slash),
            '@' => tokens.push(Token::At),
            'E' | 'e' if matches!(chars.get(i + 1), Some('+' | '-')) => {
                tokens.push(Token::Exponent(chars[i + 1] == '+'));
                i += 2;
                continue;
            }
            'G' | 'g' if rest().to_ascii_lowercase().starts_with("general") => {
                tokens.push(Token::General);
                i += 7;
                continue;
            }
            'A' | 'a' if rest().to_ascii_uppercase().starts_with("AM/PM") => {
                tokens.push(Token::Meridiem(None));
                i += 5;
                continue;
            }
            'A' | 'a' if rest().to_ascii_uppercase().starts_with("A/P") => {
                tokens.push(Token::Meridiem(Some((c, chars[i + 2]))));
                i += 3;
                continue;
            }
            'y' | 'Y' | 'm' | 'M' | 'd' | 'D' | 'h' | 'H' | 's' | 'S' | 'e' | 'E' => {
                let lower = c.to_ascii_lowercase();
                let run = chars[i..]
                    .iter()
                    .take_while(|c| c.to_ascii_lowercase() == lower)
                    .count();
                tokens.push(Token::Part(lower, run));
                i += run;
                continue;
            }
            _ => literal(tokens, &c.to_string()),
        }
        i += 1;
    }
    section
}

/// One bracket's meaning: a condition, a locale and currency, elapsed time, or a colour, which
/// is dropped.
fn bracket(
    inner: &str,
    condition: &mut Option<Condition>,
    locale: &mut Option<u32>,
    tokens: &mut Vec<Token>,
) {
    if let Some(found) = Condition::parse(inner) {
        *condition = Some(found);
    } else if let Some(currency) = inner.strip_prefix('$') {
        let (symbol, id) = currency.split_once('-').unwrap_or((currency, ""));
        if !symbol.is_empty() {
            match tokens.last_mut() {
                Some(Token::Literal(last)) => last.push_str(symbol),
                _ => tokens.push(Token::Literal(symbol.to_owned())),
            }
        }
        if let Ok(id) = u32::from_str_radix(id, 16) {
            *locale = Some(id);
        }
    } else {
        let lower = inner.to_ascii_lowercase();
        if let Some(first) = lower.chars().next()
            && matches!(first, 'h' | 'm' | 's')
            && lower.chars().all(|c| c == first)
        {
            tokens.push(Token::Elapsed(first, lower.len()));
        }
    }
}

/// The section of `sections` a number `v` takes, and whether its minus sign shows. Only a code
/// with one section for numbers shows the sign, because a code with more writes its own.
///
/// With no condition, a positive number takes the first section, a negative one the second and
/// zero the third, and a section a code does not have is the first. With a condition in the
/// first or the second section, `v` takes the first of them whose condition holds, and otherwise
/// the third when both have one, or the second.
pub fn choose(sections: &[Section], v: f64) -> (usize, bool) {
    let numeric = match sections.len() {
        n if n > 1 && n < 4 && sections[n - 1].has_at() => n - 1,
        n => n.min(3),
    };
    let places: [usize; 3] = match numeric {
        0 | 1 => [0, 0, 0],
        2 => [0, 1, 0],
        _ => [0, 1, 2],
    };
    let signed = numeric <= 1;
    let first = sections.first().and_then(|s| s.condition);
    let second = sections.get(places[1]).and_then(|s| s.condition);
    if first.is_none() && second.is_none() {
        let at = if v > 0.0 {
            places[0]
        } else if v < 0.0 {
            places[1]
        } else {
            places[2]
        };
        return (at, signed);
    }
    if first.is_some_and(|c| c.holds(v)) {
        return (places[0], signed);
    }
    if second.is_some_and(|c| c.holds(v)) {
        return (places[1], signed);
    }
    let at = if first.is_some() && second.is_some() {
        places[2]
    } else {
        places[1]
    };
    (at, signed)
}

/// The text `value` shows under the format `code`.
pub fn render(code: &str, value: Value<'_>, options: &Options) -> String {
    match value {
        Value::Boolean(true) => "TRUE".to_owned(),
        Value::Boolean(false) => "FALSE".to_owned(),
        Value::Error(text) => text.to_owned(),
        Value::Text(text) => {
            let sections: Vec<Section> = split(code).into_iter().map(lex).collect();
            match sections.last() {
                Some(last) if sections.len() == 4 || last.has_at() => text_section(last, text),
                _ => text.to_owned(),
            }
        }
        Value::Number(v) => {
            if !v.is_finite() {
                return "#NUM!".to_owned();
            }
            let sections: Vec<Section> = split(code).into_iter().map(lex).collect();
            let (at, signed) = choose(&sections, v);
            let section = &sections[at];
            if section.tokens.is_empty() && sections.len() > 1 {
                return String::new();
            }
            let local = section
                .locale
                .and_then(Locale::of_id)
                .unwrap_or(options.locale);
            let separators = local.separators;
            if section.has_at() || section.tokens.is_empty() {
                return number::general(v, separators);
            }
            if section.is_date() {
                if let Some(text) = date::render(section, v, options.date1904, local) {
                    return text;
                }
                return number::general(v, separators);
            }
            number::render(&section.tokens, v, signed, separators)
        }
    }
}

/// `text` under a text section: each `@` replaced by the text, and the literals as they are.
fn text_section(section: &Section, text: &str) -> String {
    let mut out = String::new();
    for token in &section.tokens {
        match token {
            Token::At => out.push_str(text),
            Token::Literal(literal) => out.push_str(literal),
            _ => {}
        }
    }
    out
}
