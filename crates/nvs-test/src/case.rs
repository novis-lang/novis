//! Parsing one `.nvst` file into a [`Case`].
//!
//! The line-oriented shape — what a header looks like, and where a section's
//! body ends — is [`crate::section`]'s, shared with the other case format.
//! What this module owns is the roster: which names `.nvst` knows, which of
//! them takes an argument, and what each one means once it has been read.
//! Anything before the first header is an error, so a file that is not a case
//! cannot be silently read as an empty one.
//!
//! The recognised names, and what each is for, are in this crate's own module
//! documentation — that is the one home for the format.

use std::fmt;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use crate::section;

/// What a case's output is compared against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
    /// `--EXPECT--` / `--EXPECT-ERROR--`: compared literally.
    Exact(String),
    /// `--EXPECTF--` / `--EXPECTF-ERROR--`: compared through the `%`-escape
    /// language in [`crate::expect`].
    Format(String),
}

/// The body a case's request carries, and which section spelled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// `--POST--`: urlencoded pairs exactly as they go on the wire, so the
    /// content type is `application/x-www-form-urlencoded` and a case does not
    /// write one for itself.
    Form(String),
    /// `--POST_RAW--`: the body verbatim, whatever it holds. Nothing is
    /// assumed about it, so a case that means it to be read as something in
    /// particular says so with a `Content-Type` line in `--HEADERS--` — a
    /// content type written twice is a content type that can disagree.
    Raw(String),
}

/// The scheme a request is to have effectively arrived over, which `--SCHEME--`
/// names and `Core\Request::scheme` answers.
///
/// `nvs_runtime::Scheme` is these same two cases, and this crate cannot name it
/// — it has no dependencies, for the reason its manifest gives — so the two
/// meet in one `match` at `nvs run --request`'s door. Nothing here decides
/// which scheme a request had: `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s
/// walk does that for a served request, and a case states its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// A plaintext connection, and what a case naming no scheme describes.
    Http,
    /// TLS, as a trusted proxy asserted it.
    Https,
}

impl Scheme {
    /// The one spelling a case writes this scheme as and a request file carries
    /// it in.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

/// The request a case is answering, assembled from the seven sections that
/// describe one.
///
/// Every part is optional and a case that wrote none of the seven carries no
/// `Request` at all rather than an empty one: answering a request and running
/// as a program are different things, and what a case wrote is how it says
/// which of them it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// `--GET--`, the query string as it is written past the `?`, on one line
    /// and without the `?` itself.
    pub query: Option<String>,
    /// `--POST--` or `--POST_RAW--`, which are two spellings of one body and
    /// so may not both be written.
    pub body: Option<Body>,
    /// `--COOKIE--`, one `NAME=value` per line.
    ///
    /// `.phpt` writes these `;`-joined on a single line and this format takes
    /// one per line instead, on [`Case::env`]'s rule: splitting a joined line
    /// means deciding what a `;` inside a value meant, which is a guess. The
    /// single pair a corpus case almost always writes reads the same either
    /// way, so the import stays mechanical.
    pub cookies: Vec<(String, String)>,
    /// `--HEADERS--`, one `Name: value` per line.
    ///
    /// A field's value starts past the `:` and whatever spaces follow it,
    /// which is how an HTTP field is written. A repeated name is kept rather
    /// than merged or refused, because a message may legitimately carry one
    /// twice and what that means belongs to whatever reads the field.
    pub headers: Vec<(String, String)>,
    /// `--CLIENT_IP--`, the address the request's peer is to have had, and
    /// `None` where the case named none.
    ///
    /// That `None` is an answer rather than a hole: a peer can genuinely have
    /// no address to report, which is the `null` `Core\Request::clientIp`
    /// answers (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`).
    /// It is the address the walk *settled on*, not a hop's claim — a case
    /// pinning what a forwarded header did to that claim writes the header in
    /// `--HEADERS--` and the settled address here.
    pub client_ip: Option<IpAddr>,
    /// `--SCHEME--`, and `None` where the case named none, which the request
    /// file and the carrier both read as [`Scheme::Http`].
    ///
    /// Fail-closed for `nvs_runtime::Inbound::new`'s reason: `https` is a claim
    /// only a section can make, so a case that says nothing describes a request
    /// that arrived over plaintext.
    pub scheme: Option<Scheme>,
}

/// Which `nvs` subcommand a case's own `--FILE--` is run through.
///
/// `--RUN--` names it, and the default is [`Subcommand::Run`] — a case is a
/// program whose output is the expectation. [`Subcommand::Test`] is the other
/// half of `rule:testing/nvst-is-separate`:
/// the program declares `#[Test]` classes and what the case pins is the
/// runner's own report of running them.
///
/// It applies to `--FILE--` alone. `--SKIPIF--` and `--CLEAN--` are the
/// runner's own scaffolding rather than the thing under test, so both are
/// always `nvs run`.
///
/// [`Subcommand::ConfigDumpOrigin`] is the one spelling that runs no program:
/// what it observes is the *tree* the case wrote with `--FILE <path>--`, which
/// is the only place `rule:config/later-wins-and-every-override-is-recorded`
/// 's obligation — every override recorded with both origins — is visible
/// end to end. Such a case still carries a `--FILE--`, and the runner still
/// writes it: it is the program the tree governs, and dropping the section for
/// one spelling would make the format's one required section conditional.
///
/// The roster is **closed**, and a `--format` spelling is a variant of it
/// rather than a flag string carried along: § 22's formats are a closed list
/// too, so this stays one enum whose every value is a command line this binary
/// has, and a misspelling is refused where it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Subcommand {
    /// `nvs run case.nvs` — the program is the case.
    #[default]
    Run,
    /// `nvs test case.nvs` — the program's `#[Test]` methods are the case.
    Test,
    /// `nvs test --format=json case.nvs` — § 22's JSON document is the case.
    TestJson,
    /// `nvs test --format=junit case.nvs` — § 22's JUnit XML is the case.
    TestJunit,
    /// `nvs test --list --format=json case.nvs` — § 22's discovery document is
    /// the case, and what it says is that the program's tests were located
    /// without being run.
    TestListJson,
    /// `nvs config dump --origin` — the configuration tree the case wrote into
    /// its working directory is the case, and `rule:config/check-and-dump-audit-the-tree-offline`
    /// 's listing is the expectation.
    ConfigDumpOrigin,
}

impl Subcommand {
    /// The arguments it is written as, in `--RUN--` and on the command line
    /// alike — the two being one list is what keeps them from disagreeing.
    #[must_use]
    pub fn args(self) -> &'static [&'static str] {
        match self {
            Self::Run => &["run"],
            Self::Test => &["test"],
            Self::TestJson => &["test", "--format=json"],
            Self::TestJunit => &["test", "--format=junit"],
            Self::TestListJson => &["test", "--list", "--format=json"],
            Self::ConfigDumpOrigin => &["config", "dump", "--origin"],
        }
    }

    /// Whether the command line ends in the file `--FILE--` was written to.
    ///
    /// Every spelling but one runs a program named on argv. `nvs config dump`
    /// takes configuration roots positionally instead, and naming a `.nvs`
    /// there would ask it to parse the program as TOML — so it is given none
    /// and reads `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2's `./nvs.toml` out of the working
    /// directory the case just filled, which is the tree under test.
    #[must_use]
    pub fn takes_file(self) -> bool {
        !matches!(self, Self::ConfigDumpOrigin)
    }
}

/// One file a case puts on disk beside its own `--FILE--`.
///
/// Written by `--FILE <relative/path>--`, which may appear any number of
/// times. This is what lets one case cover something that is only observable
/// across files — a `require` target, an autoload root, a shadowing vendor
/// copy (`rule:programs/no-runtime-autoload`).
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
    /// `--ARGS--`, the program's own arguments — **one per line**, and never a
    /// command line to be split.
    ///
    /// A line is an argument whatever it contains, so a space, a quote or a
    /// backslash in one needs no escaping and gets none: this format has no
    /// shell, exactly as `rule:core-classes/process-is-argv-only`
    /// gives `Core\Process` none. Empty lines are dropped, which is what makes a
    /// section written with a blank line under the header the same as an absent
    /// one — and so an *empty* argument is the one thing this section cannot
    /// express, along with a trailing space, since each line is trimmed at its
    /// end.
    ///
    /// These lines are appended to a command line, so the launcher's own
    /// argument parser sees them first and spends a **leading `--`** as its
    /// escape token, the way `cargo run --` does. A case that wants the program
    /// to read a literal `--` as its first word writes the line twice; one
    /// anywhere later is an ordinary argument. `nvs-cli`'s `arguments` field
    /// owns that rule.
    pub args: Vec<String>,
    /// `--ENV--`, the environment the case runs under — **one `NAME=value` per
    /// line**, in the order they were written.
    ///
    /// Added to the environment the runner already has rather than replacing
    /// it: a case says what it needs, never what the machine may keep, so a
    /// `PATH` or a `TMPDIR` the toolchain depends on survives. That also makes
    /// a case's own claim narrow — it can pin the variable it set, and nothing
    /// about the rest of the environment.
    ///
    /// The value is the whole of the line past the first `=`, so it may itself
    /// contain one; a line with no `=` at all is a parse error, since a
    /// variable with no value and one set to the empty string are different
    /// states and guessing which was meant is not the runner's to do. Empty
    /// lines are dropped, on [`Case::args`]' rule.
    pub env: Vec<(String, String)>,
    /// The request this case answers, or `None` where it wrote none of
    /// `--GET--`, `--POST--`, `--POST_RAW--`, `--COOKIE--`, `--HEADERS--`,
    /// `--CLIENT_IP--` and `--SCHEME--`.
    pub request: Option<Request>,
    /// Every `--FILE <relative/path>--`, in the order they were written.
    pub aux: Vec<AuxFile>,
    /// `--EXPECT--` or `--EXPECTF--`, matched against standard output.
    pub expect: Option<Expectation>,
    /// `--EXPECT-ERROR--` or `--EXPECTF-ERROR--`, matched against standard
    /// error; its presence is also what says the run is expected to fail.
    pub expect_error: Option<Expectation>,
    /// `--CLEAN--`, an Novis program run afterwards whose output is ignored.
    pub clean: Option<String>,
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

/// The section names this format knows, in the order the module doc lists
/// them. Anything else is a parse error naming the offender.
const KNOWN: &[&str] = &[
    "TEST",
    "SKIPIF",
    "ARGS",
    "ENV",
    "GET",
    "POST",
    "POST_RAW",
    "COOKIE",
    "HEADERS",
    "CLIENT_IP",
    "SCHEME",
    "FILE",
    "EXPECT",
    "EXPECTF",
    "EXPECT-ERROR",
    "EXPECTF-ERROR",
    "CLEAN",
    "RUN",
];

/// The one section name that takes an argument, and what the argument is.
const TAKES_A_PATH: &str = "FILE";

/// The names the runner writes into the working directory itself, which an
/// auxiliary file therefore may not claim.
const RESERVED_NAMES: &[&str] = &[
    "case.nvs",
    "skipif.nvs",
    "clean.nvs",
    crate::request::FILE_NAME,
];

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
    let sections = section::lex(text).map_err(|stray| {
        err(
            "text before the first section; a case starts with `--TEST--`",
            Some(stray.line),
        )
    })?;

    // The lexer reports the shape and this module holds the roster, so every
    // section is judged in the order it was written: the first thing wrong
    // with a file is still what the file is refused for.
    for (index, seen) in sections.iter().enumerate() {
        let (name, number) = (seen.name.as_str(), seen.line);
        if !KNOWN.contains(&name) {
            return Err(err(format!("unknown section `--{name}--`"), Some(number)));
        }
        match (seen.arg.as_deref(), name) {
            (None, _) => {}
            (Some(raw), TAKES_A_PATH) => {
                aux_path(raw).map_err(|why| err(why, Some(number)))?;
            }
            (Some(_), _) => {
                return Err(err(
                    format!("`--{name}--` does not take an argument"),
                    Some(number),
                ));
            }
        }
        if sections[..index]
            .iter()
            .any(|earlier| earlier.name == seen.name && earlier.arg == seen.arg)
        {
            let shown = match &seen.arg {
                Some(arg) => format!("--{name} {arg}--"),
                None => format!("--{name}--"),
            };
            return Err(err(format!("`{shown}` appears twice"), Some(number)));
        }
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
            "test --format=json" => Subcommand::TestJson,
            "test --format=junit" => Subcommand::TestJunit,
            "test --list --format=json" => Subcommand::TestListJson,
            "config dump --origin" => Subcommand::ConfigDumpOrigin,
            other => {
                return Err(err(
                    format!(
                        "`--RUN--` is `run`, `test`, `test --format=json`, \
                         `test --format=junit`, `test --list --format=json` or \
                         `config dump --origin`, not `{other}`"
                    ),
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

    if expect.is_none() && expect_error.is_none() {
        return Err(err(
            "no expectation: a case needs `--EXPECT--`, `--EXPECTF--`, `--EXPECT-ERROR--` or `--EXPECTF-ERROR--`",
            None,
        ));
    }

    // Refused where it is written rather than dropped: a case that misspells
    // the one thing it asked the runner to set would otherwise run against the
    // environment it was trying to change and pin whatever that produced.
    let mut env = Vec::new();
    if let Some((header, body)) = take("ENV") {
        for (offset, line) in body.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                return Err(err(
                    "`--ENV--` is one `NAME=value` per line, and this line has no `=`",
                    Some(header + 1 + offset),
                ));
            };
            env.push((name.to_owned(), value.to_owned()));
        }
    }

    // Two spellings of one body, merged, is the silent wrong answer this
    // format refuses everywhere else, so writing both is where it stops.
    let body = match (take("POST"), take("POST_RAW")) {
        (Some(_), Some((line, _))) => {
            return Err(err(
                "`--POST--` and `--POST_RAW--` are two spellings of one body",
                Some(line),
            ));
        }
        (Some(pairs), None) => Some(Body::Form(one_line("POST", pairs)?)),
        (None, Some((line, raw))) => {
            if raw.trim().is_empty() {
                return Err(err("`--POST_RAW--` is empty", Some(line)));
            }
            Some(Body::Raw(raw))
        }
        (None, None) => None,
    };
    let query = take("GET")
        .map(|section| one_line("GET", section))
        .transpose()?;
    let cookies = take("COOKIE")
        .map(|section| fields("COOKIE", '=', "NAME=value", section))
        .transpose()?
        .unwrap_or_default();
    let headers = take("HEADERS")
        .map(|section| fields("HEADERS", ':', "Name: value", section))
        .transpose()?
        .unwrap_or_default();
    let client_ip = take("CLIENT_IP")
        .map(|section| address("CLIENT_IP", section))
        .transpose()?;
    let scheme = take("SCHEME").map(scheme_of).transpose()?;
    // Not answering a request is a different state from answering an empty
    // one, and which of the two a case means is what it wrote.
    let request = if query.is_some()
        || body.is_some()
        || !cookies.is_empty()
        || !headers.is_empty()
        || client_ip.is_some()
        || scheme.is_some()
    {
        Some(Request {
            query,
            body,
            cookies,
            headers,
            client_ip,
            scheme,
        })
    } else {
        None
    };
    // `nvs run` is the one subcommand that takes a request, so a case that
    // described one and then asked for another is refused rather than run
    // without it: the sections would otherwise be read and then dropped.
    if request.is_some() && run != Subcommand::Run {
        return Err(err(
            "a case that describes a request is `--RUN--` `run`: no other subcommand takes one",
            take("RUN").map(|(line, _)| line),
        ));
    }

    Ok(Case {
        path: path.to_path_buf(),
        title,
        args: take("ARGS").map_or_else(Vec::new, |(_, body)| {
            body.lines()
                .map(str::trim_end)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        }),
        env,
        request,
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

/// Reads a one-line section as the client address it names, or the reason it
/// names none.
///
/// Refused where it is written rather than carried through as text: the request
/// file this becomes reads it back as an address too, and a case whose peer is
/// a typo would otherwise fail one process later with the wrong line number on
/// it.
fn address(name: &str, section: (usize, String)) -> Result<IpAddr, ParseError> {
    let line = section.0;
    let text = one_line(name, section)?;
    text.parse().map_err(|_| {
        err(
            format!("`--{name}--` is one client address, not `{text}`"),
            Some(line),
        )
    })
}

/// Reads `--SCHEME--` as one of the two schemes a request can have arrived
/// over, or the reason it is neither.
fn scheme_of(section: (usize, String)) -> Result<Scheme, ParseError> {
    let line = section.0;
    let text = one_line("SCHEME", section)?;
    match text.as_str() {
        "http" => Ok(Scheme::Http),
        "https" => Ok(Scheme::Https),
        other => Err(err(
            format!("`--SCHEME--` is `http` or `https`, not `{other}`"),
            Some(line),
        )),
    }
}

/// Reads a section written as a single line, trimmed, or the reason it cannot
/// be read as one.
fn one_line(name: &str, section: (usize, String)) -> Result<String, ParseError> {
    let (line, body) = section;
    let text = body.trim().to_owned();
    if text.is_empty() {
        return Err(err(format!("`--{name}--` is empty"), Some(line)));
    }
    if text.lines().count() > 1 {
        return Err(err(format!("`--{name}--` is one line"), Some(line)));
    }
    Ok(text)
}

/// Reads a section written as one `name<sep>value` per line into its pairs.
///
/// Empty lines are dropped on [`Case::args`]' rule, and a line holding no
/// separator or no name before it is refused where it is written: a case that
/// misspelled the one field it set would otherwise run against the request it
/// was trying to describe and pin whatever that produced.
fn fields(
    name: &str,
    sep: char,
    shape: &str,
    section: (usize, String),
) -> Result<Vec<(String, String)>, ParseError> {
    let (header, body) = section;
    let mut pairs = Vec::new();
    for (offset, line) in body.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let at = Some(header + 1 + offset);
        let Some((field, value)) = line.split_once(sep) else {
            return Err(err(
                format!("`--{name}--` is one `{shape}` per line, and this line has no `{sep}`"),
                at,
            ));
        };
        let field = field.trim();
        if field.is_empty() {
            return Err(err(
                format!("`--{name}--` is one `{shape}` per line, and this line has no name"),
                at,
            ));
        }
        pairs.push((field.to_owned(), value.trim_start().to_owned()));
    }
    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(text: &str) -> Result<Case, ParseError> {
        parse(Path::new("t.nvst"), text)
    }

    const MINIMAL: &str = "--TEST--\nthe title\n--FILE--\n<?nvs\necho 1;\n--EXPECT--\n1\n";

    /// Every section name this format knows whose partner is not in one of the
    /// two documents below it, so the three together name all of `KNOWN`.
    ///
    /// Three and not two because the exclusions are not all pairs: `--POST--`
    /// rules out `--POST_RAW--`, and describing a request at all rules out
    /// every `--RUN--` but the default.
    const EVERY_SECTION: &str = r#"--TEST--
every section, once
--RUN--
test
--SKIPIF--
<?nvs
echo "skip - always";
--ARGS--
first
second
--ENV--
NOVIS_X=1
--FILE--
<?nvs
echo "hi";
--FILE lib/helper.nvs--
<?nvs
function h(): int { return 1; }
--EXPECT--
hi
--EXPECT-ERROR--
nothing
--CLEAN--
<?nvs
echo "clean";
"#;

    /// The other half of each mutually exclusive pair, which a case cannot
    /// write in the same file as the one above.
    const THE_OTHER_HALVES: &str = r#"--TEST--
the other half of each pair
--FILE--
<?nvs
echo "hi";
--EXPECTF--
%s
--EXPECTF-ERROR--
%d
--POST_RAW--
{"name":"ada"}
"#;

    /// The four request sections a case writes together — the fifth is
    /// `--POST_RAW--` above, which `--POST--` here rules out.
    const A_REQUEST: &str = r#"--TEST--
a case that answers a request
--FILE--
<?nvs
echo "hi";
--GET--
page=2&q=novis
--POST--
name=ada&role=author
--COOKIE--
session=abc123
--HEADERS--
Accept: application/json
X-Trace: 7
--EXPECT--
hi
"#;

    #[test]
    fn every_nvst_section_parses_exactly_as_it_did_before_the_extraction() {
        // The section lexer is `crate::section`'s now, so this asks the whole
        // roster — every name, both halves of every exclusive pair, and each
        // refusal the scan used to raise while it read — for the same answers
        // it gave when the scanner lived here.
        let parsed = case(EVERY_SECTION).expect("it parses");
        assert_eq!(parsed.path, Path::new("t.nvst"));
        assert_eq!(parsed.title, "every section, once");
        assert_eq!(parsed.run, Subcommand::Test);
        assert_eq!(
            parsed.skipif.as_deref(),
            Some("<?nvs\necho \"skip - always\";\n")
        );
        assert_eq!(parsed.args, ["first", "second"]);
        assert_eq!(parsed.env, [("NOVIS_X".to_owned(), "1".to_owned())]);
        assert_eq!(parsed.file, "<?nvs\necho \"hi\";\n");
        assert_eq!(
            parsed.aux,
            [AuxFile {
                path: "lib/helper.nvs".to_owned(),
                body: "<?nvs\nfunction h(): int { return 1; }\n".to_owned(),
            }]
        );
        assert_eq!(parsed.expect, Some(Expectation::Exact("hi\n".to_owned())));
        assert_eq!(
            parsed.expect_error,
            Some(Expectation::Exact("nothing\n".to_owned()))
        );
        assert_eq!(parsed.clean.as_deref(), Some("<?nvs\necho \"clean\";\n"));

        let other = case(THE_OTHER_HALVES).expect("it parses");
        assert_eq!(
            other
                .request
                .as_ref()
                .and_then(|request| request.body.as_ref()),
            Some(&Body::Raw("{\"name\":\"ada\"}\n".to_owned()))
        );
        assert_eq!(other.expect, Some(Expectation::Format("%s\n".to_owned())));
        assert_eq!(
            other.expect_error,
            Some(Expectation::Format("%d\n".to_owned()))
        );

        let refused = |text: &str| case(text).expect_err("it is refused");
        assert_eq!(
            refused("hello\n--TEST--\nt\n"),
            ParseError {
                message: "text before the first section; a case starts with `--TEST--`".to_owned(),
                line: Some(1),
            }
        );
        assert_eq!(
            refused(""),
            ParseError {
                message: "no sections; a case starts with `--TEST--`".to_owned(),
                line: None,
            }
        );
        assert_eq!(
            refused("--TEST--\nt\n--BOGUS--\n--TEST--\nt\n").message,
            "unknown section `--BOGUS--`"
        );
        assert_eq!(refused("--TEST--\nt\n--BOGUS--\n").line, Some(3));
        assert_eq!(
            refused("--TEST--\nt\n--EXPECT hi--\n").message,
            "`--EXPECT--` does not take an argument"
        );
        assert_eq!(
            refused(&format!("{MINIMAL}--FILE--\n<?nvs\n")).message,
            "`--FILE--` appears twice"
        );
        assert_eq!(
            refused(&format!(
                "{MINIMAL}--FILE lib/h.nvs--\na\n--FILE lib/h.nvs--\nb\n"
            ))
            .message,
            "`--FILE lib/h.nvs--` appears twice"
        );
        assert!(
            refused(&format!("{MINIMAL}--FILE ../h.nvs--\na\n"))
                .message
                .contains("`..` segment"),
        );
    }

    #[test]
    fn a_case_that_says_nothing_is_run_through_nvs_run() {
        assert_eq!(case(MINIMAL).expect("it parses").run, Subcommand::Run);
    }

    #[test]
    fn a_run_section_names_the_subcommand_the_program_goes_through() {
        let parsed = case(&format!("--RUN--\ntest\n{MINIMAL}")).expect("it parses");
        assert_eq!(parsed.run, Subcommand::Test);
        assert_eq!(parsed.run.args(), ["test"]);
    }

    #[test]
    fn the_config_dump_spelling_names_no_file_on_its_command_line() {
        // `rule:config/check-and-dump-audit-the-tree-offline`'s listing is read out of the working directory, so this
        // is the one spelling whose command line ends at its own arguments —
        // naming `case.nvs` there would hand a program to a TOML parser.
        let parsed = case(&format!("--RUN--\nconfig dump --origin\n{MINIMAL}")).expect("it parses");
        assert_eq!(parsed.run, Subcommand::ConfigDumpOrigin);
        assert_eq!(parsed.run.args(), ["config", "dump", "--origin"]);
        assert!(!parsed.run.takes_file());
        assert!(Subcommand::Run.takes_file());
    }

    #[test]
    fn a_machine_format_is_a_spelling_of_the_test_subcommand() {
        // `rule:testing/report-formats`'s formats are a closed list, so each is a value of the
        // roster rather than a flag string carried along — what a `--RUN--`
        // section names is a whole command line.
        let parsed = case(&format!("--RUN--\ntest --format=json\n{MINIMAL}")).expect("it parses");
        assert_eq!(parsed.run, Subcommand::TestJson);
        assert_eq!(parsed.run.args(), ["test", "--format=json"]);
        let parsed = case(&format!("--RUN--\ntest --format=junit\n{MINIMAL}")).expect("it parses");
        assert_eq!(parsed.run.args(), ["test", "--format=junit"]);
    }

    #[test]
    fn a_run_section_naming_no_subcommand_is_refused_where_it_is_written() {
        let error = case(&format!("--RUN--\ncheck\n{MINIMAL}")).expect_err("the roster is closed");
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("`test --format=json`"), "{error}");
    }

    #[test]
    fn a_minimal_case_parses_into_its_three_sections() {
        let parsed = case(MINIMAL).expect("minimal case parses");
        assert_eq!(parsed.title, "the title");
        assert_eq!(parsed.file, "<?nvs\necho 1;\n");
        assert_eq!(parsed.expect, Some(Expectation::Exact("1\n".to_owned())));
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
    fn an_oracle_section_is_an_unknown_section() {
        let e = case(&format!("{MINIMAL}--ORACLE--\n<?php\necho 1;\n"))
            .expect_err("no case is compared against another language");
        assert_eq!(e.message, "unknown section `--ORACLE--`");
        assert_eq!(e.line, Some(8));
    }

    #[test]
    fn an_oracle_diverges_section_is_an_unknown_section() {
        let e = case(&format!("{MINIMAL}--ORACLE-DIVERGES--\nwhy\n"))
            .expect_err("no case is compared against another language");
        assert_eq!(e.message, "unknown section `--ORACLE-DIVERGES--`");
        assert_eq!(e.line, Some(8));
    }

    #[test]
    fn the_five_sections_describe_one_request() {
        let parsed = case(A_REQUEST).expect("it parses");
        let request = parsed.request.as_ref().expect("it answers a request");
        assert_eq!(request.query.as_deref(), Some("page=2&q=novis"));
        assert_eq!(
            request.body,
            Some(Body::Form("name=ada&role=author".to_owned()))
        );
        assert_eq!(
            request.cookies,
            [("session".to_owned(), "abc123".to_owned())]
        );
        assert_eq!(
            request.headers,
            [
                ("Accept".to_owned(), "application/json".to_owned()),
                ("X-Trace".to_owned(), "7".to_owned()),
            ]
        );
        // The peer sections are the two this case did not write, and a case
        // that writes neither describes a request from nobody in particular
        // over plaintext rather than one with a peer still to be decided.
        assert_eq!(request.client_ip, None);
        assert_eq!(request.scheme, None);

        let refused = case(&A_REQUEST.replace("--FILE--", "--RUN--\ntest\n--FILE--"))
            .expect_err("only `nvs run` takes a request");
        assert_eq!(
            refused.to_string(),
            "line 3: a case that describes a request is `--RUN--` `run`: no other subcommand takes one"
        );
    }

    #[test]
    fn the_peer_sections_say_who_the_request_came_from() {
        let parsed = case(
            "--TEST--\na peer\n--FILE--\n<?nvs\necho 1;\n--CLIENT_IP--\n203.0.113.7\n--SCHEME--\nhttps\n--EXPECT--\n1\n",
        )
        .expect("it parses");
        let request = parsed
            .request
            .expect("naming a peer is describing a request");
        assert_eq!(request.client_ip, Some(IpAddr::from([203, 0, 113, 7])));
        assert_eq!(request.scheme, Some(Scheme::Https));

        // Both are read here rather than carried through as text: the request
        // file reads them back as an address and a scheme too, so a case whose
        // peer is a typo fails on the line that holds it instead of one
        // process later.
        let refused = case(
            "--TEST--\na bad peer\n--FILE--\n<?nvs\necho 1;\n--CLIENT_IP--\n203.0.113.999\n--EXPECT--\n1\n",
        )
        .expect_err("that is not an address");
        assert_eq!(
            refused.to_string(),
            "line 6: `--CLIENT_IP--` is one client address, not `203.0.113.999`"
        );

        let refused = case(
            "--TEST--\na bad scheme\n--FILE--\n<?nvs\necho 1;\n--SCHEME--\nftp\n--EXPECT--\n1\n",
        )
        .expect_err("a request arrives over one of two schemes");
        assert_eq!(
            refused.to_string(),
            "line 6: `--SCHEME--` is `http` or `https`, not `ftp`"
        );
    }

    #[test]
    fn a_post_and_a_post_raw_section_together_are_a_parse_error() {
        // `--POST--` and `--POST_RAW--` are two spellings of one body, and
        // merging them is the silent wrong answer this format refuses
        // everywhere else, so the second one is where the case stops.
        let refused = case(
            "--TEST--\nboth bodies\n--FILE--\n<?nvs\necho 1;\n--POST--\na=1\n--POST_RAW--\na=1\n--EXPECT--\n1\n",
        )
        .expect_err("two spellings of one body");
        assert_eq!(
            refused.to_string(),
            "line 8: `--POST--` and `--POST_RAW--` are two spellings of one body"
        );
    }

    #[test]
    fn a_request_field_is_refused_on_the_line_that_is_wrong() {
        // The case is refused rather than the line dropped: a field that did
        // not arrive is a different request, and the case would go on to pin
        // the answer to it as though that were what it asked.
        let refused = case(
            "--TEST--\na bad field\n--FILE--\n<?nvs\necho 1;\n--HEADERS--\nAccept: text/plain\nX-Trace 7\n--EXPECT--\n1\n",
        )
        .expect_err("a header line with no `:`");
        assert_eq!(
            refused.to_string(),
            "line 8: `--HEADERS--` is one `Name: value` per line, and this line has no `:`"
        );

        let refused = case(
            "--TEST--\na bad field\n--FILE--\n<?nvs\necho 1;\n--COOKIE--\nsession\n--EXPECT--\n1\n",
        )
        .expect_err("a cookie line with no `=`");
        assert_eq!(
            refused.to_string(),
            "line 7: `--COOKIE--` is one `NAME=value` per line, and this line has no `=`"
        );
    }

    #[test]
    fn a_case_that_wrote_no_request_section_answers_no_request() {
        assert_eq!(case(MINIMAL).expect("it parses").request, None);

        let refused = case(
            "--TEST--\ntwo queries\n--FILE--\n<?nvs\necho 1;\n--GET--\na=1\nb=2\n--EXPECT--\n1\n",
        )
        .expect_err("a query string is one line");
        assert_eq!(refused.to_string(), "line 6: `--GET--` is one line");
    }

    #[test]
    fn an_ini_section_is_an_unknown_section() {
        let e = case("--TEST--\nt\n--INI--\nx=1\n--FILE--\n<?nvs\n--EXPECT--\n\n")
            .expect_err("Novis has no ini settings");
        assert_eq!(e.message, "unknown section `--INI--`");
        assert_eq!(e.line, Some(3));
    }

    /// [`Case::env`]'s rule at once: a line is split at its **first** `=` so a
    /// value may hold one, a blank line is not a pair, and the section leaves
    /// the case supported.
    #[test]
    fn every_env_line_is_one_pair_split_at_the_first_equals() {
        let parsed = case(
            "--TEST--\nt\n--ENV--\nNVS_A=1\n\nNVS_B=x=y\nNVS_EMPTY=\n--FILE--\n<?nvs\n--EXPECT--\n\n",
        )
        .expect("an env section parses");
        assert_eq!(
            parsed.env,
            [
                ("NVS_A".to_owned(), "1".to_owned()),
                ("NVS_B".to_owned(), "x=y".to_owned()),
                ("NVS_EMPTY".to_owned(), String::new()),
            ]
        );
    }

    /// A variable with no value and one set to the empty string are different
    /// states, so the runner refuses to guess which a `=`-less line meant.
    #[test]
    fn an_env_line_without_an_equals_is_refused_where_it_is_written() {
        let e = case("--TEST--\nt\n--ENV--\nNVS_A=1\nNVS_B\n--FILE--\n<?nvs\n--EXPECT--\n\n")
            .expect_err("a line with no `=` is a parse error");
        assert_eq!(e.line, Some(5), "{e}");
        assert!(e.message.contains("`NAME=value`"), "{e}");
    }

    /// One line is one argument whatever it holds, and an empty line is not an
    /// argument at all — both halves of [`Case::args`]'s rule, which is what
    /// makes a section written with a blank line the same as an absent one.
    #[test]
    fn every_args_line_is_one_argument_and_a_blank_one_is_none() {
        let parsed = case(
            "--TEST--\nt\n--ARGS--\ngreet\nada lovelace\n\n--dryRun\n--FILE--\n<?nvs\n--EXPECT--\n\n",
        )
        .expect("an args section parses");
        assert_eq!(parsed.args, ["greet", "ada lovelace", "--dryRun"]);
        let bare = case("--TEST--\nt\n--FILE--\n<?nvs\n--EXPECT--\n\n").expect("no args section");
        assert!(bare.args.is_empty());
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
