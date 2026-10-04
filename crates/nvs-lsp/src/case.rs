//! Parsing one `.lspt` file into a [`Case`].
//!
//! The line-oriented shape — what a header looks like, and where a section's
//! body ends — is [`nvs_test::section`]'s, shared with the other case format
//! and the only thing the two share
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). What this module owns
//! is `.lspt`'s own roster: which section names it knows, which request a
//! `--REQUEST--` line may name, which arguments that request takes, and where
//! the cursor is. The format itself is written down in this crate's module
//! documentation, which is its one home.
//!
//! `nvs-test` is a dependency for that one module. It links nothing and writes
//! nothing to standard output, so `tests/stdout_policy.rs`'s walk over the
//! server's dependency tree stays green — `nvs lsp-test`'s own `N passed, M failed` is
//! a line `nvs-cli` prints, where a terminal program's allowance already is.
//!
//! Nothing here answers a request or renders one. A case is data: the document
//! with its cursor taken out, the question, and the frozen answer.

use std::fmt;
use std::path::{Path, PathBuf};

use nvs_test::section;

use crate::TOKEN_TYPES;

/// The cursor marker, removed from the document before it is analysed.
pub const CURSOR: &str = "<|>";

/// The name the case's own `--FILE--` is given on disk.
///
/// A rendering that names a file — a definition as `file:L:C` — names this one
/// for the document under test, so an auxiliary file may not claim it and an
/// expectation can be frozen against a spelling that does not move.
pub const MAIN_PATH: &str = "case.nvs";

/// The section names this format knows. Anything else is refused naming it.
///
/// It is deliberately much shorter than `.nvst`'s: a `.lspt` case runs no
/// program, so there is nothing for `--ARGS--`, `--ENV--`, `--SKIPIF--` or an
/// oracle to mean here.
const KNOWN: &[&str] = &["TEST", "FILE", "REQUEST", "EXPECT"];

/// The one section name that takes an argument, and the argument is a path.
const TAKES_A_PATH: &str = "FILE";

/// Which question a case asks of its document.
///
/// The roster is `rule:ide/the-request-set-is-closed`'s, plus the requests
/// `rule:ide/five-features-are-one-reference-index` answers off the workspace
/// index and the inlay hints
/// `rule:ide/every-feature-is-staged-behind-its-dependency` stages behind the
/// type phase, and it is closed here for the same reason it is closed there: a
/// case that could name a request no runner implements is a case that passes
/// by being skipped. Each is spelled as LSP's own method name with the
/// `textDocument/` prefix dropped, and `nvs/redactions` — the one request of
/// Novis's own — as `redactions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// `textDocument/publishDiagnostics`, phase-gated unless `phase=all`.
    Diagnostics,
    /// `textDocument/hover`.
    Hover,
    /// `textDocument/definition`.
    Definition,
    /// `textDocument/completion`.
    Completion,
    /// `textDocument/semanticTokens/full`.
    SemanticTokens,
    /// `textDocument/documentSymbol`.
    DocumentSymbol,
    /// `textDocument/selectionRange`.
    SelectionRange,
    /// `textDocument/foldingRange`.
    FoldingRange,
    /// `textDocument/documentLink`.
    DocumentLink,
    /// `textDocument/codeAction`.
    CodeAction,
    /// `textDocument/codeLens`, asked with `nvs.codeLens.enable` at the
    /// roster's own default, which is on.
    CodeLens,
    /// `textDocument/references`, asked with the declaration included.
    References,
    /// `textDocument/documentHighlight`, which is [`Self::References`]'s query
    /// narrowed to the file the cursor is in.
    DocumentHighlight,
    /// `textDocument/inlayHint`, asked of the whole document: an editor asks it
    /// over the region it can show, and the narrowing to that region is
    /// `crate::server`'s rather than the answer's own shape.
    InlayHint,
    /// `nvs/redactions`.
    Redactions,
    /// `nvs/regions`.
    Regions,
}

impl Request {
    /// Every request a case may name, in the order an error message lists them.
    pub const ALL: &'static [Self] = &[
        Self::Diagnostics,
        Self::Hover,
        Self::Definition,
        Self::Completion,
        Self::SemanticTokens,
        Self::DocumentSymbol,
        Self::SelectionRange,
        Self::FoldingRange,
        Self::DocumentLink,
        Self::CodeAction,
        Self::CodeLens,
        Self::References,
        Self::DocumentHighlight,
        Self::InlayHint,
        Self::Redactions,
        Self::Regions,
    ];

    /// How a `--REQUEST--` line writes it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Diagnostics => "diagnostics",
            Self::Hover => "hover",
            Self::Definition => "definition",
            Self::Completion => "completion",
            Self::SemanticTokens => "semanticTokens",
            Self::DocumentSymbol => "documentSymbol",
            Self::SelectionRange => "selectionRange",
            Self::FoldingRange => "foldingRange",
            Self::DocumentLink => "documentLink",
            Self::CodeAction => "codeAction",
            Self::CodeLens => "codeLens",
            Self::References => "references",
            Self::DocumentHighlight => "documentHighlight",
            Self::InlayHint => "inlayHint",
            Self::Redactions => "redactions",
            Self::Regions => "regions",
        }
    }

    /// The request `name` spells, if any does.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|it| it.name() == name)
    }

    /// Whether this request is asked *at* a position rather than of the whole
    /// document.
    ///
    /// The ones that are take exactly one `<|>` and the rest take none, which
    /// is `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`'s "exactly one per
    /// case, and none for a request that needs none". The split is structural
    /// rather than a convention: a document-wide request has no cursor to be
    /// asked at, so a `<|>` in such a case is a claim the runner would have to
    /// ignore.
    ///
    /// `codeAction` is asked over a *range* and is still on this side of the
    /// split: the range a case asks over is the empty one at its cursor, which
    /// is where a light bulb appears when nothing is selected. A format that
    /// could write a selection would be freezing a claim about the editor
    /// rather than about the server.
    #[must_use]
    pub fn takes_cursor(self) -> bool {
        matches!(
            self,
            Self::Hover
                | Self::Definition
                | Self::Completion
                | Self::SelectionRange
                | Self::CodeAction
                | Self::References
                | Self::DocumentHighlight
        )
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The arguments a `--REQUEST--` line carried.
///
/// One field per argument rather than a map of strings: the set is closed per
/// request (`rule:ide/a-request-line-is-closed`), so a name nothing here holds
/// is one the parser refuses, and a runner reading a field cannot be handed a
/// value no case could have written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestArgs {
    /// `completion prefix=`: keep only the labels starting with this, so a case
    /// about `->` does not have to freeze the whole keyword list.
    pub prefix: Option<String>,
    /// `completion limit=`: keep at most this many labels.
    pub limit: Option<usize>,
    /// `diagnostics phase=all`: publish every phase's diagnostics, defeating
    /// `rule:ide/diagnostics-are-phase-gated`, which is how the gating itself
    /// gets a case in both directions.
    pub phase_all: bool,
    /// `semanticTokens types=`: render only these token types, so a case about
    /// one of them is not rewritten by every unrelated token added later.
    pub types: Vec<String>,
}

/// One file a case puts on disk beside its own `--FILE--`.
///
/// Written by `--FILE <relative/path>--`, and it means exactly what it means in
/// a `.nvst` case: this is how a go-to-definition case reaches across a
/// `require`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxFile {
    /// Where to write it, relative to the case's own directory, always with
    /// `/` separators and never escaping that directory.
    pub path: String,
    /// Its contents, verbatim.
    pub body: String,
}

/// One parsed `.lspt` file.
#[derive(Debug, Clone)]
pub struct Case {
    /// Where it was read from, for reporting.
    pub path: PathBuf,
    /// `--TEST--`, the one-line title.
    pub title: String,
    /// `--FILE--`, **with the cursor marker taken out** — the bytes the server
    /// is asked about, which are the bytes an editor would hold.
    pub document: String,
    /// Where `<|>` was, as a byte offset into [`Case::document`].
    ///
    /// `None` for a request asked of the whole document, and never `None` for
    /// one asked at a position.
    pub cursor: Option<usize>,
    /// Every `--FILE <path>--`, in the order they were written.
    pub aux: Vec<AuxFile>,
    /// `--REQUEST--`, the question.
    pub request: Request,
    /// The arguments that line carried.
    pub args: RequestArgs,
    /// `--EXPECT--`, verbatim: the rendering this answer is frozen as.
    pub expect: String,
}

/// Why a `.lspt` file could not be read as a case.
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

impl std::error::Error for ParseError {}

fn err(message: impl Into<String>, line: Option<usize>) -> ParseError {
    ParseError {
        message: message.into(),
        line,
    }
}

/// Checks an auxiliary file's path, returning it or the reason it is refused.
///
/// The path is joined onto a directory the runner owns, so this is the whole of
/// the containment argument: `/` separators only, no root, no drive letter, and
/// no `.` or `..` segment means the result cannot name anything outside it.
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
    if raw == MAIN_PATH {
        return Err(format!(
            "`{raw}` is the name the case's own `--FILE--` is written to"
        ));
    }
    Ok(raw.to_owned())
}

/// Reads the request name and its arguments off a `--REQUEST--` body.
///
/// Splitting on whitespace is the whole of the tokenizing, because every
/// argument's value is a name, a number or a comma-separated list of names —
/// none of them can contain a space, so the format needs no quoting and gets
/// none.
fn request_line(body: &str, line: usize) -> Result<(Request, RequestArgs), ParseError> {
    let text = body.trim();
    if text.is_empty() {
        return Err(err("`--REQUEST--` is empty", Some(line)));
    }
    if text.lines().count() > 1 {
        return Err(err("`--REQUEST--` is one line", Some(line)));
    }

    let mut words = text.split_whitespace();
    let name = words.next().expect("a non-empty line has a first word");
    let Some(request) = Request::from_name(name) else {
        let known: Vec<&str> = Request::ALL.iter().map(|it| it.name()).collect();
        return Err(err(
            format!(
                "`{name}` is not a request this server answers; \
                 `rule:ide/the-request-set-is-closed` names {}",
                known.join(", ")
            ),
            Some(line),
        ));
    };

    let mut args = RequestArgs::default();
    let mut seen: Vec<&str> = Vec::new();
    for word in words {
        let Some((key, value)) = word.split_once('=') else {
            return Err(err(
                format!("`{word}` is not a `key=value` argument"),
                Some(line),
            ));
        };
        if seen.contains(&key) {
            return Err(err(format!("`{key}=` appears twice"), Some(line)));
        }
        seen.push(key);
        apply_argument(request, key, value, &mut args).map_err(|why| err(why, Some(line)))?;
    }

    Ok((request, args))
}

/// Applies one `key=value` to `args`, or says why this request does not take it.
///
/// The refusal is the point: a silently-dropped argument is a case that passes
/// while testing something else, so an unknown one fails loudly rather than
/// being ignored (`rule:ide/a-request-line-is-closed`).
fn apply_argument(
    request: Request,
    key: &str,
    value: &str,
    args: &mut RequestArgs,
) -> Result<(), String> {
    match (request, key) {
        (Request::Completion, "prefix") => args.prefix = Some(value.to_owned()),
        (Request::Completion, "limit") => {
            args.limit = Some(
                value
                    .parse()
                    .map_err(|_| format!("`limit={value}` is not a number"))?,
            );
        }
        (Request::Diagnostics, "phase") => {
            if value != "all" {
                return Err(format!(
                    "`phase={value}` is not a phase; `phase=all` is the one spelling, \
                     and the gated answer is what a case with no argument gets"
                ));
            }
            args.phase_all = true;
        }
        (Request::SemanticTokens, "types") => {
            for wanted in value.split(',') {
                if !TOKEN_TYPES.iter().any(|it| it.as_str() == wanted) {
                    return Err(format!(
                        "`{wanted}` is not a token type this server emits; the legend is \
                         `nvs_lsp::TOKEN_TYPES`"
                    ));
                }
                args.types.push(wanted.to_owned());
            }
        }
        _ => {
            let takes = match request {
                Request::Completion => "`prefix=` and `limit=`",
                Request::Diagnostics => "`phase=all`",
                Request::SemanticTokens => "`types=`",
                _ => "no arguments",
            };
            return Err(format!("`{request}` takes {takes}, not `{key}=`"));
        }
    }
    Ok(())
}

/// Takes the cursor out of `document`, returning where it was.
///
/// The count is checked against what the request needs before anything is
/// removed, so the two ways a case can be wrong about its cursor — none where
/// one is needed, more than one anywhere — are refused with the request named
/// rather than silently answered at the first marker.
fn take_cursor(
    document: &mut String,
    request: Request,
    line: usize,
) -> Result<Option<usize>, ParseError> {
    let count = document.matches(CURSOR).count();
    if count > 1 {
        return Err(err(
            format!("`{CURSOR}` appears {count} times; a case has exactly one cursor or none"),
            Some(line),
        ));
    }
    match (request.takes_cursor(), count) {
        (true, 0) => Err(err(
            format!("`{request}` is asked at a position, so `--FILE--` needs a `{CURSOR}` cursor"),
            Some(line),
        )),
        (false, 1) => Err(err(
            format!(
                "`{request}` is asked of the whole document, so `--FILE--` takes no `{CURSOR}`"
            ),
            Some(line),
        )),
        (_, 0) => Ok(None),
        (_, _) => {
            let at = document.find(CURSOR).expect("one match was counted");
            document.replace_range(at..at + CURSOR.len(), "");
            Ok(Some(at))
        }
    }
}

impl Case {
    /// Reads `text` as a `.lspt` case named by `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for an unknown or repeated section, a missing
    /// required one, a request or argument outside the closed set, or a cursor
    /// that does not match what the request asks for.
    pub fn parse(path: &Path, text: &str) -> Result<Self, ParseError> {
        let sections = section::lex(text).map_err(|stray| {
            err(
                "text before the first section; a case starts with `--TEST--`",
                Some(stray.line),
            )
        })?;

        // The lexer reports the shape and this module holds the roster, so
        // every section is judged in the order it was written: the first thing
        // wrong with a file is what the file is refused for.
        for (index, seen) in sections.iter().enumerate() {
            let (name, number) = (seen.name.as_str(), seen.line);
            if !KNOWN.contains(&name) {
                return Err(err(
                    format!(
                        "unknown section `--{name}--`; a `.lspt` case is \
                         `--TEST--`, `--FILE--`, `--REQUEST--` and `--EXPECT--`"
                    ),
                    Some(number),
                ));
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

        let Some((file_line, mut document)) = take("FILE") else {
            return Err(err("no `--FILE--` section", None));
        };
        if document.trim().is_empty() {
            return Err(err("`--FILE--` is empty", Some(file_line)));
        }

        let Some((request_line_number, request_body)) = take("REQUEST") else {
            return Err(err("no `--REQUEST--` section", None));
        };
        let (request, args) = request_line(&request_body, request_line_number)?;

        let Some((_, expect)) = take("EXPECT") else {
            return Err(err("no `--EXPECT--` section", None));
        };

        // The auxiliary files are read before the cursor is taken, so a case
        // that put its cursor in one is refused for that rather than for the
        // main document then having none — the second message names the request
        // and would send the reader to the wrong section.
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
            if section.body.contains(CURSOR) {
                return Err(err(
                    format!(
                        "`--FILE {path}--` carries the `{CURSOR}` cursor; the cursor is in the \
                         case's own `--FILE--`, which is the document the request is asked about"
                    ),
                    Some(section.line),
                ));
            }
            aux.push(AuxFile {
                path,
                body: section.body.clone(),
            });
        }

        // An auxiliary file is a document the request is not asked about, so
        // the cursor is the main document's alone.
        let cursor = take_cursor(&mut document, request, file_line)?;

        Ok(Self {
            path: path.to_path_buf(),
            title,
            document,
            cursor,
            aux,
            request,
            args,
            expect,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical case, as `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`
    /// writes it.
    const MEMBER_COMPLETION: &str = "\
--TEST--
member completion survives an unclosed brace
--FILE--
<?nvs
class User { public string $name; public function greet(): string { return \"hi\"; } }
$u = new User();
$u-><|>
if (true) {
--REQUEST--
completion
--EXPECT--
greet   method    (): string
name    property  string
";

    fn parse(text: &str) -> Result<Case, ParseError> {
        Case::parse(Path::new("case.lspt"), text)
    }

    #[test]
    fn an_lspt_case_reads_its_sections_through_the_shared_lexer() {
        let case = parse(MEMBER_COMPLETION).expect("the canonical case parses");
        assert_eq!(case.title, "member completion survives an unclosed brace");
        assert_eq!(case.request, Request::Completion);
        assert_eq!(case.args, RequestArgs::default());
        assert_eq!(
            case.expect,
            "greet   method    (): string\nname    property  string\n"
        );

        // The cursor is gone from the bytes the server is asked about, and what
        // is left is where it was.
        assert!(!case.document.contains(CURSOR), "{}", case.document);
        let at = case.cursor.expect("completion is asked at a position");
        assert!(case.document[..at].ends_with("$u->"), "{at}");
        assert!(case.document[at..].starts_with("\nif (true) {"), "{at}");

        // `nvs_test::section` is the one lexer, so a header spelling it defines
        // is honoured here identically — including a body line that merely
        // contains dashes, which is not a header at all.
        let quoted = MEMBER_COMPLETION.replace("$u-><|>", "$u->x; // --EXPECT--\n$u-><|>");
        let case = parse(&quoted).expect("a body line is not a header");
        assert_eq!(
            case.expect,
            "greet   method    (): string\nname    property  string\n"
        );

        // The roster is this module's, and it is not `.nvst`'s: a section that
        // format knows is refused here rather than ignored.
        let oracle = MEMBER_COMPLETION.replace("--REQUEST--", "--ORACLE--\n<?php\n--REQUEST--");
        let refused = parse(&oracle).expect_err("`.lspt` has no `--ORACLE--`");
        assert!(refused.to_string().contains("--ORACLE--"), "{refused}");

        for missing in ["--TEST--", "--FILE--", "--REQUEST--", "--EXPECT--"] {
            let without = MEMBER_COMPLETION.replace(missing, "--TRIMMED--");
            let refused = parse(&without).expect_err("every section is required");
            assert!(refused.to_string().contains("--TRIMMED--"), "{refused}");
        }
    }

    #[test]
    fn a_second_file_section_carries_its_own_path() {
        let across_files = "\
--TEST--
a definition reaches across a require
--FILE--
<?nvs
require './lib/user.nvs';
$u = new User();
$u->gre<|>et();
--FILE lib/user.nvs--
<?nvs
class User { public function greet(): string { return \"hi\"; } }
--REQUEST--
definition
--EXPECT--
lib/user.nvs:2:36
";
        let case = parse(across_files).expect("a multi-file case parses");
        assert_eq!(case.aux.len(), 1);
        assert_eq!(case.aux[0].path, "lib/user.nvs");
        assert!(
            case.aux[0].body.starts_with("<?nvs\n"),
            "{}",
            case.aux[0].body
        );
        assert!(!case.document.contains("class User"), "{}", case.document);

        // The path is contained, spelled one way, and never the name the case's
        // own document is written to.
        for bad in ["/etc/passwd", "../escape.nvs", "lib\\user.nvs", MAIN_PATH] {
            let refused = parse(&across_files.replace("lib/user.nvs--", &format!("{bad}--")))
                .expect_err("an auxiliary path is checked");
            assert!(refused.line.is_some(), "{refused}");
        }

        // Two auxiliary files are two paths; the same path twice is a case that
        // cannot say which body it meant.
        let twice = across_files.replace(
            "--REQUEST--",
            "--FILE lib/user.nvs--\n<?nvs\nclass Other {}\n--REQUEST--",
        );
        let refused = parse(&twice).expect_err("one path is written once");
        assert!(refused.to_string().contains("appears twice"), "{refused}");

        // The cursor belongs to the document the request is asked about.
        let stray = across_files
            .replace("$u->gre<|>et();", "$u->greet();")
            .replace("class User", "class Us<|>er");
        let refused = parse(&stray).expect_err("an auxiliary file carries no cursor");
        assert!(refused.to_string().contains("lib/user.nvs"), "{refused}");
    }

    #[test]
    fn exactly_one_cursor_is_required_where_the_request_takes_one() {
        // A request asked at a position needs one, and says so by name.
        let none = MEMBER_COMPLETION.replace("$u-><|>", "$u->");
        let refused = parse(&none).expect_err("completion is asked at a position");
        assert!(refused.to_string().contains("completion"), "{refused}");
        assert!(refused.to_string().contains(CURSOR), "{refused}");

        // Two is refused wherever they are, because neither is the position.
        let two = MEMBER_COMPLETION.replace("$u = new User();", "$u<|> = new User();");
        let refused = parse(&two).expect_err("a case has one cursor");
        assert!(refused.to_string().contains("exactly one"), "{refused}");

        // A request asked of the whole document has no position to be asked at,
        // so a cursor in one is a claim the runner would have to ignore.
        let document_wide = MEMBER_COMPLETION
            .replace("completion\n", "documentSymbol\n")
            .replace("$u-><|>", "$u->");
        let case = parse(&document_wide).expect("documentSymbol needs no cursor");
        assert_eq!(case.cursor, None);
        let refused = parse(&document_wide.replace("$u->", "$u-><|>"))
            .expect_err("documentSymbol is asked of the whole document");
        assert!(refused.to_string().contains("documentSymbol"), "{refused}");

        // Which side of the split each request is on, asserted over the whole
        // closed set rather than the four that happen to have cases today.
        let at_a_position: Vec<&str> = Request::ALL
            .iter()
            .filter(|it| it.takes_cursor())
            .map(|it| it.name())
            .collect();
        assert_eq!(
            at_a_position,
            [
                "hover",
                "definition",
                "completion",
                "selectionRange",
                "codeAction",
                "references",
                "documentHighlight"
            ]
        );
    }

    #[test]
    fn an_unknown_request_or_argument_fails_the_case() {
        // `moniker`, and not a request this catalog has merely not reached yet:
        // it is a real LSP method that resolves a symbol to a cross-repository
        // identifier, which is nothing a document's own sections could freeze,
        // so the spelling stays unknown instead of turning into a fixture that
        // becomes a pass the day an arm lands under it.
        let refused = parse(&MEMBER_COMPLETION.replace("completion\n", "moniker\n"))
            .expect_err("the request set is closed");
        assert!(refused.to_string().contains("moniker"), "{refused}");
        assert!(refused.to_string().contains("completion"), "{refused}");

        // What each request takes is closed too, and the message says what it
        // does take rather than only that this is not it.
        let refused = parse(&MEMBER_COMPLETION.replace("completion\n", "completion depth=2\n"))
            .expect_err("`depth=` is nobody's argument");
        assert!(
            refused.to_string().contains("`prefix=` and `limit=`"),
            "{refused}"
        );

        let refused = parse(&MEMBER_COMPLETION.replace("completion\n", "completion limit=lots\n"))
            .expect_err("a limit is a number");
        assert!(refused.to_string().contains("not a number"), "{refused}");

        let refused = parse(&MEMBER_COMPLETION.replace("completion\n", "completion prefix\n"))
            .expect_err("an argument is `key=value`");
        assert!(refused.to_string().contains("key=value"), "{refused}");

        let case =
            parse(&MEMBER_COMPLETION.replace("completion\n", "completion prefix=gr limit=1\n"))
                .expect("both of completion's arguments parse");
        assert_eq!(case.args.prefix.as_deref(), Some("gr"));
        assert_eq!(case.args.limit, Some(1));

        // `types=` is checked against the legend the server actually declares,
        // so a case cannot restrict a rendering to a token type nothing emits.
        let whole_document = MEMBER_COMPLETION.replace("$u-><|>", "$u->");
        let case =
            parse(&whole_document.replace("completion\n", "semanticTokens types=class,method\n"))
                .expect("both names are in the legend");
        assert_eq!(case.args.types, ["class", "method"]);
        let refused = parse(&whole_document.replace("completion\n", "semanticTokens types=goto\n"))
            .expect_err("`goto` is in no legend");
        assert!(refused.to_string().contains("token type"), "{refused}");

        // `phase=all` defeats the gating; nothing else is a phase.
        let case = parse(&whole_document.replace("completion\n", "diagnostics phase=all\n"))
            .expect("`phase=all` is the one spelling");
        assert!(case.args.phase_all);
        let refused = parse(&whole_document.replace("completion\n", "diagnostics phase=types\n"))
            .expect_err("a phase is not named one at a time");
        assert!(refused.to_string().contains("phase=all"), "{refused}");
    }
}
