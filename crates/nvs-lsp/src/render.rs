//! Rendering one answer as the text a `.lspt` case freezes.
//!
//! `--EXPECT--` is exact, so the spelling it is compared against has to have
//! exactly one home (`rule:ide/the-rendering-has-one-home`). This is it: every
//! response kind the closed request set can produce comes through [`Response`],
//! and a case that seems to need a spelling of its own has found a gap here
//! rather than a licence to invent one.
//!
//! Two conventions run through every rendering, so a case reads the same way
//! whichever request produced it:
//!
//! - **An answer with nothing in it renders as `none`**, never as an empty
//!   string. An empty `--EXPECT--` cannot be told apart from a section someone
//!   forgot to fill in, and the difference between "the server answered
//!   nothing" and "nobody asked" is the whole content of some cases.
//! - **An absent optional field renders as `-`.** A hole is visible rather than
//!   silently closed up, because a rendering that drops a field turns a missing
//!   severity into a diff nobody can read.
//!
//! Positions are 1-based here and 0-based on the wire: `L:C` is what an editor
//! and a compiler diagnostic both show, and a case is read by a person. The
//! column is in the negotiated encoding's units, which is
//! `rule:ide/positions-have-one-home`'s business and not this module's — what
//! arrives is already converted.
//!
//! **Resolution is not rendering.** Where LSP answers a `Uri`, this module
//! takes the name the case wrote instead ([`Place`], [`Link`]): mapping a URI
//! back to `case.nvs` or `lib/user.nvs` needs the directory the runner
//! materialised the case into, which is the runner's alone. Nothing here reads
//! a file, resolves a path or decides an encoding.

use std::fmt;

use lsp_types::{
    CodeLens, CompletionItem, Diagnostic, DocumentSymbol, FoldingRange, Hover, HoverContents,
    InlayHint, InlayHintLabel, MarkedString, Position, Range, SelectionRange, SemanticToken,
};

use crate::case::Request;
use crate::{TOKEN_MODIFIERS, TOKEN_TYPES};

/// What an answer with nothing in it renders as.
const NONE: &str = "none";

/// What an optional field nobody set renders as.
const ABSENT: &str = "-";

/// The width the completion label column is padded to.
///
/// Fixed rather than computed from the rows: a width taken from the widest
/// entry means one completion added anywhere rewrites every line of a frozen
/// expectation, and `--EXPECT--` is not edited to make a case pass.
const LABEL_WIDTH: usize = 8;

/// The width the completion kind column is padded to, on the same terms.
const KIND_WIDTH: usize = 10;

/// How deep one level of the document outline is indented.
const INDENT: &str = "  ";

/// Where a definition is, with the file named as the case wrote it.
///
/// LSP answers a `Location`, whose `Uri` names the directory the runner
/// materialised the case into. Turning that back into `case.nvs` or
/// `lib/user.nvs` is resolution, so the runner does it and this module renders
/// the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The file, as the case spells it: [`crate::MAIN_PATH`] for the document
    /// under test, or an auxiliary file's own path.
    pub path: String,
    /// Where in it, on the wire's terms — 0-based, converted on the way out.
    pub position: Position,
}

/// One clickable path literal, with its target named as the case wrote it.
///
/// The target is a resolved path for the same reason [`Place`]'s is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The literal's own range in the document.
    pub range: Range,
    /// What it points at.
    pub target: String,
}

/// One range the client conceals, as `nvs/redactions` answers it.
///
/// The kind is an open string on the wire (`secretLiteral` today), so it is one
/// here too — ADR 0101 § 1 owns the roster, and a rendering that closed it
/// would have to change for a qualifier this module knows nothing about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redaction {
    /// The bytes concealed — a literal token or an interpolation slot, and
    /// never an identifier (`rule:ide/redaction-covers-bytes-only`).
    pub range: Range,
    /// Why they are concealed.
    pub kind: String,
}

/// One span of a document that is not Novis, and which service owns it.
///
/// **Two fields, and the second one is not an instruction.** A language names
/// the editor's own service — HTML, and the CSS and JavaScript it embeds — and
/// says nothing about what that service may then be asked for. Formatting is
/// what it may not be asked for
/// (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`), and
/// that is the client's registration rather than a field here: a region that
/// could carry a formatter is a `.nvs` file with two of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The bytes the named service answers inside.
    pub range: Range,
    /// Which service that is, as `crate::regions::HTML` spells it.
    pub language: String,
}

/// One fix an editor may offer, with the edit it would apply.
///
/// **One edit and not a list.** Every fix this server offers is a translation
/// of one [`nvs_diagnostics::Suggestion`]
/// (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`), and
/// a suggestion is one span and the text to put there. A fix wanting two edits
/// is one the checker would have to compute, which is the far side of the
/// boundary that rule draws. The html-literal refactor replaces one span too.
///
/// The kind is a string for [`Redaction`]'s reason turned around: LSP's own
/// kinds are an open hierarchy ([`crate::CODE_ACTION_KINDS`]). A fix's kind is
/// decided by what the client asked for rather than by the fix, and the
/// refactor's is always its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// What the client shows for it — the suggestion's own message, so what a
    /// terminal prints under a diagnostic and what a light bulb offers are one
    /// sentence written once.
    pub title: String,
    /// The kind it is filed under, which is what decides whether
    /// `editor.codeActionsOnSave` runs it.
    pub kind: String,
    /// The bytes it replaces.
    pub range: Range,
    /// What it puts there. Empty means a deletion.
    pub replacement: String,
}

/// One answer, in the shape the request that produced it answers.
///
/// One variant per request in `rule:ide/the-request-set-is-closed`'s list, and
/// [`Response::request`] maps each back — so a request whose answer nothing here
/// renders cannot be added without this enum saying so.
#[derive(Debug, Clone)]
pub enum Response {
    /// `textDocument/publishDiagnostics`.
    Diagnostics(Vec<Diagnostic>),
    /// `textDocument/hover`.
    Hover(Option<Hover>),
    /// `textDocument/definition`.
    Definition(Option<Place>),
    /// `textDocument/completion`, already filtered by the case's `prefix=` and
    /// `limit=`: this module renders what it is handed.
    Completion(Vec<CompletionItem>),
    /// `textDocument/semanticTokens/full`, delta-encoded as the wire carries
    /// it — decoding is part of the spelling, so no case does it by hand.
    SemanticTokens(Vec<SemanticToken>),
    /// `textDocument/documentSymbol`.
    DocumentSymbol(Vec<DocumentSymbol>),
    /// `textDocument/selectionRange`.
    SelectionRange(Option<SelectionRange>),
    /// `textDocument/foldingRange`.
    FoldingRange(Vec<FoldingRange>),
    /// `textDocument/documentLink`.
    DocumentLink(Vec<Link>),
    /// `textDocument/codeAction`.
    CodeAction(Vec<Action>),
    /// `textDocument/codeLens`, as the server answers it with
    /// `nvs.codeLens.enable` on. The refusal that setting buys is a `None`
    /// answer and not an empty list, and it is nothing this renders: no case
    /// can write a setting.
    CodeLens(Vec<CodeLens>),
    /// `textDocument/references`, with the declaration among the uses.
    ///
    /// A list and not an option, so the two answers the server tells apart on
    /// the wire — `null` for a cursor on no name it can follow, `[]` for a name
    /// nothing uses — arrive here as the same empty list and freeze as `none`.
    /// That difference is about which question was answerable and not about
    /// where a name is used, so it is pinned where it is visible: a `-p
    /// nvs-lsp` test reading the wire, in `tests/references.rs`.
    References(Vec<Place>),
    /// `textDocument/documentHighlight`, as places and not as ranges.
    ///
    /// The same shape [`Response::References`] freezes, because the answer is
    /// that query narrowed to one file and a case reads the two rows against
    /// each other: what a highlight case pins is *which* of the uses stayed in
    /// the list, and a second spelling would hide that behind a diff of
    /// formats. The end of a hit is dropped for the same reason a place carries
    /// no end anywhere else here — it is the name's own length, which the
    /// document already shows.
    DocumentHighlight(Vec<Place>),
    /// `textDocument/inlayHint`, as the walk produced it over the whole
    /// document.
    ///
    /// Every hint the document carries and not the ones a client can see: the
    /// range an editor asks about is narrowed in `crate::server`, and a case
    /// writes a document and a question rather than a viewport.
    Hints(Vec<InlayHint>),
    /// `nvs/redactions`.
    Redactions(Vec<Redaction>),
    /// `nvs/regions`.
    Regions(Vec<Region>),
}

impl Response {
    /// Which request this is the answer to.
    #[must_use]
    pub fn request(&self) -> Request {
        match self {
            Self::Diagnostics(_) => Request::Diagnostics,
            Self::Hover(_) => Request::Hover,
            Self::Definition(_) => Request::Definition,
            Self::Completion(_) => Request::Completion,
            Self::SemanticTokens(_) => Request::SemanticTokens,
            Self::DocumentSymbol(_) => Request::DocumentSymbol,
            Self::SelectionRange(_) => Request::SelectionRange,
            Self::FoldingRange(_) => Request::FoldingRange,
            Self::DocumentLink(_) => Request::DocumentLink,
            Self::CodeAction(_) => Request::CodeAction,
            Self::CodeLens(_) => Request::CodeLens,
            Self::References(_) => Request::References,
            Self::DocumentHighlight(_) => Request::DocumentHighlight,
            Self::Hints(_) => Request::InlayHint,
            Self::Redactions(_) => Request::Redactions,
            Self::Regions(_) => Request::Regions,
        }
    }

    /// The text a `.lspt` case freezes for this answer, ending in one newline.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Diagnostics(items) => lines(sorted(items.iter().map(diagnostic))),
            Self::Hover(answer) => answer.as_ref().map_or_else(text_of_none, hover),
            Self::Definition(answer) => lines(answer.iter().map(place).collect()),
            Self::Completion(items) => lines(sorted(items.iter().map(completion))),
            Self::SemanticTokens(tokens) => lines(semantic_tokens(tokens)),
            Self::DocumentSymbol(symbols) => {
                let mut out = Vec::new();
                outline(symbols, 0, &mut out);
                lines(out)
            }
            Self::SelectionRange(answer) => lines(answer.as_ref().map_or_else(Vec::new, ancestry)),
            Self::FoldingRange(items) => lines(items.iter().map(folding_range).collect()),
            Self::DocumentLink(items) => lines(items.iter().map(link).collect()),
            Self::CodeAction(items) => lines(sorted(items.iter().map(action))),
            Self::CodeLens(items) => lines(sorted(items.iter().map(lens))),
            Self::References(items) => lines(sorted(items.iter().map(reference))),
            Self::DocumentHighlight(items) => lines(sorted(items.iter().map(reference))),
            Self::Hints(items) => lines(sorted(items.iter().map(hint))),
            Self::Redactions(items) => lines(items.iter().map(redaction).collect()),
            Self::Regions(items) => lines(items.iter().map(region).collect()),
        }
    }
}

/// Joins rendered entries, or says the answer was empty.
fn lines(entries: Vec<String>) -> String {
    if entries.is_empty() {
        return text_of_none();
    }
    let mut out = entries.join("\n");
    out.push('\n');
    out
}

/// [`NONE`] as a whole rendering.
fn text_of_none() -> String {
    format!("{NONE}\n")
}

/// Sorts rendered entries by the key each carries, dropping the keys.
///
/// The key is what the rendering is sorted *by* — a position, a label — and the
/// entry itself breaks a tie, so two answers at one position freeze in an order
/// that does not depend on which walk produced them.
fn sorted<K: Ord>(entries: impl Iterator<Item = (K, String)>) -> Vec<String> {
    let mut entries: Vec<(K, String)> = entries.collect();
    entries.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
    entries.into_iter().map(|(_, text)| text).collect()
}

/// `L:C`, 1-based, from the wire's 0-based pair.
fn position(at: Position) -> String {
    format!("{}:{}", at.line + 1, at.character + 1)
}

/// `L:C-L:C`.
fn range(span: Range) -> String {
    format!("{}-{}", position(span.start), position(span.end))
}

/// The sort key of a range: where it starts, then where it ends.
fn span_key(span: Range) -> (u32, u32, u32, u32) {
    (
        span.start.line,
        span.start.character,
        span.end.line,
        span.end.character,
    )
}

/// An LSP kind's own name, with its leading letter lowered.
///
/// The names come from `lsp_types`' `Debug`, which its enum macro derives from
/// the constant names LSP itself specifies — so `EnumMember` renders as
/// `enumMember`, which is the spelling the protocol uses and the one
/// `rule:ide/novis-ships-names-not-colours` wants in a rendering. A value
/// outside the roster renders as its type and number rather than as a guess,
/// so a handler emitting one shows up as a diff instead of a plausible line.
fn spelled(kind: &impl fmt::Debug) -> String {
    let name = format!("{kind:?}");
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => name,
    }
}

/// An optional value, rendered or written as the absent marker.
fn or_absent<T>(value: Option<&T>, render: impl Fn(&T) -> String) -> String {
    value.map_or_else(|| ABSENT.to_owned(), render)
}

/// Pads `text` to `width`, always leaving at least one space after it.
///
/// A column that overflows pushes the rest of the line right rather than
/// running into it: the entry stays readable and, more to the point, stays
/// unambiguous to whoever is reading the diff.
fn column(text: &str, width: usize) -> String {
    if text.chars().count() < width {
        format!("{text:<width$}")
    } else {
        format!("{text} ")
    }
}

/// `L:C-L:C severity CODE message`.
fn diagnostic(item: &Diagnostic) -> ((u32, u32, u32, u32), String) {
    let severity = or_absent(item.severity.as_ref(), spelled);
    let code = or_absent(item.code.as_ref(), |code| match code {
        lsp_types::NumberOrString::Number(number) => number.to_string(),
        lsp_types::NumberOrString::String(text) => text.clone(),
    });
    // One entry is one line, so a message that wraps is written with its breaks
    // escaped rather than spilling into what reads as another diagnostic.
    let message = item.message.replace('\n', "\\n");
    (
        span_key(item.range),
        format!("{} {severity} {code} {message}", range(item.range)),
    )
}

/// A hover's markdown, verbatim.
fn hover(answer: &Hover) -> String {
    let body = match &answer.contents {
        HoverContents::Scalar(one) => marked(one),
        HoverContents::Array(many) => many.iter().map(marked).collect::<Vec<_>>().join("\n"),
        HoverContents::Markup(markup) => markup.value.clone(),
    };
    let body = body.trim_end();
    if body.is_empty() {
        return text_of_none();
    }
    format!("{body}\n")
}

/// One piece of `MarkedString` hover content.
///
/// The language-tagged form is written as the fenced block it means, because
/// that is what the client renders and a case freezes what a developer sees.
fn marked(piece: &MarkedString) -> String {
    match piece {
        MarkedString::String(text) => text.clone(),
        MarkedString::LanguageString(code) => {
            format!("```{}\n{}\n```", code.language, code.value)
        }
    }
}

/// `file:L:C`.
fn place(at: &Place) -> String {
    format!("{}:{}", at.path, position(at.position))
}

/// `file:L:C`, keyed by the file and then the position in it.
///
/// Sorted where the one place a `definition` answers is not, because this
/// answer is the index's declaration side and its occurrence side put end to
/// end across every file that holds a use. The order it arrives in is the
/// query's, and a case freezes *where* a name is used rather than in what
/// order a walk reached it.
fn reference(at: &Place) -> ((&str, u32, u32), String) {
    (
        (at.path.as_str(), at.position.line, at.position.character),
        place(at),
    )
}

/// The row as an editor shows it, keyed by label: the label with what is
/// written directly after it, the kind, and the text at the row's right — the
/// item's description where it carries label details, its detail otherwise.
/// What a client shows only in a panel, the detail behind a description and
/// the documentation `crate::card` resolves, is not a row and is not here.
fn completion(item: &CompletionItem) -> (String, String) {
    let kind = or_absent(item.kind.as_ref(), spelled);
    let (after, right) = match &item.label_details {
        Some(details) => (
            details.detail.clone().unwrap_or_default(),
            details.description.clone().unwrap_or_default(),
        ),
        None => (String::new(), item.detail.clone().unwrap_or_default()),
    };
    let text = format!(
        "{}{}{right}",
        column(&format!("{}{after}", item.label), LABEL_WIDTH),
        column(&kind, KIND_WIDTH)
    );
    (item.label.clone(), text.trim_end().to_owned())
}

/// Where each token starts, with the wire's deltas undone.
///
/// A delta is relative to the token before it, so nothing outside this module
/// can read a token's position without redoing this loop — and two copies of it
/// would disagree the first time a legend or an encoding moved. Both readers
/// are here: the rendering below, and [`crate::coverage`], which asks what
/// construct each name the server coloured belongs to.
pub(crate) fn absolute(tokens: &[SemanticToken]) -> Vec<Position> {
    let (mut line, mut character) = (0_u32, 0_u32);
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        line += token.delta_line;
        character = if token.delta_line == 0 {
            character + token.delta_start
        } else {
            token.delta_start
        };
        out.push(Position { line, character });
    }
    out
}

/// `L:C+len type modifiers`, one line per token, with the wire's deltas undone.
///
/// The deltas are relative to the token before, so a case freezing them raw
/// would be reading an encoding rather than an answer — and one token inserted
/// ahead of the interesting one would rewrite the whole expectation.
fn semantic_tokens(tokens: &[SemanticToken]) -> Vec<String> {
    let mut out = Vec::with_capacity(tokens.len());
    for (token, at) in tokens.iter().zip(absolute(tokens)) {
        let at = position(at);
        let kind = index(TOKEN_TYPES, token.token_type, "type");
        let modifiers = modifiers(token.token_modifiers_bitset);
        let entry = format!("{at}+{} {kind} {modifiers}", token.length);
        out.push(entry.trim_end().to_owned());
    }
    out
}

/// The name at `at` in a legend, or a marker naming the index nothing holds.
fn index(legend: &[impl AsStr], at: u32, what: &str) -> String {
    usize::try_from(at)
        .ok()
        .and_then(|at| legend.get(at))
        .map_or_else(|| format!("<{what} {at}>"), |name| name.as_str().to_owned())
}

/// The modifier names a bitset names, in the legend's own order.
fn modifiers(bits: u32) -> String {
    let mut names = Vec::new();
    for (at, modifier) in TOKEN_MODIFIERS.iter().enumerate() {
        let Ok(at) = u32::try_from(at) else { break };
        if bits & (1 << at) != 0 {
            names.push(modifier.as_str());
        }
    }
    // A bit outside the legend is a handler bug, and one this says out loud
    // rather than rendering the same line a correct answer would produce.
    let known: u32 = u32::try_from(TOKEN_MODIFIERS.len())
        .ok()
        .and_then(|width| 1_u32.checked_shl(width))
        .map_or(u32::MAX, |past| past - 1);
    if bits & !known != 0 {
        names.push("<unknown>");
    }
    names.join(",")
}

/// One symbol per line, indented by its depth, deepest last within a parent.
///
/// Document order, not sorted: an outline is what the file looks like, and
/// sorting it would hide the one thing it is read for.
fn outline(symbols: &[DocumentSymbol], depth: usize, out: &mut Vec<String>) {
    for symbol in symbols {
        out.push(format!(
            "{}{} {}",
            INDENT.repeat(depth),
            symbol.name,
            spelled(&symbol.kind)
        ));
        if let Some(children) = &symbol.children {
            outline(children, depth + 1, out);
        }
    }
}

/// The selection chain, innermost first — the response is a linked list of
/// parents, and each link is one expand-selection step.
fn ancestry(innermost: &SelectionRange) -> Vec<String> {
    let mut out = vec![range(innermost.range)];
    let mut walk = innermost.parent.as_deref();
    while let Some(step) = walk {
        out.push(range(step.range));
        walk = step.parent.as_deref();
    }
    out
}

/// `L-L kind`, in lines: a folding range is line-granular, which is exactly why
/// `rule:ide/redaction-covers-bytes-only` cannot be built on one.
fn folding_range(item: &FoldingRange) -> String {
    let kind = or_absent(item.kind.as_ref(), spelled);
    format!("{}-{} {kind}", item.start_line + 1, item.end_line + 1)
}

/// `L:C-L:C -> target`.
fn link(item: &Link) -> String {
    format!("{} -> {}", range(item.range), item.target)
}

/// `L:C-L:C kind`.
fn redaction(item: &Redaction) -> String {
    format!("{} {}", range(item.range), item.kind)
}

/// `L:C-L:C language`, which is [`redaction`]'s shape because it is the same
/// claim: a span of the document, and one word saying what it is.
fn region(item: &Region) -> String {
    format!("{} {}", range(item.range), item.language)
}

/// `L:C kind label`, keyed by where the hint is drawn.
///
/// The padding either side of a hint is not rendered: it is the gap an editor
/// leaves between the hint and the code beside it, which is a drawing question
/// the client answers. What a case freezes is where a hint sits, what it says
/// and which kind it is, which is the whole of what this server decided.
fn hint(item: &InlayHint) -> ((u32, u32), String) {
    let kind = or_absent(item.kind.as_ref(), spelled);
    let label = match &item.label {
        InlayHintLabel::String(text) => text.clone(),
        InlayHintLabel::LabelParts(parts) => parts.iter().map(|part| part.value.as_str()).collect(),
    };
    (
        (item.position.line, item.position.character),
        format!("{} {kind} {label}", position(item.position)),
    )
}

/// `L:C title`, keyed by the bytes the lens is anchored to.
///
/// Where those bytes start and not how far they run: they are the
/// declaration's own name, whose extent `documentSymbol` already freezes, and
/// two lenses above two members written on one line have to be two entries a
/// reader can tell apart. A lens whose command nobody set renders [`ABSENT`],
/// which is also what an editor shows above one.
fn lens(item: &CodeLens) -> ((u32, u32, u32, u32), String) {
    let title = or_absent(item.command.as_ref(), |command| command.title.clone());
    (
        span_key(item.range),
        format!("{} {title}", position(item.range.start)),
    )
}

/// `L:C-L:C kind title -> "replacement"`.
///
/// The replacement is the one field a rendering quotes, and it is quoted
/// because it is the one field whose exact bytes are the answer: a deletion is
/// empty, and a fix that inserts a space is a fix an expectation would
/// otherwise freeze as a line an editor's whitespace trim silently corrects.
/// Both it and the title write their line breaks escaped, on [`diagnostic`]'s
/// terms — one entry is one line, and a fix spanning lines is not two actions.
fn action(item: &Action) -> ((u32, u32, u32, u32), String) {
    let title = item.title.replace('\n', "\\n");
    let replacement = item.replacement.replace('\n', "\\n");
    (
        span_key(item.range),
        format!(
            "{} {} {title} -> \"{replacement}\"",
            range(item.range),
            item.kind
        ),
    )
}

/// What a legend entry answers when asked for its name.
///
/// `SemanticTokenType` and `SemanticTokenModifier` each have an inherent
/// `as_str`, and neither implements a trait that would let one function take
/// both — so this is that trait, and it exists for [`index`] alone.
trait AsStr {
    /// The name this entry carries on the wire.
    fn as_str(&self) -> &str;
}

impl AsStr for lsp_types::SemanticTokenType {
    fn as_str(&self) -> &str {
        Self::as_str(self)
    }
}

impl AsStr for lsp_types::SemanticTokenModifier {
    fn as_str(&self) -> &str {
        Self::as_str(self)
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{
        CompletionItemKind, DiagnosticSeverity, FoldingRangeKind, InlayHintKind, MarkupContent,
        MarkupKind, NumberOrString, SemanticTokenType, SymbolKind,
    };

    use super::*;

    fn at(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn span(line: u32, from: u32, to: u32) -> Range {
        Range {
            start: at(line, from),
            end: at(line, to),
        }
    }

    fn diagnostic_of(line: u32, code: &str, message: &str) -> Diagnostic {
        Diagnostic {
            range: span(line, 0, 4),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(code.to_owned())),
            message: message.to_owned(),
            ..Diagnostic::default()
        }
    }

    fn completion_of(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_owned(),
            kind: Some(kind),
            detail: Some(detail.to_owned()),
            ..CompletionItem::default()
        }
    }

    /// `DocumentSymbol` carries LSP's own deprecated `deprecated` field, so
    /// every literal of one needs this; the crate forbids `unsafe` and warns on
    /// everything else, and this is the one allowance a test here takes.
    #[expect(
        deprecated,
        reason = "lsp_types::DocumentSymbol::deprecated is LSP's own field"
    )]
    fn symbol_of(name: &str, kind: SymbolKind, children: Vec<DocumentSymbol>) -> DocumentSymbol {
        DocumentSymbol {
            name: name.to_owned(),
            detail: None,
            kind,
            tags: None,
            deprecated: None,
            range: span(0, 0, 1),
            selection_range: span(0, 0, 1),
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        }
    }

    /// One answer per request in the closed set, each with something in it.
    fn answers() -> Vec<Response> {
        vec![
            Response::Diagnostics(vec![diagnostic_of(1, "E0102", "expected an expression")]),
            Response::Hover(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "`string`".to_owned(),
                }),
                range: None,
            })),
            Response::Definition(Some(Place {
                path: "lib/user.nvs".to_owned(),
                position: at(1, 35),
            })),
            Response::Completion(vec![completion_of(
                "greet",
                CompletionItemKind::METHOD,
                "(): string",
            )]),
            Response::SemanticTokens(vec![SemanticToken {
                delta_line: 1,
                delta_start: 6,
                length: 4,
                token_type: 1,
                token_modifiers_bitset: 0,
            }]),
            Response::DocumentSymbol(vec![symbol_of("User", SymbolKind::CLASS, Vec::new())]),
            Response::SelectionRange(Some(SelectionRange {
                range: span(2, 0, 2),
                parent: None,
            })),
            Response::FoldingRange(vec![FoldingRange {
                start_line: 1,
                end_line: 4,
                kind: Some(FoldingRangeKind::Comment),
                ..FoldingRange::default()
            }]),
            Response::DocumentLink(vec![Link {
                range: span(1, 8, 24),
                target: "lib/user.nvs".to_owned(),
            }]),
            Response::CodeAction(vec![Action {
                title: "rename to `UserAccount`".to_owned(),
                kind: "quickfix".to_owned(),
                range: span(1, 6, 18),
                replacement: "UserAccount".to_owned(),
            }]),
            Response::CodeLens(vec![CodeLens {
                range: span(1, 6, 10),
                command: Some(lsp_types::Command {
                    title: "2 references".to_owned(),
                    command: String::new(),
                    arguments: None,
                }),
                data: None,
            }]),
            // Out of order and across two files, so the sort the rendering
            // applies is visible in what it produces.
            Response::References(vec![
                Place {
                    path: "lib/user.nvs".to_owned(),
                    position: at(1, 6),
                },
                Place {
                    path: "case.nvs".to_owned(),
                    position: at(6, 13),
                },
            ]),
            // One file, because that is the whole of what narrows this answer
            // from the one above it.
            Response::DocumentHighlight(vec![
                Place {
                    path: "case.nvs".to_owned(),
                    position: at(6, 13),
                },
                Place {
                    path: "case.nvs".to_owned(),
                    position: at(1, 6),
                },
            ]),
            Response::Hints(vec![
                hinted(at(1, 10), ": int", InlayHintKind::TYPE),
                hinted(at(2, 15), "count:", InlayHintKind::PARAMETER),
            ]),
            Response::Redactions(vec![Redaction {
                range: span(3, 18, 30),
                kind: "secretLiteral".to_owned(),
            }]),
            Response::Regions(vec![Region {
                range: span(4, 0, 12),
                language: "html".to_owned(),
            }]),
        ]
    }

    /// One hint, with every field a case cannot see left unset.
    fn hinted(position: Position, label: &str, kind: InlayHintKind) -> InlayHint {
        InlayHint {
            position,
            label: InlayHintLabel::String(label.to_owned()),
            kind: Some(kind),
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: Some(true),
            data: None,
        }
    }

    /// Every answer with nothing in it, one per request, in the same order.
    fn empty_answers() -> Vec<Response> {
        vec![
            Response::Diagnostics(Vec::new()),
            Response::Hover(None),
            Response::Definition(None),
            Response::Completion(Vec::new()),
            Response::SemanticTokens(Vec::new()),
            Response::DocumentSymbol(Vec::new()),
            Response::SelectionRange(None),
            Response::FoldingRange(Vec::new()),
            Response::DocumentLink(Vec::new()),
            Response::CodeAction(Vec::new()),
            Response::CodeLens(Vec::new()),
            Response::References(Vec::new()),
            Response::DocumentHighlight(Vec::new()),
            Response::Hints(Vec::new()),
            Response::Redactions(Vec::new()),
            Response::Regions(Vec::new()),
        ]
    }

    #[test]
    fn every_response_kind_renders_through_one_module() {
        // The set is closed and this module covers it: a request whose answer
        // nothing here renders fails on this line rather than on the day a case
        // invents a spelling for it (`rule:ide/the-rendering-has-one-home`).
        let covered: Vec<Request> = answers().iter().map(Response::request).collect();
        assert_eq!(covered, Request::ALL);
        assert_eq!(
            empty_answers()
                .iter()
                .map(Response::request)
                .collect::<Vec<_>>(),
            Request::ALL
        );

        // Every rendering is at least one line and ends in exactly one newline,
        // because `--EXPECT--`'s body does.
        for answer in answers() {
            let text = answer.render();
            assert!(text.ends_with('\n'), "{:?}: {text:?}", answer.request());
            assert!(!text.ends_with("\n\n"), "{:?}: {text:?}", answer.request());
            assert!(!text.starts_with('\n'), "{:?}: {text:?}", answer.request());
        }

        // An answer with nothing in it says so, so a frozen expectation is
        // never an empty section that could equally be one nobody filled in.
        for answer in empty_answers() {
            assert_eq!(answer.render(), "none\n", "{:?}", answer.request());
        }

        // The spellings themselves, one per kind, as ADR 0099 § 5 writes them.
        assert_eq!(
            Response::Diagnostics(vec![
                diagnostic_of(3, "E0301", "`$x` is assigned to but was never declared"),
                diagnostic_of(1, "E0102", "expected an expression"),
            ])
            .render(),
            "2:1-2:5 error E0102 expected an expression\n\
             4:1-4:5 error E0301 `$x` is assigned to but was never declared\n"
        );
        assert_eq!(
            Response::Hover(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::LanguageString(
                    lsp_types::LanguageString {
                        language: "nvs".to_owned(),
                        value: "function greet(): string".to_owned(),
                    }
                )),
                range: None,
            }))
            .render(),
            "```nvs\nfunction greet(): string\n```\n"
        );
        assert_eq!(
            Response::Definition(Some(Place {
                path: "lib/user.nvs".to_owned(),
                position: at(1, 35),
            }))
            .render(),
            "lib/user.nvs:2:36\n"
        );
        assert_eq!(
            Response::DocumentSymbol(vec![symbol_of(
                "User",
                SymbolKind::CLASS,
                vec![
                    symbol_of("$name", SymbolKind::PROPERTY, Vec::new()),
                    symbol_of("greet", SymbolKind::METHOD, Vec::new()),
                ],
            )])
            .render(),
            "User class\n  $name property\n  greet method\n"
        );
        assert_eq!(
            Response::SelectionRange(Some(SelectionRange {
                range: span(2, 0, 6),
                parent: Some(Box::new(SelectionRange {
                    range: span(2, 0, 12),
                    parent: None,
                })),
            }))
            .render(),
            "3:1-3:7\n3:1-3:13\n"
        );
        assert_eq!(
            Response::FoldingRange(vec![FoldingRange {
                start_line: 1,
                end_line: 4,
                kind: None,
                ..FoldingRange::default()
            }])
            .render(),
            "2-5 -\n"
        );
        assert_eq!(
            Response::DocumentLink(vec![Link {
                range: span(1, 8, 24),
                target: "lib/user.nvs".to_owned(),
            }])
            .render(),
            "2:9-2:25 -> lib/user.nvs\n"
        );
        assert_eq!(
            Response::Redactions(vec![Redaction {
                range: span(3, 18, 30),
                kind: "secretLiteral".to_owned(),
            }])
            .render(),
            "4:19-4:31 secretLiteral\n"
        );
        assert_eq!(
            Response::Regions(vec![Region {
                range: span(4, 0, 12),
                language: "html".to_owned(),
            }])
            .render(),
            "5:1-5:13 html\n"
        );

        // A highlight list is spelled and sorted exactly as a reference list
        // is, one file's worth of it, so the two rows of the matrix can be read
        // against each other.
        assert_eq!(
            Response::DocumentHighlight(vec![
                Place {
                    path: "case.nvs".to_owned(),
                    position: at(6, 13),
                },
                Place {
                    path: "case.nvs".to_owned(),
                    position: at(1, 6),
                },
            ])
            .render(),
            "case.nvs:2:7\ncase.nvs:7:14\n"
        );

        // A hint says where it is drawn, which kind it is and the text itself,
        // and the padding an editor draws around it is not part of that — a
        // case that could freeze a space would be freezing the client.
        assert_eq!(
            Response::Hints(vec![
                hinted(at(3, 12), "left:", InlayHintKind::PARAMETER),
                hinted(at(1, 10), ": int", InlayHintKind::TYPE),
            ])
            .render(),
            "2:11 type : int\n4:13 parameter left:\n"
        );

        // Two actions at one cursor sort by where they edit, and a deletion is
        // the line whose replacement is nothing at all.
        assert_eq!(
            Response::CodeAction(vec![
                Action {
                    title: "drop it".to_owned(),
                    kind: "quickfix".to_owned(),
                    range: span(2, 4, 9),
                    replacement: String::new(),
                },
                Action {
                    title: "rename to `UserAccount`".to_owned(),
                    kind: "quickfix".to_owned(),
                    range: span(1, 6, 18),
                    replacement: "UserAccount".to_owned(),
                },
            ])
            .render(),
            "2:7-2:19 quickfix rename to `UserAccount` -> \"UserAccount\"\n\
             3:5-3:10 quickfix drop it -> \"\"\n"
        );

        // The completion rendering is the one the format's own worked example
        // freezes, columns included, and it is sorted by label rather than by
        // whatever order the walk produced.
        assert_eq!(
            Response::Completion(vec![
                completion_of("name", CompletionItemKind::PROPERTY, "string"),
                completion_of("greet", CompletionItemKind::METHOD, "(): string"),
            ])
            .render(),
            "greet   method    (): string\nname    property  string\n"
        );

        // A field nobody set is a visible hole, not a closed-up line.
        assert_eq!(
            Response::Completion(vec![CompletionItem {
                label: "$total".to_owned(),
                ..CompletionItem::default()
            }])
            .render(),
            "$total  -\n"
        );

        // Semantic tokens arrive delta-encoded, and undoing that is part of the
        // spelling: a case freezes where a token is, never how the wire got
        // there. The names come from the legend `initialize` declared.
        let tokens = vec![
            SemanticToken {
                delta_line: 1,
                delta_start: 6,
                length: 4,
                token_type: 1,
                token_modifiers_bitset: 0,
            },
            SemanticToken {
                delta_line: 0,
                delta_start: 6,
                length: 5,
                token_type: 6,
                token_modifiers_bitset: 0b11,
            },
            SemanticToken {
                delta_line: 2,
                delta_start: 2,
                length: 3,
                token_type: 7,
                token_modifiers_bitset: 0,
            },
        ];
        let class = TOKEN_TYPES
            .iter()
            .position(|it| *it == SemanticTokenType::CLASS)
            .expect("the legend carries `class`");
        assert_eq!(class, 1, "the sample tokens name the legend by index");
        let (first, second) = (
            TOKEN_MODIFIERS[0].as_str().to_owned(),
            TOKEN_MODIFIERS[1].as_str().to_owned(),
        );
        assert_eq!(
            Response::SemanticTokens(tokens).render(),
            format!("2:7+4 class\n2:13+5 method {first},{second}\n4:3+3 property\n",)
        );

        // An index or a bit the legend does not hold is said out loud, because
        // a rendering that guessed would be indistinguishable from a right one.
        assert_eq!(
            Response::SemanticTokens(vec![SemanticToken {
                delta_line: 0,
                delta_start: 0,
                length: 1,
                token_type: 99,
                token_modifiers_bitset: 1 << 30,
            }])
            .render(),
            "1:1+1 <type 99> <unknown>\n"
        );

        // A message that wraps stays one entry, so the line count of a
        // rendering is the number of things the server answered.
        assert_eq!(
            Response::Diagnostics(vec![diagnostic_of(0, "E0102", "one\ntwo")]).render(),
            "1:1-1:5 error E0102 one\\ntwo\n"
        );
    }
}
