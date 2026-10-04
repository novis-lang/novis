//! `initialize`, what a client reads out of it, and what it configured in it.
//!
//! The capability set and the negotiated encoding are both decided once, at the
//! handshake, and cached by the client for the life of the session. A mistake
//! in either is therefore invisible: a capability that is missing means the
//! editor never asks, and an encoding that is wrong means every column on every
//! non-ASCII line is off by a little. Neither produces an error anywhere, so
//! both are pinned here.
//!
//! The settings travel the same way and are invisible in the same way. A
//! spelling this server does not look for is a setting a developer wrote and
//! nothing acts on, with no error anywhere to say so, and
//! `rule:ide/contributions-are-frozen-and-only-ever-added` makes correcting one
//! later a break in somebody's `settings.json` rather than a fix. What
//! `nvs_lsp::Settings` reads out of `initializationOptions`, and what it falls
//! back to, is therefore pinned here beside them.
//!
//! The handshake is driven over `lsp_server::Connection::memory()` rather than
//! a subprocess. That is the same code path `run` takes — `serve` is the whole
//! of it, and only the descriptors differ — and it keeps this file from being a
//! test of process spawning.

use std::collections::BTreeSet;
use std::path::Path;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::{
    ClientCapabilities, GeneralClientCapabilities, InitializeParams, InitializeResult,
    PositionEncodingKind, WorkspaceFolder,
};
use nvs_lsp::{CheckScope, Client, PhpNames, Settings};

/// Run one full `initialize`/`initialized`/`shutdown`/`exit` exchange against a
/// server in this process, and hand back what it declared.
fn handshake(capabilities: ClientCapabilities) -> InitializeResult {
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));

    let params = InitializeParams {
        capabilities,
        ..InitializeParams::default()
    };
    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(1),
            "initialize".to_owned(),
            params,
        )))
        .expect("the server is still reading");

    let answer = match client.receiver.recv() {
        Ok(Message::Response(response)) => response,
        other => panic!("expected the `initialize` response, got {other:?}"),
    };
    client
        .sender
        .send(Message::Notification(Notification::new(
            "initialized".to_owned(),
            serde_json::json!({}),
        )))
        .expect("the server is still reading");

    // `shutdown` then `exit`, so the loop returns and the thread's `Result` can
    // be unwrapped — a server that failed on the wire must not be read as one
    // that answered.
    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(2),
            "shutdown".to_owned(),
            serde_json::json!(null),
        )))
        .expect("the server is still reading");
    let _shutdown_ack = client.receiver.recv();
    client
        .sender
        .send(Message::Notification(Notification::new(
            "exit".to_owned(),
            serde_json::json!(null),
        )))
        .expect("the server is still reading");

    serving
        .join()
        .expect("the server thread did not panic")
        .expect("the server served the handshake without a protocol error");

    let result = answer
        .response_result
        .expect("`initialize` was answered, not refused");
    serde_json::from_value(result).expect("the answer deserializes as an `InitializeResult`")
}

/// A client that offers `encodings` under `general.positionEncodings`, and
/// nothing else.
fn client_offering(encodings: &[PositionEncodingKind]) -> ClientCapabilities {
    ClientCapabilities {
        general: Some(GeneralClientCapabilities {
            position_encodings: Some(encodings.to_vec()),
            ..GeneralClientCapabilities::default()
        }),
        ..ClientCapabilities::default()
    }
}

// covers: tools:editor/nvs-lsp
#[test]
fn initialize_declares_exactly_the_capabilities_this_goal_ships() {
    // `rule:ide/the-request-set-is-closed` and
    // `rule:ide/five-features-are-one-reference-index`, read back off the wire
    // rather than off the source: what a client acts on is the JSON, and a
    // field that serializes under a different name than the one the
    // specification gives is a capability nothing ever uses.
    //
    // The absences are the load-bearing half. `rename` and `workspaceSymbol`
    // each look one line away from being free and are not: both need an edit
    // and a query the symbol index does not hold. ADR 0099 § 3 is where the
    // test a request outside M4B's nine has to pass is written down, and this
    // assertion is what makes failing it visible.
    let declared = nvs_lsp::declared_capabilities(PositionEncodingKind::UTF8);
    let keys: BTreeSet<&str> = declared
        .as_object()
        .expect("`ServerCapabilities` is a JSON object")
        .keys()
        .map(String::as_str)
        .collect();

    let expected: BTreeSet<&str> = [
        // `publishDiagnostics` has no provider field of its own: it is a
        // notification, and what gates it is a document being open.
        "textDocumentSync",
        "hoverProvider",
        "definitionProvider",
        "completionProvider",
        // The three that answer out of what hover already reads. There is no
        // `declarationProvider` beside them, and `tests/navigation.rs` holds
        // that as a claim rather than leaving it to this list's silence.
        "signatureHelpProvider",
        "typeDefinitionProvider",
        "implementationProvider",
        "documentSymbolProvider",
        "selectionRangeProvider",
        "foldingRangeProvider",
        "documentLinkProvider",
        "semanticTokensProvider",
        "codeActionProvider",
        // The two readers of the one workspace symbol index that answer a
        // cursor. They are here rather than in M4B's nine because they are
        // `rule:ide/five-features-are-one-reference-index`'s, and the index
        // they read is what they were waiting for.
        "referencesProvider",
        "documentHighlightProvider",
        // The same rule's third reader. It answers a document rather than a
        // cursor, and it is the one a client can turn off — which it does by
        // the setting and not by this field, since a capability is cached at
        // `initialize` and a setting arrives in the same message.
        "codeLensProvider",
        // The fourth, and the one `lsp_types` has no struct field for —
        // `nvs_lsp::declared_capabilities` is what puts it in the object, which
        // is why this test reads the object and not the struct.
        "typeHierarchyProvider",
        // The one request ADR 0099 § 3 deferred by name and this goal reverses,
        // bounded to the two idioms `nvs_lsp::hints` answers: everything else
        // that record held back is still held back.
        "inlayHintProvider",
        // Not a request. It is the answer to "in what units are the positions
        // in every one of the above", and LSP 3.17 puts it here.
        "positionEncoding",
    ]
    .into_iter()
    .collect();

    assert_eq!(
        keys,
        expected,
        "the declared capability set is not the closed list. Extra: {:?}. Missing: {:?}.",
        keys.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&keys).collect::<Vec<_>>()
    );
}

#[test]
fn the_server_declares_no_formatting_provider() {
    // The load-bearing half of
    // `rule:ide/a-template-region-gets-the-editors-services-and-formatter`, asked
    // of the server rather than of the client: a `.nvs` file has exactly one
    // formatter and it is `nvs fmt`, run by the extension over the whole file
    // (`rule:tooling/fmt-is-never-a-diagnostic`). A provider declared here
    // would be a second one — `editor.formatOnSave` would reach it inside a
    // template region, and `nvs fmt --check` would then fail for a reason that
    // is not "this file is laid out differently".
    //
    // Named on its own rather than left to the closed list above because the
    // list says what is absent by saying nothing, and an absence a rule turns
    // on has to fail with its own name on it. The three spellings are checked
    // together: on-type formatting is the one a reviewer forgets, since it is
    // the only one an editor never shows in a menu.
    let declared = nvs_lsp::declared_capabilities(PositionEncodingKind::UTF8);
    let object = declared
        .as_object()
        .expect("`ServerCapabilities` is a JSON object");

    for field in [
        "documentFormattingProvider",
        "documentRangeFormattingProvider",
        "documentOnTypeFormattingProvider",
    ] {
        assert!(
            !object.contains_key(field),
            "`{field}` is declared; a `.nvs` file's only formatter is `nvs fmt`"
        );
    }
}

#[test]
fn initialize_reports_the_binary_version() {
    // `rule:ide/the-extension-refuses-a-binary-it-does-not-understand`: the
    // client compares this against what it was built for and refuses to start
    // on a mismatch, so a server that omits it is one an old extension will
    // happily talk nonsense to.
    let info = handshake(ClientCapabilities::default())
        .server_info
        .expect("`initialize` reports a `serverInfo`");

    assert_eq!(info.name, "nvs lsp");
    assert_eq!(
        info.version.as_deref(),
        Some(env!("CARGO_PKG_VERSION")),
        "the reported version is the binary's own, not a string kept beside it"
    );
}

#[test]
fn the_negotiated_encoding_is_the_one_the_client_offered() {
    // Offered in the client's own preference order, with `utf-16` first: LSP
    // 3.17 makes the choice the server's, and Novis takes `utf-8` whenever it
    // is on the list because a span here is already a byte offset.
    let both = client_offering(&[PositionEncodingKind::UTF16, PositionEncodingKind::UTF8]);
    assert_eq!(
        handshake(both).capabilities.position_encoding,
        Some(PositionEncodingKind::UTF8)
    );

    // A client that says nothing at all is a 3.16 client, and the protocol's
    // default for those is `utf-16`. Assuming `utf-8` there would shift every
    // column past the first non-ASCII character on a line, silently.
    assert_eq!(
        handshake(ClientCapabilities::default())
            .capabilities
            .position_encoding,
        Some(PositionEncodingKind::UTF16)
    );
}

#[test]
fn a_client_offering_only_utf16_gets_utf16() {
    // The half that is easy to get wrong by preferring what the server wants:
    // an encoding the client did not offer is one it cannot decode, so
    // "prefer `utf-8`" is a preference among what was offered and never a
    // choice made over the client's head.
    let only_utf16 = client_offering(&[PositionEncodingKind::UTF16]);
    assert_eq!(
        handshake(only_utf16).capabilities.position_encoding,
        Some(PositionEncodingKind::UTF16)
    );

    // And a client offering something neither side can use falls back the same
    // way rather than echoing it.
    let exotic = client_offering(&[PositionEncodingKind::UTF32]);
    assert_eq!(
        handshake(exotic).capabilities.position_encoding,
        Some(PositionEncodingKind::UTF16)
    );
}

/// A client that sent `options` as its `initializationOptions` and nothing
/// else.
///
/// The JSON is written the way the extension holds it — the `nvs` section as a
/// nested object, which is what `workspace.getConfiguration("nvs")` is — so a
/// case here reads as the `settings.json` it comes from.
fn configured(options: serde_json::Value) -> InitializeParams {
    InitializeParams {
        initialization_options: Some(options),
        ..InitializeParams::default()
    }
}

/// A client that opened `path` as its one workspace folder.
fn rooted_at(path: &Path) -> InitializeParams {
    InitializeParams {
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: nvs_lsp::uri_of(path).expect("a temp path is UTF-8"),
            name: "workspace".to_owned(),
        }]),
        ..InitializeParams::default()
    }
}

#[test]
fn every_setting_is_read_off_initialization_options() {
    // The spellings are `rule:ide/contributions-are-frozen-and-only-ever-added`'s
    // roster verbatim, which is the whole of what this pins: a server reading
    // `checkScope` or `nvs.check.scope` would find nothing in what the editor
    // sends and report no error about it.
    let settings = Settings::from_initialize(&configured(serde_json::json!({
        "check": { "scope": "workspace" },
        "codeLens": { "enable": false },
        "completion": { "phpNames": "resolved" },
    })));

    assert_eq!(settings.scope, CheckScope::Workspace);
    assert!(
        !settings.code_lens,
        "`nvs.codeLens.enable` was turned off and the server did not notice"
    );
    assert_eq!(settings.php_names, PhpNames::Resolved);
    assert_eq!(
        Settings::from_initialize(&configured(serde_json::json!({
            "completion": { "phpNames": "off" },
        })))
        .php_names,
        PhpNames::Off
    );
}

#[test]
fn a_command_is_sent_only_to_a_client_that_named_it() {
    // `rule:ide/an-accepted-type-writes-what-follows-it`: the two commands are
    // the editor's own, so a client says which of them it runs.
    let capabilities: lsp_types::ClientCapabilities = serde_json::from_value(serde_json::json!({
        "textDocument": { "completion": { "completionItem": { "snippetSupport": true } } },
        "experimental": { "commands": [Client::SUGGEST] },
    }))
    .expect("the capabilities are the protocol's own shape");
    let client = Settings::from_initialize(&InitializeParams {
        capabilities,
        ..InitializeParams::default()
    })
    .client;
    assert_eq!(
        client,
        Client {
            snippets: true,
            suggest: true,
            parameter_hints: false,
        }
    );
    assert_eq!(
        Settings::from_initialize(&InitializeParams::default()).client,
        Client::default(),
        "a client that said nothing gets plain text and no command"
    );
}

#[test]
fn a_client_that_configured_nothing_gets_the_roster_defaults() {
    // `rule:ide/check-scope-defaults-to-the-workspace`: an unconfigured client
    // and a client that sent `"workspace"` are the same server.
    assert_eq!(
        Settings::from_initialize(&InitializeParams::default()),
        Settings::default()
    );
    assert_eq!(Settings::default().scope, CheckScope::Workspace);
    assert!(
        Settings::default().code_lens,
        "a lens is offered unless it was turned off"
    );
    assert_eq!(
        Settings::default().php_names,
        PhpNames::All,
        "every PHP built-in is a candidate until a developer says otherwise"
    );
    assert_eq!(Settings::default().root, None);

    assert_eq!(
        Settings::from_initialize(&configured(serde_json::json!({
            "check": { "scope": "open" },
        })))
        .scope,
        CheckScope::Open
    );
}

#[test]
fn a_value_neither_setting_can_hold_leaves_it_at_its_default() {
    // A typo in a `settings.json` must not be the reason a developer has no
    // language server: there is no place to report one from at `initialize`
    // time, so the only two outcomes are this and a session that never starts.
    let nonsense = Settings::from_initialize(&configured(serde_json::json!({
        "check": { "scope": "everything" },
        "codeLens": { "enable": "yes" },
        "completion": { "phpNames": "some" },
    })));

    assert_eq!(nonsense, Settings::default());

    // And a section that is not an object at all, which is what an editor sends
    // for a setting whose schema a user's own JSON disagrees with.
    assert_eq!(
        Settings::from_initialize(&configured(serde_json::json!({ "check": 7 }))),
        Settings::default()
    );
}

#[test]
fn the_workspace_root_is_the_folder_the_client_named() {
    // Without this the `Workspace` scope above has no tree to walk and quietly
    // degrades to the open documents, which reads as a setting that does
    // nothing rather than as a client that named no folder.
    let dir = nvs_repo::scratch("lsp-handshake-root");
    assert_eq!(
        Settings::from_initialize(&rooted_at(&dir)).root.as_deref(),
        Some(dir.path())
    );

    // A client too old for `workspaceFolders` names the same directory in the
    // deprecated field, and gets a workspace pass rather than silence.
    #[allow(deprecated)]
    let old = InitializeParams {
        root_uri: Some(nvs_lsp::uri_of(&dir).expect("a scratch path is UTF-8")),
        ..InitializeParams::default()
    };
    assert_eq!(
        Settings::from_initialize(&old).root.as_deref(),
        Some(dir.path())
    );
}
