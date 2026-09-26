//! What a completion item documents, read when a client asks for one item.
//!
//! `completionItem/resolve` fills in the `documentation` of the one row a
//! client is showing, and nothing else: the list `crate::completion` answers
//! names every type in reach and every member of a class, and a card on each
//! of those rows would be most of the bytes of every list, sent on every
//! keystroke and read for one row. So a row carries a **key** under `data`
//! instead — the qualified name of the type it names, or the owner and the
//! member's name — and the card is looked up here for the row the client
//! resolved. The key is what makes the lookup possible without the cursor: a
//! resolve request carries the item and nothing about the document it was
//! offered in, so `data` also carries the document's URI, stamped on by the
//! server after the list was built ([`keyed_to`]).
//!
//! The card is the same text `crate::hover` shows for the declaration: a
//! `Core` member's reference card from its registry row
//! (`rule:core-api/reference-card`), a `Core` class's, enum's, constant's or
//! case's one-line description from the same registry, and a user
//! declaration's `///` run. One renderer for both requests, so what a list
//! says about a name and what hovering it says never disagree.

use std::collections::{BTreeMap, BTreeSet};

use lsp_types::{CompletionItem, Documentation, MarkupContent, MarkupKind, Uri};
use nvs_hir::QName;
use nvs_stdlib::registry;
use nvs_types::derive;
use serde_json::{Map, Value};

use crate::definition::{MemberKind, site_of};
use crate::document::{Analysed, Documents, analyse};
use crate::hover::{markdown, reference_card};

/// Every item's `data`, with the document it was offered in written into it.
///
/// A key without a URI reaches the registry and nothing else, which is what
/// the `.lspt` suite and the crate's tests see; the server stamps the URI on
/// the way out.
#[must_use]
pub fn keyed_to(items: Vec<CompletionItem>, uri: &Uri) -> Vec<CompletionItem> {
    items
        .into_iter()
        .map(|item| match item.data {
            Some(Value::Object(mut key)) => {
                key.insert("uri".to_owned(), Value::from(uri.as_str()));
                CompletionItem {
                    data: Some(Value::Object(key)),
                    ..item
                }
            }
            _ => item,
        })
        .collect()
}

/// `item`, with the documentation its key reaches — or `item` as it came, for
/// one carrying no key, a key naming nothing, or a declaration nobody wrote a
/// line above.
#[must_use]
pub fn resolve(documents: &Documents, item: CompletionItem) -> CompletionItem {
    let Some(Value::Object(key)) = item.data.as_ref() else {
        return item;
    };
    let Some(card) = card_of(documents, key) else {
        return item;
    };
    CompletionItem {
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: card,
        })),
        ..item
    }
}

/// The Markdown the key names, trimmed, or `None` where nothing documents it.
fn card_of(documents: &Documents, key: &Map<String, Value>) -> Option<String> {
    let text = key.get("type").and_then(Value::as_str);
    let owner = key.get("owner").and_then(Value::as_str);
    let member = key.get("member").and_then(Value::as_str);
    let card = match (text, owner, member) {
        (Some(name), _, _) => core_type(name).or_else(|| declared(documents, key, name, None)),
        (None, Some(owner), Some(member)) => core_member(owner, member).or_else(|| {
            [
                MemberKind::Method,
                MemberKind::Property,
                MemberKind::Constant,
                MemberKind::TypeAlias,
            ]
            .into_iter()
            .find_map(|kind| declared(documents, key, owner, Some((member, kind))))
        }),
        _ => None,
    }?;
    let card = card.trim();
    (!card.is_empty()).then(|| card.to_owned())
}

/// A `Core` class's, enum's or compiler attribute's own line, where its
/// registry row carries one.
fn core_type(name: &str) -> Option<String> {
    if let Some(class) = registry::class(name) {
        return class.doc.map(|doc| doc.short.to_owned());
    }
    if let Some(core) = registry::core_enum(name) {
        return core.doc.map(|doc| doc.short.to_owned());
    }
    derive::attribute_doc(name).map(|doc| doc.short.to_owned())
}

/// The hover for a `Core` class, enum or compiler attribute: its declaration
/// line as a code block, then its card — a class's line, an enum's line and
/// one line per documented case, an attribute's line and where it is written.
/// `None` for a name none of the three rosters holds.
///
/// The same lines [`core_type`] shows on a completion row, with the
/// declaration in front of them: a hover is read in place of opening the
/// declaration, so it starts with what the declaration would have said.
pub(crate) fn core_type_hover(name: &str) -> Option<String> {
    if let Some(class) = registry::class(name) {
        let mut out = format!("```nvs\nclass {name}\n```");
        if let Some(doc) = class.doc {
            out.push_str("\n\n");
            out.push_str(doc.short);
        }
        if let Some(intro) = class.intro() {
            out.push_str("\n\n");
            out.push_str(intro);
        }
        return Some(out);
    }
    if let Some(core) = registry::core_enum(name) {
        let mut out = format!("```nvs\nenum {name}\n```");
        if let Some(doc) = core.doc {
            out.push_str("\n\n");
            out.push_str(doc.short);
            if !doc.cases.is_empty() {
                out.push_str("\n\n**Cases**\n");
                for case in doc.cases {
                    out.push_str(&format!("\n- `{}` — {}", case.name, case.desc));
                }
            }
        }
        return Some(out);
    }
    let doc = derive::attribute_doc(name)?;
    Some(format!(
        "```nvs\n{}\n```\n\n{}\n\nWritten above {}.",
        doc.spelled(),
        doc.short,
        doc.site
    ))
}

/// The hover for a `Core` constant or enum case: `Owner::NAME` as a code
/// block, then the constant's sentence or the case's line — and for a case
/// nobody described on its own, its enum's line. `None` for a member no `Core`
/// class or enum declares.
pub(crate) fn core_member_hover(owner: &str, member: &str) -> Option<String> {
    let line = format!("```nvs\n{owner}::{member}\n```");
    let desc = if let Some(class) = registry::class(owner) {
        class.constant(member)?.desc
    } else {
        let core = registry::core_enum(owner)?;
        core.cases.iter().find(|(case, _)| *case == member)?;
        core.doc.map_or("", |doc| {
            doc.cases
                .iter()
                .find(|case| case.name == member)
                .map_or(doc.short, |case| case.desc)
        })
    };
    Some(if desc.is_empty() {
        line
    } else {
        format!("{line}\n\n{desc}")
    })
}

/// What a `Core` namespace holds, for a cursor on one segment of a written
/// name: the classes, enums, interfaces and attributes directly under it, each
/// with its line, and the namespaces under it. `None` for a namespace nothing
/// is under, which is every namespace outside `Core`.
///
/// A namespace has no declaration to open and no card of its own, so this is
/// the one answer a cursor on `Http` in `Core\Http\Method` has, and it is what
/// the completion list under `Core\Http\` offers, read the same way
/// (`crate::completion`'s `under`).
pub(crate) fn namespace_card(namespace: &str) -> Option<String> {
    let mut members: BTreeMap<&str, &str> = BTreeMap::new();
    let mut nested: BTreeSet<&str> = BTreeSet::new();
    nvs_footprint::every_class();
    let classes = registry::CLASSES
        .iter()
        .map(|class| (class.name, class.doc.map_or("", |doc| doc.short)));
    let enums = registry::ENUMS
        .iter()
        .map(|core| (core.name, core.doc.map_or("", |doc| doc.short)));
    let interfaces = registry::DERIVE_INTERFACES.iter().map(|name| (*name, ""));
    let attributes = derive::ATTRIBUTE_DOCS
        .iter()
        .map(|doc| (doc.name, doc.short));
    for (name, short) in classes.chain(enums).chain(interfaces).chain(attributes) {
        let Some(rest) = name
            .strip_prefix(namespace)
            .and_then(|rest| rest.strip_prefix('\\'))
        else {
            continue;
        };
        match rest.split_once('\\') {
            Some((head, _)) => {
                nested.insert(head);
            }
            None => {
                members.insert(rest, short);
            }
        }
    }
    if members.is_empty() && nested.is_empty() {
        return None;
    }
    let mut out = format!("```nvs\nnamespace {namespace}\n```\n");
    for (name, short) in members {
        if short.is_empty() {
            out.push_str(&format!("\n- `{name}`"));
        } else {
            out.push_str(&format!("\n- `{name}` — {short}"));
        }
    }
    for name in nested {
        out.push_str(&format!("\n- `{name}\\` — a namespace"));
    }
    Some(out)
}

/// A `Core` member's reference card, a constant's sentence, or a case's line
/// — and for a case nobody described on its own, its enum's line.
fn core_member(owner: &str, member: &str) -> Option<String> {
    if let Some(class) = registry::class(owner) {
        if let Some(row) = class.members().find(|row| row.name == member) {
            return row.doc.map(|doc| reference_card(row, doc));
        }
        return class.constant(member).map(|row| row.desc.to_owned());
    }
    let core = registry::core_enum(owner)?;
    let doc = core.doc?;
    Some(
        doc.cases
            .iter()
            .find(|case| case.name == member)
            .map_or(doc.short, |case| case.desc)
            .to_owned(),
    )
}

/// The `///` run above a user declaration, in the document the key names.
fn declared(
    documents: &Documents,
    key: &Map<String, Value>,
    class: &str,
    member: Option<(&str, MemberKind)>,
) -> Option<String> {
    let uri: Uri = key.get("uri")?.as_str()?.parse().ok()?;
    let analysed: Analysed = analyse(documents, &uri)?;
    let site = site_of(&analysed, &QName::parse(class), member)?;
    let doc = site.doc?;
    Some(markdown(analysed.map.file(site.span.file).text(), doc))
}
