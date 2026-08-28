//! Parsing one `.nvst` file into a [`Case`].
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
    /// `--ORACLE-DIVERGES--`: the one-line reason Novis deliberately differs
    /// from PHP here, which is why this case has no twin to compare against.
    Diverges(String),
}

/// Which `nvs` subcommand a case's own `--FILE--` is run through.
///
/// `--RUN--` names it, and the default is [`Subcommand::Run`] — a case is a
/// program whose output is the expectation. [`Subcommand::Test`] is the other
/// half of [ADR 0079](../../../docs/adr/0079-testing-and-assertions.md) § 23:
/// the program declares `#[Test]` classes and what the case pins is the
/// runner's own report of running them.
///
/// It applies to `--FILE--` alone. `--SKIPIF--` and `--CLEAN--` are the
/// runner's own scaffolding rather than the thing under test, so both are
/// always `nvs run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Subcommand {
    /// `nvs run case.nvs` — the program is the case.
    #[default]
    Run,
    /// `nvs test case.nvs` — the program's `#[Test]` methods are the case.
    Test,
}

impl Subcommand {
    /// The word it is written as, in `--RUN--` and on the command line alike.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Test => "test",
        }
    }
}

/// One file a case puts on disk beside its own `--FILE--`.
///
/// Written by `--FILE <relative/path>--`, which may appear any number of
/// times. This is what lets one case cover something that is only observable
/// across files — a `require` target, an autoload root, a shadowing vendor
/// copy ([ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxFile {
    /// Where to write it, relative to the case's working directory, always
    /// with `/` separators and never escaping that directory.
    pub path: String,
    /// Its contents, verbatim.
    pub body: String,
}

/// One parsed `.nvst` file.
#[derive(Debug, Clone)]
pub struct Case {
    /// Where it was read from, for reporting.
    pub path: PathBuf,
    /// `--TEST--`, the one-line title.
    pub title: String,
    /// `--SKIPIF--`, an Novis program whose output decides whether to run.
    pub skipif: Option<String>,
    /// `--FILE--`, the Novis program under test.
    pub file: String,
    /// `--RUN--`, the subcommand that program is run through.
    pub run: Subcommand,
    /// Every `--FILE <relative/path>--`, in the order they were written.
    pub aux: Vec<AuxFile>,
    /// `--EXPECT--` or `--EXPECTF--`, matched against standard output.
    pub expect: Option<Expectation>,
    /// `--EXPECT-ERROR--` or `--EXPECTF-ERROR--`, matched against standard
    /// error; its presence is also what says the run is expected to fail.
    pub expect_error: Option<Expectation>,
    /// `--CLEAN--`, an Novis program run afterwards whose output is ignored.
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

/// Why a `.nvst` file could not be read as a case.
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

/// One section as the line scanner found it, before any of it is interpreted.
struct Section {
    /// The header's name, e.g. `EXPECT`.
    name: String,
    /// The header's argument, for the one name that takes one.
    arg: Option<String>,
    /// The 1-based line the header is on.
    line: usize,
    /// Every line under the header, each still carrying its newline.
    body: String,
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
    "RUN",
];

/// The three `.phpt` sections that parse for the M11 importer's sake but have
/// nothing to act on yet, each with the milestone that changes that.
const NOT_YET: &[(&str, &str)] = &[
    (
        "INI",
        "`nvs.toml` is not read until M6 (ADR 0064), so an --INI-- section cannot be honoured",
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

/// The one section name that takes an argument, and what the argument is.
const TAKES_A_PATH: &str = "FILE";

/// The names the runner writes into the working directory itself, which an
/// auxiliary file therefore may not claim.
const RESERVED_NAMES: &[&str] = &["case.nvs", "skipif.nvs", "clean.nvs", "oracle.php"];

/// Returns a header's section name and its argument, if `line` is a header.
///
/// A header is `--NAME--` or `--NAME argument--`; the name keeps the narrow
/// character set it always had, so widening this cannot reclassify a body
/// line that merely contains dashes.
fn header(line: &str) -> Option<(&str, Option<&str>)> {
    let line = line.trim_end();
    let inner = line.strip_prefix("--")?.strip_suffix("--")?;
    let (name, arg) = match inner.split_once(' ') {
        Some((name, arg)) => (name, Some(arg.trim())),
        None => (inner, None),
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    Some((name, arg))
}

/// Checks an auxiliary file's path, returning it or the reason it is refused.
///
/// The path is joined onto a temporary directory the runner owns, so this is
/// the whole of the containment argument: `/` separators only, no root, no
/// drive letter, and no `.` or `..` segment means the result cannot name
/// anything outside that directory.
fn aux_path(raw: &str) -> Result<String, String> {
    if raw.is_empty() {
        return Err("`--FILE <path>--` needs a path after the name".to_owned());
    }
    if raw.contains('\\') {
        return Err(format!(
            "`{raw}` separates with `\\`; an auxiliary path is written with `/` on both legs"
        ));
    }
    if raw.starts_with('/') || raw.contains(':') {
        return Err(format!("`{raw}` is not a relative path"));
    }
    if raw
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(format!(
            "`{raw}` has an empty, `.` or `..` segment; an auxiliary file stays under the case's own directory"
        ));
    }
    if RESERVED_NAMES.contains(&raw) {
        return Err(format!("`{raw}` is the name the runner writes itself"));
    }
    Ok(raw.to_owned())
}

/// Reads `text` as a `.nvst` case named by `path`.
///
/// # Errors
///
/// Returns [`ParseError`] for an unknown section, a repeated section, a
/// missing required section, or two sections that contradict each other.
pub fn parse(path: &Path, text: &str) -> Result<Case, ParseError> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some((name, arg)) = header(line) {
            if !KNOWN.contains(&name) {
                return Err(err(format!("unknown section `--{name}--`"), Some(number)));
            }
            let arg = match (arg, name) {
                (None, _) => None,
                (Some(raw), TAKES_A_PATH) => {
                    Some(aux_path(raw).map_err(|why| err(why, Some(number)))?)
                }
                (Some(_), _) => {
                    return Err(err(
                        format!("`--{name}--` does not take an argument"),
                        Some(number),
                    ));
                }
            };
            if let Some(previous) = current.take() {
                sections.push(previous);
            }
            if sections
                .iter()
                .any(|seen| seen.name == name && seen.arg == arg)
            {
                let shown = match &arg {
                    Some(arg) => format!("--{name} {arg}--"),
                    None => format!("--{name}--"),
                };
                return Err(err(format!("`{shown}` appears twice"), Some(number)));
            }
            current = Some(Section {
                name: name.to_owned(),
                arg,
                line: number,
                body: String::new(),
            });
        } else if let Some(section) = current.as_mut() {
            section.body.push_str(line);
            section.body.push('\n');
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
            .find(|seen| seen.name == name && seen.arg.is_none())
            .map(|seen| (seen.line, seen.body.clone()))
    };

    let mut aux = Vec::new();
    for section in sections
        .iter()
        .filter(|seen| seen.name == TAKES_A_PATH && seen.arg.is_some())
    {
        let path = section.arg.clone().expect("filtered to the argument form");
        if section.body.trim().is_empty() {
            return Err(err(
                format!("`--FILE {path}--` is empty"),
                Some(section.line),
            ));
        }
        aux.push(AuxFile {
            path,
            body: section.body.clone(),
        });
    }

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

    // The roster is closed, so a misspelling is refused where it is written
    // rather than silently running the case the other way.
    let run = match take("RUN") {
        None => Subcommand::Run,
        Some((line, body)) => match body.trim() {
            "run" => Subcommand::Run,
            "test" => Subcommand::Test,
            other => {
                return Err(err(
                    format!("`--RUN--` is `run` or `test`, not `{other}`"),
                    Some(line),
                ));
            }
        },
    };

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
                    "`--ORACLE-DIVERGES--` holds the one-line reason Novis differs from PHP here",
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
        run,
        aux,
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
        parse(Path::new("t.nvst"), text)
    }

    const MINIMAL: &str = "--TEST--\nthe title\n--FILE--\n<?nvs\necho 1;\n--EXPECT--\n1\n";

    #[test]
    fn a_case_that_says_nothing_is_run_through_nvs_run() {
        assert_eq!(case(MINIMAL).expect("it parses").run, Subcommand::Run);
    }

    #[test]
    fn a_run_section_names_the_subcommand_the_program_goes_through() {
        let parsed = case(&format!("--RUN--\ntest\n{MINIMAL}")).expect("it parses");
        assert_eq!(parsed.run, Subcommand::Test);
        assert_eq!(parsed.run.as_str(), "test");
    }

    #[test]
    fn a_run_section_naming_no_subcommand_is_refused_where_it_is_written() {
        let error = case(&format!("--RUN--\ncheck\n{MINIMAL}")).expect_err("the roster is closed");
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("`run` or `test`"), "{error}");
    }

    #[test]
    fn a_minimal_case_parses_into_its_three_sections() {
        let parsed = case(MINIMAL).expect("minimal case parses");
        assert_eq!(parsed.title, "the title");
        assert_eq!(parsed.file, "<?nvs\necho 1;\n");
        assert_eq!(parsed.expect, Some(Expectation::Exact("1\n".to_owned())));
        assert!(parsed.oracle.is_none());
        assert!(parsed.unsupported.is_none());
    }

    #[test]
    fn a_body_line_that_looks_like_a_header_but_is_not_uppercase_stays_body() {
        let parsed = case("--TEST--\nt\n--FILE--\n<?nvs\necho \"--a--\";\n--EXPECT--\n--a--\n")
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
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n").expect_err("no expectation is rejected");
        assert!(e.message.contains("no expectation"), "{e}");
    }

    #[test]
    fn exact_and_format_expectations_cannot_both_be_given() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--EXPECT--\n1\n--EXPECTF--\n%d\n")
            .expect_err("two spellings of one answer are rejected");
        assert!(e.message.contains("two answers"), "{e}");
    }

    #[test]
    fn an_error_expectation_is_enough_on_its_own() {
        let parsed = case("--TEST--\nt\n--FILE--\n<?nvs\nbad\n--EXPECTF-ERROR--\n%a\n")
            .expect("an error expectation stands alone");
        assert!(parsed.expects_failure());
        assert!(parsed.expect.is_none());
    }

    #[test]
    fn an_oracle_stands_in_for_an_expectation() {
        let parsed = case("--TEST--\nt\n--FILE--\n<?nvs\necho 1;\n--ORACLE--\n<?php\necho 1;\n")
            .expect("an oracle is an expectation");
        assert_eq!(
            parsed.oracle,
            Some(Oracle::Php("<?php\necho 1;\n".to_owned()))
        );
    }

    #[test]
    fn a_divergence_must_state_its_reason_and_its_own_expectation() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--ORACLE-DIVERGES--\nADR 0063 R7\n")
            .expect_err("a divergence with no expectation is rejected");
        assert!(e.message.contains("states its own"), "{e}");

        let parsed = case(
            "--TEST--\nt\n--FILE--\n<?nvs\necho 1;\n--EXPECT--\n1\n--ORACLE-DIVERGES--\nADR 0063 R7\n",
        )
        .expect("a divergence with an expectation parses");
        assert_eq!(
            parsed.oracle,
            Some(Oracle::Diverges("ADR 0063 R7".to_owned()))
        );
    }

    #[test]
    fn the_two_oracle_sections_are_mutually_exclusive() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--EXPECT--\n1\n--ORACLE--\n<?php\n--ORACLE-DIVERGES--\nwhy\n")
            .expect_err("both oracle spellings at once is rejected");
        assert!(e.message.contains("two answers"), "{e}");
    }

    #[test]
    fn the_three_deferred_sections_parse_but_mark_the_case_unsupported() {
        for (name, needle) in [("INI", "M6"), ("ARGS", "M8"), ("ENV", "M8")] {
            let text = format!("--TEST--\nt\n--{name}--\nx=1\n--FILE--\n<?nvs\n--EXPECT--\n\n");
            let parsed = case(&text).expect("a deferred section still parses");
            let why = parsed.unsupported.expect("it marks the case unsupported");
            assert!(why.contains(needle), "{why}");
        }
    }

    #[test]
    fn an_empty_deferred_section_does_not_mark_the_case() {
        let parsed = case("--TEST--\nt\n--ARGS--\n\n--FILE--\n<?nvs\n--EXPECT--\n\n")
            .expect("an empty deferred section parses");
        assert!(parsed.unsupported.is_none());
    }

    #[test]
    fn auxiliary_files_keep_their_paths_and_their_order() {
        let parsed = case(
            "--TEST--\nt\n--FILE--\n<?nvs\nrequire './src/B.nvs';\n--FILE src/B.nvs--\n<?nvs\nautoload 'App' from './';\n--FILE src/App/Greeter.nvs--\n<?nvs\nclass Greeter {}\n--EXPECT--\n\n",
        )
        .expect("auxiliary files parse");
        assert_eq!(parsed.file, "<?nvs\nrequire './src/B.nvs';\n");
        assert_eq!(
            parsed
                .aux
                .iter()
                .map(|a| a.path.as_str())
                .collect::<Vec<_>>(),
            ["src/B.nvs", "src/App/Greeter.nvs"]
        );
        assert!(parsed.aux[0].body.contains("autoload"));
    }

    #[test]
    fn an_auxiliary_path_cannot_climb_out_of_the_case_directory() {
        for path in [
            "../x.nvs",
            "a/../x.nvs",
            "/etc/x.nvs",
            "C:/x.nvs",
            "./x.nvs",
        ] {
            let text =
                format!("--TEST--\nt\n--FILE--\n<?nvs\n--FILE {path}--\n<?nvs\n--EXPECT--\n\n");
            let e = case(&text).expect_err("an escaping path is rejected");
            assert!(e.message.contains(path), "{e}");
        }
    }

    #[test]
    fn an_auxiliary_path_cannot_claim_a_name_the_runner_writes() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--FILE case.nvs--\n<?nvs\n--EXPECT--\n\n")
            .expect_err("the runner's own name is rejected");
        assert!(e.message.contains("the runner writes itself"), "{e}");
    }

    #[test]
    fn an_auxiliary_path_is_written_with_forward_slashes() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--FILE src\\B.nvs--\n<?nvs\n--EXPECT--\n\n")
            .expect_err("a backslash separator is rejected");
        assert!(e.message.contains("both legs"), "{e}");
    }

    #[test]
    fn the_same_auxiliary_path_cannot_be_written_twice() {
        let e = case(
            "--TEST--\nt\n--FILE--\n<?nvs\n--FILE a.nvs--\n<?nvs\n--FILE a.nvs--\n<?nvs\n--EXPECT--\n\n",
        )
        .expect_err("a repeated auxiliary path is rejected");
        assert!(e.message.contains("`--FILE a.nvs--` appears twice"), "{e}");
    }

    #[test]
    fn an_empty_auxiliary_file_is_rejected() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--FILE a.nvs--\n\n--EXPECT--\n\n")
            .expect_err("an empty auxiliary file is rejected");
        assert!(e.message.contains("is empty"), "{e}");
    }

    #[test]
    fn only_the_file_section_takes_an_argument() {
        let e = case("--TEST--\nt\n--FILE--\n<?nvs\n--EXPECT a.nvs--\n\n")
            .expect_err("an argument on another section is rejected");
        assert!(e.message.contains("does not take an argument"), "{e}");
    }

    #[test]
    fn skipif_and_clean_survive_the_round_trip() {
        let parsed = case(
            "--TEST--\nt\n--SKIPIF--\n<?nvs\necho \"skip why\";\n--FILE--\n<?nvs\n--EXPECT--\n\n--CLEAN--\n<?nvs\n",
        )
        .expect("skipif and clean parse");
        assert!(parsed.skipif.expect("skipif").contains("skip why"));
        assert!(parsed.clean.is_some());
    }
}
