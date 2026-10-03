//! What a reference list, an occurrence highlight and a lens answer, off a
//! running server.
//!
//! `rule:ide/five-features-are-one-reference-index`'s readers that answer a
//! request, driven over `lsp_server::Connection::memory()` the way `publish.rs`
//! is: the index these answer from is the one `serve` holds, so a test that
//! built its own would prove nothing about whether the server has one at all.
//! The claims are that the answer crosses a file the client never opened, that
//! the declaration is sent only when it is asked for, that a highlight is the
//! same query narrowed to the open document, that a lens counts what the whole
//! index holds and says nothing at all when `nvs.codeLens.enable` is off, that
//! an edit is in the next answer, and that `nvs/checkWorkspace` reaches under
//! `"open"` what only `"workspace"` reaches otherwise.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument, Notification as _};
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, Location,
    TextDocumentContentChangeEvent, TextDocumentItem, TypeHierarchyItem, Uri,
    VersionedTextDocumentIdentifier, WorkspaceFolder,
};

/// The class, declaring `$count` on line 2 and using it on line 4.
const LIB: &str = "<?nvs\nclass Counter {\n    public int $count = 0;\n    \
                   public function bump(): int {\n        return $this->count;\n    }\n}\n";

/// The open document, using `$count` on line 4.
const MAIN: &str = "<?nvs\nrequire 'lib.nvs';\nvar $c = new Counter();\n\
                    echo $c->bump();\necho $c->count;\n";

/// A third file, using `$count` on line 3 and required by nothing.
///
/// It resolves `Counter` through `lib.nvs` exactly as [`MAIN`] does, so the
/// only thing keeping its use out of a reference list is that no open
/// document's `require` graph reaches it — which is precisely what
/// `nvs.check.scope` decides.
const OTHER: &str = "<?nvs\nrequire 'lib.nvs';\nvar $c = new Counter();\necho $c->count;\n";

/// An interface and its two implementors, for the hierarchy readers.
///
/// One file and three declarations, because what is under test is the edge the
/// index records and not the graph the front end resolved it through — and a
/// shape a reader most needs shown rather than reconstructed is exactly this
/// one (`rule:classes/interface-default-methods`).
const SHAPES: &str = "<?nvs\ninterface Shape { public function area(): int; }\n\
                      class Square implements Shape { public function area(): int { return 1; } }\n\
                      class Circle implements Shape { public function area(): int { return 2; } }\n";

/// The cursor: inside `Square` in its own declaration, which is line 2 of
/// [`SHAPES`].
const ON_SQUARE: (u32, u32) = (2, 8);

/// [`MAIN`] with a second use of `$count`, on line 5.
const MAIN_AGAIN: &str = "<?nvs\nrequire 'lib.nvs';\nvar $c = new Counter();\n\
                          echo $c->bump();\necho $c->count;\necho $c->count;\n";

/// The cursor: inside `count` in `echo $c->count;`, which is line 4 of
/// [`MAIN`].
const CURSOR: (u32, u32) = (4, 11);

/// A directory of this run's own, removed when the test that made it ends.
///
/// The fixtures are on disk because a `require` resolves against the requiring
/// file's own directory before any source map is consulted — `publish.rs` and
/// `index.rs` both need the same thing for the same reason.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("nvs-refs-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory");
        fs::write(path.join("lib.nvs"), LIB).expect("a fixture file");
        fs::write(path.join("main.nvs"), MAIN).expect("a fixture file");
        Self { path }
    }

    fn uri(&self, name: &str) -> Uri {
        nvs_lsp::uri_of(&self.path.join(name)).expect("a temp path is UTF-8")
    }

    /// One more file beside the fixtures, for the test that needs a file no
    /// open document's graph reaches.
    fn write(&self, name: &str, text: &str) {
        fs::write(self.path.join(name), text).expect("a fixture file");
    }

    /// This directory as the one workspace folder a client named.
    fn folder(&self) -> WorkspaceFolder {
        WorkspaceFolder {
            uri: nvs_lsp::uri_of(&self.path).expect("a temp path is UTF-8"),
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
/// `initialize`, then shuts it down and asserts it served without a protocol
/// error.
fn served(exchange: impl FnOnce(&Connection)) {
    served_with(InitializeParams::default(), exchange);
}

/// The same, against a server handed `params` at the handshake.
///
/// That is how a test says what the client configured: `initialize` is the one
/// message a setting travels in (`nvs_lsp::settings`), so a scope is chosen
/// before the first document is opened or it is not chosen at all.
fn served_with(params: InitializeParams, exchange: impl FnOnce(&Connection)) {
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));

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
    // this suite never read is not a protocol error.
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

/// Sends one request and returns its result, skipping every `publishDiagnostics`
/// that arrives first.
///
/// Skipping rather than draining beforehand: a publish is sent for every edit
/// and this suite makes several, so a test that counted them would be a test
/// about `publish.rs`'s subject written in the wrong file.
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

/// `textDocument/didOpen`.
fn open(client: &Connection, uri: &Uri, text: &str) {
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "novis".to_owned(),
            version: 1,
            text: text.to_owned(),
        },
    };
    notify(
        client,
        Notification::new(DidOpenTextDocument::METHOD.to_owned(), params),
    );
}

/// `textDocument/didChange`, carrying the whole document as `FULL` sync means.
fn change(client: &Connection, uri: &Uri, version: i32, text: &str) {
    let params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_owned(),
        }],
    };
    notify(
        client,
        Notification::new(DidChangeTextDocument::METHOD.to_owned(), params),
    );
}

/// Asks `textDocument/references` at [`CURSOR`] in `uri`.
fn references(client: &Connection, id: i32, uri: &Uri, declaration: bool) -> Vec<(String, u32)> {
    let answer = ask(
        client,
        id,
        "textDocument/references",
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": CURSOR.0, "character": CURSOR.1 },
            "context": { "includeDeclaration": declaration },
        }),
    );
    let locations: Vec<Location> =
        serde_json::from_value(answer).expect("a reference list is an array of locations");
    locations
        .iter()
        .map(|found| (named(&found.uri), found.range.start.line))
        .collect()
}

/// Asks `textDocument/documentHighlight` at [`CURSOR`] in `uri`.
fn highlights(client: &Connection, id: i32, uri: &Uri) -> Vec<u32> {
    let answer = ask(
        client,
        id,
        "textDocument/documentHighlight",
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": CURSOR.0, "character": CURSOR.1 },
        }),
    );
    let found: Vec<lsp_types::DocumentHighlight> =
        serde_json::from_value(answer).expect("a highlight list is an array of highlights");
    found.iter().map(|hit| hit.range.start.line).collect()
}

/// Asks `textDocument/codeLens` for `uri`, as one line-and-title pair per lens.
fn lenses(client: &Connection, id: i32, uri: &Uri) -> Vec<(u32, String)> {
    let answer = ask(
        client,
        id,
        "textDocument/codeLens",
        serde_json::json!({ "textDocument": { "uri": uri } }),
    );
    let found: Vec<lsp_types::CodeLens> =
        serde_json::from_value(answer).expect("a lens list is an array of lenses");
    found
        .iter()
        .map(|lens| {
            let shown = lens.command.as_ref().expect("a lens carries its command");
            (lens.range.start.line, shown.title.clone())
        })
        .collect()
}

/// Asks `textDocument/prepareTypeHierarchy` at `at` in `uri`.
fn prepare(client: &Connection, id: i32, uri: &Uri, at: (u32, u32)) -> Vec<TypeHierarchyItem> {
    let answer = ask(
        client,
        id,
        "textDocument/prepareTypeHierarchy",
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": at.0, "character": at.1 },
        }),
    );
    serde_json::from_value(answer).expect("a prepared hierarchy is an array of items")
}

/// Asks one of the two `typeHierarchy/…` requests about `item`, as the names it
/// answers with.
fn related(client: &Connection, id: i32, method: &str, item: &TypeHierarchyItem) -> Vec<String> {
    let answer = ask(
        client,
        id,
        method,
        serde_json::json!({ "item": serde_json::to_value(item).expect("an item") }),
    );
    let found: Vec<TypeHierarchyItem> =
        serde_json::from_value(answer).expect("a hierarchy answer is an array of items");
    found.iter().map(|item| item.name.clone()).collect()
}

/// The file name a location names, which is what an assertion can be read in.
fn named(uri: &Uri) -> String {
    nvs_lsp::path_of(uri)
        .expect("a location names a file")
        .file_name()
        .expect("a location names a file")
        .to_string_lossy()
        .into_owned()
}

/// A reference list reaches a file the client never opened, and carries the
/// declaration only when it was asked for.
///
/// The whole point of the index having a home in `serve`: `lib.nvs` is open in
/// no editor tab, and its use of `$count` is still in the answer
/// (`rule:ide/five-features-are-one-reference-index`).
#[test]
fn references_cross_a_file_the_client_never_opened() {
    let dir = TempDir::new("cross");
    let main = dir.uri("main.nvs");

    served(|client| {
        open(client, &main, MAIN);

        assert_eq!(
            references(client, 2, &main, false),
            vec![("lib.nvs".to_owned(), 4), ("main.nvs".to_owned(), 4)],
            "the use inside `Counter::bump` is in a file nobody opened"
        );
        assert_eq!(
            references(client, 3, &main, true),
            vec![
                ("lib.nvs".to_owned(), 2),
                ("lib.nvs".to_owned(), 4),
                ("main.nvs".to_owned(), 4),
            ],
            "`includeDeclaration` adds the property's own declaration and \
             nothing else"
        );
    });
}

/// `nvs.check.scope` is what decides whether a file nothing requires is in the
/// index at all.
///
/// The two halves are one fixture and one difference: the same directory, the
/// same open document, the same cursor, the same workspace folder, and a
/// handshake that named one value of
/// `rule:ide/check-scope-defaults-to-the-workspace` or the other. Under `"open"`
/// `other.nvs` is not there — nothing opened it and nothing requires it — and
/// under `"workspace"` it is, which is the setting reaching
/// `SymbolIndex::build` and nothing else.
#[test]
fn the_configured_scope_decides_whether_an_unrequired_file_is_indexed() {
    let dir = TempDir::new("scope");
    dir.write("other.nvs", OTHER);
    let main = dir.uri("main.nvs");

    let open_scope = |client: &Connection| {
        open(client, &main, MAIN);
        assert_eq!(
            references(client, 2, &main, false),
            vec![("lib.nvs".to_owned(), 4), ("main.nvs".to_owned(), 4)],
            "`other.nvs` is under the workspace root and in no open document's \
             graph, so open scope must not reach it"
        );
    };
    let narrowed = InitializeParams {
        workspace_folders: Some(vec![dir.folder()]),
        initialization_options: Some(serde_json::json!({
            "check": { "scope": "open" },
        })),
        ..InitializeParams::default()
    };
    served_with(narrowed, open_scope);

    let workspace = InitializeParams {
        workspace_folders: Some(vec![dir.folder()]),
        initialization_options: Some(serde_json::json!({
            "check": { "scope": "workspace" },
        })),
        ..InitializeParams::default()
    };
    served_with(workspace, |client| {
        open(client, &main, MAIN);
        assert_eq!(
            references(client, 2, &main, false),
            vec![
                ("lib.nvs".to_owned(), 4),
                ("main.nvs".to_owned(), 4),
                ("other.nvs".to_owned(), 3),
            ],
            "at workspace scope the index holds every `.nvs` file under the \
             root the client named"
        );
    });
}

/// `nvs/checkWorkspace` is one workspace pass under `"open"`, and it leaves the
/// setting as it was.
///
/// The same fixture as the test above. Before the pass `other.nvs` is not in
/// the reference list. The pass answers how many files the index now holds —
/// the three under the root — and after it `other.nvs` is in the list, and it
/// stays there after an edit to the open document.
#[test]
fn a_workspace_pass_on_demand_indexes_every_file_under_the_root() {
    let dir = TempDir::new("check-workspace");
    dir.write("other.nvs", OTHER);
    let main = dir.uri("main.nvs");

    let narrowed = InitializeParams {
        workspace_folders: Some(vec![dir.folder()]),
        initialization_options: Some(serde_json::json!({
            "check": { "scope": "open" },
        })),
        ..InitializeParams::default()
    };
    served_with(narrowed, |client| {
        open(client, &main, MAIN);
        assert_eq!(
            references(client, 2, &main, false),
            vec![("lib.nvs".to_owned(), 4), ("main.nvs".to_owned(), 4)],
            "before the pass, open scope does not reach `other.nvs`"
        );

        let indexed = ask(client, 3, "nvs/checkWorkspace", serde_json::json!(null));
        assert_eq!(
            indexed,
            serde_json::json!(3),
            "the pass answers how many files the index holds"
        );
        let whole = vec![
            ("lib.nvs".to_owned(), 4),
            ("main.nvs".to_owned(), 4),
            ("other.nvs".to_owned(), 3),
        ];
        assert_eq!(references(client, 4, &main, false), whole);

        change(client, &main, 2, MAIN);
        assert_eq!(
            references(client, 5, &main, false),
            whole,
            "an edit refreshes what it reaches and keeps what the pass added"
        );
    });
}

/// An occurrence highlight is the reference query narrowed to the open file.
#[test]
fn a_highlight_is_the_reference_query_filtered_to_the_open_document() {
    let dir = TempDir::new("highlight");
    let main = dir.uri("main.nvs");

    served(|client| {
        open(client, &main, MAIN);

        assert_eq!(
            highlights(client, 2, &main),
            vec![4],
            "the use in `lib.nvs` belongs to a reference list and never to a \
             highlight of this document"
        );
    });
}

/// A lens counts every use the index holds, and not the uses in the file the
/// lens is shown in.
///
/// The declarations are all in `lib.nvs` and two of the three uses counted are
/// written in `main.nvs`, which is
/// `references_cross_a_file_the_client_never_opened`'s claim read from the
/// other end: one index, and a lens is a query against it rather than a walk of
/// the document it appears in.
#[test]
fn a_lens_counts_every_use_the_index_holds_of_a_declaration() {
    let dir = TempDir::new("lens");
    let lib = dir.uri("lib.nvs");
    let main = dir.uri("main.nvs");

    served(|client| {
        open(client, &main, MAIN);
        open(client, &lib, LIB);

        assert_eq!(
            lenses(client, 2, &lib),
            vec![
                (1, "1 reference".to_owned()),
                (2, "2 references".to_owned()),
                (3, "1 reference".to_owned()),
            ],
            "`Counter` is constructed once, `$count` is read twice — once in \
             each file — and `bump` is called once"
        );
    });
}

/// `nvs.codeLens.enable` turned off is no answer rather than an empty one.
///
/// The index is built and the declarations are in it either way: the setting
/// decides whether a lens is offered, not what this server knows
/// (`rule:ide/contributions-are-frozen-and-only-ever-added` freezes the name).
#[test]
fn a_lens_is_not_offered_when_the_client_turned_it_off() {
    let dir = TempDir::new("lens-off");
    let lib = dir.uri("lib.nvs");

    let off = InitializeParams {
        initialization_options: Some(serde_json::json!({
            "codeLens": { "enable": false },
        })),
        ..InitializeParams::default()
    };
    served_with(off, |client| {
        open(client, &lib, LIB);

        assert_eq!(
            ask(
                client,
                2,
                "textDocument/codeLens",
                serde_json::json!({ "textDocument": { "uri": lib } }),
            ),
            serde_json::Value::Null,
            "the setting is what decides the answer"
        );
    });
}

/// A type hierarchy is the index's own `extends`/`implements` edge, walked one
/// level per request.
///
/// The client's walk is the test's: prepare at a cursor, ask that item what is
/// above it, and ask *that* item what is below it — which is how a hierarchy
/// view expands, and the reason each answer is the direct edge rather than the
/// flattened chain (`rule:ide/five-features-are-one-reference-index`).
#[test]
fn a_type_hierarchy_walks_the_index_one_level_per_request() {
    let dir = TempDir::new("hierarchy");
    dir.write("shapes.nvs", SHAPES);
    let shapes = dir.uri("shapes.nvs");

    served(|client| {
        open(client, &shapes, SHAPES);

        let prepared = prepare(client, 2, &shapes, ON_SQUARE);
        assert_eq!(
            prepared
                .iter()
                .map(|item| item.name.clone())
                .collect::<Vec<_>>(),
            vec!["Square".to_owned()],
            "the cursor is in a declaration's own name, which is never a \
             resolved use and has to be found in the index instead"
        );

        let above = "typeHierarchy/supertypes";
        assert_eq!(
            related(client, 3, above, &prepared[0]),
            vec!["Shape".to_owned()]
        );

        let shape = prepare(client, 4, &shapes, (1, 12));
        assert_eq!(
            related(client, 5, "typeHierarchy/subtypes", &shape[0]),
            vec!["Square".to_owned(), "Circle".to_owned()],
            "the reverse edge is every declaration naming `Shape`, in source \
             order"
        );
        assert!(
            related(client, 6, above, &shape[0]).is_empty(),
            "`Shape` extends nothing, and an interface that does is the same \
             field read the same way"
        );
    });
}

/// An edit is in the next reference list, which is the refresh the server
/// applies where it applies the edit.
#[test]
fn an_edit_is_in_the_next_reference_list() {
    let dir = TempDir::new("edit");
    let main = dir.uri("main.nvs");

    served(|client| {
        open(client, &main, MAIN);
        assert_eq!(references(client, 2, &main, false).len(), 2);

        change(client, &main, 2, MAIN_AGAIN);

        assert_eq!(
            references(client, 3, &main, false),
            vec![
                ("lib.nvs".to_owned(), 4),
                ("main.nvs".to_owned(), 4),
                ("main.nvs".to_owned(), 5),
            ],
            "the second use was typed after the index was built"
        );
        assert_eq!(highlights(client, 4, &main), vec![4, 5]);
    });
}
