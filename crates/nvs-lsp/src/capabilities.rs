//! What `initialize` declares, and the one encoding negotiation behind it.
//!
//! A capability is a promise to answer a request, and a client that reads one
//! it was not offered simply never asks. So the closed list of
//! `rule:ide/the-request-set-is-closed` is written **here and nowhere else**:
//! [`server_capabilities`] is the whole declaration, the acceptance test reads
//! it back field by field, and a request that grows the set has to change this
//! function to exist at all.
//!
//! The provider fields are declared for the whole goal rather than switched on
//! stage by stage. A capability is data the client caches at `initialize` and
//! never re-reads; turning one on later means the editor that started against
//! an earlier binary keeps asking nothing. What a request answers today is the
//! handler's business, and `rule:ide/the-request-set-is-closed` is about the
//! list, not the schedule.

use lsp_types::{
    ClientCapabilities, CodeActionKind, CodeActionOptions, CodeActionProviderCapability,
    CompletionOptions, DocumentLinkOptions, FoldingRangeProviderCapability,
    HoverProviderCapability, InitializeParams, InitializeResult, OneOf, PositionEncodingKind,
    SelectionRangeProviderCapability, SemanticTokenModifier, SemanticTokenType,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
};

/// The name this server reports in `serverInfo`.
///
/// It is the subcommand that starts it, because that is what an operator has
/// to type when a client reports the server it is talking to.
pub const SERVER_NAME: &str = "nvs lsp";

/// The semantic token types this server emits, in the order that **is** their
/// wire encoding.
///
/// A semantic token names its type by index into this list, so appending is
/// the only safe edit: reordering silently recolours every token in every
/// document. ADR 0099 § 4 chose each one because the TextMate grammar
/// structurally cannot answer it, and every name here is LSP's own — a type
/// outside the standard legend is matched by no theme and renders as unstyled
/// body text (`rule:ide/novis-ships-names-not-colours`).
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::CLASS,
    SemanticTokenType::INTERFACE,
    SemanticTokenType::ENUM,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::TYPE,
    SemanticTokenType::METHOD,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::TYPE_PARAMETER,
];

/// The semantic token modifiers this server emits, in their wire order.
///
/// `defaultLibrary` is LSP's own and marks a `Core` class, so the standard
/// library is visibly not user code. `tainted` and `secret` are Novis's two
/// additions and are the reason this layer is built at all
/// (`rule:ide/semantic-tokens-carry-the-qualifiers`): a qualifier travels with
/// a value through the program, and an editor showing it at every use site is
/// the cheapest teaching surface the language has. Being non-standard, they
/// need the client's `semanticTokenScopes` mapping to reach a theme, which is
/// goal 15's half.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DEFAULT_LIBRARY,
    SemanticTokenModifier::new("tainted"),
    SemanticTokenModifier::new("secret"),
];

/// The code action kinds this server produces.
///
/// Two actions, both translations of a `Suggestion` a `Diagnostic` already
/// carries (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`),
/// so they are `quickfix`. `source.fixAll.nvs` is what lets
/// `editor.codeActionsOnSave` run them beside format-on-save, per
/// `rule:tooling/fmt-is-never-a-diagnostic` — `nvs fmt` stays layout-only, so
/// a fix that changes meaning has to arrive through this channel instead.
pub const CODE_ACTION_KINDS: &[CodeActionKind] = &[
    CodeActionKind::QUICKFIX,
    CodeActionKind::new("source.fixAll.nvs"),
];

/// The version this server reports in `serverInfo`.
///
/// It is the binary's own, so the client can refuse a mismatch rather than
/// speak a protocol shape the server does not have
/// (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`).
#[must_use]
pub fn server_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// The `serverInfo` block: who is answering, and at what version.
#[must_use]
pub fn server_info() -> ServerInfo {
    ServerInfo {
        name: SERVER_NAME.to_owned(),
        version: Some(server_version()),
    }
}

/// The legend a semantic-token response is decoded against.
///
/// The client registers an equal one or every token lands one type off, which
/// no unit test on either side can see alone — ADR 0099 § 4 puts that check in
/// the extension-host run.
#[must_use]
pub fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Pick the position encoding both sides will use.
///
/// LSP 3.17 makes this the client's list in preference order and the server's
/// choice out of it, with `utf-16` the protocol's default when the client says
/// nothing — a 3.16 client has no `general.positionEncodings` at all, and
/// assuming `utf-8` there would shift every column on every non-ASCII line.
///
/// **`utf-8` is taken whenever it is offered**, because a Novis span is a byte
/// offset: at `utf-8` a position is `nvs_diagnostics`' `line_col` with no
/// conversion, and at `utf-16` every column pays `utf16_col`. The encoding is
/// the only thing decided here; the arithmetic itself has one home and it is
/// that crate (`rule:ide/positions-have-one-home`).
#[must_use]
pub fn negotiate_encoding(client: &ClientCapabilities) -> PositionEncodingKind {
    let offered = client
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_deref())
        .unwrap_or_default();

    if offered.contains(&PositionEncodingKind::UTF8) {
        PositionEncodingKind::UTF8
    } else {
        PositionEncodingKind::UTF16
    }
}

/// Everything this server promises to answer, at the negotiated `encoding`.
///
/// Every `Some` here is one entry of `rule:ide/the-request-set-is-closed`'s
/// list and every other provider field is `None` on purpose:
/// `documentHighlight`, `references`, `rename`, `workspaceSymbol` and inlay
/// hints each need the workspace index M10 builds, and declaring one before
/// then is a promise answered with an error.
#[must_use]
pub fn server_capabilities(encoding: PositionEncodingKind) -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(encoding),
        // `Full` and not `Incremental`: the analysis reparses the whole
        // document anyway (ADR 0099 § 1 traded incremental reparse away), so
        // applying a range edit here would buy nothing and add a second place
        // a document's text can be wrong. It is also what makes
        // `publishDiagnostics` reachable — diagnostics are published for open
        // documents only, and this is what "open" means.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            // Nothing is resolved lazily: a completion item's detail is the
            // type the analysis already computed, so there is no second round
            // trip to save.
            resolve_provider: Some(false),
            // The three positions where a member list appears rather than a
            // keyword list. `$` is not among them — a variable completion is
            // offered wherever an expression is, and a trigger character there
            // would suppress the client's own re-filtering as the name is
            // typed.
            trigger_characters: Some(vec![
                "-".to_owned(),
                ">".to_owned(),
                ":".to_owned(),
                "\\".to_owned(),
            ]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            completion_item: None,
        }),
        document_symbol_provider: Some(OneOf::Left(true)),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        document_link_provider: Some(DocumentLinkOptions {
            // The target is the resolved path and it is known when the link is
            // produced, so there is nothing to resolve on demand.
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(CODE_ACTION_KINDS.to_vec()),
            work_done_progress_options: WorkDoneProgressOptions::default(),
            resolve_provider: Some(false),
        })),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: semantic_tokens_legend(),
                // No `range` and no delta. Both are optimisations over a full
                // pass this goal has not measured as too slow, and a delta
                // needs the previous response kept per document — state whose
                // only justification would be a number nobody has.
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
        )),
        ..ServerCapabilities::default()
    }
}

/// The whole `initialize` answer for the client that sent `params`.
///
/// `nvs/redactions` appears nowhere in it, and that is not an omission: it is
/// Novis's own request (`rule:security/redaction-ranges-come-from-the-server`),
/// LSP has no capability field for one, and the client knows it is available
/// from `serverInfo` naming this server at all.
#[must_use]
pub fn initialize_result(params: &InitializeParams) -> InitializeResult {
    InitializeResult {
        capabilities: server_capabilities(negotiate_encoding(&params.capabilities)),
        server_info: Some(server_info()),
    }
}
