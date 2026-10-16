//! What a client is sent, unasked, as it opens, edits and closes documents.
//!
//! Driven over `lsp_server::Connection::memory()` the way `handshake.rs` is:
//! this is `serve`'s own loop with only the descriptors different, so what the
//! test reads off the channel is what an editor would read off the pipe.
//!
//! The claims here are the ones no unit test over `Documents` can reach —
//! that a notification is actually *sent*, that it is sent once per open
//! document the edit made stale, and that each carries that document's own
//! file and nothing else. The gate inside one is
//! `rule:ide/diagnostics-are-phase-gated`'s, pinned in `document.rs`.

use std::fs;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::{
    DiagnosticTag, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, NumberOrString, PublishDiagnosticsParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Uri,
    VersionedTextDocumentIdentifier, WorkspaceFolder,
};

/// A file that parses, resolves and type-checks with nothing to report.
const CLEAN: &str = "<?nvs\nclass Typed {}\n";

/// A class keeping one private property it uses and one it does not, so a run
/// of this file says both what dimming reports and what it leaves alone.
///
/// `$kept` is on line 2 and `$used` on line 3, counting the way the wire does.
const HOARD: &str = "<?nvs\nclass Hoard {\n    private int $kept = 0;\n    \
                     private int $used = 0;\n    public function total(): int {\n        \
                     return $this->used;\n    }\n}\n";

/// A directory of this run's own, removed when the test that made it ends.
///
/// The fixtures are on disk rather than buffers alone because a `require`
/// resolves against the requiring file's directory: an open buffer overlays the
/// file under it, and there has to be a file under it for the graph to name.
struct TempDir {
    path: nvs_repo::Scratch,
}

impl TempDir {
    fn new(name: &str) -> Self {
        Self {
            path: nvs_repo::scratch(&format!("lsp-publish-{name}")),
        }
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.path.join(name), text).expect("a fixture file");
    }

    fn uri(&self, name: &str) -> Uri {
        nvs_lsp::uri_of(&self.path.join(name)).expect("a temp path is UTF-8")
    }
}

/// Runs `exchange` against a server in this process that has already answered
/// `initialize`, then shuts it down and asserts it served without a protocol
/// error.
fn served(exchange: impl FnOnce(&Connection)) {
    served_with(InitializeParams::default(), exchange);
}

/// The same, against a server handed `params` at the handshake — which is the
/// one message a setting travels in (`nvs_lsp::settings`).
fn served_with(params: InitializeParams, exchange: impl FnOnce(&Connection)) {
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));

    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(1),
            "initialize".to_owned(),
            params,
        )))
        .expect("the server is still reading");
    match client.receiver.recv() {
        Ok(Message::Response(_)) => {}
        other => panic!("expected the `initialize` response, got {other:?}"),
    }
    notify(
        &client,
        Notification::new("initialized".to_owned(), serde_json::json!({})),
    );

    exchange(&client);

    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(2),
            "shutdown".to_owned(),
            serde_json::json!(null),
        )))
        .expect("the server is still reading");
    // Anything still queued ahead of the acknowledgement is drained: a publish
    // the exchange did not read is not a protocol error.
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

/// `textDocument/didOpen`.
fn open(client: &Connection, uri: &Uri, version: i32, text: &str) {
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "novis".to_owned(),
            version,
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

/// `textDocument/didClose`.
fn close(client: &Connection, uri: &Uri) {
    let params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    };
    notify(
        client,
        Notification::new(DidCloseTextDocument::METHOD.to_owned(), params),
    );
}

/// The next `textDocument/publishDiagnostics` the server sends.
///
/// Waited for with a bound rather than forever, so a server that publishes
/// nothing fails this suite instead of hanging it.
fn published(client: &Connection) -> PublishDiagnosticsParams {
    match client.receiver.recv_timeout(Duration::from_secs(30)) {
        Ok(Message::Notification(notification))
            if notification.method == PublishDiagnostics::METHOD =>
        {
            serde_json::from_value(notification.params)
                .expect("the params deserialize as a `PublishDiagnosticsParams`")
        }
        other => panic!("expected a `textDocument/publishDiagnostics`, got {other:?}"),
    }
}

/// The codes in one publish, in the order the client will show them.
fn codes(params: &PublishDiagnosticsParams) -> Vec<String> {
    params
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) => Some(code.clone()),
            _ => None,
        })
        .collect()
}

/// The messages of the diagnostics in one publish that a client will render as
/// dimming, in the order it will render them.
///
/// Keyed on the tag and not on the severity: `Unnecessary` is what fades a
/// name, and a hint carrying no tag is an ordinary hint.
fn dimmed(params: &PublishDiagnosticsParams) -> Vec<String> {
    params
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .tags
                .as_ref()
                .is_some_and(|tags| tags.contains(&DiagnosticTag::UNNECESSARY))
        })
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// A client that set `nvs.check.scope` to the open documents alone.
fn open_scope() -> InitializeParams {
    InitializeParams {
        initialization_options: Some(serde_json::json!({
            "check": { "scope": "open" },
        })),
        ..InitializeParams::default()
    }
}

/// That directory as the one workspace folder a client named, with
/// `nvs.check.scope` set to walk it.
fn workspace_scope(dir: &TempDir) -> InitializeParams {
    InitializeParams {
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: nvs_lsp::uri_of(&dir.path).expect("a temp path is UTF-8"),
            name: "workspace".to_owned(),
        }]),
        initialization_options: Some(serde_json::json!({
            "check": { "scope": "workspace" },
        })),
        ..InitializeParams::default()
    }
}

/// `nvs.lsp.debounce` as a window nothing in a test reaches the end of, so what
/// runs a waiting analysis is the message behind it and never a clock.
fn never_elapses() -> InitializeParams {
    InitializeParams {
        initialization_options: Some(serde_json::json!({
            "lsp": { "debounce": 600_000 },
        })),
        ..InitializeParams::default()
    }
}

/// Whether any of `codes` is one a phase that reads the tree reported —
/// resolution's, and the type check's across all three bands it occupies.
fn reads_the_tree(codes: &[String]) -> bool {
    codes.iter().any(|code| {
        ["E03", "E04", "E07", "E08"]
            .iter()
            .any(|band| code.starts_with(band))
    })
}

/// Unused-member dimming, which open scope does not produce and workspace
/// scope, the default, produces exactly once.
///
/// Two halves of one rule. `rule:ide/five-features-are-one-reference-index`'s
/// fifth reader is only correct at workspace scope — a private member with no
/// occurrence in an index that spans one buffer's graph is a member this server
/// has not looked hard enough for — so
/// `rule:ide/check-scope-defaults-to-the-workspace` has it silent at open
/// scope rather than wrong. Silent is asserted as the absence of a tag, not as
/// a different message.
#[test]
fn unused_member_dimming_is_silent_at_open_scope_and_correct_at_workspace_scope() {
    let dir = TempDir::new("dimming");
    dir.write("hoard.nvs", HOARD);
    let uri = dir.uri("hoard.nvs");

    served_with(open_scope(), |client| {
        open(client, &uri, 1, HOARD);
        assert!(
            dimmed(&published(client)).is_empty(),
            "open scope indexes one graph, which is not enough to call anything \
             unused"
        );
    });

    served_with(workspace_scope(&dir), |client| {
        open(client, &uri, 1, HOARD);
        let params = published(client);

        assert_eq!(
            dimmed(&params),
            vec![
                "a property `Hoard::$kept` is private and nothing in this \
                 workspace uses it"
            ],
            "`$used` is read by `total` and `total` is public, so exactly one \
             declaration here is unreachable"
        );

        // On the name itself, which is what an editor fades: a whole-declaration
        // range would grey out the type and the initializer with it.
        let tagged = params
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.tags.is_some())
            .expect("the dimming diagnostic");
        assert_eq!(tagged.range.start.line, 2);
        assert_eq!(
            tagged.severity,
            Some(lsp_types::DiagnosticSeverity::HINT),
            "a hint, so it is faded rather than listed beside the errors"
        );
        assert!(
            tagged.code.is_none(),
            "no compiler phase reported this, so there is no code to look up"
        );
    });
}

/// The lifecycle of one document: opening it publishes for it, and fixing it
/// publishes the empty list that clears what was there.
#[test]
fn opening_a_document_publishes_for_it_and_fixing_it_clears() {
    let dir = TempDir::new("one");
    let broken = "<?nvs\nvar $broken = ;\n";
    dir.write("main.nvs", broken);

    served(|client| {
        open(client, &dir.uri("main.nvs"), 1, broken);
        let opened = published(client);
        assert_eq!(opened.uri, dir.uri("main.nvs"));
        assert_eq!(
            opened.version,
            Some(1),
            "the version the analysis read is what says which buffer this is about"
        );

        let reported = codes(&opened);
        assert!(
            reported.iter().any(|code| code.starts_with("E01")),
            "the parse error was not published: {reported:?}"
        );
        assert!(
            !reads_the_tree(&reported),
            "the cascade under the broken parse reached the client: {reported:?}"
        );

        change(client, &dir.uri("main.nvs"), 2, CLEAN);
        let fixed = published(client);
        assert_eq!(fixed.uri, dir.uri("main.nvs"));
        assert_eq!(fixed.version, Some(2));
        assert!(
            fixed.diagnostics.is_empty(),
            "an edit that fixed the file published {:?} rather than the empty \
             list that clears it — a client keeps its last squiggles until it \
             is told otherwise",
            codes(&fixed)
        );
    });
}

/// Editing a file another open document requires republishes that document,
/// with its own diagnostics untouched and none of the edited file's.
#[test]
fn editing_a_required_file_publishes_for_the_requiring_document_too() {
    let dir = TempDir::new("graph");
    let clean_lib = "<?nvs\nclass Counter {\n    public function count(): int {\n        \
                     return 1;\n    }\n}\n";
    let broken_lib = "<?nvs\nclass Counter {\n    public function count(): int {\n        \
                      return \"not an int\";\n    }\n}\n";
    let main = "<?nvs\nrequire 'lib.nvs';\nvar $stray = $nope;\n";
    dir.write("lib.nvs", clean_lib);
    dir.write("main.nvs", main);

    served(|client| {
        // Opened one at a time, and each publishes only for itself: `lib.nvs`
        // is not in `main.nvs`'s graph yet because `main.nvs` is not open, and
        // `main.nvs` is in nobody's.
        open(client, &dir.uri("lib.nvs"), 1, clean_lib);
        let lib = published(client);
        assert_eq!(lib.uri, dir.uri("lib.nvs"));
        assert!(lib.diagnostics.is_empty(), "{:?}", codes(&lib));

        open(client, &dir.uri("main.nvs"), 1, main);
        let opened = published(client);
        assert_eq!(opened.uri, dir.uri("main.nvs"));
        let its_own = codes(&opened);
        assert!(
            its_own.iter().any(|code| code.starts_with("E03")),
            "main.nvs's own resolution error is the premise of this case and it \
             is not here: {its_own:?}"
        );

        // The edit to the required file, which is stale in two documents at
        // once: the one being typed in, and the one that read it.
        change(client, &dir.uri("lib.nvs"), 2, broken_lib);
        let after = [published(client), published(client)];
        let for_lib = after
            .iter()
            .find(|params| params.uri == dir.uri("lib.nvs"))
            .expect("the edited document is published for");
        let for_main = after
            .iter()
            .find(|params| params.uri == dir.uri("main.nvs"))
            .expect("the document that requires the edited file was left stale");

        let broke = codes(for_lib);
        assert!(
            reads_the_tree(&broke),
            "the type error the edit introduced was not published: {broke:?}"
        );
        assert_eq!(
            codes(for_main),
            its_own,
            "main.nvs was published a diagnostic about a file it only requires: \
             one notification is about one URI, and lib.nvs has its own"
        );
    });
}

/// Closing a document retracts what it was published, and stops it being
/// analysed at all.
#[test]
fn a_closed_document_is_published_for_one_last_time_with_nothing_in_it() {
    let dir = TempDir::new("closed");
    let broken = "<?nvs\nvar $broken = ;\n";
    dir.write("main.nvs", broken);
    dir.write("other.nvs", CLEAN);

    served(|client| {
        open(client, &dir.uri("main.nvs"), 1, broken);
        assert!(!published(client).diagnostics.is_empty());

        close(client, &dir.uri("main.nvs"));
        let retracted = published(client);
        assert_eq!(retracted.uri, dir.uri("main.nvs"));
        assert!(
            retracted.diagnostics.is_empty(),
            "a squiggle outlived the buffer it was about: {:?}",
            codes(&retracted)
        );
        assert_eq!(
            retracted.version, None,
            "a retraction is about no version, because the buffer it would name is gone"
        );

        // Nothing follows it. Messages are answered in order, so the next
        // publish being about the document opened next is what says the closed
        // one was not analysed again.
        open(client, &dir.uri("other.nvs"), 1, CLEAN);
        assert_eq!(published(client).uri, dir.uri("other.nvs"));
    });
}

/// Two keystrokes in one buffer are one analysis, and the one that runs is the
/// second's.
///
/// `nvs.lsp.debounce` with a window no part of this case waits out, so the timing
/// is the message order and not a clock: the second `didChange` replaces the
/// first, and opening another document is what runs what is left. Were the first
/// edit analysed as well, the publish after the edits would name version 2 and
/// the one after that would still be `main.nvs`.
#[test]
fn a_keystroke_in_the_debounce_window_replaces_the_one_waiting() {
    let dir = TempDir::new("debounce");
    let broken = "<?nvs\nvar $broken = ;\n";
    dir.write("main.nvs", CLEAN);
    dir.write("other.nvs", CLEAN);

    served_with(never_elapses(), |client| {
        // An open is not a keystroke, so it is published for at once.
        open(client, &dir.uri("main.nvs"), 1, CLEAN);
        assert_eq!(published(client).version, Some(1));

        change(client, &dir.uri("main.nvs"), 2, broken);
        change(client, &dir.uri("main.nvs"), 3, CLEAN);
        open(client, &dir.uri("other.nvs"), 1, CLEAN);

        let flushed = published(client);
        assert_eq!(flushed.uri, dir.uri("main.nvs"));
        assert_eq!(
            flushed.version,
            Some(3),
            "the analysis that ran is the one the last keystroke asked for"
        );
        assert!(
            flushed.diagnostics.is_empty(),
            "the superseded keystroke's own diagnostics reached the client: {:?}",
            codes(&flushed)
        );
        assert_eq!(
            published(client).uri,
            dir.uri("other.nvs"),
            "version 2 was analysed too, so a second publish about main.nvs came \
             before the document opened last"
        );
    });
}
