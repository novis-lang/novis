//! The index's readers, asked by name rather than at a cursor.
//!
//! An editor asks about the name under a cursor, and `nvs code` asks about a
//! name typed out, so each question here takes a symbol spelled as
//! [`crate::index`]'s module doc spells it: `Shop\Cart`,
//! `Shop\Cart::addItem`, `Shop\Cart::$total`. The editor feature that answers
//! the same question calls the same function — `textDocument/references` is
//! [`references`], `textDocument/implementation` is [`implementations`] and
//! `textDocument/rename` is [`rename`] — so the editor and the command never
//! disagree about an answer (`rule:ide/five-features-are-one-reference-index`).
//!
//! Every function reads a [`Workspace`] or the index inside one and builds
//! nothing: the index is built in `server.rs`, which is the module that owns
//! one. [`declaration`], [`types`] and [`rename`] analyse one file again,
//! because the index keeps names and sites and neither a `///` run, an
//! inferred type nor a file's tokens.

use std::ops::RangeInclusive;
use std::path::Path;

use lsp_types::{InlayHintKind, InlayHintLabel};
use nvs_diagnostics::{PositionEncoding, canonical_key};
use nvs_hir::QName;
use nvs_hir::members::edit_distance;

use crate::definition::{MemberKind, site_of};
use crate::document::analyse_file;
use crate::hints;
use crate::hover::markdown;
use crate::index::{DeclKind, Declaration, Site, SymbolIndex};
use crate::position::offset_at;
use crate::rename::{self, Renamed};
use crate::server::Workspace;

/// What a reference to a name is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseKind {
    /// The declaration itself.
    Declared,
    /// A `use` line that imports the name.
    Import,
    /// Code that uses the name.
    Use,
}

impl UseKind {
    /// The word `nvs code refs` prints for this kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Import => "import",
            Self::Use => "use",
        }
    }
}

/// One place a name is written, and what it is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reference<'a> {
    /// The declaration, an import or a use.
    pub kind: UseKind,
    /// Where it is written.
    pub site: &'a Site,
}

/// Every place `symbol` is written: the declaration first when
/// `include_declaration` asks for it, then every import and use in path and
/// then source order.
///
/// A name the index holds and nothing uses answers its declaration alone, and
/// a `Core` name answers the project's uses of it with no declaration. Only a
/// use that resolved to `symbol` is listed, so a method of the same name on
/// another class is not.
#[must_use]
pub fn references<'a>(
    index: &'a SymbolIndex,
    symbol: &str,
    include_declaration: bool,
) -> Vec<Reference<'a>> {
    let mut found = Vec::new();
    if include_declaration && let Some(declared) = index.declaration(symbol) {
        found.push(Reference {
            kind: UseKind::Declared,
            site: &declared.site,
        });
    }
    let mut uses: Vec<Reference<'a>> = index
        .occurrences(symbol)
        .into_iter()
        .map(|occurrence| Reference {
            kind: UseKind::Use,
            site: &occurrence.site,
        })
        .chain(
            index
                .imports_of(symbol)
                .into_iter()
                .map(|import| Reference {
                    kind: UseKind::Import,
                    site: &import.site,
                }),
        )
        .collect();
    uses.sort_by(|a, b| a.site.cmp(b.site));
    found.extend(uses);
    found
}

/// What sits below and above one type or method.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Implementations<'a> {
    /// For a type, every type that extends or implements it, at any depth,
    /// nearer levels first. For a method, every method that overrides it.
    pub below: Vec<&'a Declaration>,
    /// For a type, what it directly extends or implements. For a method, the
    /// nearest method it overrides on each branch above its class.
    pub above: Vec<&'a Declaration>,
}

/// The types or methods below and above `symbol`.
///
/// A method is a name the index declares as one, or a `Class::name` it does
/// not declare at all, such as a `Core` method. Everything else is read as a
/// type, an enum inside a class included.
#[must_use]
pub fn implementations<'a>(index: &'a SymbolIndex, symbol: &str) -> Implementations<'a> {
    let is_method = index.declaration(symbol).map_or_else(
        || symbol.contains("::"),
        |declared| declared.kind == DeclKind::Method,
    );
    if is_method {
        return Implementations {
            below: index.overriders(symbol),
            above: index.overridden(symbol),
        };
    }
    let mut below = index.subtypes(symbol);
    let mut next = 0;
    while let Some(ty) = below.get(next).copied() {
        next += 1;
        for deeper in index.subtypes(&ty.symbol) {
            if deeper.symbol != symbol && !below.iter().any(|seen| seen.symbol == deeper.symbol) {
                below.push(deeper);
            }
        }
    }
    Implementations {
        below,
        above: index.supertypes(symbol),
    }
}

/// One declaration, with the `///` run written above it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card<'a> {
    /// The declaration the index holds: its kind and its site.
    pub declaration: &'a Declaration,
    /// The `///` run as hover shows it, each line's marker removed, or `None`
    /// where nothing was written above the declaration.
    pub doc: Option<String>,
}

/// The declaration of `symbol`, or `None` for a name the index does not hold.
///
/// The doc comment is read from a fresh analysis of the file that declares
/// the name, through the function hover reads it with.
#[must_use]
pub fn declaration<'a>(workspace: &'a Workspace, symbol: &str) -> Option<Card<'a>> {
    let declared = workspace.index().declaration(symbol)?;
    let analysed = analyse_file(workspace.documents(), &declared.site.path, 0);
    let doc = analysed.and_then(|analysed| {
        let (class, member) = site_key(declared);
        let site = site_of(&analysed, &QName::parse(class), member)?;
        let text = markdown(analysed.map.file(site.span.file).text(), site.doc?);
        (!text.trim().is_empty()).then_some(text)
    });
    Some(Card {
        declaration: declared,
        doc,
    })
}

/// The owner and the member `crate::definition::site_of` finds `declared` by.
fn site_key(declared: &Declaration) -> (&str, Option<(&str, MemberKind)>) {
    let symbol = declared.symbol.as_str();
    let member = |kind: MemberKind| {
        symbol
            .rsplit_once("::")
            .map(|(owner, name)| (owner, Some((name.trim_start_matches('$'), kind))))
    };
    let split = match declared.kind {
        DeclKind::Method => member(MemberKind::Method),
        DeclKind::Property => member(MemberKind::Property),
        DeclKind::Const | DeclKind::EnumCase => member(MemberKind::Constant),
        DeclKind::Class | DeclKind::Interface | DeclKind::Enum | DeclKind::TypeAlias => None,
    };
    split.unwrap_or((symbol, None))
}

/// One declaration of a file, with the declarations it owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outline<'a> {
    /// The declaration.
    pub declaration: &'a Declaration,
    /// What it declares inside it, in source order: a class's members, an
    /// enum's cases, an enum inside a class with its own cases.
    pub members: Vec<Outline<'a>>,
}

/// Every declaration in `path`, nested by owner, in source order, and nothing
/// for a file the index does not hold.
///
/// A declaration's owner is the name before its last `::`, where the same file
/// declares that name.
#[must_use]
pub fn outline<'a>(index: &'a SymbolIndex, path: &Path) -> Vec<Outline<'a>> {
    let declared = index.declarations_in(path);
    nested(declared, None)
}

/// The declarations of `declared` whose owner in the file is `owner`.
fn nested<'a>(declared: &'a [Declaration], owner: Option<&str>) -> Vec<Outline<'a>> {
    declared
        .iter()
        .filter(|candidate| owner_in(declared, candidate) == owner)
        .map(|candidate| Outline {
            declaration: candidate,
            members: nested(declared, Some(&candidate.symbol)),
        })
        .collect()
}

/// The name `candidate` is declared inside, when `declared` holds it.
fn owner_in<'a>(declared: &[Declaration], candidate: &'a Declaration) -> Option<&'a str> {
    let (owner, _) = candidate.symbol.rsplit_once("::")?;
    declared
        .iter()
        .any(|other| other.symbol == owner)
        .then_some(owner)
}

/// One variable whose type the checker inferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typed {
    /// The variable as written, `$` included.
    pub name: String,
    /// The type, as an inlay hint shows it without its `: `.
    pub ty: String,
    /// Where the variable's name is written.
    pub site: Site,
}

/// Every variable declared with `var` on lines `lines` of `path`, with the
/// type the checker inferred for it, in source order.
///
/// The lines count from 1, as an editor shows them, and both ends are
/// included. These are the `: T` hints the editor draws, from the same
/// function. `None` for a file that cannot be read.
#[must_use]
pub fn types(workspace: &Workspace, path: &Path, lines: RangeInclusive<u32>) -> Option<Vec<Typed>> {
    let analysed = analyse_file(workspace.documents(), path, 0)?;
    let file = analysed.map.file(analysed.entry);
    let text = file.text();
    let site_path = canonical_key(path);
    Some(
        hints::for_document(&analysed, PositionEncoding::Utf8)
            .into_iter()
            .filter(|hint| hint.kind == Some(InlayHintKind::TYPE))
            .filter(|hint| lines.contains(&(hint.position.line + 1)))
            .filter_map(|hint| {
                let InlayHintLabel::String(label) = hint.label else {
                    return None;
                };
                let end = offset_at(file, hint.position, PositionEncoding::Utf8);
                let written = text.get(..end as usize)?;
                let start = written
                    .trim_end_matches(|ch: char| ch.is_alphanumeric() || ch == '_')
                    .strip_suffix('$')
                    .map_or(end as usize, str::len);
                Some(Typed {
                    name: text.get(start..end as usize)?.to_owned(),
                    ty: label.strip_prefix(": ").unwrap_or(&label).to_owned(),
                    site: Site {
                        path: site_path.clone(),
                        start: u32::try_from(start).ok()?,
                        end,
                    },
                })
            })
            .collect(),
    )
}

/// What [`rename`] renames: a name the index declares, or the symbol at one
/// place in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target<'a> {
    /// A name in the index's form, as `Shop\Cart::addItem`.
    Name(&'a str),
    /// A place in a file, the only way to name a local. The path is a
    /// canonical key, and the line and the column count from 1, the column in
    /// characters.
    Position {
        /// The file.
        path: &'a Path,
        /// The line, from 1.
        line: u32,
        /// The column, from 1, in characters.
        column: u32,
    },
}

/// Every edit that renames `target` to `new_name`, through the function the
/// editor's rename calls.
///
/// A name renames at the first byte of its declaration's name, which is where
/// the editor's rename at the declaration starts too. A position renames at
/// that offset of its file. A `$` on `new_name` is optional, as it is in the
/// editor.
///
/// # Errors
///
/// Returns the refusal message the editor shows, or a message for a name the
/// index does not declare or a file it does not hold.
pub fn rename(
    workspace: &Workspace,
    target: Target<'_>,
    new_name: &str,
) -> Result<Renamed, String> {
    let index = workspace.index();
    // A name carries its byte offset, and a position its line and column,
    // which become an offset once the file is analysed.
    let (path, start, position) = match target {
        Target::Name(symbol) => {
            let site = &index
                .declaration(symbol)
                .ok_or_else(|| format!("This project does not declare `{symbol}`."))?
                .site;
            (
                site.path.as_path(),
                Some(site.start),
                lsp_types::Position::default(),
            )
        }
        Target::Position { path, line, column } => (
            path,
            None,
            lsp_types::Position {
                line: line.saturating_sub(1),
                character: column.saturating_sub(1),
            },
        ),
    };
    let not_held = || format!("`{}` is not a file of this project.", path.display());
    if !index.holds(path) {
        return Err(not_held());
    }
    let analysed = analyse_file(workspace.documents(), path, 0).ok_or_else(not_held)?;
    let offset = start.unwrap_or_else(|| {
        offset_at(
            analysed.map.file(analysed.entry),
            position,
            PositionEncoding::Utf32,
        )
    });
    rename::rename(workspace.documents(), index, &analysed, offset, new_name)
}

/// The names the index declares that are closest to `name`, nearest first,
/// at most `limit` of them.
///
/// Names are compared without case and without their backslashes, so a name
/// the shell stripped the backslashes from finds the name it meant. A name is
/// also as close as its last part, so `addItem` finds every method of that
/// name. Ties are broken in symbol order, so the list is the same every time.
#[must_use]
pub fn nearest<'a>(index: &'a SymbolIndex, name: &str, limit: usize) -> Vec<&'a str> {
    let wanted = folded(name);
    let wanted_last = folded(last_part(name));
    let mut ranked: Vec<(usize, &'a str)> = index
        .files()
        .flat_map(|path| index.declarations_in(path))
        .map(|declared| {
            let symbol = declared.symbol.as_str();
            let whole = edit_distance(&wanted, &folded(symbol));
            let last = edit_distance(&wanted_last, &folded(last_part(symbol)));
            (whole.min(last), symbol)
        })
        .collect();
    ranked.sort_unstable();
    ranked.dedup_by(|a, b| a.1 == b.1);
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, symbol)| symbol)
        .collect()
}

/// `name` in lower case and without backslashes.
fn folded(name: &str) -> String {
    name.chars()
        .filter(|ch| *ch != '\\')
        .flat_map(char::to_lowercase)
        .collect()
}

/// The part of `name` after its last `\` or `::`.
fn last_part(name: &str) -> &str {
    let after_member = name.rsplit_once("::").map_or(name, |(_, member)| member);
    after_member
        .rsplit_once('\\')
        .map_or(after_member, |(_, last)| last)
}
