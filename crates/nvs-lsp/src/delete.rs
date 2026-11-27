//! Delete: a declaration nothing uses, with its doc comment, its attributes and
//! the blank line it leaves (`rule:tooling/nvs-code-answers-about-the-project`).
//!
//! [`delete`] is the engine both clients call: `nvs code delete` writes what it
//! returns, and the editor's *Delete unused …* code action sends it as one
//! edit. It returns [`Rewrite`]s and the file it deletes, or a refusal message.
//!
//! # What is refused
//!
//! - a declaration with an occurrence, a `@see` or `@throws` mention, or a
//!   `use` line that imports it: each is a use;
//! - a method that overrides another or is overridden, since deleting it
//!   changes which body runs;
//! - a declaration [`SymbolIndex::unused`] does not list, such as a route
//!   handler, a test, a constructor or a name a string writes: the program can
//!   still reach it, and a doubtful case is a use;
//! - a declaration that shares its statement with another one, as
//!   `const A = 1, B = 2;` does;
//! - an `extend` block, an `attribute` declaration and its fields;
//! - the file of a type that is its sole declaration, while a `require` names
//!   that file.
//!
//! # Decision: a sole declaration deletes its file
//!
//! A type whose file holds nothing else but its `namespace` and `use` lines
//! and comments deletes the file. An empty autoloaded file would be an error
//! (`rule:programs/one-declaration-per-autoloaded-file`). The check reads the
//! file as empty, so a reader that still loads it is analysed without it.
//!
//! # What it spends
//!
//! Tooling only. One analysis of the declaration's file to find its
//! statement, one analysis of each reader of a deleted file to find a
//! `require`, and the check of [`crate::refactor::checked`].

use std::path::{Path, PathBuf};

use nvs_diagnostics::canonical_key;
use nvs_syntax::ast::{ClassMember, ClassMemberKind, Stmt, StmtKind};

use crate::document::{Documents, analyse_file};
use crate::index::{DeclKind, Declaration, Site, SymbolIndex, Unused};
use crate::rename::Rewrite;

/// What a deletion changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Deleted {
    /// Every edit, keyed by each file's current path. Empty when
    /// [`file`](Self::file) is set.
    pub rewrites: Vec<Rewrite>,
    /// The file deleted with its sole declaration.
    pub file: Option<PathBuf>,
}

/// What a span found in a file's tree is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A statement: a type, a function or a constant of the file.
    Statement,
    /// A member of a class, an interface or an enum, or an enum case.
    Member,
}

/// The edit that deletes `symbol`, or the message that says why it cannot be
/// deleted, written for the person who asked.
///
/// `project` is what [`SymbolIndex::unused`] reads besides the index.
///
/// # Errors
///
/// Returns the refusal message.
pub fn delete(
    documents: &Documents,
    index: &SymbolIndex,
    symbol: &str,
    project: &Unused,
) -> Result<Deleted, String> {
    let declared = index
        .declaration(symbol)
        .ok_or_else(|| format!("`{symbol}` is not declared in this project."))?;
    refuse_used(documents, index, declared)?;
    let listed = index
        .unused(&declared.site.path, project)
        .iter()
        .any(|unused| unused.symbol == declared.symbol && unused.site == declared.site);
    if !listed {
        return Err(format!(
            "Nothing in the source uses `{symbol}`, but the program can still reach it. It may \
             be a route, a test, a constructor or a callback, have an attribute, or have its name \
             written in a string. It was not deleted."
        ));
    }
    let path = declared.site.path.clone();
    let analysed = analyse_file(documents, &path, 0)
        .ok_or_else(|| format!("`{}` cannot be read.", shown(&path)))?;
    let text = analysed.map.file(analysed.entry).text().to_owned();
    let stmts = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
        .map(|loaded| loaded.stmts.clone())
        .ok_or_else(|| format!("`{}` cannot be read.", shown(&path)))?;
    let mut spans = Vec::new();
    in_stmts(&stmts, &mut spans);
    let (start, end) = statement_of(declared, &spans).ok_or_else(|| {
        format!("`{symbol}` cannot be deleted. Only a whole declaration can be deleted.")
    })?;
    if let Some(other) = index.declarations_in(&path).iter().find(|other| {
        other.site != declared.site
            && start <= other.site.start
            && other.site.end <= end
            && !other.symbol.starts_with(&format!("{symbol}::"))
    }) {
        return Err(format!(
            "`{symbol}` is declared together with `{}`, so it was not deleted.",
            other.symbol
        ));
    }
    let (from, to) = removed(&text, start as usize, end as usize);
    if is_type(declared.kind) && only_header(&text, from, to) {
        refuse_required(documents, index, &path)?;
        let emptied = Rewrite {
            site: Site {
                path: path.clone(),
                start: 0,
                end: at(text.len()),
            },
            text: String::new(),
        };
        crate::refactor::checked(documents, index, vec![emptied], &[])
            .map_err(|broken| broken.refusal())?;
        return Ok(Deleted {
            rewrites: Vec::new(),
            file: Some(path),
        });
    }
    let rewrite = Rewrite {
        site: Site {
            path,
            start: at(from),
            end: at(to),
        },
        text: String::new(),
    };
    let checked = crate::refactor::checked(documents, index, vec![rewrite], &[])
        .map_err(|broken| broken.refusal())?;
    Ok(Deleted {
        rewrites: checked.rewrites,
        file: None,
    })
}

/// Refuses a declaration that is used, mentioned, imported, overrides or is
/// overridden, or is a kind this module does not delete.
fn refuse_used(
    documents: &Documents,
    index: &SymbolIndex,
    declared: &Declaration,
) -> Result<(), String> {
    let symbol = declared.symbol.as_str();
    if matches!(
        declared.kind,
        DeclKind::Extend | DeclKind::Attribute | DeclKind::Field
    ) {
        return Err("You can delete a class, an interface, an enum, a type alias, a method, a \
                    property, a constant or an enum case."
            .to_owned());
    }
    let used = |site: &Site, how: &str| {
        format!(
            "`{symbol}` is {how} at {}, so it was not deleted.",
            place(documents, site)
        )
    };
    if let Some(found) = index.occurrences(symbol).first() {
        return Err(used(&found.site, "used"));
    }
    if let Some(found) = index.mentions_of(symbol).first() {
        return Err(used(&found.site, "named in a doc comment"));
    }
    if let Some(found) = index.imports_of(symbol).first() {
        return Err(used(&found.site, "imported"));
    }
    if declared.kind == DeclKind::Method
        && !(index.overridden(symbol).is_empty() && index.overriders(symbol).is_empty())
    {
        return Err(format!(
            "`{symbol}` overrides a method or is overridden. Deleting it changes which method \
             runs, so it was not deleted."
        ));
    }
    Ok(())
}

/// Refuses the deletion of `path` while a file the index holds names it in a
/// `require`.
fn refuse_required(documents: &Documents, index: &SymbolIndex, path: &Path) -> Result<(), String> {
    let key = canonical_key(path);
    for entry in index.readers(&key) {
        let Some(analysed) = analyse_file(documents, &entry, 0) else {
            continue;
        };
        for loaded in &analysed.loaded {
            for (span, target) in &loaded.requires {
                if analysed.map.file(*target).path().map(canonical_key) != Some(key.clone()) {
                    continue;
                }
                let file = analysed.map.file(loaded.id);
                let Some(here) = file.path() else {
                    continue;
                };
                let site = Site {
                    path: canonical_key(here),
                    start: span.start,
                    end: span.end,
                };
                return Err(format!(
                    "`{}` is named by a `require` at {}, so it was not deleted.",
                    shown(path),
                    place(documents, &site)
                ));
            }
        }
    }
    Ok(())
}

/// Every statement and member span in `stmts`, with what it is.
fn in_stmts(stmts: &[Stmt], spans: &mut Vec<(u32, u32, Shape)>) {
    for stmt in stmts {
        spans.push((stmt.span.start, stmt.span.end, Shape::Statement));
        match &stmt.kind {
            StmtKind::NamespaceDecl(namespace) => {
                if let Some(body) = &namespace.body {
                    in_stmts(&body.stmts, spans);
                }
            }
            StmtKind::ClassDecl(decl) => in_members(&decl.members, spans),
            StmtKind::InterfaceDecl(decl) => in_members(&decl.members, spans),
            StmtKind::EnumDecl(decl) => {
                for case in &decl.cases {
                    spans.push((case.span.start, case.span.end, Shape::Member));
                }
                in_members(&decl.members, spans);
            }
            _ => {}
        }
    }
}

/// [`in_stmts`] over the members of a class, an interface or an enum.
fn in_members(members: &[ClassMember], spans: &mut Vec<(u32, u32, Shape)>) {
    for member in members {
        spans.push((member.span.start, member.span.end, Shape::Member));
        if let ClassMemberKind::Enum(decl) = &member.kind {
            for case in &decl.cases {
                spans.push((case.span.start, case.span.end, Shape::Member));
            }
            in_members(&decl.members, spans);
        }
    }
}

/// The smallest span of `spans` around the name of `declared`, when it is the
/// shape that kind of declaration is written as. A member of an `extend`
/// block finds the block's statement, and is refused.
fn statement_of(declared: &Declaration, spans: &[(u32, u32, Shape)]) -> Option<(u32, u32)> {
    let (start, end, shape) = spans
        .iter()
        .filter(|(start, end, _)| *start <= declared.site.start && declared.site.end <= *end)
        .min_by_key(|(start, end, _)| end - start)?;
    let owned = declared.symbol.contains("::");
    let expected = if is_type(declared.kind) && !owned {
        Shape::Statement
    } else {
        Shape::Member
    };
    (*shape == expected).then_some((*start, *end))
}

/// Whether `kind` is a type.
const fn is_type(kind: DeclKind) -> bool {
    matches!(
        kind,
        DeclKind::Class | DeclKind::Interface | DeclKind::Enum | DeclKind::TypeAlias
    )
}

/// The bytes to delete for a declaration written from `start` to `end`: its
/// whole lines, the `///` and `#[` lines above it, and one blank line beside
/// it. A declaration that shares a line with other code is deleted alone.
fn removed(text: &str, start: usize, end: usize) -> (usize, usize) {
    let line_start = |at: usize| text[..at].rfind('\n').map_or(0, |found| found + 1);
    let line_end = |at: usize| text[at..].find('\n').map_or(text.len(), |found| at + found + 1);
    let mut from = line_start(start);
    let to_line = line_end(end);
    if !text[from..start].trim().is_empty() || !text[end..to_line].trim().is_empty() {
        return (start, end);
    }
    while from > 0 {
        let above = line_start(from - 1);
        let line = text[above..from].trim();
        if line.starts_with("///") || line.starts_with("#[") {
            from = above;
        } else {
            break;
        }
    }
    let mut to = to_line;
    let after = line_end(to);
    if to < text.len() && text[to..after].trim().is_empty() {
        to = after;
    } else if from > 0 {
        let above = line_start(from - 1);
        if text[above..from].trim().is_empty() {
            from = above;
        }
    }
    (from, to)
}

/// Whether `text` without the bytes `from..to` holds only the `<?nvs` line,
/// blank lines, comments, and `namespace` and `use` lines.
fn only_header(text: &str, from: usize, to: usize) -> bool {
    let rest = format!("{}{}", &text[..from], &text[to..]);
    rest.lines().all(|line| {
        let line = line.trim();
        line.is_empty()
            || line == "<?nvs"
            || line.starts_with("//")
            || line.starts_with("namespace ")
            || line.starts_with("use ")
    })
}

/// A byte offset as a position.
fn at(offset: usize) -> u32 {
    u32::try_from(offset).unwrap_or(u32::MAX)
}

/// The file name of `path`.
fn shown(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// `file:line` for `site`, the line counted from 1.
fn place(documents: &Documents, site: &Site) -> String {
    let mut map = nvs_diagnostics::SourceMap::new();
    documents.overlay(&mut map);
    let line = map.load(&site.path).ok().map_or(0, |id| {
        let text = map.file(id).text();
        text.get(..site.start as usize)
            .map_or(0, |head| head.matches('\n').count())
            + 1
    });
    format!("{}:{line}", shown(&site.path))
}
