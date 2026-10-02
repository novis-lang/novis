//! The three requests answered out of what `hover` already reads, and the
//! fourth that is deliberately not answered.
//!
//! `rule:ide/every-feature-is-staged-behind-its-dependency` stages each feature
//! behind the piece it needs, and these three needed nothing further: the row a
//! `Core` member hovers with is the row signature help shows, the type the
//! checker recorded for an expression is where `typeDefinition` goes, and the
//! subtype edges the one workspace index already keeps are what
//! `implementation` lists.
//!
//! None of the four is a `.lspt` request. A `nvs_lsp::Request` variant owes a
//! full row of the coverage matrix — one case per construct the corpus knows —
//! and the claims here are about one answer each rather than about every
//! construct, so they are written where a claim of that size belongs.
//!
//! The signature-help halves analyse a buffer alone, which is all an analysis
//! needs (`tests/completion.rs`); the two navigations are driven over
//! `lsp_server::Connection::memory()` the way `tests/references.rs` drives its
//! readers, because what they claim is that the answer crosses a `require` into
//! a file the request never named.
//!
//! One hover is here too: a path argument's, which answers an absolute path a
//! `.lspt` case cannot freeze because it names the machine the case ran on.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::notification::{DidOpenTextDocument, Notification as _};
use lsp_types::{
    DidOpenTextDocumentParams, InitializeParams, Location, ParameterLabel, PositionEncodingKind,
    SignatureHelp, TextDocumentItem, Uri, WorkspaceFolder,
};
use nvs_lsp::{Analysed, CURSOR, Documents, analyse, hover, uri_of};

/// The class every signature-help document below calls into.
///
/// Two methods and a property, because the claim needs three things in one
/// document: two parameters to be between, a second row so a nested call is
/// visibly a different answer, and a member access to sit on that is not itself
/// a call.
const ADDER: &str = "class Adder {\n    public int $base = 0;\n    \
                     public function add(int $a, int $b): int { return $a + $b; }\n    \
                     public function twice(int $n): int { return $n + $n; }\n}";

/// The row `Adder::add` renders as, which is the one every case below is
/// answered with unless it says otherwise.
const ADD_ROW: &str = "Adder::add(int $a, int $b): int";

/// A document declaring [`ADDER`], holding a receiver, and ending in `tail`.
fn document(tail: &str) -> String {
    format!("<?nvs\n{ADDER}\nvar $x = new Adder();\n{tail}\n")
}

/// `source` analysed as its own entry point, out of an open buffer alone —
/// nothing here `require`s anything, so no directory is needed
/// (`nvs_diagnostics::SourceMap::load`).
fn analysed(source: &str) -> Analysed {
    let uri = uri_of(&std::env::temp_dir().join("nvs-navigation-case.nvs"))
        .expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    analyse(&documents, &uri).expect("an open document analyses")
}

/// The signature help answered at the one `<|>` in `document`.
fn help(document: &str) -> SignatureHelp {
    let at = document
        .find(CURSOR)
        .expect("the document writes exactly one cursor");
    let offset = u32::try_from(at).expect("a test document is short");
    hover::help_at(&analysed(&document.replace(CURSOR, "")), offset)
        .expect("the cursor is inside a call the checker resolved")
}

/// One help answer's label, and each parameter read back out of that label by
/// the offsets that arrived with it.
///
/// Read back rather than compared against a second list written here, because
/// the offsets are what a client acts on: a parameter entry pointing at the
/// wrong span of the label is highlighted in the wrong place, and a comparison
/// against text this file holds would never see it.
fn row(help: &SignatureHelp) -> (String, Vec<String>) {
    let [signature] = help.signatures.as_slice() else {
        panic!(
            "a resolved call has exactly one signature: {:?}",
            help.signatures
        );
    };
    let label: Vec<u16> = signature.label.encode_utf16().collect();
    let params = signature
        .parameters
        .as_ref()
        .expect("a signature carries its parameter list")
        .iter()
        .map(|param| match &param.label {
            ParameterLabel::LabelOffsets([start, end]) => {
                let slice = label
                    .get(*start as usize..*end as usize)
                    .unwrap_or_else(|| panic!("[{start}, {end}) is outside `{}`", signature.label));
                String::from_utf16(slice).expect("an offset pair falls on a character boundary")
            }
            ParameterLabel::Simple(text) => {
                panic!(
                    "a parameter is offsets into the label, not a substring to search for: {text}"
                )
            }
        })
        .collect();
    (signature.label.clone(), params)
}

#[test]
fn signature_help_names_the_active_parameter_of_the_enclosing_call() {
    // The cursor's position within the argument list is counted off the
    // argument spans the parser produced, so what varies down this list is only
    // where the caret is — the call, and therefore the row, is the same one
    // until the last two rows put a second call in the way.
    let places = [
        (
            "before the first argument",
            "echo $x->add(<|>1, 2);",
            0,
            ADD_ROW,
        ),
        (
            "at the first argument's last byte",
            "echo $x->add(1<|>, 2);",
            0,
            ADD_ROW,
        ),
        ("just past the comma", "echo $x->add(1,<|> 2);", 1, ADD_ROW),
        (
            "inside the second argument",
            "echo $x->add(1, <|>2);",
            1,
            ADD_ROW,
        ),
        (
            "on the method's own name",
            "echo $x->add<|>(1, 2);",
            0,
            ADD_ROW,
        ),
        // A property access is a target of its own, and hover stops on it. This
        // request does not: what encloses the cursor is still the call, and the
        // walk keeps going outward past every target that is not one.
        (
            "on a member access written as an argument",
            "echo $x->add($x-><|>base, 2);",
            0,
            ADD_ROW,
        ),
        // A call inside a call: the inner one encloses the cursor, so its row
        // is the answer and its parameters are what is being counted.
        (
            "inside a nested call",
            "echo $x->add(1, $x->twice(<|>3));",
            0,
            "Adder::twice(int $n): int",
        ),
        // And one byte later the inner call is closed, so the cursor is back in
        // the outer one on the argument that inner call was.
        (
            "just past a nested call",
            "echo $x->add(1, $x->twice(3)<|>);",
            1,
            ADD_ROW,
        ),
    ];

    for (where_, tail, expected, expected_row) in places {
        let answer = help(&document(tail));
        assert_eq!(
            row(&answer).0,
            expected_row,
            "the cursor {where_} was answered about a different call than the one enclosing it"
        );
        assert_eq!(
            answer.active_parameter,
            Some(expected),
            "the cursor {where_} is on a different parameter than the one reported"
        );
        assert_eq!(
            answer.signatures[0].active_parameter, answer.active_parameter,
            "the per-signature field a 3.16 client prefers disagrees with the one below it"
        );
    }
}

/// The Markdown a hover at the one `<|>` in `source` answers with.
fn hovered(source: &str) -> (Analysed, Option<String>) {
    let cursor = source.find(CURSOR).expect("a cursor is written");
    let analysed = analysed(&source.replacen(CURSOR, "", 1));
    let at = u32::try_from(cursor).expect("a test document is short");
    let value = hover::at(
        &analysed,
        &nvs_lsp::completion_files::CompletionFiles::default(),
        at,
        nvs_diagnostics::PositionEncoding::Utf8,
    )
    .map(|hover| {
        let lsp_types::HoverContents::Markup(markup) = hover.contents else {
            panic!("a hover is Markdown");
        };
        markup.value
    });
    (analysed, value)
}

/// A literal at a path parameter hovers as the absolute path the compiler
/// resolved it to, and says whether a file or a directory is there. The path
/// is absolute and depends on the machine, so a `.lspt` case cannot freeze it.
#[test]
fn a_path_argument_hovers_as_the_path_it_names_and_whether_it_exists() {
    let (analysed, value) = hovered("<?nvs\necho Core\\IO::read('data/no<|>ne.json');");
    let (_, joined) = analysed
        .exprs
        .path_literals()
        .next()
        .expect("the checker resolved the literal");
    assert!(Path::new(joined).is_absolute() && joined.ends_with("none.json"));
    assert_eq!(
        value.as_deref(),
        Some(format!("```text\n{joined}\n```\n\nNothing exists at this path.").as_str())
    );

    // An absolute literal names itself. The temporary directory exists.
    let temp = std::env::temp_dir();
    let written = temp.to_string_lossy().replace('\\', "/");
    let (_, value) = hovered(&format!(
        "<?nvs\nvar $names = Core\\IO::list('{written}<|>');"
    ));
    assert_eq!(
        value,
        Some(format!("```text\n{written}\n```\n\nThis directory exists.")),
    );

    // A literal at a parameter that is not a path is not answered as one.
    let (_, value) = hovered("<?nvs\necho Core\\Str::length('data/no<|>ne.json');");
    assert!(value.is_none_or(|value| !value.contains("exists")));
}

#[test]
fn signature_help_renders_a_core_row_and_a_declared_list_the_same_way() {
    // `Core\Str::length`'s row is frozen by
    // `tests/lsp/hover/a-core-member-answers-its-registry-row.lspt` as the line
    // hover shows above its card. The same text here is the whole claim: one
    // renderer answers both requests, so a registry row and a class's declared
    // parameter list cannot come to be spelled two ways.
    let core = row(&help(&document(r#"echo Core\Str::length(<|>"hello");"#)));
    assert_eq!(
        core.0, "Core\\Str::length(string $s): uint",
        "a `Core` row is spelled differently here than where hover freezes it"
    );
    assert_eq!(core.1, ["string $s"], "the `Core` row's one parameter");

    let declared = row(&help(&document("echo $x->add(<|>1, 2);")));
    assert_eq!(
        declared.0, ADD_ROW,
        "a declared method's row is not in the `Core` row's shape"
    );
    assert_eq!(declared.1, ["int $a", "int $b"], "the declared parameters");

    // And the shape itself, asserted the same way on both: the declaring class,
    // `::`, the member, its parameters between parentheses separated by `, `,
    // and the return type after a colon. A row that put its parameters anywhere
    // else would still have passed the two equalities above by being written
    // into this file twice.
    for (what, (label, params)) in [("a `Core` row", core), ("a declared method", declared)] {
        let opener = format!("({}): ", params.join(", "));
        let (head, tail) = label.split_once(&opener).unwrap_or_else(|| {
            panic!("{what} does not hold its parameters as `{opener}`: {label}")
        });
        assert!(
            head.contains("::"),
            "{what} does not name the class that declares it: {label}"
        );
        assert!(
            !tail.is_empty(),
            "{what} does not end in a return type: {label}"
        );
    }
}

/// The required file: an interface with two implementors, and a class whose
/// method answers another class.
///
/// One file for both navigations because what is under test is that the answer
/// leaves the document the request named — the client opens `main.nvs` and
/// every declaration either request reaches is here.
const LIB: &str = "<?nvs\ninterface Shape { public function area(): int; }\n\
                   class Square implements Shape { public function area(): int { return 1; } }\n\
                   class Circle implements Shape { public function area(): int { return 2; } }\n\
                   class Engine { public function power(): int { return 1; } }\n\
                   class Car { public function engine(): Engine { return new Engine(); } }\n";

/// The open document, reaching [`LIB`] through one `require`.
const MAIN: &str = "<?nvs\nrequire 'lib.nvs';\nvar $c = new Car();\n\
                    echo $c->engine()->power();\n";

/// The cursor for the type navigation: inside `engine` on line 4 of [`MAIN`].
///
/// The expression is a call, so what the checker recorded for it is its return
/// type — and that type's declaration is in [`LIB`], which is the crossing this
/// test exists for.
const ON_ENGINE_CALL: (u32, u32) = (3, 10);

/// The cursor for the implementor navigation: inside `Shape` where [`LIB`]
/// declares it, on line 2.
const ON_SHAPE: (u32, u32) = (1, 11);

/// A directory of this run's own, removed when the test that made it ends.
///
/// On disk because `nvs_hir::requires` canonicalizes a `require` target against
/// the filesystem before any source map is consulted — `tests/references.rs`
/// and `tests/index.rs` both need the same thing for the same reason.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("nvs-nav-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory");
        fs::write(path.join("lib.nvs"), LIB).expect("a fixture file");
        fs::write(path.join("main.nvs"), MAIN).expect("a fixture file");
        Self { path }
    }

    fn uri(&self, name: &str) -> Uri {
        uri_of(&self.path.join(name)).expect("a temp path is UTF-8")
    }

    fn folder(&self) -> WorkspaceFolder {
        WorkspaceFolder {
            uri: uri_of(&self.path).expect("a temp path is UTF-8"),
            name: "workspace".to_owned(),
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Runs `exchange` against a server in this process that has already answered
/// `initialize` for `folder`, then shuts it down and asserts it served without
/// a protocol error.
fn served(folder: WorkspaceFolder, exchange: impl FnOnce(&Connection)) {
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));

    let params = InitializeParams {
        workspace_folders: Some(vec![folder]),
        ..InitializeParams::default()
    };
    ask(
        &client,
        1,
        "initialize",
        serde_json::to_value(params).expect("params"),
    );
    notify(
        &client,
        Notification::new("initialized".to_owned(), serde_json::json!({})),
    );

    exchange(&client);

    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(99),
            "shutdown".to_owned(),
            serde_json::json!(null),
        )))
        .expect("the server is still reading");
    // Anything still queued ahead of the acknowledgement is drained: a publish
    // this test never read is not a protocol error.
    while !matches!(client.receiver.recv(), Ok(Message::Response(_)) | Err(_)) {}
    notify(
        &client,
        Notification::new("exit".to_owned(), serde_json::json!(null)),
    );

    serving
        .join()
        .expect("the server thread did not panic")
        .expect("the server served without a protocol error");
}

fn notify(client: &Connection, notification: Notification) {
    client
        .sender
        .send(Message::Notification(notification))
        .expect("the server is still reading");
}

/// Tells the server `uri` is open, holding `text`.
fn opened(client: &Connection, uri: &Uri, text: &str) {
    notify(
        client,
        Notification::new(
            DidOpenTextDocument::METHOD.to_owned(),
            serde_json::to_value(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "nvs".to_owned(),
                    version: 1,
                    text: text.to_owned(),
                },
            })
            .expect("params"),
        ),
    );
}

/// Sends one request and returns its result, skipping every
/// `publishDiagnostics` that arrives first.
fn ask(client: &Connection, id: i32, method: &str, params: serde_json::Value) -> serde_json::Value {
    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(id),
            method.to_owned(),
            params,
        )))
        .expect("the server is still reading");

    loop {
        match client.receiver.recv_timeout(Duration::from_secs(30)) {
            Ok(Message::Response(response)) => {
                return response
                    .response_result
                    .unwrap_or_else(|error| panic!("`{method}` was refused: {error:?}"));
            }
            Ok(Message::Notification(_)) => {}
            other => panic!("expected the `{method}` response, got {other:?}"),
        }
    }
}

/// The position parameters of a cursor request, at `(line, character)` in
/// `uri`, zero-based as they are on the wire.
fn at(uri: &Uri, (line, character): (u32, u32)) -> serde_json::Value {
    serde_json::json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
    })
}

/// The file name a location names, without the directory this run invented.
fn name_of(location: &Location) -> String {
    Path::new(location.uri.path().as_str())
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[test]
fn type_definition_and_implementation_resolve_through_the_graph() {
    let dir = TempDir::new("graph");
    served(dir.folder(), |client| {
        opened(client, &dir.uri("main.nvs"), MAIN);
        opened(client, &dir.uri("lib.nvs"), LIB);

        // The cursor is on a call in `main.nvs` and the answer is a class in
        // `lib.nvs`: the type the checker recorded for the call is followed to
        // where it was declared, which is a file the request never named and
        // only the resolved `require` graph reaches.
        let found: Location = serde_json::from_value(ask(
            client,
            2,
            "textDocument/typeDefinition",
            at(&dir.uri("main.nvs"), ON_ENGINE_CALL),
        ))
        .expect("one location, since a type is declared in one place");
        assert_eq!(
            (
                name_of(&found),
                found.range.start.line,
                found.range.start.character
            ),
            ("lib.nvs".to_owned(), 4, 6),
            "`typeDefinition` did not reach `class Engine` in the required file"
        );

        // And the same index, read for the edges it already keeps: every type
        // recorded as implementing the one under the cursor, which is the
        // type hierarchy's subtype query answered as a jump list.
        let found: Vec<Location> = serde_json::from_value(ask(
            client,
            3,
            "textDocument/implementation",
            at(&dir.uri("lib.nvs"), ON_SHAPE),
        ))
        .expect("a list, which may be empty");
        let mut places: Vec<(String, u32, u32)> = found
            .iter()
            .map(|found| {
                (
                    name_of(found),
                    found.range.start.line,
                    found.range.start.character,
                )
            })
            .collect();
        places.sort();
        assert_eq!(
            places,
            [("lib.nvs".to_owned(), 2, 6), ("lib.nvs".to_owned(), 3, 6),],
            "`implementation` did not answer both of the interface's implementors"
        );
    });
}

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// Which of this crate's own source files hold `needle` in code — a line that
/// is a comment does not count, because a doc comment saying which request is
/// *not* answered would otherwise read as one answering it.
fn code_hits(needle: &str) -> Vec<(String, usize)> {
    let mut sources = Vec::new();
    rust_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut sources,
    );
    assert!(sources.len() > 5, "the crate's source was not found");

    let mut found = Vec::new();
    for path in sources {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let hits = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| line.contains(needle))
            .count();
        if hits > 0 {
            found.push((
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                hits,
            ));
        }
    }
    found
}

/// Go-to-declaration is not answered, and it is excluded with a reason rather
/// than by omission.
///
/// LSP separates `textDocument/declaration` from `textDocument/definition` for
/// languages that have two sites — a C header and its `.c` file. Novis has one:
/// a class, a method and a property are declared where their body is, so the
/// two requests would walk to the same span and a client offered both would
/// give a reader two menu entries for one jump. The exclusion is therefore two
/// facts, and this holds both — the capability is not declared, so a conforming
/// client never asks, and nothing in the crate names the request, so a client
/// that asks anyway reaches `answer`'s refusal arm like any other method
/// outside `rule:ide/the-request-set-is-closed`'s list.
#[test]
fn declaration_is_not_answered_because_it_would_answer_identically() {
    let declared = nvs_lsp::declared_capabilities(PositionEncodingKind::UTF8);
    assert!(
        declared.get("declarationProvider").is_none(),
        "`declarationProvider` is declared, so a client will ask: {declared}"
    );

    // Both spellings, because either one appearing is something answering or
    // preparing to: `textDocument/declaration` is the method string's and
    // `GotoDeclaration` is `lsp_types`'. Neither needle is the bare word
    // `declaration`, which this crate's index and its go-to-definition walk are
    // full of for an unrelated reason.
    let named: Vec<(String, usize)> = ["textDocument/declaration", "GotoDeclaration"]
        .into_iter()
        .flat_map(code_hits)
        .collect();
    assert!(
        named.is_empty(),
        "the crate names go-to-declaration, which would answer what \
         `textDocument/definition` already answers: {named:?}"
    );
}
