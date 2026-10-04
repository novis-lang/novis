//! What `initialize` declares, and the one encoding negotiation behind it.
//!
//! A capability is a promise to answer a request, and a client that reads one
//! it was not offered simply never asks. So the closed list of
//! `rule:ide/the-request-set-is-closed` is written **here and nowhere else**:
//! [`server_capabilities`] is the declaration, [`declared_capabilities`] adds
//! the one field `lsp_types` has no struct member for, the acceptance test
//! reads the two of them back as one object key by key, and a request that
//! grows the set has to change this file to exist at all.
//!
//! The provider fields are declared for the whole goal rather than switched on
//! stage by stage. A capability is data the client caches at `initialize` and
//! never re-reads; turning one on later means the editor that started against
//! an earlier binary keeps asking nothing. What a request answers today is the
//! handler's business, and `rule:ide/the-request-set-is-closed` is about the
//! list, not the schedule.

use lsp_types::{
    ClientCapabilities, CodeActionKind, CodeActionOptions, CodeActionProviderCapability,
    CodeLensOptions, CompletionOptions, DocumentLinkOptions, FoldingRangeProviderCapability,
    HoverProviderCapability, ImplementationProviderCapability, InitializeParams, OneOf,
    PositionEncodingKind, SelectionRangeProviderCapability, SemanticTokenModifier,
    SemanticTokenType, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SignatureHelpOptions,
    TextDocumentSyncCapability, TextDocumentSyncKind, TypeDefinitionProviderCapability,
    WorkDoneProgressOptions,
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
/// goal `editor`'s half.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DEFAULT_LIBRARY,
    SemanticTokenModifier::new("tainted"),
    SemanticTokenModifier::new("secret"),
];

/// The code action kinds this server produces.
///
/// Every fix is a translation of a `Suggestion` a `Diagnostic` already carries
/// (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`), so
/// it is `quickfix`. `source.fixAll.nvs` is what lets
/// `editor.codeActionsOnSave` run the fixes beside format-on-save, per
/// `rule:tooling/fmt-is-never-a-diagnostic` — `nvs fmt` stays layout-only, so
/// a fix that changes meaning has to arrive through this channel instead.
/// The third kind is the html-template refactor's ([`crate::html_template::KIND`]),
/// narrow enough that a client asking for it by kind gets that action alone.
pub const CODE_ACTION_KINDS: &[CodeActionKind] = &[
    CodeActionKind::QUICKFIX,
    CodeActionKind::new("source.fixAll.nvs"),
    CodeActionKind::new(crate::html_template::KIND),
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
/// list or of `rule:ide/five-features-are-one-reference-index`'s, and every
/// other provider field is `None` on purpose: `rename` and `workspaceSymbol`
/// each still need something no store this server holds can answer from, and
/// declaring one before then is a promise answered with an error.
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
        // The three requests answered out of what `hover` already reads.
        // `declarationProvider` is not among them and is not an omission: Novis
        // has no declaration site separate from a definition, so the request
        // would answer exactly what the field above it answers, and a client
        // offered both would give a reader two menu entries for one jump.
        signature_help_provider: Some(SignatureHelpOptions {
            // The character that opens an argument list and the one that ends
            // an argument. `)` is not among them — it closes the call, and a
            // popup opened there is one about a call that is finished. LSP
            // counts every trigger character as a re-trigger, so naming these
            // twice would say nothing more.
            trigger_characters: Some(vec!["(".to_owned(), ",".to_owned()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        // The two readers of the workspace symbol index that answer a cursor.
        // Neither takes options: a partial-result token would promise to stream
        // a list this server builds in one pass, and there is no second phase
        // to report work-done progress over.
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        // The same index's third reader, and the one that answers a document
        // rather than a cursor. Nothing is resolved lazily: the count a lens
        // shows is two lookups in a map the index already holds, so a second
        // round trip per lens would cost more than it saves. `nvs.codeLens.enable`
        // is not read here — this module's rule above is that a provider field
        // is the closed list and not the client's configuration of it, and a
        // capability the client cached at `initialize` is never re-read anyway.
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        completion_provider: Some(CompletionOptions {
            // What a row shows is on the row — its label, what is written
            // after it, the type at its right — and what the row documents
            // is not: a list names every type in reach, and a card on each
            // would be most of every response, read for one row. So a row
            // carries a key, and `crate::card` reads its documentation when
            // the client asks for that one item.
            resolve_provider: Some(true),
            // The last character of each spelling a list follows and no word
            // character starts: `->`, `::`, a namespace separator, the `$` of
            // a variable — which a client's word pattern does not read as a
            // word on its own, so it would never ask — and the `<?` of a
            // half-written open tag. Each is also an operator's character, so
            // `crate::completion::continues_a_trigger` answers a triggered
            // request only where the whole spelling was written. `-` is not
            // among them: it finishes nothing. The last four open and extend a
            // literal: a quote opens a path, a name or a string a completion
            // file offers values in, a `/` starts a path's next segment, and
            // `.`, `/` and `:` start the next segment of a value split on that
            // separator. A client offers nothing in a string unless asked, and
            // these are answered only inside such a literal.
            trigger_characters: Some(vec![
                ">".to_owned(),
                ":".to_owned(),
                "\\".to_owned(),
                "$".to_owned(),
                "?".to_owned(),
                "'".to_owned(),
                "\"".to_owned(),
                "/".to_owned(),
                ".".to_owned(),
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
        // No resolve provider: `crate::hints` answers a label and a position
        // and has nothing further to compute on demand, so declaring one would
        // promise a second round trip that only ever returns what the first
        // already carried.
        inlay_hint_provider: Some(OneOf::Left(true)),
        ..ServerCapabilities::default()
    }
}

/// Every capability this server declares, as the client reads it: the fields
/// [`server_capabilities`] can express, and the one it cannot.
///
/// `lsp_types` 0.97 carries LSP 3.17's three type-hierarchy requests, their
/// params, its options struct and the *client* capability for it — and its
/// `ServerCapabilities` has no `typeHierarchyProvider` member at all. So the
/// promise `rule:ide/five-features-are-one-reference-index`'s fourth reader
/// needs cannot be written in the struct above, and is written here on the
/// same terms as every other one. The day that crate grows the field, this
/// function's body is one line shorter and nothing else here moves.
#[must_use]
pub fn declared_capabilities(encoding: PositionEncodingKind) -> serde_json::Value {
    let mut declared = serde_json::to_value(server_capabilities(encoding))
        .expect("`ServerCapabilities` is a struct of serializable fields");
    if let Some(fields) = declared.as_object_mut() {
        fields.insert(
            "typeHierarchyProvider".to_owned(),
            serde_json::Value::Bool(true),
        );
    }
    declared
}

/// The whole `initialize` answer for the client that sent `params`, with the
/// encoding that answer declares.
///
/// JSON rather than an `lsp_types::InitializeResult` because
/// [`declared_capabilities`] is JSON for the reason given there, and the pair
/// rather than the JSON alone because every position this server counts from
/// here on is counted in the encoding this answer names — digging it back out
/// of a string key would be a second answer to what this function just
/// settled.
///
/// None of `nvs/redactions`, `nvs/regions`, `nvs/imports`, `nvs/importEdits`,
/// `nvs/checkWorkspace` and `nvs/directives` appears in it, and that is not an
/// omission: they are Novis's own requests
/// (`rule:ide/redaction-ranges-come-from-the-server`,
/// `rule:ide/a-template-region-gets-the-editors-services-and-formatter`,
/// `rule:ide/a-pasted-type-carries-its-use-line`,
/// `rule:ide/check-scope-defaults-to-the-workspace`,
/// `rule:ide/the-extension-claims-nvs-only`), LSP has no capability field for
/// any of them, and the client knows all six are available from `serverInfo`
/// naming this server at all.
///
/// **No formatting provider appears either, and that absence is a decision.**
/// `rule:tooling/fmt-is-never-a-diagnostic` gives a `.nvs` file exactly one
/// formatter, `nvs fmt`, run by the client — so declaring one here, even one
/// that answered with no edits, is what would put a second formatter inside the
/// template region the request above reports. `tests/handshake.rs` pins it.
#[must_use]
pub fn initialize_result(params: &InitializeParams) -> (PositionEncodingKind, serde_json::Value) {
    let negotiated = negotiate_encoding(&params.capabilities);
    let answer = serde_json::json!({
        "capabilities": declared_capabilities(negotiated.clone()),
        "serverInfo": server_info(),
    });
    (negotiated, answer)
}
