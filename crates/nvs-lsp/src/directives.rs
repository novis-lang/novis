//! Completion inside a file named `nvs.toml`: the keys of the block the cursor
//! is in, and the block headers after a `[`.
//!
//! `nvs/directives` is the sixth request of Novis's own
//! (`rule:ide/the-request-set-is-closed`), and it is the one completion
//! `rule:ide/the-extension-claims-nvs-only` lets the extension offer outside a
//! `.nvs` file. The extension registers a provider for files named `nvs.toml`
//! and claims no TOML language, so the editor merges this answer with whatever
//! a TOML extension offers. The document is never opened on the server: the
//! client sends its text with the cursor, and nothing is analysed or published
//! for it.
//!
//! **The table is the default file the binary ships**
//! ([`nvs_config::default_file`]). It writes every leaf key the parser accepts,
//! once and commented out, under the header of its block and below the comment
//! saying what it does, and `bun nv directives --check-template` fails the build
//! when the file and the parser's tree disagree. So reading the file is reading
//! the parser's key set, with the text an operator reads beside each key, and
//! there is no list here to drift from either. `tests/directives.rs` holds the
//! other direction: every key offered under a block parses under it.
//!
//! **A block whose name is the operator's** — `[db.<name>]`, `[mail.<name>]`,
//! `[storage.<name>]` — is written in the default file under one example name.
//! [`NAMED`] maps any other name onto that example, and the same test parses
//! each example's keys under another name.
//!
//! **What is offered, and where.** After a `[` or `[[` at the start of a line,
//! the headers the file writes with that many brackets. On a line that is
//! still a bare key, the keys of the block of the nearest live header above
//! the cursor, minus the ones that section already sets. Nothing in a comment,
//! after an `=`, or above the first header: the root table holds no key.
//!
//! What it spends is the parsed table, built on the first request and kept for
//! the life of the process: one entry per setting line of the default file,
//! borrowing its text, and one owned comment per entry.

use std::sync::OnceLock;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, InsertTextFormat,
    MarkupContent, MarkupKind, Position, Range, TextEdit,
};
use nvs_diagnostics::{BytePos, PositionEncoding, SourceFile, SourceMap};

use crate::position::{offset_at, position_at};

/// The method the request is asked under, namespaced for
/// [`crate::redactions::METHOD`]'s reason.
pub const METHOD: &str = "nvs/directives";

/// The blocks an operator names, as the prefix and the example name the
/// default file writes the block under.
pub const NAMED: &[(&str, &str)] = &[("db", "main"), ("mail", "default"), ("storage", "local")];

/// One block header the default file writes, live or commented out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The dotted name between the brackets.
    pub name: &'static str,
    /// Whether it is an array of tables, `[[name]]`.
    pub array: bool,
}

/// One setting line of the default file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The dotted name of the header the line sits under.
    pub block: &'static str,
    /// The key, as it is written before the `=`.
    pub key: &'static str,
    /// The value the line shows.
    pub value: &'static str,
    /// The note that ends the line: `default`, `default: …` or `example`.
    pub note: &'static str,
    /// The comment above the line, without its `# ` markers.
    pub doc: String,
}

/// The default file, read as headers and settings in the order it writes them.
#[derive(Debug, Default)]
pub struct Table {
    /// Every header, once each.
    pub headers: Vec<Header>,
    /// Every setting line.
    pub entries: Vec<Entry>,
}

/// The table, parsed from the default file on first use.
#[must_use]
pub fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| parse(nvs_config::default_file()))
}

/// Read a default file into its headers and settings.
///
/// A line is one of four things. A header is `[name]` or `[[name]]`, with or
/// without one `#` in front. A setting is `#key = value  # note`, with no space
/// after the `#`. A comment is a `#` followed by a space or nothing, and the
/// run of them closest above a setting is that setting's text. Anything else —
/// a blank line, a rule of dashes — ends a comment run.
#[must_use]
pub fn parse(file: &'static str) -> Table {
    let mut table = Table::default();
    let mut block = "";
    let mut doc = String::new();
    let mut in_comment = false;
    for line in file.lines() {
        let line = line.trim_end();
        if let Some(header) = header_of(line.strip_prefix('#').unwrap_or(line)) {
            block = header.name;
            if !table.headers.contains(&header) {
                table.headers.push(header);
            }
            doc.clear();
            in_comment = false;
        } else if let Some((key, value, note)) = setting_of(line) {
            table.entries.push(Entry {
                block,
                key,
                value,
                note,
                doc: doc.clone(),
            });
            in_comment = false;
        } else if let Some(text) = comment_of(line) {
            if !in_comment {
                doc.clear();
            }
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(text);
            in_comment = true;
        } else {
            in_comment = false;
        }
    }
    table
}

/// `[name]` or `[[name]]`, read off a line with any leading `#` already gone.
fn header_of(line: &'static str) -> Option<Header> {
    let (name, array) = match line.strip_prefix("[[") {
        Some(rest) => (rest.strip_suffix("]]")?, true),
        None => (line.strip_prefix('[')?.strip_suffix(']')?, false),
    };
    (!name.is_empty() && name.chars().all(|c| key_char(c) || c == '.'))
        .then_some(Header { name, array })
}

/// `#key = value  # note`, as its three parts. The key may be dotted, as `grants.read` is under
/// `[[extension]]`.
fn setting_of(line: &'static str) -> Option<(&'static str, &'static str, &'static str)> {
    let rest = line.strip_prefix('#')?;
    let (key, rest) = rest.split_once('=')?;
    let key = key.trim_end();
    if key.is_empty() || !key.chars().all(|c| key_char(c) || c == '.') {
        return None;
    }
    let at = ["# default", "# example"]
        .iter()
        .filter_map(|note| rest.rfind(note))
        .max()?;
    Some((key, rest[..at].trim(), &rest[at + 2..]))
}

/// The text of a comment line, or `None` for a line that is not one or is a
/// rule of dashes.
fn comment_of(line: &'static str) -> Option<&'static str> {
    let rest = line.strip_prefix('#')?;
    if !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    let text = rest.trim();
    (!text.starts_with("---")).then_some(text)
}

/// A character a bare TOML key may hold.
fn key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// The block in `table` that a header named `name` takes its keys from:
/// itself, or for a block the operator named, the example [`NAMED`] gives.
#[must_use]
pub fn resolve(table: &Table, name: &str) -> Option<String> {
    if table.headers.iter().any(|header| header.name == name) {
        return Some(name.to_owned());
    }
    let (prefix, rest) = name.split_once('.')?;
    let (_, example) = NAMED.iter().find(|(named, _)| *named == prefix)?;
    let tail = rest.split_once('.').map(|(_, tail)| tail);
    let mapped = match tail {
        Some(tail) => format!("{prefix}.{example}.{tail}"),
        None => format!("{prefix}.{example}"),
    };
    table
        .headers
        .iter()
        .any(|header| header.name == mapped)
        .then_some(mapped)
}

/// What `nvs/directives` carries: the whole text of the file, and the cursor.
#[derive(Debug, Clone)]
pub struct Params {
    /// The document's text as the editor holds it.
    pub text: String,
    /// The cursor.
    pub position: Position,
}

impl Params {
    /// Read one request's params off the wire, field by field on
    /// [`crate::regions::Params::from_value`]'s terms.
    ///
    /// # Errors
    ///
    /// The `serde_json` error for params that are not an object, or whose
    /// `text` is not a string or whose `position` is not a position.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::from_value(value)?;
        let text = serde_json::from_value(fields.remove("text").unwrap_or_default())?;
        let position = serde_json::from_value(fields.remove("position").unwrap_or_default())?;
        Ok(Self { text, position })
    }
}

/// The completion items for the cursor at `position` in `text`, with every
/// range counted in `encoding`.
#[must_use]
pub fn complete(text: &str, position: Position, encoding: PositionEncoding) -> Vec<CompletionItem> {
    let mut map = SourceMap::new();
    let id = map.add("nvs.toml", text);
    let file = map.file(id);
    let offset = offset_at(file, position, encoding) as usize;
    let start = text[..offset].rfind('\n').map_or(0, |at| at + 1);
    let end = text[offset..]
        .find('\n')
        .map_or(text.len(), |at| offset + at);
    let before = &text[start..offset];
    let after = text[offset..end].trim_end_matches('\r');
    let written = before.trim_start();
    let table = table();

    if let Some(rest) = written.strip_prefix('[') {
        let array = rest.starts_with('[');
        let partial = rest.strip_prefix('[').unwrap_or(rest);
        if !partial.chars().all(|c| key_char(c) || c == '.') {
            return Vec::new();
        }
        let range = replacing(file, offset, partial.len(), encoding);
        let close = match (array, after.is_empty()) {
            (_, false) => "",
            (true, true) => "]]",
            (false, true) => "]",
        };
        return table
            .headers
            .iter()
            .filter(|header| header.array == array)
            .enumerate()
            .map(|(at, header)| CompletionItem {
                label: header.name.to_owned(),
                kind: Some(CompletionItemKind::MODULE),
                sort_text: Some(format!("{at:04}")),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: format!("{}{close}", header.name),
                })),
                ..CompletionItem::default()
            })
            .collect();
    }

    if !written.chars().all(key_char) {
        return Vec::new();
    }
    let Some((name, set)) = section(text, start) else {
        return Vec::new();
    };
    let Some(block) = resolve(table, name) else {
        return Vec::new();
    };
    let range = replacing(file, offset, written.len(), encoding);
    table
        .entries
        .iter()
        .filter(|entry| entry.block == block && !set.contains(&entry.key))
        .enumerate()
        .map(|(at, entry)| CompletionItem {
            label: entry.key.to_owned(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(format!("{} = {}  # {}", entry.key, entry.value, entry.note)),
            documentation: (!entry.doc.is_empty()).then(|| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: entry.doc.clone(),
                })
            }),
            sort_text: Some(format!("{at:04}")),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: format!("{} = ${{1:{}}}", entry.key, snippet_escaped(entry.value)),
            })),
            ..CompletionItem::default()
        })
        .collect()
}

/// The live header above the line that starts at `line_start`, and the keys
/// its section already sets, above the cursor and below it.
fn section(text: &str, line_start: usize) -> Option<(&str, Vec<&str>)> {
    let header_start = text[..line_start]
        .lines()
        .rev()
        .find_map(|line| live_header(line))?;
    let below = text[line_start..].lines().skip(1);
    let above = text[..line_start]
        .lines()
        .rev()
        .take_while(|line| live_header(line).is_none());
    let set = above
        .chain(below.take_while(|line| live_header(line).is_none()))
        .filter_map(|line| {
            let (key, _) = line.trim_start().split_once('=')?;
            let key = key.trim_end();
            (!key.is_empty() && key.chars().all(key_char)).then_some(key)
        })
        .collect();
    Some((header_start, set))
}

/// The name of a live `[name]` or `[[name]]` line.
fn live_header(line: &str) -> Option<&str> {
    let line = line.trim();
    let name = match line.strip_prefix("[[") {
        Some(rest) => rest.strip_suffix("]]")?,
        None => line.strip_prefix('[')?.strip_suffix(']')?,
    };
    Some(name.trim())
}

/// The range of the `len` bytes before `offset`, which the item replaces.
fn replacing(file: &SourceFile, offset: usize, len: usize, encoding: PositionEncoding) -> Range {
    let to = BytePos::try_from(offset).unwrap_or(BytePos::MAX);
    let from = BytePos::try_from(offset - len).unwrap_or(BytePos::MAX);
    Range {
        start: position_at(file, from, encoding),
        end: position_at(file, to, encoding),
    }
}

/// `value` as snippet text: `$`, `}` and `\` are the three characters a
/// snippet reads.
fn snippet_escaped(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '$' | '}' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
