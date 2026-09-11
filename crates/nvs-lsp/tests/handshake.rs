//! `initialize`, and the two things a client reads out of it.
//!
//! The capability set and the negotiated encoding are both decided once, at the
//! handshake, and cached by the client for the life of the session. A mistake
//! in either is therefore invisible: a capability that is missing means the
//! editor never asks, and an encoding that is wrong means every column on every
//! non-ASCII line is off by a little. Neither produces an error anywhere, so
//! both are pinned here.
//!
//! The handshake is driven over `lsp_server::Connection::memory()` rather than
//! a subprocess. That is the same code path `run` takes — `serve` is the whole
//! of it, and only the descriptors differ — and it keeps this file from being a
//! test of process spawning.

use std::collections::BTreeSet;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::{
    ClientCapabilities, GeneralClientCapabilities, InitializeParams, InitializeResult,
    PositionEncodingKind,
};

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
    // The absences are the load-bearing half. `rename`, `workspaceSymbol` and
    // inlay hints each look one line away from being free and are not: the
    // first two need an edit and a query the symbol index does not hold, and
    // the third is its own settings question. ADR 0099 § 3 is where the test a
    // request outside M4B's nine has to pass is written down, and this
    // assertion is what makes failing it visible.
    let declared = serde_json::to_value(nvs_lsp::server_capabilities(PositionEncodingKind::UTF8))
        .expect("the capabilities serialize");
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
