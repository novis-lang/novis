//! The connection, the handshake, and the loop that reads it.
//!
//! One reader thread, one writer thread and this one — the three
//! `rule:ide/the-server-is-synchronous` names, and no fourth. A message is
//! taken off the channel, answered, and the next one is taken; there is no
//! executor to hand it to and nothing here returns a `Future`.
//!
//! **[`answer`] is the dispatch, and a refusal is one of its answers.** Every
//! request gets a response: an arm's when there is one, and `MethodNotFound`
//! otherwise, because a client waiting on a response that never comes is a
//! client that hangs. The set with an arm grows as
//! `rule:ide/the-request-set-is-closed`'s list is implemented; the set that
//! gets a refusal is everything outside that list, permanently.
//!
//! An arm answers out of the store it is handed and writes nothing back to it.
//! A request carries no version — the client is asking about whatever it last
//! sent — so `rule:ide/the-server-is-synchronous`'s version check has nothing
//! to compare against here, and the one thing that would be wrong is analysing
//! a document the client has since replaced, which cannot happen on a thread
//! that reads the next message only after this one is answered.
//!
//! **`textDocument/publishDiagnostics` is the one thing sent unasked**, and
//! [`publish`] is the whole of it: a document-sync notification is applied to
//! the store, and every open document it made stale is analysed and published
//! for. Which documents those are is [`Documents::to_republish`]'s answer
//! rather than this module's, and what goes into one is
//! [`crate::diagnostics::for_document`]'s.

use std::error::Error;
use std::path::PathBuf;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    DocumentLinkRequest, DocumentSymbolRequest, FoldingRangeRequest, GotoDefinition, Request as _,
    SelectionRangeRequest,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentLink, DocumentLinkParams, DocumentSymbolParams, DocumentSymbolResponse, FoldingRange,
    FoldingRangeParams, GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, Location,
    PublishDiagnosticsParams, Range, SelectionRange, SelectionRangeParams, Uri,
};
use nvs_diagnostics::PositionEncoding;

use crate::capabilities::initialize_result;
use crate::definition;
use crate::diagnostics::{Phases, for_document};
use crate::document::{Documents, analyse, path_of, uri_of};
use crate::folding;
use crate::links;
use crate::position::{encoding_of, offset_at};
use crate::selection;
use crate::symbols;

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
                let answered = answer(&documents, encoding, request);
                connection.sender.send(answered.into())?;
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

/// The response to one request.
///
/// The seam every request slice lands an arm in, and the one place a method
/// name is matched. Three outcomes and no fourth: an arm's answer, a refusal
/// for a method outside `rule:ide/the-request-set-is-closed`'s list, and
/// `InvalidParams` for one inside it whose payload will not deserialize —
/// which is a client sending something other than what the handshake agreed,
/// and is worth saying rather than answering as if the document were empty.
///
/// [`crate::suite`]'s own `answer` is the same seam for a `.lspt` case, and the
/// two share what does the work rather than the dispatch: a case names its
/// request in a `--REQUEST--` line and never a method string, and it holds no
/// `RequestId` to answer with.
fn answer(documents: &Documents, encoding: PositionEncoding, request: Request) -> Response {
    let Request { id, method, params } = request;

    match method.as_str() {
        DocumentSymbolRequest::METHOD => {
            match serde_json::from_value::<DocumentSymbolParams>(params) {
                Ok(params) => Response::new_ok(
                    id,
                    document_symbol(documents, encoding, &params.text_document.uri),
                ),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        FoldingRangeRequest::METHOD => match serde_json::from_value::<FoldingRangeParams>(params) {
            Ok(params) => Response::new_ok(
                id,
                folding_range(documents, encoding, &params.text_document.uri),
            ),
            Err(error) => unreadable(id, &method, &error),
        },
        GotoDefinition::METHOD => match serde_json::from_value::<GotoDefinitionParams>(params) {
            Ok(params) => Response::new_ok(id, definition(documents, encoding, &params)),
            Err(error) => unreadable(id, &method, &error),
        },
        SelectionRangeRequest::METHOD => {
            match serde_json::from_value::<SelectionRangeParams>(params) {
                Ok(params) => Response::new_ok(id, selection_range(documents, encoding, &params)),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        DocumentLinkRequest::METHOD => match serde_json::from_value::<DocumentLinkParams>(params) {
            Ok(params) => Response::new_ok(
                id,
                document_link(documents, encoding, &params.text_document.uri),
            ),
            Err(error) => unreadable(id, &method, &error),
        },
        _ => Response::new_err(
            id,
            ErrorCode::MethodNotFound as i32,
            format!("`{method}` is not answered by this server"),
        ),
    }
}

/// `InvalidParams`, for a request on the list whose payload will not read.
///
/// One place rather than one per arm: every arm can fail this way, and what a
/// person is shown should not depend on which request it was.
fn unreadable(id: RequestId, method: &str, error: &serde_json::Error) -> Response {
    Response::new_err(
        id,
        ErrorCode::InvalidParams as i32,
        format!("`{method}` carried parameters this server could not read: {error}"),
    )
}

/// `textDocument/documentSymbol` — the outline of one open document.
///
/// A document this server has nothing open for answers an empty outline rather
/// than an error: the client asked what is in a file, and "nothing this server
/// knows of" is an answer to that. The walk itself is [`symbols::for_document`],
/// which the `.lspt` suite calls too, so what an editor draws and what a case
/// freezes cannot drift apart.
fn document_symbol(
    documents: &Documents,
    encoding: PositionEncoding,
    uri: &Uri,
) -> DocumentSymbolResponse {
    let outline = analyse(documents, uri).map_or_else(Vec::new, |analysed| {
        symbols::for_document(&analysed, encoding)
    });
    // `Nested` and never `Flat`: the outline is a tree and `SymbolInformation`
    // would flatten a class's members out beside it.
    DocumentSymbolResponse::Nested(outline)
}

/// `textDocument/foldingRange` — where one open document collapses.
///
/// Empty for a document this server has nothing open for, on
/// [`document_symbol`]'s terms. The walk is [`folding::for_document`], which
/// the `.lspt` suite calls too, so what an editor folds and what a case freezes
/// cannot drift apart.
fn folding_range(
    documents: &Documents,
    encoding: PositionEncoding,
    uri: &Uri,
) -> Vec<FoldingRange> {
    analyse(documents, uri).map_or_else(Vec::new, |analysed| {
        folding::for_document(&analysed, encoding)
    })
}

/// `textDocument/definition` — where the name under the cursor is declared.
///
/// `null` for a document this server has nothing open for, for a cursor on
/// nothing it can follow, and for a declaration whose file this process cannot
/// spell as a URI — the three are one answer on the wire, and LSP has no shape
/// for telling them apart.
///
/// `Scalar` and not `Array`: a name resolves to one declaration or to none
/// (`rule:classes/names-resolve-case-sensitively`), so a list would be a shape
/// promising alternatives that cannot exist. The lookup is
/// [`definition::at`], which the `.lspt` suite calls too.
fn definition(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let position = params.text_document_position_params.position;
    let analysed = analyse(
        documents,
        &params.text_document_position_params.text_document.uri,
    )?;
    let offset = offset_at(analysed.map.file(analysed.entry), position, encoding);
    let declared = definition::at(&analysed, offset, encoding)?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri_of(&declared.path)?,
        range: declared.range,
    }))
}

/// `textDocument/selectionRange` — the expand-selection chain at each position.
///
/// **One chain per position, in the same order**, which is the protocol's own
/// requirement rather than a courtesy: the client pairs the two lists by index,
/// so a short list silently moves every answer after the gap onto the wrong
/// cursor. That is why the two ways of having nothing to say are different
/// here. A document this server has nothing open for answers `null` — the whole
/// question is unanswerable, and `null` is the shape LSP gives that. A position
/// inside no node answers the empty range there and no parent: expand selection
/// has nowhere to go from a caret between two statements, and saying so keeps
/// the pairing that dropping it would break.
///
/// The chain itself is [`selection::at`], which the `.lspt` suite calls too.
fn selection_range(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &SelectionRangeParams,
) -> Option<Vec<SelectionRange>> {
    let analysed = analyse(documents, &params.text_document.uri)?;
    let file = analysed.map.file(analysed.entry);
    Some(
        params
            .positions
            .iter()
            .map(|position| {
                let offset = offset_at(file, *position, encoding);
                selection::at(&analysed, offset, encoding).unwrap_or(SelectionRange {
                    range: Range {
                        start: *position,
                        end: *position,
                    },
                    parent: None,
                })
            })
            .collect(),
    )
}

/// `textDocument/documentLink` — every `require` in one open document.
///
/// Empty for a document this server has nothing open for, on
/// [`document_symbol`]'s terms. A link whose target this process cannot spell
/// as a URI is dropped rather than sent with none: `target: None` means "ask me
/// again", and a second question would get the same answer.
///
/// No tooltip. LSP shows one in place of the target, and the target is the file
/// path, which is the more useful of the two things there is to say.
fn document_link(
    documents: &Documents,
    encoding: PositionEncoding,
    uri: &Uri,
) -> Vec<DocumentLink> {
    let Some(analysed) = analyse(documents, uri) else {
        return Vec::new();
    };
    links::for_document(&analysed, encoding)
        .into_iter()
        .filter_map(|link| {
            Some(DocumentLink {
                range: link.range,
                target: Some(uri_of(&link.target)?),
                tooltip: None,
                data: None,
            })
        })
        .collect()
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
