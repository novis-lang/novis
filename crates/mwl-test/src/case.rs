//! Parsing one `.mwlt` file into a [`Case`].
//!
//! The format is line-oriented: a section header is a line that is exactly
//! `--NAME--`, and a section's body is every line after it up to the next
//! header or end of file. Anything before the first header is an error, so a
//! file that is not a case cannot be silently read as an empty one.
//!
//! The recognised names, and what each is for, are in this crate's own module
//! documentation — that is the one home for the format.

use std::fmt;
use std::path::{Path, PathBuf};

/// What a case's output is compared against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
    /// `--EXPECT--` / `--EXPECT-ERROR--`: compared literally.
    Exact(String),
    /// `--EXPECTF--` / `--EXPECTF-ERROR--`: compared through the `%`-escape
    /// language in [`crate::expect`].
    Format(String),
}

/// The PHP side of a differential case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Oracle {
    /// `--ORACLE--`: a PHP twin whose standard output the case's own standard
    /// output must equal.
    Php(String),
    /// `--ORACLE-DIVERGES--`: the one-line reason MWL deliberately differs
    /// from PHP here, which is why this case has no twin to compare against.
    Diverges(String),
}

/// One parsed `.mwlt` file.
#[derive(Debug, Clone)]
pub struct Case {
    /// Where it was read from, for reporting.
    pub path: PathBuf,
    /// `--TEST--`, the one-line title.
    pub title: String,
    /// `--SKIPIF--`, an MWL program whose output decides whether to run.
    pub skipif: Option<String>,
    /// `--FILE--`, the MWL program under test.
    pub file: String,
    /// `--EXPECT--` or `--EXPECTF--`, matched against standard output.
    pub expect: Option<Expectation>,
    /// `--EXPECT-ERROR--` or `--EXPECTF-ERROR--`, matched against standard
    /// error; its presence is also what says the run is expected to fail.
    pub expect_error: Option<Expectation>,
    /// `--CLEAN--`, an MWL program run afterwards whose output is ignored.
    pub clean: Option<String>,
    /// `--ORACLE--` or `--ORACLE-DIVERGES--`.
    pub oracle: Option<Oracle>,
    /// A section that parsed but cannot be honoured yet, and the milestone
    /// that makes it reachable. A case carrying one is reported as a failure
    /// rather than run, so nothing passes by ignoring half of what it asked
    /// for.
    pub unsupported: Option<String>,
}

impl Case {
    /// True when this case expects the run to fail.
    #[must_use]
    pub fn expects_failure(&self) -> bool {
        self.expect_error.is_some()
    }
}

/// Why a `.mwlt` file could not be read as a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The one-line explanation, already phrased for a terminal.
    pub message: String,
    /// The 1-based line the problem is on, when there is one.
    pub line: Option<usize>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

fn err(message: impl Into<String>, line: Option<usize>) -> ParseError {
    ParseError {
        message: message.into(),
        line,
    }
}

/// The section names this format knows, in the order the module doc lists
/// them. Anything else is a parse error naming the offender.
const KNOWN: &[&str] = &[
    "TEST",
    "SKIPIF",
    "INI",
    "ARGS",
    "ENV",
    "FILE",
    "EXPECT",
    "EXPECTF",
    "EXPECT-ERROR",
    "EXPECTF-ERROR",
    "CLEAN",
    "ORACLE",
    "ORACLE-DIVERGES",
];

/// The three `.phpt` sections that parse for the M11 importer's sake but have
/// nothing to act on yet, each with the milestone that changes that.
const NOT_YET: &[(&str, &str)] = &[
    (
        "INI",
        "`mwl.toml` is not read until M6 (ADR 0064), so an --INI-- section cannot be honoured",
    ),
    (
        "ARGS",
        "argv is unreachable until `Core\\Cli` lands at M8, so an --ARGS-- section cannot be honoured",
    ),
    (
        "ENV",
        "the environment is unreachable until `Core\\Env` lands at M8, so an --ENV-- section cannot be honoured",
    ),
];

/// Returns the section name if `line` is a header, else `None`.
fn header(line: &str) -> Option<&str> {
    let line = line.trim_end();
    let name = line.strip_prefix("--")?.strip_suffix("--")?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    Some(name)
}

/// Reads `text` as a `.mwlt` case named by `path`.
///
/// # Errors
///
/// Returns [`ParseError`] for an unknown section, a repeated section, a
/// missing required section, or two sections that contradict each other.
pub fn parse(path: &Path, text: &str) -> Result<Case, ParseError> {
    let mut sections: Vec<(String, usize, String)> = Vec::new();
    let mut current: Option<(String, usize, String)> = None;

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some(name) = header(line) {
            if !KNOWN.contains(&name) {
                return Err(err(format!("unknown section `--{name}--`"), Some(number)));
            }
            if let Some(previous) = current.take() {
                sections.push(previous);
            }
            if sections.iter().any(|(seen, _, _)| seen == name) {
                return Err(err(format!("`--{name}--` appears twice"), Some(number)));
            }
            current = Some((name.to_owned(), number, String::new()));
        } else if let Some((_, _, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        } else if !line.trim().is_empty() {
            return Err(err(
                "text before the first section; a case starts with `--TEST--`",
                Some(number),
            ));
        }
    }
    if let Some(previous) = current.take() {
        sections.push(previous);
    }
    if sections.is_empty() {
        return Err(err("no sections; a case starts with `--TEST--`", None));
    }

    let take = |name: &str| -> Option<(usize, String)> {
        sections
            .iter()
            .find(|(seen, _, _)| seen == name)
            .map(|(_, line, body)| (*line, body.clone()))
    };

    let Some((title_line, title)) = take("TEST") else {
        return Err(err("no `--TEST--` section", None));
    };
    let title = title.trim().to_owned();
    if title.is_empty() {
        return Err(err("`--TEST--` is empty", Some(title_line)));
    }
    if title.lines().count() > 1 {
        return Err(err("`--TEST--` is one line", Some(title_line)));
    }

    let Some((file_line, file)) = take("FILE") else {
        return Err(err("no `--FILE--` section", None));
    };
    if file.trim().is_empty() {
        return Err(err("`--FILE--` is empty", Some(file_line)));
    }

    let expect = pick(
        &take("EXPECT"),
        &take("EXPECTF"),
        "`--EXPECT--` and `--EXPECTF--` are two answers to one question",
    )?;
    let expect_error = pick(
        &take("EXPECT-ERROR"),
        &take("EXPECTF-ERROR"),
        "`--EXPECT-ERROR--` and `--EXPECTF-ERROR--` are two answers to one question",
    )?;

    let oracle = match (take("ORACLE"), take("ORACLE-DIVERGES")) {
        (Some(_), Some((line, _))) => {
            return Err(err(
                "`--ORACLE--` and `--ORACLE-DIVERGES--` are two answers to one question",
                Some(line),
            ));
        }
        (Some((line, php)), None) => {
            if php.trim().is_empty() {
                return Err(err("`--ORACLE--` is empty", Some(line)));
            }
            Some(Oracle::Php(php))
        }
        (None, Some((line, reason))) => {
            let reason = reason.trim().to_owned();
            if reason.is_empty() {
                return Err(err(
                    "`--ORACLE-DIVERGES--` holds the one-line reason MWL differs from PHP here",
                    Some(line),
                ));
            }
            if reason.lines().count() > 1 {
                return Err(err("`--ORACLE-DIVERGES--` is one line", Some(line)));
            }
            if expect.is_none() && expect_error.is_none() {
                return Err(err(
                    "a diverging case states its own `--EXPECT--`, since there is no twin to compare with",
                    Some(line),
                ));
            }
            Some(Oracle::Diverges(reason))
        }
        (None, None) => None,
    };

    if expect.is_none() && expect_error.is_none() && !matches!(oracle, Some(Oracle::Php(_))) {
        return Err(err(
            "no expectation: a case needs `--EXPECT--`, `--EXPECTF--`, `--EXPECT-ERROR--`, `--EXPECTF-ERROR--` or `--ORACLE--`",
            None,
        ));
    }

    let unsupported = NOT_YET.iter().find_map(|(name, why)| {
        take(name).and_then(|(_, body)| (!body.trim().is_empty()).then(|| (*why).to_owned()))
    });

    Ok(Case {
        path: path.to_path_buf(),
        title,
        skipif: take("SKIPIF")
            .map(|(_, body)| body)
            .filter(|body| !body.trim().is_empty()),
        file,
        expect,
        expect_error,
        clean: take("CLEAN")
            .map(|(_, body)| body)
            .filter(|body| !body.trim().is_empty()),
        oracle,
        unsupported,
    })
}

/// Turns a mutually exclusive exact/format pair into one [`Expectation`].
fn pick(
    exact: &Option<(usize, String)>,
    format: &Option<(usize, String)>,
    conflict: &str,
) -> Result<Option<Expectation>, ParseError> {
    match (exact, format) {
        (Some(_), Some((line, _))) => Err(err(conflict, Some(*line))),
        (Some((_, body)), None) => Ok(Some(Expectation::Exact(body.clone()))),
        (None, Some((_, body))) => Ok(Some(Expectation::Format(body.clone()))),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(text: &str) -> Result<Case, ParseError> {
        parse(Path::new("t.mwlt"), text)
    }

    const MINIMAL: &str = "--TEST--\nthe title\n--FILE--\n<?mwl\necho 1;\n--EXPECT--\n1\n";

    #[test]
    fn a_minimal_case_parses_into_its_three_sections() {
        let parsed = case(MINIMAL).expect("minimal case parses");
        assert_eq!(parsed.title, "the title");
        assert_eq!(parsed.file, "<?mwl\necho 1;\n");
        assert_eq!(parsed.expect, Some(Expectation::Exact("1\n".to_owned())));
        assert!(parsed.oracle.is_none());
        assert!(parsed.unsupported.is_none());
    }

    #[test]
    fn a_body_line_that_looks_like_a_header_but_is_not_uppercase_stays_body() {
        let parsed = case("--TEST--\nt\n--FILE--\n<?mwl\necho \"--a--\";\n--EXPECT--\n--a--\n")
            .expect("mixed-case dashes are body text");
        assert!(parsed.file.contains("--a--"));
    }

    #[test]
    fn an_unknown_section_names_itself() {
        let e = case("--TEST--\nt\n--NOPE--\nx\n").expect_err("unknown section is rejected");
        assert!(e.message.contains("--NOPE--"), "{e}");
        assert_eq!(e.line, Some(3));
    }

    #[test]
    fn a_repeated_section_is_rejected() {
        let e = case("--TEST--\na\n--TEST--\nb\n").expect_err("a repeat is rejected");
        assert!(e.message.contains("twice"), "{e}");
    }

    #[test]
    fn text_before_the_first_section_is_rejected() {
        let e = case("hello\n--TEST--\nt\n").expect_err("a preamble is rejected");
        assert_eq!(e.line, Some(1));
    }

    #[test]
    fn a_case_with_no_expectation_at_all_is_rejected() {
        let e = case("--TEST--\nt\n--FILE--\n<?mwl\n").expect_err("no expectation is rejected");
        assert!(e.message.contains("no expectation"), "{e}");
    }

    #[test]
    fn exact_and_format_expectations_cannot_both_be_given() {
        let e = case("--TEST--\nt\n--FILE--\n<?mwl\n--EXPECT--\n1\n--EXPECTF--\n%d\n")
            .expect_err("two spellings of one answer are rejected");
        assert!(e.message.contains("two answers"), "{e}");
    }

    #[test]
    fn an_error_expectation_is_enough_on_its_own() {
        let parsed = case("--TEST--\nt\n--FILE--\n<?mwl\nbad\n--EXPECTF-ERROR--\n%a\n")
            .expect("an error expectation stands alone");
        assert!(parsed.expects_failure());
        assert!(parsed.expect.is_none());
    }

    #[test]
    fn an_oracle_stands_in_for_an_expectation() {
        let parsed = case("--TEST--\nt\n--FILE--\n<?mwl\necho 1;\n--ORACLE--\n<?php\necho 1;\n")
            .expect("an oracle is an expectation");
        assert_eq!(
            parsed.oracle,
            Some(Oracle::Php("<?php\necho 1;\n".to_owned()))
        );
    }

    #[test]
    fn a_divergence_must_state_its_reason_and_its_own_expectation() {
        let e = case("--TEST--\nt\n--FILE--\n<?mwl\n--ORACLE-DIVERGES--\nADR 0063 R7\n")
            .expect_err("a divergence with no expectation is rejected");
        assert!(e.message.contains("states its own"), "{e}");

        let parsed = case(
            "--TEST--\nt\n--FILE--\n<?mwl\necho 1;\n--EXPECT--\n1\n--ORACLE-DIVERGES--\nADR 0063 R7\n",
        )
        .expect("a divergence with an expectation parses");
        assert_eq!(
            parsed.oracle,
            Some(Oracle::Diverges("ADR 0063 R7".to_owned()))
        );
    }

    #[test]
    fn the_two_oracle_sections_are_mutually_exclusive() {
        let e = case("--TEST--\nt\n--FILE--\n<?mwl\n--EXPECT--\n1\n--ORACLE--\n<?php\n--ORACLE-DIVERGES--\nwhy\n")
            .expect_err("both oracle spellings at once is rejected");
        assert!(e.message.contains("two answers"), "{e}");
    }

    #[test]
    fn the_three_deferred_sections_parse_but_mark_the_case_unsupported() {
        for (name, needle) in [("INI", "M6"), ("ARGS", "M8"), ("ENV", "M8")] {
            let text = format!("--TEST--\nt\n--{name}--\nx=1\n--FILE--\n<?mwl\n--EXPECT--\n\n");
            let parsed = case(&text).expect("a deferred section still parses");
            let why = parsed.unsupported.expect("it marks the case unsupported");
            assert!(why.contains(needle), "{why}");
        }
    }

    #[test]
    fn an_empty_deferred_section_does_not_mark_the_case() {
        let parsed = case("--TEST--\nt\n--ARGS--\n\n--FILE--\n<?mwl\n--EXPECT--\n\n")
            .expect("an empty deferred section parses");
        assert!(parsed.unsupported.is_none());
    }

    #[test]
    fn skipif_and_clean_survive_the_round_trip() {
        let parsed = case(
            "--TEST--\nt\n--SKIPIF--\n<?mwl\necho \"skip why\";\n--FILE--\n<?mwl\n--EXPECT--\n\n--CLEAN--\n<?mwl\n",
        )
        .expect("skipif and clean parse");
        assert!(parsed.skipif.expect("skipif").contains("skip why"));
        assert!(parsed.clean.is_some());
    }
}
