//! The connection, the handshake, and the loop that reads it.
//!
//! One reader thread, one writer thread and this one — the three
//! `rule:ide/the-server-is-synchronous` names, and no fourth. A message is
//! taken off the channel, answered, and the next one is taken; there is no
//! executor to hand it to and nothing here returns a `Future`.
//!
//! **The dispatch is a refusal, and refusing is the correct answer.** A request
//! this server does not answer gets `MethodNotFound` rather than silence,
//! because a client waiting on a response that never comes is a client that
//! hangs. The set that gets a real answer grows as
//! `rule:ide/the-request-set-is-closed`'s list is implemented; the set that
//! gets a refusal is everything else, permanently.
//!
//! **`textDocument/publishDiagnostics` is the one thing sent unasked**, and
//! [`publish`] is the whole of it: a document-sync notification is applied to
//! the store, and every open document it made stale is analysed and published
//! for. Which documents those are is [`Documents::to_republish`]'s answer
//! rather than this module's, and what goes into one is
//! [`crate::diagnostics::for_document`]'s.

use std::error::Error;
use std::path::PathBuf;

use lsp_server::{Connection, ErrorCode, Message, Notification, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, PublishDiagnosticsParams, Uri,
};
use nvs_diagnostics::PositionEncoding;

use crate::capabilities::initialize_result;
use crate::diagnostics::{Phases, for_document};
use crate::document::{Documents, analyse, path_of};
use crate::position::encoding_of;

/// What a failure on the wire is reported as.
///
/// A protocol error, a malformed `initialize`, and a writer thread that died
/// are three different types and one outcome — this server cannot continue —
/// so the caller gets the message rather than a taxonomy it would only print.
pub type ServerError = Box<dyn Error + Sync + Send>;

/// Serve LSP on this process's standard input and output until the client
/// says `exit`.
///
/// The two io threads own the descriptors from here on, which is the whole of
/// `rule:ide/stdout-belongs-to-the-protocol`'s mechanism: nothing else in the
/// process may write a byte to stdout while this is running, and a `println!`
/// anywhere in the graph desynchronises the framing rather than printing
/// something a user sees.
///
/// # Errors
///
/// Returns the first protocol or io failure. Both are terminal: a framing
/// error means the stream's position is no longer known.
pub fn run() -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    serve(&connection)?;
    // After `exit`, and only after: joining before the loop returns would
    // block on a reader that is still holding stdin open.
    io_threads.join()?;
    Ok(())
}

/// Serve LSP over `connection`, whatever it is carrying.
///
/// Split from [`run`] so the handshake can be driven over
/// `lsp_server::Connection::memory()`, which is how the acceptance tests read
/// back what `initialize` declared without a subprocess or a real stdio pair.
///
/// # Errors
///
/// As [`run`].
pub fn serve(connection: &Connection) -> Result<(), ServerError> {
    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params)?;
    let declared = initialize_result(&params);
    // Settled once, and read back off what the client is about to be told
    // rather than negotiated a second time here: every position sent
    // afterwards is counted in it, and a server counting in an encoding it did
    // not declare is wrong by a little on every non-ASCII line and reports no
    // error anywhere.
    let encoding = declared
        .capabilities
        .position_encoding
        .as_ref()
        .map_or(PositionEncoding::Utf16, encoding_of);
    connection.initialize_finish(id, serde_json::to_value(declared)?)?;

    // This thread owns the store, which is the whole of the analysis half of
    // `rule:ide/the-server-is-synchronous`: one place holds what the client has
    // open, and it is the place that reads the channel.
    let mut documents = Documents::new();

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                // `shutdown` is answered and then waited on: the client sends
                // `exit` next, and `handle_shutdown` consumes it. Returning
                // here is what ends the loop.
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                let refusal = Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("`{}` is not answered by this server", request.method),
                );
                connection.sender.send(refusal.into())?;
            }
            Message::Notification(notification) => {
                if let Some(changed) = apply(&mut documents, notification) {
                    publish(connection, &mut documents, encoding, &changed)?;
                }
            }
            // A response is an answer to a request this server has not yet
            // learned to send.
            Message::Response(_) => {}
        }
    }

    Ok(())
}

/// What a document-sync notification changed, and so what has to be published
/// next.
struct Changed {
    /// The file whose text is different now, when the document names one.
    ///
    /// A path rather than the URI the notification carried, because an open
    /// document is republished when its own last analysis *read* this file, and
    /// a graph is walked by path.
    path: Option<PathBuf>,
    /// The document that is no longer open, when this was a `didClose`.
    closed: Option<Uri>,
}

/// Applies a document-sync notification to the store, and says what it changed.
///
/// `None` when nothing did: a notification this server does not act on is
/// dropped by definition — the protocol gives it no reply — and a payload that
/// will not deserialize is dropped on the same terms, since there is no error
/// response to send and the alternative to ignoring it is dying on a message
/// the client thinks it has already delivered. An edit to a document nobody
/// opened is the third case, and publishing for it would be publishing for a
/// file that is not open.
fn apply(documents: &mut Documents, notification: Notification) -> Option<Changed> {
    let Notification { method, params } = notification;

    match method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = serde_json::from_value::<DidOpenTextDocumentParams>(params).ok()?;
            let opened = params.text_document;
            let path = path_of(&opened.uri);
            documents.open(opened.uri, opened.version, opened.text);
            Some(Changed { path, closed: None })
        }
        DidChangeTextDocument::METHOD => {
            let params = serde_json::from_value::<DidChangeTextDocumentParams>(params).ok()?;
            let document = params.text_document;
            // `TextDocumentSyncKind::FULL`, which `server_capabilities`
            // declares: an event carries the whole document and no range.
            // One that carries a range is a client contradicting the
            // negotiation, and taking its text as a whole document would
            // truncate the buffer to a single edit.
            let mut edited = false;
            for change in params.content_changes {
                if change.range.is_none() {
                    edited |= documents.change(&document.uri, document.version, change.text);
                }
            }
            if edited {
                Some(Changed {
                    path: path_of(&document.uri),
                    closed: None,
                })
            } else {
                None
            }
        }
        DidCloseTextDocument::METHOD => {
            let params = serde_json::from_value::<DidCloseTextDocumentParams>(params).ok()?;
            let uri = params.text_document.uri;
            let path = path_of(&uri);
            if documents.close(&uri) {
                Some(Changed {
                    path,
                    closed: Some(uri),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Publishes for every open document `changed` made stale, and for nothing
/// else.
///
/// The list is [`Documents::to_republish`]'s: the edited document itself, and
/// every open document whose last analysis read the edited file. A document
/// nobody opened is not on it, which is where this stays a document server
/// rather than M10's workspace index
/// (`rule:ide/an-open-document-is-its-own-entry-point`).
///
/// Each one is analysed as its own entry point and sent its own file's
/// diagnostics, **including when there are none** — a client keeps what it was
/// last sent until it is told otherwise, so the empty list is the only thing
/// that clears a squiggle the edit fixed.
///
/// # Errors
///
/// Fails when the writer thread is gone, which is terminal for the same reason
/// a framing error is.
fn publish(
    connection: &Connection,
    documents: &mut Documents,
    encoding: PositionEncoding,
    changed: &Changed,
) -> Result<(), ServerError> {
    if let Some(closed) = &changed.closed {
        // A closed document is published for one last time, with nothing in
        // it. Not a contradiction of "for open documents only": this retracts
        // rather than reports, and leaving it out would leave a squiggle
        // outliving the buffer it was about.
        send(connection, closed, None, Vec::new())?;
    }

    let Some(path) = &changed.path else {
        return Ok(());
    };
    // Owned, because the walk below needs the store mutably to record each
    // graph. One URI per open document at most, and usually one.
    let stale: Vec<Uri> = documents.to_republish(path).into_iter().cloned().collect();

    for uri in stale {
        let Some(analysed) = analyse(documents, &uri) else {
            continue;
        };
        // Rebuilt from the walk that has just run, which is what makes the
        // *next* edit to a file this document requires reach this document.
        documents.record_graph(&uri, analysed.files());
        let diagnostics = for_document(&analysed, Phases::Gated, encoding);
        // Asked again, immediately before the send: `analyse_current` refuses
        // to start a walk for a version already superseded, and this is the
        // other end of the same claim, for one that was overtaken while it ran
        // (`rule:ide/the-server-is-synchronous`).
        if documents.is_current(&uri, analysed.version) {
            send(connection, &uri, Some(analysed.version), diagnostics)?;
        }
    }

    Ok(())
}

/// Sends one `textDocument/publishDiagnostics`.
///
/// `version` is the one the analysis read, so a client can tell an answer about
/// the buffer on screen from one about the keystroke before it. `None` is a
/// retraction, which is about no version at all.
///
/// # Errors
///
/// As [`publish`].
fn send(
    connection: &Connection,
    uri: &Uri,
    version: Option<i32>,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<(), ServerError> {
    let params = PublishDiagnosticsParams::new(uri.clone(), diagnostics, version);
    let notification = Notification::new(PublishDiagnostics::METHOD.to_owned(), params);
    connection.sender.send(notification.into())?;
    Ok(())
}
