//! What a copied range imports, and the `use` lines a paste of it needs.
//!
//! `nvs/imports` and `nvs/importEdits` are the third and fourth requests of
//! Novis's own (`rule:ide/the-request-set-is-closed`), and together they are
//! `rule:ide/a-pasted-type-carries-its-use-line`: a type name copied out of
//! one file resolves in that file through its `use` lines and its namespace,
//! and pasted into another it resolves through nothing until somebody writes
//! the import again. The editor asks the first at copy time, in the source
//! document, and keeps the answer on the clipboard beside the text; it asks
//! the second at paste time, in the destination, and applies the edits with
//! the paste.
//!
//! **Two requests and not one, because the two documents are open at two
//! different moments.** A paste may land minutes after the copy, in another
//! window, with the source closed or edited since; what the copied text meant
//! is a fact about the source *as it was*, and the only moment the server can
//! read it is the copy. The clipboard is where that fact travels, which is
//! also how an editor that copies from one instance and pastes into another
//! still gets the imports.
//!
//! **The answer is what the source resolved, never what the text looks like.**
//! [`in_range`] walks the names the range writes — the receiver of a `::`
//! access, the class after `new`, every name in an annotation — and puts each
//! through `nvs_hir::resolve_ref` with the namespace and the imports in force
//! at that position, which is the same lookup `definition` follows a click
//! with. A name that resolves to nothing declared is not carried: an import
//! for a class nobody declares fixes nothing. A qualified name is carried as
//! written and is never imported, because it is absolute
//! (`rule:statements/a-qualified-name-is-absolute`) and means the same thing
//! everywhere.
//!
//! **The edits are the completion's own `use` line, written for several names
//! at once.** [`edits`] asks what each carried name would have to be spelled
//! as at the destination — `completion`'s `written_as` — and imports only a
//! name that would otherwise not resolve as it was written, under a short name
//! nothing at the destination already answers to
//! (`completion::taken_short_names`), at the one place `nvs_hir::import_site`
//! puts every `use` line this server writes. A destination with no such place
//! gets no edit and the paste is plain text, which is what it was before.
//!
//! What it spends is one analysis of the source at copy time and one of the
//! destination at paste time, both of documents the server already holds, and
//! nothing between the two.

use lsp_types::{Position, Range, TextDocumentIdentifier, TextEdit};
use nvs_diagnostics::{BytePos, PositionEncoding, Span};
use nvs_hir::QName;
use nvs_stdlib::registry;
use nvs_syntax::ast::Stmt;
use nvs_syntax::walk;
use rustc_hash::FxHashMap;

use crate::completion::{taken_short_names, written_as};
use crate::definition::{imports_of, namespace_at, resolved_name, type_names, written_types_in};
use crate::document::Analysed;
use crate::index::SymbolIndex;
use crate::position::range_at;

/// The method the copy-time request is asked under.
///
/// Namespaced rather than `textDocument/`-prefixed for
/// [`crate::redactions::METHOD`]'s reason: it is not LSP's, and the prefix is
/// what says which one an editor is looking at.
pub const METHOD: &str = "nvs/imports";

/// The method the paste-time request is asked under.
pub const EDITS_METHOD: &str = "nvs/importEdits";

/// One type name a range wrote, and what it resolved to.
///
/// Both spellings travel because the paste needs both: `written` is what the
/// pasted text will say, and `symbol` is what a `use` line has to name for it
/// to keep meaning the same thing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import {
    /// The fully-qualified name the written one resolved to, spelled as source
    /// writes it — `Core\Request`.
    pub symbol: String,
    /// The name as the range wrote it — `Request`.
    pub written: String,
}

impl Import {
    /// This import as the wire carries it.
    #[must_use]
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({ "symbol": self.symbol, "written": self.written })
    }

    /// One import read off the wire.
    ///
    /// # Errors
    ///
    /// The `serde_json` error for a value that is not an object with a string
    /// `symbol` and a string `written`.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::from_value(value)?;
        Ok(Self {
            symbol: serde_json::from_value(fields.remove("symbol").unwrap_or_default())?,
            written: serde_json::from_value(fields.remove("written").unwrap_or_default())?,
        })
    }
}

/// What `nvs/imports` carries: the document, and the range copied out of it.
#[derive(Debug, Clone)]
pub struct Params {
    /// The document the range is in.
    pub text_document: TextDocumentIdentifier,
    /// The copied range.
    pub range: Range,
}

impl Params {
    /// Read one request's params off the wire, field by field on
    /// [`crate::regions::Params::from_value`]'s terms.
    ///
    /// # Errors
    ///
    /// The `serde_json` error for params that are not an object, or whose
    /// `textDocument` or `range` is missing or malformed. The server turns one
    /// into `InvalidParams`.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::from_value(value)?;
        Ok(Self {
            text_document: serde_json::from_value(
                fields.remove("textDocument").unwrap_or_default(),
            )?,
            range: serde_json::from_value(fields.remove("range").unwrap_or_default())?,
        })
    }
}

/// What `nvs/importEdits` carries: the destination, where the paste lands in
/// it, and the imports `nvs/imports` answered for the copied text.
#[derive(Debug, Clone)]
pub struct EditsParams {
    /// The document being pasted into.
    pub text_document: TextDocumentIdentifier,
    /// Where the pasted text starts.
    pub position: Position,
    /// What the copied text imported where it was copied from.
    pub imports: Vec<Import>,
}

impl EditsParams {
    /// Read one request's params off the wire.
    ///
    /// # Errors
    ///
    /// The `serde_json` error for params that are not an object, or whose
    /// `textDocument`, `position` or `imports` is missing or malformed.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::from_value(value)?;
        let imports: Vec<serde_json::Value> =
            serde_json::from_value(fields.remove("imports").unwrap_or_default())?;
        Ok(Self {
            text_document: serde_json::from_value(
                fields.remove("textDocument").unwrap_or_default(),
            )?,
            position: serde_json::from_value(fields.remove("position").unwrap_or_default())?,
            imports: imports
                .into_iter()
                .map(Import::from_value)
                .collect::<Result<_, _>>()?,
        })
    }
}

/// The entry document's own top-level statements, or `None` for an analysis
/// that never parsed it.
pub(crate) fn entry_stmts(analysed: &Analysed) -> Option<&[Stmt]> {
    analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
        .map(|loaded| loaded.stmts.as_slice())
}

/// The node kinds `nvs_syntax::walk` gives a written class name of its own:
/// the receiver of a `::` access, and the class after `new`.
const NAMED: &[&str] = &["ConstFetch", "New"];

/// Every type name written inside `[start, end]` of the entry document, with
/// what each resolved to, sorted and each named once. The module doc says
/// which names, and why a qualified or an undeclared one is left out.
#[must_use]
pub fn in_range(analysed: &Analysed, start: BytePos, end: BytePos) -> Vec<Import> {
    let Some(stmts) = entry_stmts(analysed) else {
        return Vec::new();
    };
    let mut spans: Vec<Span> = Vec::new();
    for root in walk::of_stmts(stmts) {
        for node in std::iter::once(&root).chain(root.descendants()) {
            if NAMED.contains(&node.kind)
                && let Some(name) = node.name
            {
                spans.push(name);
            }
        }
    }
    for root in written_types_in(stmts, start, end) {
        type_names(root, &mut spans);
    }
    let text = analysed.map.file(analysed.entry).text();
    let mut found: Vec<Import> = spans
        .into_iter()
        .filter(|span| start <= span.start && span.end <= end)
        .filter_map(|span| {
            let written = text.get(span.range())?;
            if written.contains('\\') {
                return None;
            }
            let symbol = resolved_name(analysed, written, span.start)?;
            declared(analysed, &symbol).then(|| Import {
                symbol: symbol.to_string(),
                written: written.to_owned(),
            })
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Whether `symbol` names a type somebody declares: one the analysis
/// resolved, or one the `Core` registry holds.
fn declared(analysed: &Analysed, symbol: &QName) -> bool {
    analysed.module.symbols.get(symbol).is_some()
        || registry::type_names()
            .iter()
            .any(|core| QName::parse(core) == *symbol)
}

/// The edits that make every one of `imports` resolve as it was written at
/// `offset` of the entry document, which is at most one: the `use` lines the
/// destination lacks, together, at `nvs_hir::import_site`'s place. Empty where
/// every name already resolves there, where none can be imported under its
/// short name, or where the destination has no place to write one.
#[must_use]
pub fn edits(
    analysed: &Analysed,
    symbols: &SymbolIndex,
    offset: BytePos,
    imports: &[Import],
    encoding: PositionEncoding,
) -> Vec<TextEdit> {
    let file = analysed.map.file(analysed.entry);
    let Some(site) =
        entry_stmts(analysed).and_then(|stmts| nvs_hir::import_site(stmts, file, offset))
    else {
        return Vec::new();
    };
    let in_force = imports_of(analysed);
    let short: FxHashMap<String, String> = in_force
        .iter()
        .map(|(name, target)| (target.to_string(), name.clone()))
        .collect();
    let here = namespace_at(analysed, offset);
    let mut taken = taken_short_names(symbols, &in_force, &here);
    let mut needed: Vec<QName> = Vec::new();
    let mut sorted: Vec<&Import> = imports.iter().collect();
    sorted.sort();
    sorted.dedup();
    for import in sorted {
        if written_as(&import.symbol, &here, &short) == import.written {
            continue;
        }
        let last = import.symbol.rsplit('\\').next().unwrap_or(&import.symbol);
        if import.written != last || taken.contains(last) {
            continue;
        }
        taken.insert(last.to_owned());
        needed.push(QName::parse(&import.symbol));
    }
    if needed.is_empty() {
        return Vec::new();
    }
    vec![TextEdit {
        range: range_at(file, site.span(analysed.entry), encoding),
        new_text: site.use_lines(needed.iter()),
    }]
}
