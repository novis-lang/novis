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
//! An arm answers out of the two stores it is handed — the open documents and
//! the one workspace symbol index — and out of what the client configured, and
//! writes nothing back to any of them. The
//! index is [`serve`]'s because the store it is built from is, and it is
//! refreshed in this loop rather than in an arm, which is what keeps every one
//! of its five features a reader
//! (`rule:ide/five-features-are-one-reference-index`).
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
//! [`crate::diagnostics::for_document`]'s and, at workspace scope,
//! [`crate::diagnostics::dimming`]'s.
//!
//! **An edit's share of that work waits out `nvs.lsp.debounce`**, and the next
//! keystroke in the same buffer replaces it, so a line being typed is analysed
//! once rather than once per character. [`reanalyse`] is the work and [`serve`]'s
//! loop is what holds it back; a request arriving in the window runs it first,
//! because an answer is about the buffer as it stands.

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Instant;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, CodeLensRequest, Completion, DocumentHighlightRequest, DocumentLinkRequest,
    DocumentSymbolRequest, FoldingRangeRequest, GotoDefinition, GotoImplementation,
    GotoTypeDefinition, HoverRequest, InlayHintRequest, References, Request as _,
    SelectionRangeRequest, SemanticTokensFullRequest, SignatureHelpRequest, TypeHierarchyPrepare,
    TypeHierarchySubtypes, TypeHierarchySupertypes,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeLens, CodeLensParams,
    Command, CompletionItem, CompletionParams, CompletionResponse, CompletionTriggerKind,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams, DocumentLink,
    DocumentLinkParams, DocumentSymbolParams, DocumentSymbolResponse, FoldingRange,
    FoldingRangeParams, GotoDefinitionParams, GotoDefinitionResponse, HoverParams,
    InitializeParams, InlayHint, InlayHintParams, Location, Position, PublishDiagnosticsParams,
    Range, ReferenceParams, SelectionRange, SelectionRangeParams, SemanticTokens,
    SemanticTokensParams, SemanticTokensResult, SignatureHelp, SignatureHelpParams, SymbolKind,
    TextDocumentIdentifier, TextEdit, TypeHierarchyItem, TypeHierarchyPrepareParams,
    TypeHierarchySubtypesParams, TypeHierarchySupertypesParams, Uri, WorkspaceEdit,
};
use nvs_diagnostics::{BytePos, PositionEncoding, SourceId, SourceMap};

use crate::actions;
use crate::capabilities::initialize_result;
use crate::completion;
use crate::definition;
use crate::diagnostics::{Phases, dimming, for_document};
use crate::document::{Analysed, Documents, analyse, path_of, uri_of};
use crate::folding;
use crate::hints;
use crate::hover;
use crate::index::{CheckScope, DeclKind, Declaration, Site, SymbolIndex, symbol_at};
use crate::links;
use crate::position::{encoding_of, offset_at, range_of};
use crate::redactions;
use crate::regions;
use crate::selection;
use crate::semantic;
use crate::settings::Settings;
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
/// The connection is dropped before the threads are joined, and that order is
/// the whole of why this process ever exits. The reader thread stops at the
/// `exit` notification, but the writer thread runs until the last sender on
/// its channel is gone — and the connection holds one. Joining while it is
/// still alive blocks forever, which an editor sees as an `nvs lsp` that
/// survives every session it started and has to be killed.
///
/// # Errors
///
/// Returns the first protocol or io failure. Both are terminal: a framing
/// error means the stream's position is no longer known.
pub fn run() -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    serve(&connection)?;
    drop(connection);
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
    // Negotiated once, and what every position from here on is counted in is
    // the encoding the answer about to be sent declares rather than a second
    // reading of the same client capability: a server counting in an encoding
    // it did not declare is wrong by a little on every non-ASCII line and
    // reports no error anywhere.
    let (negotiated, declared) = initialize_result(&params);
    let encoding = encoding_of(&negotiated);
    connection.initialize_finish(id, declared)?;

    // This thread owns the store, which is the whole of the analysis half of
    // `rule:ide/the-server-is-synchronous`: one place holds what the client has
    // open, and it is the place that reads the channel.
    let mut documents = Documents::new();
    // What the client configured, read once because `initialize` is the one
    // message that carries it ([`crate::settings`]). Each of these decides what
    // this server builds, or when it builds it, rather than what an answer says,
    // so they are read here and not at a request: `scope` and the root are the
    // tree the index below is constructed over, `code_lens` is whether a lens is
    // offered at all, and `debounce` is how long an edit waits before it is
    // analysed.
    let settings = Settings::from_initialize(&params);

    // Before the index, which analyses every file under the root: a file some
    // program autoloads resolves its names through that program's map, and the
    // survey is what finds the programs
    // (`rule:ide/an-open-document-is-its-own-entry-point`).
    documents.survey(settings.scope, settings.root.as_deref());

    // The one index `rule:ide/five-features-are-one-reference-index` names,
    // held here because this is what owns the store it is built from. Empty at
    // this point at open scope — nothing is open yet — and filled by the
    // refresh below as documents arrive; under
    // `rule:ide/check-scope-defaults-to-the-workspace`'s `Workspace`, which is
    // the default, it already holds every `.nvs` file under the root the
    // client named.
    let mut index = SymbolIndex::build(&documents, settings.scope, settings.root.as_deref());

    // The edit whose analysis `nvs.lsp.debounce` is holding back, and the
    // moment it runs if nothing gets there first. `None` is a server at rest,
    // which is what it is between one burst of typing and the next.
    let mut waiting: Option<(Changed, Instant)> = None;

    loop {
        // What is left of that window, and `None` for a server at rest, which
        // waits on the channel with no deadline at all.
        let left = waiting
            .as_ref()
            .map(|(_, ready)| ready.saturating_duration_since(Instant::now()));
        let message = match left {
            Some(left) => match connection.receiver.recv_timeout(left) {
                Ok(message) => message,
                Err(error) if error.is_timeout() => {
                    // The window elapsed with nothing behind it, so the
                    // keystroke it was waiting out was the last one.
                    if let Some((pending, _)) = waiting.take() {
                        reanalyse(
                            connection,
                            &mut documents,
                            &mut index,
                            settings.scope,
                            encoding,
                            &pending,
                        )?;
                    }
                    continue;
                }
                // The reader thread is gone, which is the end this loop reached
                // before it ever waited for anything.
                Err(_) => return Ok(()),
            },
            None => match connection.receiver.recv() {
                Ok(message) => message,
                Err(_) => return Ok(()),
            },
        };

        match message {
            Message::Request(request) => {
                // `shutdown` is answered and then waited on: the client sends
                // `exit` next, and `handle_shutdown` consumes it. Returning
                // here is what ends the loop — and a deferred analysis is
                // dropped with it, because a client that is leaving has no use
                // for a squiggle.
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                // An answer is about the buffer as it stands, so a request cuts
                // the window short rather than being answered off an index the
                // keystroke in it has already made stale. What the debounce
                // buys is the analysis a later keystroke makes pointless, never
                // an answer that is behind the screen.
                if let Some((pending, _)) = waiting.take() {
                    reanalyse(
                        connection,
                        &mut documents,
                        &mut index,
                        settings.scope,
                        encoding,
                        &pending,
                    )?;
                }
                let answered = answer(&documents, &index, &settings, encoding, request);
                connection.sender.send(answered.into())?;
            }
            Message::Notification(notification) => {
                let Some(changed) = apply(&mut documents, notification) else {
                    continue;
                };
                // A keystroke in the buffer whose analysis is already waiting
                // replaces it, and that analysis never runs — the whole of what
                // `nvs.lsp.debounce` buys. Anything else flushes it first, so
                // diagnostics still arrive in the order the edits did.
                if let Some((pending, _)) = waiting.take()
                    && !(changed.edited && pending.path == changed.path)
                {
                    reanalyse(
                        connection,
                        &mut documents,
                        &mut index,
                        settings.scope,
                        encoding,
                        &pending,
                    )?;
                }
                // Only an edit waits. An open or a close is one deliberate
                // action whose diagnostics the developer is already looking at
                // the file for, and a zero debounce is the setting turned off
                // rather than a window of no length.
                if changed.edited && !settings.debounce.is_zero() {
                    waiting = Some((changed, Instant::now() + settings.debounce));
                } else {
                    reanalyse(
                        connection,
                        &mut documents,
                        &mut index,
                        settings.scope,
                        encoding,
                        &changed,
                    )?;
                }
            }
            // A response is an answer to a request this server has not yet
            // learned to send.
            Message::Response(_) => {}
        }
    }
}

/// Analyses what `changed` made stale and publishes for every open document on
/// that list, which is the work one document-sync notification owes and the work
/// `nvs.lsp.debounce` defers.
///
/// The index is refreshed for every edit and not just for the documents
/// [`publish`] re-analyses: a file nobody has open is still a file a reference
/// list has to be right about, which is where the index stops being the document
/// server `rule:ide/an-open-document-is-its-own-entry-point` describes. Before
/// the publish because that one takes the store mutably, and on this thread
/// nothing reads the index in between.
///
/// # Errors
///
/// As [`publish`].
fn reanalyse(
    connection: &Connection,
    documents: &mut Documents,
    index: &mut SymbolIndex,
    scope: CheckScope,
    encoding: PositionEncoding,
    changed: &Changed,
) -> Result<(), ServerError> {
    if let Some(path) = &changed.path {
        // First, because both analyses below borrow what it finds.
        documents.resurvey(path);
        index.refresh(documents, path);
    }
    publish(connection, documents, index, scope, encoding, changed)
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
fn answer(
    documents: &Documents,
    index: &SymbolIndex,
    settings: &Settings,
    encoding: PositionEncoding,
    request: Request,
) -> Response {
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
        SemanticTokensFullRequest::METHOD => {
            match serde_json::from_value::<SemanticTokensParams>(params) {
                Ok(params) => Response::new_ok(
                    id,
                    semantic_tokens(documents, encoding, &params.text_document.uri),
                ),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        GotoDefinition::METHOD => match serde_json::from_value::<GotoDefinitionParams>(params) {
            Ok(params) => Response::new_ok(id, definition(documents, encoding, &params)),
            Err(error) => unreadable(id, &method, &error),
        },
        // The same parameters as `definition`: LSP declares one position shape
        // for all three of these, and `lsp_types` spells the other two as
        // aliases of this one rather than as types of their own.
        GotoTypeDefinition::METHOD => {
            match serde_json::from_value::<GotoDefinitionParams>(params) {
                Ok(params) => Response::new_ok(id, type_definition(documents, encoding, &params)),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        GotoImplementation::METHOD => {
            match serde_json::from_value::<GotoDefinitionParams>(params) {
                Ok(params) => {
                    Response::new_ok(id, implementation(documents, index, encoding, &params))
                }
                Err(error) => unreadable(id, &method, &error),
            }
        }
        HoverRequest::METHOD => match serde_json::from_value::<HoverParams>(params) {
            Ok(params) => Response::new_ok(id, hover(documents, encoding, &params)),
            Err(error) => unreadable(id, &method, &error),
        },
        SignatureHelpRequest::METHOD => {
            match serde_json::from_value::<SignatureHelpParams>(params) {
                Ok(params) => Response::new_ok(id, signature_help(documents, encoding, &params)),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        Completion::METHOD => match serde_json::from_value::<CompletionParams>(params) {
            Ok(params) => Response::new_ok(
                id,
                completion(documents, index, settings, encoding, &params),
            ),
            Err(error) => unreadable(id, &method, &error),
        },
        InlayHintRequest::METHOD => match serde_json::from_value::<InlayHintParams>(params) {
            Ok(params) => Response::new_ok(id, inlay_hints(documents, encoding, &params)),
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
        CodeActionRequest::METHOD => match serde_json::from_value::<CodeActionParams>(params) {
            Ok(params) => Response::new_ok(id, code_actions(documents, encoding, &params)),
            Err(error) => unreadable(id, &method, &error),
        },
        References::METHOD => match serde_json::from_value::<ReferenceParams>(params) {
            Ok(params) => Response::new_ok(id, references(documents, index, encoding, &params)),
            Err(error) => unreadable(id, &method, &error),
        },
        DocumentHighlightRequest::METHOD => {
            match serde_json::from_value::<DocumentHighlightParams>(params) {
                Ok(params) => {
                    Response::new_ok(id, document_highlight(documents, index, encoding, &params))
                }
                Err(error) => unreadable(id, &method, &error),
            }
        }
        CodeLensRequest::METHOD => match serde_json::from_value::<CodeLensParams>(params) {
            Ok(params) => Response::new_ok(
                id,
                code_lens(
                    documents,
                    index,
                    settings,
                    encoding,
                    &params.text_document.uri,
                ),
            ),
            Err(error) => unreadable(id, &method, &error),
        },
        TypeHierarchyPrepare::METHOD => {
            match serde_json::from_value::<TypeHierarchyPrepareParams>(params) {
                Ok(params) => Response::new_ok(
                    id,
                    prepare_type_hierarchy(documents, index, encoding, &params),
                ),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        TypeHierarchySupertypes::METHOD => {
            match serde_json::from_value::<TypeHierarchySupertypesParams>(params) {
                Ok(params) => Response::new_ok(
                    id,
                    hierarchy_items(documents, &index.supertypes(&params.item.name), encoding),
                ),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        TypeHierarchySubtypes::METHOD => {
            match serde_json::from_value::<TypeHierarchySubtypesParams>(params) {
                Ok(params) => Response::new_ok(
                    id,
                    hierarchy_items(documents, &index.subtypes(&params.item.name), encoding),
                ),
                Err(error) => unreadable(id, &method, &error),
            }
        }
        redactions::METHOD => match serde_json::from_value::<TextDocumentIdentifier>(params) {
            Ok(document) => {
                Response::new_ok(id, redaction_ranges(documents, encoding, &document.uri))
            }
            Err(error) => unreadable(id, &method, &error),
        },
        regions::METHOD => match regions::Params::from_value(params) {
            Ok(params) => Response::new_ok(id, template_regions(documents, encoding, &params)),
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

/// `textDocument/semanticTokens/full` — what every name in one open document
/// is.
///
/// An empty answer rather than `null` for a document this server has nothing
/// open for, on [`document_symbol`]'s terms. No `result_id` is set: that is the
/// handle a `semanticTokens/full/delta` request would send back, and
/// `rule:ide/the-request-set-is-closed` does not admit one — a client offered no
/// delta capability re-reads the whole document, which is what
/// `rule:ide/one-grammar-one-tree` already costs on every analysis.
///
/// The walk is [`semantic::for_document`], which the `.lspt` suite calls too, so
/// what an editor colours and what a case freezes cannot drift apart.
fn semantic_tokens(
    documents: &Documents,
    encoding: PositionEncoding,
    uri: &Uri,
) -> SemanticTokensResult {
    let data = analyse(documents, uri).map_or_else(Vec::new, |analysed| {
        semantic::for_document(&analysed, encoding, &[])
    });
    SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
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

/// `textDocument/references` — every use of the name under the cursor, anywhere
/// the index reaches.
///
/// `null` for a document this server has nothing open for and for a cursor on
/// no name it can follow, on [`definition`]'s terms. An **empty list** is a
/// different answer and a real one: a name the index holds that nothing uses is
/// exactly what unused-member dimming is about, so it must not read as "ask me
/// again somewhere else".
///
/// `context.include_declaration` is honoured out of
/// [`SymbolIndex::declaration`] rather than by widening the occurrence query:
/// the two sides of the index are separate, and a client that did not ask for
/// the declaration would otherwise be sent it.
fn references(
    documents: &Documents,
    index: &SymbolIndex,
    encoding: PositionEncoding,
    params: &ReferenceParams,
) -> Option<Vec<Location>> {
    let position = params.text_document_position.position;
    let analysed = analyse(documents, &params.text_document_position.text_document.uri)?;
    let offset = offset_at(analysed.map.file(analysed.entry), position, encoding);
    uses_of(
        documents,
        index,
        &analysed,
        offset,
        params.context.include_declaration,
        encoding,
    )
}

/// [`references`]' answer, from an analysis and an offset already in hand.
///
/// Split out for the `.lspt` runner, which asks this of a case it has already
/// analysed ([`references_of_case`]). What the wire half adds is reading a
/// position out of the parameters and converting it, so the index query itself
/// stays written once.
fn uses_of(
    documents: &Documents,
    index: &SymbolIndex,
    analysed: &Analysed,
    offset: BytePos,
    include_declaration: bool,
    encoding: PositionEncoding,
) -> Option<Vec<Location>> {
    let symbol = symbol_at(analysed, offset)?;

    let mut sites: Vec<&Site> = Vec::new();
    if include_declaration && let Some(declared) = index.declaration(&symbol) {
        sites.push(&declared.site);
    }
    sites.extend(
        index
            .occurrences(&symbol)
            .into_iter()
            .map(|occurrence| &occurrence.site),
    );
    Some(locations(documents, &sites, encoding))
}

/// `textDocument/documentHighlight` — every use of the name under the cursor in
/// the document it was written in.
///
/// **[`references`]' query, filtered to the open file**, and that is the whole
/// of why this waited for the index rather than shipping with the rest of the
/// cursor answers: it needs resolution applied to *every* occurrence rather
/// than to the one under the cursor (ADR 0099 § 3). A walk of its own here is
/// what `rule:ide/five-features-are-one-reference-index` refuses.
///
/// Every hit is `Text`, never `Read` or `Write`. Which of those an access is is
/// a fact about the access, and the index records where a name was used rather
/// than what was done to it — a kind this server cannot compute is better left
/// as the one LSP defaults to than guessed at.
fn document_highlight(
    documents: &Documents,
    index: &SymbolIndex,
    encoding: PositionEncoding,
    params: &DocumentHighlightParams,
) -> Option<Vec<DocumentHighlight>> {
    let position = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let analysed = analyse(documents, uri)?;
    // The entry document's own file, because the answer cannot leave it: no
    // second file is loaded here where [`locations`] has to load one per hit.
    let file = analysed.map.file(analysed.entry);
    let offset = offset_at(file, position, encoding);
    let here = path_of(uri)?;

    Some(
        uses_in(index, &analysed, &here, offset)?
            .iter()
            .map(|site| DocumentHighlight {
                range: range_of(file, site.start, site.end, encoding),
                kind: Some(DocumentHighlightKind::TEXT),
            })
            .collect(),
    )
}

/// The sites in `here` that are uses of the name at `offset`.
///
/// [`uses_of`]'s narrowing, written once for the wire half and the `.lspt`
/// runner ([`highlights_of_case`]) the way that function is written once for
/// [`references`] and [`references_of_case`]. Sites rather than an answer,
/// because the two halves spell one differently: the wire needs a range in the
/// open document and a case needs a place it can name a file in.
///
/// The declaration is not among them. `occurrences_in` is uses only, and an
/// editor asking this is asking what else on screen is this same name rather
/// than where it came from — which is [`references`]' question, and the one
/// request whose parameters carry a flag for it.
fn uses_in<'index>(
    index: &'index SymbolIndex,
    analysed: &Analysed,
    here: &Path,
    offset: BytePos,
) -> Option<Vec<&'index Site>> {
    let symbol = symbol_at(analysed, offset)?;
    Some(
        index
            .occurrences_in(here)
            .iter()
            .filter(|occurrence| occurrence.symbol == symbol)
            .map(|occurrence| &occurrence.site)
            .collect(),
    )
}

/// Every site as a [`Location`], each file's positions counted in its own text.
///
/// One [`SourceMap`] for the whole answer with the open buffers overlaid on it,
/// and each file loaded once however many sites are in it: a site is a path and
/// two byte offsets rather than a span, so turning one into a range means
/// having read the file it is in, and a reference list is usually several sites
/// per file.
///
/// A file this process cannot load, or cannot spell as a URI, drops its own
/// sites and nothing else. The rest of the list is still every reference the
/// developer asked about, which beats refusing the answer over one file that
/// has been deleted since it was indexed.
fn locations(documents: &Documents, sites: &[&Site], encoding: PositionEncoding) -> Vec<Location> {
    let (map, loaded) = overlaid(documents, sites.iter().map(|site| site.path.as_path()));

    sites
        .iter()
        .filter_map(|site| {
            let id = (*loaded.get(&site.path)?)?;
            Some(Location {
                uri: uri_of(&site.path)?,
                range: range_of(map.file(id), site.start, site.end, encoding),
            })
        })
        .collect()
}

/// One source map with the open buffers overlaid, holding each of `paths`
/// loaded exactly once.
///
/// A file this process cannot load is `None` rather than an error, so what was
/// written in it drops out of the answer and the rest of it stands — which
/// beats refusing a whole reference list or hierarchy over one file that has
/// been deleted since it was indexed.
fn overlaid<'a>(
    documents: &Documents,
    paths: impl Iterator<Item = &'a Path>,
) -> (SourceMap, HashMap<PathBuf, Option<SourceId>>) {
    let mut map = SourceMap::new();
    documents.overlay(&mut map);
    let mut loaded: HashMap<PathBuf, Option<SourceId>> = HashMap::new();
    for path in paths {
        if !loaded.contains_key(path) {
            loaded.insert(path.to_path_buf(), map.load(path).ok());
        }
    }
    (map, loaded)
}

/// `textDocument/prepareTypeHierarchy` — the type under the cursor, as the item
/// the two requests below are then asked about.
///
/// A developer opens a hierarchy from the `class C` line, which is a
/// declaration rather than a resolved use. [`symbol_at`] answers both readings
/// of a cursor, so this request asks it once and the reachable case and the
/// interesting one are one call.
///
/// A class and an interface, and nothing else. They are what
/// [`nvs_hir::ClassGraph`] holds an entry for, and an enum has neither an
/// `extends` grammar nor an `implements` clause it is allowed to write
/// (`rule:enums/no-class-machinery`), so a hierarchy rooted at one could only
/// ever be the item itself.
fn prepare_type_hierarchy(
    documents: &Documents,
    index: &SymbolIndex,
    encoding: PositionEncoding,
    params: &TypeHierarchyPrepareParams,
) -> Option<Vec<TypeHierarchyItem>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let analysed = analyse(documents, uri)?;
    let offset = offset_at(
        analysed.map.file(analysed.entry),
        params.text_document_position_params.position,
        encoding,
    );

    let symbol = symbol_at(&analysed, offset)?;
    let declared = index.declaration(&symbol)?;
    if !matches!(declared.kind, DeclKind::Class | DeclKind::Interface) {
        return None;
    }
    Some(hierarchy_items(documents, &[declared], encoding))
}

/// Every declaration as a hierarchy item, each file's positions counted in its
/// own text.
///
/// **The item's `name` is the index's own spelling of the symbol, and it is
/// also its identity.** LSP keeps a `data` field for a server that needs to
/// recognise its own item when the client sends it back, and this one does not:
/// a name here is already the fully-qualified string every query in
/// [`crate::index`] is keyed on, so carrying it twice would be two spellings of
/// one thing and a second one to keep in step.
///
/// `range` and `selection_range` are the same range, which is the declared
/// name's own bytes. The index keeps a name and not a declaration's extent
/// (`crate::index::Site`), and a range invented to enclose more than that would
/// be a guess the client then reveals.
fn hierarchy_items(
    documents: &Documents,
    found: &[&Declaration],
    encoding: PositionEncoding,
) -> Vec<TypeHierarchyItem> {
    let (map, loaded) = overlaid(
        documents,
        found.iter().map(|declared| declared.site.path.as_path()),
    );

    found
        .iter()
        .filter_map(|declared| {
            let id = (*loaded.get(&declared.site.path)?)?;
            let range = range_of(
                map.file(id),
                declared.site.start,
                declared.site.end,
                encoding,
            );
            Some(TypeHierarchyItem {
                name: declared.symbol.clone(),
                kind: symbol_kind(declared.kind),
                tags: None,
                detail: None,
                uri: uri_of(&declared.site.path)?,
                range,
                selection_range: range,
                data: None,
            })
        })
        .collect()
}

/// What an editor shows a declaration of this kind as.
///
/// A type alias is `TypeParameter` because LSP has no kind for an alias and
/// that is the one whose icon says "a name standing for a type" rather than
/// "a thing with members".
const fn symbol_kind(kind: DeclKind) -> SymbolKind {
    match kind {
        DeclKind::Class => SymbolKind::CLASS,
        DeclKind::Interface => SymbolKind::INTERFACE,
        DeclKind::Enum => SymbolKind::ENUM,
        DeclKind::TypeAlias => SymbolKind::TYPE_PARAMETER,
        DeclKind::Method => SymbolKind::METHOD,
        DeclKind::Property => SymbolKind::PROPERTY,
        DeclKind::Const => SymbolKind::CONSTANT,
        DeclKind::EnumCase => SymbolKind::ENUM_MEMBER,
    }
}

/// `textDocument/codeLens` — how many uses the index holds of each name this
/// document declares.
///
/// The third reader `rule:ide/five-features-are-one-reference-index` names, and
/// the cheapest of them: [`SymbolIndex::declarations_in`] is one map lookup and
/// the count is [`SymbolIndex::occurrences`], so no front end runs here at all.
/// That matters more here than for a cursor answer — a client asks for the
/// lenses of every visible document and asks again after every edit, which is
/// the cost `nvs.codeLens.enable` exists to let a developer refuse, and
/// refusing it is `None` rather than an empty list.
///
/// **A lens is a label and not a link.** Its command is the empty string, which
/// renders the title and runs nothing. A clickable one would have to name a
/// command id, the roster `rule:ide/contributions-are-frozen-and-only-ever-added`
/// freezes holds none that shows a reference list, and an editor's own built-in
/// id would be an error message in the other client
/// (`rule:ide/one-server-two-thin-clients`).
///
/// **Every declaration the index holds gets one**, an enum case included: a
/// case's reads are recorded against the case, so the count above one is the
/// count of the sites that read it.
fn code_lens(
    documents: &Documents,
    index: &SymbolIndex,
    settings: &Settings,
    encoding: PositionEncoding,
    uri: &Uri,
) -> Option<Vec<CodeLens>> {
    if !settings.code_lens {
        return None;
    }
    let path = path_of(uri)?;
    // One file loaded and no graph: a site is two byte offsets into the text of
    // the file it is in, and the open buffer is what the client counts its own
    // positions in.
    let mut map = SourceMap::new();
    documents.overlay(&mut map);
    let id = map.load(&path).ok()?;
    let file = map.file(id);

    Some(
        index
            .declarations_in(&path)
            .iter()
            .map(|declared| CodeLens {
                range: range_of(file, declared.site.start, declared.site.end, encoding),
                command: Some(Command {
                    title: reference_count(index.occurrences(&declared.symbol).len()),
                    command: String::new(),
                    arguments: None,
                }),
                data: None,
            })
            .collect(),
    )
}

/// The settings one `.lspt` case is answered under.
///
/// The roster's own defaults, with the root set to the directory the runner
/// materialised the case into: a case's `--FILE--` sections are the whole of
/// its world, so `Workspace` over that directory is what makes the index hold
/// exactly them and nothing of the host it ran on. The scope is written out
/// rather than left to the default because a case is answered at workspace
/// scope whatever a client's default is
/// (`rule:ide/check-scope-defaults-to-the-workspace`).
fn case_settings(root: &Path) -> Settings {
    Settings {
        scope: CheckScope::Workspace,
        root: Some(root.to_owned()),
        ..Settings::default()
    }
}

/// [`code_lens`] for one `.lspt` case, over an index built for that case alone.
///
/// The index is built here and not in [`crate::suite`] because this module is
/// the one that owns one: a runner holding its own would be the second builder
/// of the thing `rule:ide/five-features-are-one-reference-index` keeps to one,
/// which is what `crates/nvs-lsp/tests/index.rs` counts. A case is one
/// document at one moment, so this index is built for the question and dropped
/// with the answer — the refresh a session needs has nothing to do here.
pub(crate) fn lenses_of_case(
    documents: &Documents,
    root: &Path,
    uri: &Uri,
    encoding: PositionEncoding,
) -> Option<Vec<CodeLens>> {
    let settings = case_settings(root);
    let index = SymbolIndex::build(documents, settings.scope, settings.root.as_deref());
    code_lens(documents, &index, &settings, encoding, uri)
}

/// [`completion`] for one `.lspt` case, over an index built for that case
/// alone and for [`lenses_of_case`]'s reason.
///
/// The items and not a [`CompletionResponse`], because the runner narrows them
/// by the two arguments a completion case may write before it renders any of
/// them ([`crate::suite`]), and a wire shape has no place in between.
pub(crate) fn items_of_case(
    documents: &Documents,
    analysed: &Analysed,
    root: &Path,
    offset: BytePos,
) -> Vec<CompletionItem> {
    let settings = case_settings(root);
    let index = SymbolIndex::build(documents, settings.scope, settings.root.as_deref());
    completion::at(
        analysed,
        &index,
        offset,
        settings.php_names,
        settings.client,
        PositionEncoding::Utf8,
    )
}

/// [`references`] for one `.lspt` case, over an index built for that case
/// alone and for [`lenses_of_case`]'s reason.
///
/// **The declaration is included**, because that is what "find all references"
/// in an editor sends and a `--REQUEST--` line carries no argument to say
/// otherwise (`rule:ide/a-request-line-is-closed`). It is also the reading
/// that makes a case say something: the answer to "where is this name" is the
/// same list whether the cursor sits on the declaration or on a use, and a
/// corpus that left the declaration out would freeze two different lists for
/// one question.
pub(crate) fn references_of_case(
    documents: &Documents,
    analysed: &Analysed,
    root: &Path,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Vec<Location>> {
    let settings = case_settings(root);
    let index = SymbolIndex::build(documents, settings.scope, settings.root.as_deref());
    uses_of(documents, &index, analysed, offset, true, encoding)
}

/// [`document_highlight`] for one `.lspt` case, over an index built for that
/// case alone and for [`lenses_of_case`]'s reason.
///
/// [`Location`]s rather than the [`DocumentHighlight`]s the wire carries, so
/// the runner turns a hit into a place exactly as it does for
/// [`references_of_case`] and a highlight case freezes the same spelling a
/// reference case does. Every one of them is in `entry` by construction, which
/// is what the case is pinning: the kind each hit carries is `Text` and the
/// same for all of them, so there is nothing else a rendering could say.
pub(crate) fn highlights_of_case(
    documents: &Documents,
    analysed: &Analysed,
    root: &Path,
    entry: &Path,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Vec<Location>> {
    let settings = case_settings(root);
    let index = SymbolIndex::build(documents, settings.scope, settings.root.as_deref());
    let sites = uses_in(&index, analysed, entry, offset)?;
    Some(locations(documents, &sites, encoding))
}

/// What a lens says above a declaration `count` things refer to.
///
/// Singular, plural, and a word rather than a `0`: the line is read above a
/// declaration in an editor, where what is around it is prose and not a table.
fn reference_count(count: usize) -> String {
    match count {
        0 => "no references".to_owned(),
        1 => "1 reference".to_owned(),
        _ => format!("{count} references"),
    }
}

/// `textDocument/completion` — what may be written at the cursor.
///
/// A plain `Array`, never a `CompletionList`: this server offers what it has
/// resolved and has nothing more to offer if the developer types another
/// letter, so `isIncomplete` would be a promise to recompute that is never
/// worth keeping. Nothing is filtered by the prefix already typed either — the
/// client does that, and it does it without a round trip. The lookup is
/// [`completion::at`], which the `.lspt` suite calls too.
///
/// A request a trigger character raised is answered only where that character
/// finished a spelling ([`completion::continues_a_trigger`]): the editor asks
/// on the keystroke, and a `:` or a `>` is far more often an operator than the
/// end of `::` or `->`.
fn completion(
    documents: &Documents,
    index: &SymbolIndex,
    settings: &Settings,
    encoding: PositionEncoding,
    params: &CompletionParams,
) -> CompletionResponse {
    let position = params.text_document_position.position;
    let Some(analysed) = analyse(documents, &params.text_document_position.text_document.uri)
    else {
        return CompletionResponse::Array(Vec::new());
    };
    let offset = offset_at(analysed.map.file(analysed.entry), position, encoding);
    let triggered = params
        .context
        .as_ref()
        .is_some_and(|context| context.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER);
    if triggered && !completion::continues_a_trigger(&analysed, offset) {
        return CompletionResponse::Array(Vec::new());
    }
    CompletionResponse::Array(completion::at(
        &analysed,
        index,
        offset,
        settings.php_names,
        settings.client,
        encoding,
    ))
}

/// `textDocument/hover` — what the name under the cursor documents.
///
/// `null` for a document this server has nothing open for and for a cursor on
/// nothing it can say anything about, which are one answer on the wire for
/// [`definition`]'s reason. The lookup is [`hover::at`], which the `.lspt`
/// suite calls too, so what an editor shows and what a case freezes cannot
/// drift apart.
fn hover(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &HoverParams,
) -> Option<lsp_types::Hover> {
    let position = params.text_document_position_params.position;
    let analysed = analyse(
        documents,
        &params.text_document_position_params.text_document.uri,
    )?;
    let offset = offset_at(analysed.map.file(analysed.entry), position, encoding);
    hover::at(&analysed, offset, encoding)
}

/// `textDocument/signatureHelp` — the row of the call the cursor is inside.
///
/// `null` for a document this server has nothing open for and for a cursor
/// inside no call the checker resolved, which are one answer on the wire for
/// [`definition`]'s reason. The lookup is [`hover::help_at`], which is where
/// the row's one spelling lives.
fn signature_help(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &SignatureHelpParams,
) -> Option<SignatureHelp> {
    let position = params.text_document_position_params.position;
    let analysed = analyse(
        documents,
        &params.text_document_position_params.text_document.uri,
    )?;
    let offset = offset_at(analysed.map.file(analysed.entry), position, encoding);
    hover::help_at(&analysed, offset)
}

/// `textDocument/typeDefinition` — where the type of what the cursor is on was
/// declared.
///
/// [`definition`] pointed one step further: that one answers where the *name*
/// under the cursor was declared, and this one reads the type the checker
/// recorded for it and answers where **that** was. The walk is the same one, so
/// a type declared in a required file is found for [`definition`]'s reason
/// rather than a second one.
///
/// `null` on [`definition`]'s terms, and additionally for a type that is not a
/// declaration — the lookup is [`definition::type_at`], which owns that list.
fn type_definition(
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
    let declared = definition::type_at(&analysed, offset, encoding)?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri_of(&declared.path)?,
        range: declared.range,
    }))
}

/// `textDocument/implementation` — every type the index records as extending or
/// implementing the one under the cursor.
///
/// [`SymbolIndex::subtypes`] is the query, which is
/// `rule:ide/five-features-are-one-reference-index`'s type-hierarchy reader
/// asked from a second place rather than a sixth feature arriving: the
/// hierarchy shows those edges as a tree and this shows them as a jump list.
/// The cursor is read as [`prepare_type_hierarchy`] reads it, so standing on a
/// declaration's own name and standing on a use of it are one answer.
///
/// `null` for a document this server has nothing open for and for a cursor on
/// no name the index holds. An **empty list** is a different answer and a real
/// one, for [`references`]' reason: an interface nothing implements is a fact
/// rather than a question to ask again somewhere else.
fn implementation(
    documents: &Documents,
    index: &SymbolIndex,
    encoding: PositionEncoding,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let analysed = analyse(documents, uri)?;
    let offset = offset_at(
        analysed.map.file(analysed.entry),
        params.text_document_position_params.position,
        encoding,
    );
    let symbol = symbol_at(&analysed, offset)?;
    let found = index.subtypes(&symbol);
    let sites: Vec<&Site> = found.iter().map(|declared| &declared.site).collect();
    Some(GotoDefinitionResponse::Array(locations(
        documents, &sites, encoding,
    )))
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
/// The hints inside the range the client asked about.
///
/// **The document is walked whole and the answer narrowed here**, which is
/// where [`crate::hints`] leaves it: the walk reads one analysis, and an editor
/// asks this request again for every range it scrolls onto, so a walk bounded
/// by the range would repeat most of itself per frame to save a filter.
///
/// The bounds are inclusive at both ends. A hint sits *between* two characters
/// rather than on one, so a hint at the first position of the visible range is
/// one the reader can see, and so is a hint at its last.
fn inlay_hints(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &InlayHintParams,
) -> Vec<InlayHint> {
    let Some(analysed) = analyse(documents, &params.text_document.uri) else {
        return Vec::new();
    };
    hints::for_document(&analysed, encoding)
        .into_iter()
        .filter(|hint| {
            at_or_after(hint.position, params.range.start)
                && at_or_after(params.range.end, hint.position)
        })
        .collect()
}

/// Whether `position` is at or after `limit`, in the order a document reads.
fn at_or_after(position: Position, limit: Position) -> bool {
    (position.line, position.character) >= (limit.line, limit.character)
}

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

/// `textDocument/codeAction` — the fixes offered over the range asked about.
///
/// Every one is a `Suggestion` a diagnostic already carried, translated by
/// [`actions::at`], which the `.lspt` suite calls too — so a light bulb and a
/// frozen case offer the same fix
/// (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
///
/// The edit crosses as a [`WorkspaceEdit`] naming this document alone: one
/// `Suggestion` is one span in one file, and the file is the one the client
/// asked about. `diagnostics` is left unset — the client is holding the
/// published diagnostic already, and re-deriving the wire value here would be a
/// second place the same range is computed.
///
/// An empty list rather than `null` for a document nothing is open for, on
/// [`document_symbol`]'s terms.
fn code_actions(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    let Some(analysed) = analyse(documents, &params.text_document.uri) else {
        return Vec::new();
    };
    let file = analysed.map.file(analysed.entry);
    let start = offset_at(file, params.range.start, encoding);
    let end = offset_at(file, params.range.end, encoding);
    let kind = actions::Kind::asked_for(params.context.only.as_deref());
    actions::at(&analysed, start, end, kind, encoding)
        .into_iter()
        .map(|action| {
            CodeActionOrCommand::CodeAction(CodeAction {
                title: action.title,
                kind: Some(CodeActionKind::new(kind.name())),
                edit: Some(WorkspaceEdit {
                    changes: Some(
                        [(
                            params.text_document.uri.clone(),
                            vec![TextEdit {
                                range: action.range,
                                new_text: action.replacement,
                            }],
                        )]
                        .into_iter()
                        .collect(),
                    ),
                    ..WorkspaceEdit::default()
                }),
                ..CodeAction::default()
            })
        })
        .collect()
}

/// `nvs/redactions` — which bytes of one open document the client conceals.
///
/// The one request of Novis's own, so nothing in `lsp_types` spells its params
/// or its answer: a [`TextDocumentIdentifier`] goes in, exactly as ADR 0101 § 1
/// names it, and a JSON array of `{range, kind}` comes back.
///
/// An empty array rather than `null` for a document this server has nothing
/// open for, and the distinction carries more here than anywhere else on
/// [`document_symbol`]'s terms: `rule:ide/redaction-ranges-come-from-the-server`
/// has the client hold its last answer when none arrives, so an answer of
/// "nothing to conceal" must be an answer.
///
/// The walk is [`redactions::for_document`], which the `.lspt` suite calls too,
/// so what an editor conceals and what a case freezes cannot drift apart.
fn redaction_ranges(
    documents: &Documents,
    encoding: PositionEncoding,
    uri: &Uri,
) -> Vec<serde_json::Value> {
    analyse(documents, uri).map_or_else(Vec::new, |analysed| {
        redactions::for_document(&analysed, encoding)
            .into_iter()
            .map(|item| serde_json::json!({ "range": item.range, "kind": item.kind }))
            .collect()
    })
}

/// `nvs/regions` — which spans of one open document are markup rather than
/// Novis.
///
/// [`redaction_ranges`]'s sibling: a document goes in, a JSON array comes back,
/// and an empty array is an answer rather than an omission — a client that
/// holds its last list keeps forwarding into a region the developer has since
/// deleted.
///
/// **Over a text, not over an analysis.** A boundary is a lexical fact and this
/// question arrives on the heels of an edit, so the text is lexed where every
/// other handler here resolves a graph and type-checks it; which text that is —
/// the params' own, or the buffer's when they carry none — is
/// [`regions::for_request`]'s, and the module doc beside it is where both
/// decisions are written down.
fn template_regions(
    documents: &Documents,
    encoding: PositionEncoding,
    params: &regions::Params,
) -> Vec<serde_json::Value> {
    regions::for_request(documents, encoding, params)
        .iter()
        .map(regions::wire)
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
    /// Whether this was an edit to an open buffer — a keystroke — rather than a
    /// document opening or closing.
    ///
    /// Only an edit is held back by `nvs.lsp.debounce`, and only an edit is
    /// replaced by the next one: two keystrokes in one buffer are one analysis,
    /// where a close arriving behind an edit is two things that both happened.
    edited: bool,
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
            Some(Changed {
                path,
                closed: None,
                edited: false,
            })
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
                    edited: true,
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
                    edited: false,
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
/// The index joins that list at the end, which is unused-member dimming
/// (`rule:ide/five-features-are-one-reference-index`) and the one thing here a
/// compiler phase did not report. It is empty unless `scope` is
/// `CheckScope::Workspace`, so the default publishes exactly what it published
/// before the setting existed.
///
/// # Errors
///
/// Fails when the writer thread is gone, which is terminal for the same reason
/// a framing error is.
fn publish(
    connection: &Connection,
    documents: &mut Documents,
    index: &SymbolIndex,
    scope: CheckScope,
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
        let mut diagnostics = for_document(&analysed, Phases::Gated, encoding);
        if let Some(path) = path_of(&uri) {
            diagnostics.extend(dimming(
                &index.unused_private(&path),
                scope,
                &analysed,
                encoding,
            ));
        }
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
