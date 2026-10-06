//! Comparing what a case printed with what it said it would print.
//!
//! Two spellings: `--EXPECT--` is literal and `--EXPECTF--` carries
//! `%`-escapes, the closed set below:
//!
//! | escape | matches |
//! |---|---|
//! | `%e` | one directory separator, `/` or `\` |
//! | `%s` | one or more characters, not a newline |
//! | `%S` | zero or more characters, not a newline |
//! | `%a` | one or more characters, newlines included |
//! | `%A` | zero or more characters, newlines included |
//! | `%w` | zero or more whitespace characters |
//! | `%i` | a signed integer |
//! | `%d` | one or more digits |
//! | `%x` | one or more hexadecimal digits |
//! | `%f` | a floating-point number |
//! | `%c` | exactly one character |
//! | `%%` | a literal `%` |
//!
//! The matcher is hand-written rather than a translation to a regex crate:
//! this crate stays dependency-free (see the manifest), the pattern language
//! is the table above and nothing more, and both sides are a few lines of
//! terminal output, so a backtracking match over `char`s is not worth
//! optimising.
//!
//! ## Normalisation, and why there is any
//!
//! Both sides are normalised before comparison: `\r\n` becomes `\n`, trailing
//! whitespace goes from every line, and trailing blank lines go from the
//! whole. Without it every case would pass on one of Novis's CI legs and fail
//! on another for a reason that has nothing to do with what it tests.
//! An expectation that genuinely cares about a trailing space is spelled with
//! `--EXPECTF--` and a `%c`.

use crate::case::Expectation;

/// Strips the differences between two platforms writing the same output.
#[must_use]
pub fn normalize(text: &str) -> String {
    let unified = text.replace("\r\n", "\n");
    let mut lines: Vec<&str> = unified.lines().map(str::trim_end).collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// True when `actual` satisfies `expectation`.
#[must_use]
pub fn matches(expectation: &Expectation, actual: &str) -> bool {
    let actual = normalize(actual);
    match expectation {
        Expectation::Exact(expected) => normalize(expected) == actual,
        Expectation::Format(pattern) => format_matches(&normalize(pattern), &actual),
    }
}

/// The text a failure report shows as "expected", with the escapes left in.
#[must_use]
pub fn shown(expectation: &Expectation) -> String {
    match expectation {
        Expectation::Exact(text) | Expectation::Format(text) => normalize(text),
    }
}

/// One piece of a compiled `--EXPECTF--` pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// Literal text, matched as itself.
    Lit(Vec<char>),
    /// `min` or more characters of `class`.
    Run { min: usize, class: Class },
    /// Exactly one character of `class`.
    One(Class),
    /// `%i`.
    Int,
    /// `%f`.
    Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Any,
    NotNewline,
    Space,
    Digit,
    Hex,
    Separator,
}

impl Class {
    fn holds(self, c: char) -> bool {
        match self {
            Self::Any => true,
            Self::NotNewline => c != '\n',
            Self::Space => c.is_whitespace(),
            Self::Digit => c.is_ascii_digit(),
            Self::Hex => c.is_ascii_hexdigit(),
            Self::Separator => c == '/' || c == '\\',
        }
    }
}

/// True when `actual` satisfies the `--EXPECTF--` pattern `pattern`.
#[must_use]
pub fn format_matches(pattern: &str, actual: &str) -> bool {
    let toks = compile(pattern);
    let actual: Vec<char> = actual.chars().collect();
    match_from(&toks, 0, &actual, 0)
}

fn compile(pattern: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut lit: Vec<char> = Vec::new();
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            lit.push(c);
            continue;
        }
        let Some(&escape) = chars.peek() else {
            // A trailing `%` is a literal `%`; nothing else it could be.
            lit.push('%');
            continue;
        };
        let tok = match escape {
            '%' => {
                chars.next();
                lit.push('%');
                continue;
            }
            'e' => Tok::One(Class::Separator),
            'c' => Tok::One(Class::Any),
            's' => Tok::Run {
                min: 1,
                class: Class::NotNewline,
            },
            'S' => Tok::Run {
                min: 0,
                class: Class::NotNewline,
            },
            'a' => Tok::Run {
                min: 1,
                class: Class::Any,
            },
            'A' => Tok::Run {
                min: 0,
                class: Class::Any,
            },
            'w' => Tok::Run {
                min: 0,
                class: Class::Space,
            },
            'd' => Tok::Run {
                min: 1,
                class: Class::Digit,
            },
            'x' => Tok::Run {
                min: 1,
                class: Class::Hex,
            },
            'i' => Tok::Int,
            'f' => Tok::Float,
            // An unknown escape is two literal characters, which is what
            // makes `100%` and `50% off` behave in an expectation.
            _ => {
                lit.push('%');
                continue;
            }
        };
        chars.next();
        if !lit.is_empty() {
            toks.push(Tok::Lit(std::mem::take(&mut lit)));
        }
        toks.push(tok);
    }
    if !lit.is_empty() {
        toks.push(Tok::Lit(lit));
    }
    toks
}

/// Every length `tok` could consume at `pos`, longest first.
///
/// Longest first makes the common case — a wildcard followed by literal text
/// that appears once — settle on its first try.
fn candidates(tok: &Tok, text: &[char], pos: usize) -> Vec<usize> {
    let rest = &text[pos..];
    match tok {
        Tok::Lit(lit) => {
            if rest.len() >= lit.len() && rest[..lit.len()] == lit[..] {
                vec![lit.len()]
            } else {
                Vec::new()
            }
        }
        Tok::One(class) => match rest.first() {
            Some(&c) if class.holds(c) => vec![1],
            _ => Vec::new(),
        },
        Tok::Run { min, class } => {
            let max = rest.iter().take_while(|&&c| class.holds(c)).count();
            if max < *min {
                Vec::new()
            } else {
                (*min..=max).rev().collect()
            }
        }
        Tok::Int => numeric_candidates(rest, false),
        Tok::Float => numeric_candidates(rest, true),
    }
}

/// Lengths of every prefix of `rest` that is a whole integer, or float when
/// `float` is set, longest first.
fn numeric_candidates(rest: &[char], float: bool) -> Vec<usize> {
    let mut out = Vec::new();
    for len in (1..=rest.len()).rev() {
        if is_number(&rest[..len], float) {
            out.push(len);
        }
    }
    out
}

fn is_number(text: &[char], float: bool) -> bool {
    let mut i = 0;
    if matches!(text.first(), Some('+' | '-')) {
        i = 1;
    }
    let digits = text[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    i += digits;
    if !float {
        return digits > 0 && i == text.len();
    }
    let mut seen = digits;
    if text.get(i) == Some(&'.') {
        i += 1;
        let after = text[i..].iter().take_while(|c| c.is_ascii_digit()).count();
        seen += after;
        i += after;
    }
    if seen == 0 {
        return false;
    }
    if matches!(text.get(i), Some('e' | 'E')) {
        i += 1;
        if matches!(text.get(i), Some('+' | '-')) {
            i += 1;
        }
        let exponent = text[i..].iter().take_while(|c| c.is_ascii_digit()).count();
        if exponent == 0 {
            return false;
        }
        i += exponent;
    }
    i == text.len()
}

fn match_from(toks: &[Tok], t: usize, text: &[char], pos: usize) -> bool {
    let Some(tok) = toks.get(t) else {
        return pos == text.len();
    };
    candidates(tok, text, pos)
        .into_iter()
        .any(|len| match_from(toks, t + 1, text, pos + len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_erases_line_endings_and_trailing_blank_lines() {
        assert_eq!(normalize("a\r\nb  \n\n\n"), "a\nb");
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("\n\n"), "");
    }

    #[test]
    fn an_exact_expectation_ignores_the_platforms_line_ending() {
        let e = Expectation::Exact("one\ntwo\n".to_owned());
        assert!(matches(&e, "one\r\ntwo\r\n"));
        assert!(!matches(&e, "one\ntwo\nthree"));
    }

    #[test]
    fn each_wildcard_matches_what_the_escape_table_says() {
        assert!(format_matches("a%sb", "axxxb"));
        assert!(!format_matches("a%sb", "ab"), "%s wants at least one");
        assert!(format_matches("a%Sb", "ab"));
        assert!(!format_matches("a%sb", "a\nb"), "%s stops at a newline");
        assert!(format_matches("a%ab", "a\nb"));
        assert!(format_matches("a%Ab", "ab"));
        assert!(format_matches("a%wb", "a \t b"));
        assert!(format_matches("[%d]", "[42]"));
        assert!(!format_matches("[%d]", "[-42]"), "%d is unsigned");
        assert!(format_matches("[%i]", "[-42]"));
        assert!(format_matches("[%x]", "[deadBEEF00]"));
        assert!(format_matches("[%c]", "[q]"));
        assert!(!format_matches("[%c]", "[qq]"));
        assert!(format_matches("a%eb", "a/b"));
        assert!(format_matches("a%eb", "a\\b"));
    }

    #[test]
    fn a_float_escape_takes_the_shapes_a_float_is_printed_in() {
        for text in ["1", "-1.5", "+0.25", "1e10", "1.5E-3", "0.0"] {
            assert!(format_matches("[%f]", &format!("[{text}]")), "{text}");
        }
        for text in ["", ".", "1e", "1.2.3", "e5"] {
            assert!(!format_matches("[%f]", &format!("[{text}]")), "{text}");
        }
    }

    #[test]
    fn a_doubled_percent_is_one_literal_percent() {
        assert!(format_matches("100%% done", "100% done"));
        assert!(!format_matches("100%% done", "100%% done"));
    }

    #[test]
    fn an_unknown_escape_stays_two_literal_characters() {
        assert!(format_matches("50% off", "50% off"));
    }

    #[test]
    fn a_wildcard_backtracks_when_the_greedy_match_overshoots() {
        // `%s` takes the whole line first, then gives characters back until
        // the `-->` after it lines up with the last one in the text.
        assert!(format_matches("%s--> end", "a --> b --> end"));
        assert!(format_matches("%a\nlast", "one\ntwo\nlast"));
    }

    #[test]
    fn a_pattern_with_no_escapes_is_an_exact_comparison() {
        assert!(format_matches("plain", "plain"));
        assert!(!format_matches("plain", "plainer"));
    }
}
