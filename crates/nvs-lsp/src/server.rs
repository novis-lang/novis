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

use std::error::Error;

use lsp_server::{Connection, ErrorCode, Message, Notification, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams,
};

use crate::capabilities::initialize_result;
use crate::document::Documents;

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
    connection.initialize_finish(id, serde_json::to_value(initialize_result(&params))?)?;

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
            Message::Notification(notification) => apply(&mut documents, notification),
            // A response is an answer to a request this server has not yet
            // learned to send.
            Message::Response(_) => {}
        }
    }

    Ok(())
}

/// Applies a document-sync notification to the store.
///
/// A notification this server does not act on is dropped by definition — the
/// protocol gives it no reply — and a payload that will not deserialize is
/// dropped on the same terms: there is no error response to send, so the
/// alternative to ignoring it is dying on a message the client thinks it has
/// already delivered.
///
/// **Nothing is published here.** The store is what a request is answered out
/// of, and the analysis that an edit makes stale — re-running it for the edited
/// document and for every open document whose graph names it — is the
/// diagnostics slice's, hanging off [`crate::analyse`] and
/// [`Documents::to_republish`].
fn apply(documents: &mut Documents, notification: Notification) {
    let Notification { method, params } = notification;

    match method.as_str() {
        DidOpenTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
                let opened = params.text_document;
                documents.open(opened.uri, opened.version, opened.text);
            }
        }
        DidChangeTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                let document = params.text_document;
                // `TextDocumentSyncKind::FULL`, which `server_capabilities`
                // declares: an event carries the whole document and no range.
                // One that carries a range is a client contradicting the
                // negotiation, and taking its text as a whole document would
                // truncate the buffer to a single edit.
                for change in params.content_changes {
                    if change.range.is_none() {
                        documents.change(&document.uri, document.version, change.text);
                    }
                }
            }
        }
        DidCloseTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                documents.close(&params.text_document.uri);
            }
        }
        _ => {}
    }
}
