//! Rename: a member at every site the index holds it, a local at every site
//! its body writes it, and the `require` paths a moved file changes.
//!
//! The three requests `textDocument/prepareRename`, `textDocument/rename` and
//! `workspace/willRenameFiles` are answered here as [`Rewrite`]s or as a
//! refusal message, and nothing here is protocol: `crate::server` reads the
//! parameters, converts positions and builds the `WorkspaceEdit`, and the
//! `.lspt` runner renders the same rewrites.
//!
//! # What is renamed
//!
//! - **A type** — a class, an interface, an enum or a type alias declared
//!   outside a class — at its declaration, at every occurrence the index holds
//!   and in every `use` line that imports it. Only the last segment changes, so
//!   `\Shop\User` and `use Shop\User;` keep their namespace. Before any edit,
//!   every file the index holds is read for the old name as a name or at the
//!   end of a string, and a place the index holds no site at refuses the
//!   rename: such a place is a site the edits would miss. A type declared
//!   alone in a file named after it, `User.nvs` for `User`, moves that file
//!   to the new name too, because the name is how an `autoload` finds it.
//!   Each `require` literal that names the file is written again as [`moved`]
//!   writes it, and a file already at the new path refuses the rename.
//! - **A member** — a method, a property, a class constant or an enum case —
//!   at its declaration and at every occurrence the index holds
//!   ([`SymbolIndex::declaration`], [`SymbolIndex::occurrences`]). A method
//!   takes its whole override family with it, closed over
//!   [`SymbolIndex::overridden`] and [`SymbolIndex::overriders`] in both
//!   directions, so no override stops overriding. A property site keeps or
//!   drops the `$` the way its own text has it: `public string $name` and
//!   `User::$count` write it, `$u->name` does not.
//! - **A local** of a function, a method or a script body: every `$name` the
//!   declaring body writes as a variable read, a declaration, a `foreach`,
//!   `catch`, destructuring or `static` binding, the binding's own
//!   declaration included. A nested function or method is a scope of its own
//!   and is skipped. An anonymous function reads the body's locals by value,
//!   so a capture inside one is renamed with the rest.
//!
//! # What is refused, each with a message the editor shows
//!
//! A cursor on no symbol; `$this`; a type alias a class declares; a type
//! whose name is written somewhere the index holds no site; a parameter, because a call may
//! pass it by name; a local an anonymous function inside the body declares
//! again; a symbol whose declaration the index does not hold, which is every
//! `Core` name and every file outside the workspace; a method that overrides
//! one declared outside the workspace; and a new name that is no identifier
//! or that clashes. A clash is a refusal and never an alias
//! (`rule:ide/no-refactoring-introduces-an-alias`): a member's new name may not
//! be declared already on any class of the family, above it or below it; a
//! local's may not be bound already in its body; and a type's may not be a
//! built-in type, the name of another type in the workspace, or a name any
//! file with a site already writes.
//!
//! # A moved file
//!
//! [`moved`] reads each `require` edge the graph walk recorded
//! (`nvs_hir::requires::Loaded::requires`, the edge `crate::links` underlines),
//! in every graph that read a moved file, and writes the literal again so it
//! names the file at its new place: relative to the requiring file's new
//! directory with `/` separators when it was relative, absolute when it was
//! absolute, and in the same quotes.
//!
//! # What it costs
//!
//! Nothing is kept. A rename reads the index the server already holds and the
//! text of each file a site is in, once per request. A file move analyses
//! each entry point that read a moved file once, per request.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, SourceId, SourceMap, Span, canonical_key};
use nvs_syntax::{Token, TokenKind, tokenize, walk};
use nvs_types::LocalBinding;

use crate::definition::{covers, text_of};
use crate::document::{Analysed, Documents, analyse_file};
use crate::index::{DeclKind, Site, SymbolIndex, symbol_at};

/// One edit: the bytes of `site` replaced by `text`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rewrite {
    /// What is replaced.
    pub site: Site,
    /// What replaces it.
    pub text: String,
}

/// What a rename changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Renamed {
    /// Every edit, keyed by each file's current path.
    pub rewrites: Vec<Rewrite>,
    /// The file a renamed type is declared in and the path it moves to, for a
    /// type whose file is named after it. A client moves it after it applies
    /// [`rewrites`](Self::rewrites).
    pub file: Option<(PathBuf, PathBuf)>,
}

/// The name a class gives its constructor, which `new` calls by that name.
const CONSTRUCTOR: &str = "constructor";

/// The message for a cursor on nothing this module renames, worded for the
/// editor and for `nvs code rename` alike.
const NOTHING: &str = "There is nothing to rename here. You can rename a class, a variable, a \
                       method, a property, a constant or an enum case.";

/// The message for a cursor on a type alias a class declares.
const AN_OWNED_ALIAS: &str = "Renaming a type alias that a class declares is not available yet.";

/// The names a type may not take, because the language already uses each one
/// for a type of its own.
const BUILT_IN: [&str; 19] = [
    "null", "bool", "int", "uint", "float", "decimal", "string", "array", "mixed", "void", "never",
    "object", "callable", "iterable", "self", "static", "parent", "true", "false",
];

/// The span of the name at `offset` that a rename would change, or the
/// message that says why nothing there can be renamed.
///
/// # Errors
///
/// Returns the refusal message, written for the person at the editor.
pub fn prepare(index: &SymbolIndex, analysed: &Analysed, offset: BytePos) -> Result<Site, String> {
    Ok(subject(index, analysed, offset)?.at)
}

/// Every edit that renames the symbol at `offset` to `new_name`, or the
/// message that says why it cannot be renamed.
///
/// # Errors
///
/// Returns the refusal message, written for the person at the editor.
pub fn rename(
    documents: &Documents,
    index: &SymbolIndex,
    analysed: &Analysed,
    offset: BytePos,
    new_name: &str,
) -> Result<Renamed, String> {
    let subject = subject(index, analysed, offset)?;
    let new = new_name.strip_prefix('$').unwrap_or(new_name);
    if !is_identifier(new) {
        return Err(format!("`{new_name}` is not a valid name."));
    }
    match subject.what {
        What::Type { symbol, old, sites } => {
            if new == old {
                return Err("The new name is the same as the old one.".to_owned());
            }
            types(documents, index, &symbol, &old, new, &sites)
        }
        What::Members {
            family,
            kind,
            old,
            sites,
        } => {
            if new == old {
                return Err("The new name is the same as the old one.".to_owned());
            }
            members(documents, index, &family, kind, &old, new, &sites).map(|rewrites| Renamed {
                rewrites,
                file: None,
            })
        }
        What::Local {
            path,
            name,
            bindings,
            nested,
            sites,
        } => {
            if new == name {
                return Err("The new name is the same as the old one.".to_owned());
            }
            if new == "this" {
                return Err(
                    "`$this` is the object a method runs on, and it cannot be a variable name."
                        .to_owned(),
                );
            }
            if bindings.iter().any(|binding| binding.name == new) || nested.contains(new) {
                return Err(format!("This code already has a variable named `${new}`."));
            }
            let rewrites = sites
                .into_iter()
                .map(|span| Rewrite {
                    site: Site {
                        path: path.clone(),
                        start: span.start,
                        end: span.end,
                    },
                    text: format!("${new}"),
                })
                .collect();
            Ok(Renamed {
                rewrites,
                file: None,
            })
        }
    }
}

/// Every edit to a `require` literal that moving each `(old, new)` path makes
/// necessary, in file and then source order.
///
/// A path that is a directory moves every indexed file under it. The edits
/// are keyed by each file's current path, because a client applies them
/// before it moves anything.
#[must_use]
pub fn moved(
    documents: &Documents,
    index: &SymbolIndex,
    moves: &[(PathBuf, PathBuf)],
) -> Vec<Rewrite> {
    let mut renamed: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for (old, new) in moves {
        let old = canonical_key(old);
        let new = new_key(new);
        for file in index.files() {
            if let Ok(rest) = file.strip_prefix(&old)
                && !rest.as_os_str().is_empty()
            {
                renamed.insert(file.to_path_buf(), new.join(rest));
            }
        }
        renamed.insert(old, new);
    }

    let entries: BTreeSet<PathBuf> = renamed
        .keys()
        .flat_map(|file| index.readers(file))
        .collect();
    let mut seen: BTreeSet<(PathBuf, BytePos)> = BTreeSet::new();
    let mut found = Vec::new();
    for entry in entries {
        // The version is never compared: an open buffer is overlaid whatever
        // it is, and nothing is published from this analysis.
        let Some(analysed) = analyse_file(documents, &entry, 0) else {
            continue;
        };
        for loaded in &analysed.loaded {
            let file = analysed.map.file(loaded.id);
            let Some(here) = file.path().map(canonical_key) else {
                continue;
            };
            for (span, target) in &loaded.requires {
                let Some(there) = analysed.map.file(*target).path().map(canonical_key) else {
                    continue;
                };
                let (from, to) = (renamed.get(&here), renamed.get(&there));
                if from.is_none() && to.is_none() {
                    continue;
                }
                if !seen.insert((here.clone(), span.start)) {
                    continue;
                }
                let written = text_of(file, *span);
                let Some(text) = rewritten(written, from.unwrap_or(&here), to.unwrap_or(&there))
                else {
                    continue;
                };
                found.push(Rewrite {
                    site: Site {
                        path: here.clone(),
                        start: span.start,
                        end: span.end,
                    },
                    text,
                });
            }
        }
    }
    found.sort();
    found
}

/// What a rename at one cursor is about, and the name the cursor is on.
struct Subject {
    /// The bytes of the name under the cursor.
    at: Site,
    /// What a rename there changes.
    what: What,
}

/// The three things a rename changes.
enum What {
    /// A class, an interface, an enum or a type alias declared outside a
    /// class.
    Type {
        /// Its fully-qualified name, spelled as the index spells it.
        symbol: String,
        /// Its last segment, which is the name a rename changes.
        old: String,
        /// Its declaration, every occurrence and every `use` that imports it.
        sites: Vec<Site>,
    },
    /// One member, or a method with every method that overrides it or that it
    /// overrides.
    Members {
        /// Each symbol, spelled as the index spells it.
        family: Vec<String>,
        /// What they declare.
        kind: DeclKind,
        /// The member's name without a `$`.
        old: String,
        /// Every declaration and occurrence of every symbol in `family`.
        sites: Vec<Site>,
    },
    /// One local of one body.
    Local {
        /// The file the body is in.
        path: PathBuf,
        /// Its name without the `$`.
        name: String,
        /// Every name the declaring body binds.
        bindings: Vec<LocalBinding>,
        /// Every name an anonymous function inside the body binds.
        nested: BTreeSet<String>,
        /// Every place the body writes the local.
        sites: Vec<Span>,
    },
}

/// What the cursor at `offset` would rename, or why nothing there can be.
fn subject(index: &SymbolIndex, analysed: &Analysed, offset: BytePos) -> Result<Subject, String> {
    let file = analysed.map.file(analysed.entry);
    let entry = file.path().map(canonical_key).ok_or(NOTHING)?;
    let tokens = tokenize(file, &mut Diagnostics::new());
    let token = token_at(&tokens, offset).ok_or(NOTHING)?;

    // A use resolves outward from the innermost node, so the `$u` of
    // `$u->greet()` answers `greet`. A member is the subject only where one
    // of its own sites is under the cursor.
    let symbol = symbol_at(analysed, offset);
    if let Some(symbol) = &symbol {
        let sites = sites_of(index, std::slice::from_ref(symbol));
        let here = sites
            .iter()
            .find(|site| site.path == entry && site.start <= offset && offset <= site.end);
        if let Some(here) = here {
            let at = here.clone();
            return member(index, symbol, at);
        }
    }
    if token.kind == TokenKind::Variable {
        let written = text_of(file, token.span);
        if written == "$this" {
            return Err(
                "`$this` is the object a method runs on, and it cannot be renamed.".to_owned(),
            );
        }
        return local(analysed, file, &tokens, &entry, token.span, &written[1..]);
    }
    match symbol {
        // A type whose name the index records nowhere under the cursor, such
        // as a `Core` class written in an annotation.
        Some(symbol) if !symbol.contains("::") && index.declaration(&symbol).is_none() => {
            Err(outside(index, &symbol))
        }
        _ => Err(NOTHING.to_owned()),
    }
}

/// The name token at `offset`, preferring one the offset is strictly inside
/// over one it ends.
fn token_at(tokens: &[Token], offset: BytePos) -> Option<Token> {
    tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Variable | TokenKind::Ident))
        .filter(|token| covers(token.span, offset))
        .min_by_key(|token| token.span.end == offset)
        .copied()
}

/// The member subject for `symbol`, with `at` the site under the cursor.
fn member(index: &SymbolIndex, symbol: &str, at: Site) -> Result<Subject, String> {
    let Some(declared) = index.declaration(symbol) else {
        return Err(outside(index, symbol));
    };
    let kind = declared.kind;
    if matches!(
        kind,
        DeclKind::Class | DeclKind::Interface | DeclKind::Enum | DeclKind::TypeAlias
    ) {
        if symbol.contains("::") {
            return Err(AN_OWNED_ALIAS.to_owned());
        }
        let mut sites = sites_of(index, &[symbol.to_owned()]);
        sites.extend(
            index
                .imports_of(symbol)
                .into_iter()
                .map(|it| it.site.clone()),
        );
        sites.sort();
        sites.dedup();
        return Ok(Subject {
            at,
            what: What::Type {
                symbol: symbol.to_owned(),
                old: symbol.rsplit('\\').next().unwrap_or(symbol).to_owned(),
                sites,
            },
        });
    }
    let Some((_, written)) = symbol.rsplit_once("::") else {
        return Err(NOTHING.to_owned());
    };
    let old = written.trim_start_matches('$').to_owned();
    let family = if kind == DeclKind::Method {
        if old == CONSTRUCTOR {
            return Err(
                "`constructor` is the name `new` calls, so it cannot be renamed.".to_owned(),
            );
        }
        let family = family(index, symbol);
        for method in &family {
            overrides_outside(index, method, &old)?;
        }
        family
    } else {
        vec![symbol.to_owned()]
    };
    let sites = sites_of(index, &family);
    Ok(Subject {
        at,
        what: What::Members {
            family,
            kind,
            old,
            sites,
        },
    })
}

/// Why `symbol`, which the index holds no declaration for, is refused.
fn outside(index: &SymbolIndex, symbol: &str) -> String {
    match symbol.rsplit_once("::") {
        // A member of a class the workspace declares, written where the index
        // records no declaration, such as a promoted constructor parameter.
        Some((owner, _)) if index.declaration(owner).is_some() => {
            format!("`{symbol}` has no declaration that can be renamed.")
        }
        _ => format!("`{symbol}` is declared outside this workspace, so it cannot be renamed."),
    }
}

/// `method`, with every method it overrides and every method that overrides
/// it, at any distance and in both directions, sorted.
fn family(index: &SymbolIndex, method: &str) -> Vec<String> {
    let mut found = vec![method.to_owned()];
    let mut next = 0;
    while let Some(current) = found.get(next).cloned() {
        next += 1;
        for related in index
            .overridden(&current)
            .into_iter()
            .chain(index.overriders(&current))
        {
            if !found.contains(&related.symbol) {
                found.push(related.symbol.clone());
            }
        }
    }
    found.sort();
    found
}

/// An error when a type above the class of `method` is declared outside the
/// workspace and has, or may have, a method named `name`.
///
/// A `Core` class is read from the registry, so its methods are known. Any
/// other type the index does not hold may declare anything, so a method
/// below one is refused.
fn overrides_outside(index: &SymbolIndex, method: &str, name: &str) -> Result<(), String> {
    let Some((owner, _)) = method.rsplit_once("::") else {
        return Ok(());
    };
    for above in outside_above(index, owner) {
        match nvs_stdlib::registry::class(&above) {
            Some(core) => {
                if core_has(core, name) {
                    return Err(format!(
                        "`{method}` overrides a method of `{above}`, which is declared outside \
                         this workspace, so it cannot be renamed."
                    ));
                }
            }
            None => {
                return Err(format!(
                    "`{method}` may override a method of `{above}`, which is declared outside \
                     this workspace, so it cannot be renamed."
                ));
            }
        }
    }
    Ok(())
}

/// Whether the `Core` class `core` has a method named `name`.
fn core_has(core: &nvs_stdlib::registry::CoreClass, name: &str) -> bool {
    core.methods
        .iter()
        .chain(core.instance.iter())
        .any(|method| method.name == name)
}

/// Every type above `class` that the index holds no declaration for, reached
/// through the types it does hold.
fn outside_above(index: &SymbolIndex, class: &str) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::from([class.to_owned()]);
    let mut pending = vec![class.to_owned()];
    let mut found = Vec::new();
    while let Some(current) = pending.pop() {
        let Some(declared) = index.declaration(&current) else {
            continue;
        };
        for above in &declared.supertypes {
            if !seen.insert(above.clone()) {
                continue;
            }
            if index.declaration(above).is_some() {
                pending.push(above.clone());
            } else {
                found.push(above.clone());
            }
        }
    }
    found
}

/// `class`, every type above it and every type below it, at any distance.
fn lineage(index: &SymbolIndex, class: &str) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = BTreeSet::from([class.to_owned()]);
    for upwards in [true, false] {
        let mut pending = vec![class.to_owned()];
        while let Some(current) = pending.pop() {
            let next = if upwards {
                index.supertypes(&current)
            } else {
                index.subtypes(&current)
            };
            for declared in next {
                if found.insert(declared.symbol.clone()) {
                    pending.push(declared.symbol.clone());
                }
            }
        }
    }
    found
}

/// Every declaration and occurrence of each of `symbols`, sorted.
fn sites_of(index: &SymbolIndex, symbols: &[String]) -> Vec<Site> {
    let mut sites: Vec<Site> = symbols
        .iter()
        .flat_map(|symbol| {
            index
                .declaration(symbol)
                .map(|declared| declared.site.clone())
                .into_iter()
                .chain(
                    index
                        .occurrences(symbol)
                        .into_iter()
                        .map(|used| used.site.clone()),
                )
        })
        .collect();
    sites.sort();
    sites.dedup();
    sites
}

/// The edits that rename the members of `family` from `old` to `new`, after
/// the clash check.
fn members(
    documents: &Documents,
    index: &SymbolIndex,
    family: &[String],
    kind: DeclKind,
    old: &str,
    new: &str,
    sites: &[Site],
) -> Result<Vec<Rewrite>, String> {
    if kind == DeclKind::Method && new == CONSTRUCTOR {
        return Err(
            "`constructor` is the name `new` calls, so a method cannot be renamed to it."
                .to_owned(),
        );
    }
    if matches!(kind, DeclKind::Const | DeclKind::EnumCase) && new.eq_ignore_ascii_case("class") {
        return Err("`class` is not allowed as the name of a constant or an enum case.".to_owned());
    }
    let spelled = if kind == DeclKind::Property {
        format!("${new}")
    } else {
        new.to_owned()
    };
    for member in family {
        let Some((owner, _)) = member.rsplit_once("::") else {
            continue;
        };
        for class in lineage(index, owner) {
            if let Some(clash) = index.declaration(&format!("{class}::{spelled}")) {
                return Err(format!(
                    "`{class}` already has {} named `{spelled}`.",
                    clash.kind.describe()
                ));
            }
        }
        if kind == DeclKind::Method {
            for above in outside_above(index, owner) {
                if nvs_stdlib::registry::class(&above).is_some_and(|core| core_has(core, new)) {
                    return Err(format!("`{above}` already has a method named `{new}`."));
                }
            }
        }
    }

    let mut texts = Texts::new(documents);
    let mut rewrites = Vec::new();
    for site in sites {
        let written = texts
            .text(site)
            .ok_or_else(|| format!("`{}` could not be read.", plain(&site.path).display()))?;
        let text = if written == old {
            new.to_owned()
        } else if written.strip_prefix('$') == Some(old) {
            format!("${new}")
        } else {
            return Err(format!(
                "`{}` is written in `{}` in a way that rename cannot change, so it cannot be renamed.",
                family.first().map_or(old, String::as_str),
                plain(&site.path).display()
            ));
        };
        rewrites.push(Rewrite {
            site: site.clone(),
            text,
        });
    }
    Ok(rewrites)
}

/// The edits that rename the type `symbol` from `old` to `new`, after three
/// checks.
///
/// The new name may not be a built-in type, and no type in the workspace may
/// have it already. No file a site is in may write it already, because that
/// file may import a type of that name or read it as something else. And every
/// place the workspace writes `old` as a name or as a string must be a site the
/// index holds, because a place it does not hold is one the rename would miss.
///
/// A type whose file is named after it moves its file too ([`named_after`]),
/// with the `require` edits that move makes, unless a file is already there.
fn types(
    documents: &Documents,
    index: &SymbolIndex,
    symbol: &str,
    old: &str,
    new: &str,
    sites: &[Site],
) -> Result<Renamed, String> {
    if BUILT_IN.iter().any(|it| it.eq_ignore_ascii_case(new)) {
        return Err(format!(
            "`{new}` is a built-in type, so a type cannot be renamed to it."
        ));
    }
    for file in index.files() {
        for declared in index.declarations_in(file) {
            let short = declared
                .symbol
                .rsplit('\\')
                .next()
                .unwrap_or(&declared.symbol);
            if !declared.symbol.contains("::") && short.eq_ignore_ascii_case(new) {
                return Err(format!(
                    "`{}` already exists, so `{symbol}` cannot be renamed to `{new}`.",
                    declared.symbol
                ));
            }
        }
    }

    let mut texts = Texts::new(documents);
    let touched: BTreeSet<&Path> = sites.iter().map(|site| site.path.as_path()).collect();
    for path in &touched {
        let file = texts
            .file(path)
            .ok_or_else(|| format!("`{}` could not be read.", plain(path).display()))?;
        let tokens = tokenize(file, &mut Diagnostics::new());
        if tokens
            .iter()
            .any(|token| token.kind == TokenKind::Ident && text_of(file, token.span) == new)
        {
            return Err(format!(
                "`{}` already uses the name `{new}`, so `{symbol}` cannot be renamed to it.",
                plain(path).display()
            ));
        }
    }
    for path in index.files() {
        let Some(file) = texts.file(path) else {
            continue;
        };
        let tokens = tokenize(file, &mut Diagnostics::new());
        let missed = tokens.iter().find(|token| {
            let written = text_of(file, token.span);
            let named = match token.kind {
                TokenKind::Ident => written == old,
                TokenKind::SingleQuotedString | TokenKind::StringPart => {
                    let inner = written.trim_matches(['\'', '"']);
                    inner == old || inner.ends_with(&format!("\\{old}"))
                }
                _ => false,
            };
            named
                && !sites.iter().any(|site| {
                    site.path == path
                        && site.start <= token.span.start
                        && token.span.end <= site.end
                })
        });
        if let Some(missed) = missed {
            // `line_col` counts from 0, and an editor shows lines from 1.
            let (line, _) = file.line_col(missed.span.start);
            return Err(format!(
                "`{old}` is written in `{}` on line {} in a place rename cannot change, so \
                 `{symbol}` cannot be renamed.",
                plain(path).display(),
                line + 1
            ));
        }
    }

    let mut rewrites = Vec::new();
    for site in sites {
        let written = texts
            .text(site)
            .ok_or_else(|| format!("`{}` could not be read.", plain(&site.path).display()))?;
        let text = if written == old {
            new.to_owned()
        } else if let Some(prefix) = written.strip_suffix(old).filter(|it| it.ends_with('\\')) {
            format!("{prefix}{new}")
        } else {
            return Err(format!(
                "`{symbol}` is written in `{}` in a way that rename cannot change, so it cannot be \
                 renamed.",
                plain(&site.path).display()
            ));
        };
        rewrites.push(Rewrite {
            site: site.clone(),
            text,
        });
    }

    let file = named_after(index, symbol, old).map(|from| {
        let to = from.with_file_name(format!("{new}.nvs"));
        (from, to)
    });
    if let Some((from, to)) = &file {
        // `User.nvs` to `user.nvs` names the same file on a disk that ignores
        // case, and that file is the one being moved.
        let same_file = from
            .file_name()
            .zip(to.file_name())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right));
        if to.exists() && !same_file {
            return Err(format!(
                "`{}` already exists, so `{symbol}` cannot be renamed to `{new}`.",
                plain(to).display()
            ));
        }
        rewrites.extend(moved(documents, index, &[(from.clone(), to.clone())]));
        rewrites.sort();
        rewrites.dedup();
    }
    Ok(Renamed { rewrites, file })
}

/// The file `symbol` is declared in, when that file declares nothing else and
/// is named `old` plus `.nvs`. An `autoload` finds a type by that file name, so
/// the file is renamed with the type.
fn named_after(index: &SymbolIndex, symbol: &str, old: &str) -> Option<PathBuf> {
    let path = &index.declaration(symbol)?.site.path;
    let named = path.file_stem().is_some_and(|stem| stem == old)
        && path.extension().is_some_and(|extension| extension == "nvs");
    (named && index.sole_kinds().any(|(file, _)| file == path)).then(|| path.clone())
}

/// The text of the files sites are in, with the open buffers overlaid and each
/// file read once.
struct Texts {
    map: SourceMap,
    loaded: HashMap<PathBuf, Option<SourceId>>,
}

impl Texts {
    fn new(documents: &Documents) -> Self {
        let mut map = SourceMap::new();
        documents.overlay(&mut map);
        Self {
            map,
            loaded: HashMap::new(),
        }
    }

    /// The bytes of `site`, or `None` for a file that cannot be read.
    fn text(&mut self, site: &Site) -> Option<String> {
        let range = usize::try_from(site.start).ok()?..usize::try_from(site.end).ok()?;
        self.file(&site.path)?.text().get(range).map(str::to_owned)
    }

    /// The whole file at `path`, or `None` for a file that cannot be read.
    fn file(&mut self, path: &Path) -> Option<&SourceFile> {
        let map = &mut self.map;
        let id = (*self
            .loaded
            .entry(path.to_path_buf())
            .or_insert_with(|| map.load(path).ok()))?;
        Some(self.map.file(id))
    }
}

/// The local subject for the variable `$name` written at `token`.
fn local(
    analysed: &Analysed,
    file: &SourceFile,
    tokens: &[Token],
    entry: &Path,
    token: Span,
    name: &str,
) -> Result<Subject, String> {
    let scopes: Vec<(Span, &[LocalBinding])> = analysed
        .exprs
        .local_scopes()
        .filter(|(body, _)| body.file == analysed.entry)
        .collect();
    let offset = token.start;
    let mut covering: Vec<&(Span, &[LocalBinding])> = scopes
        .iter()
        .filter(|(body, _)| body.start <= offset && offset < body.end)
        .collect();
    covering.sort_by_key(|(body, _)| body.end - body.start);
    let Some(&&(body, bindings)) = covering
        .iter()
        .find(|(_, bindings)| bindings.iter().any(|binding| binding.name == name))
    else {
        return Err(format!(
            "`${name}` is not declared here, so it cannot be renamed."
        ));
    };
    let binding = bindings
        .iter()
        .find(|binding| binding.name == name)
        .ok_or(NOTHING)?;
    if binding.declared.start < body.start {
        return Err(format!(
            "`${name}` is a parameter, and a call can pass it by name. Renaming a parameter is \
             not available."
        ));
    }

    let stmts = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
        .map(|loaded| loaded.stmts.as_slice())
        .unwrap_or_default();
    let roots = walk::of_stmts(stmts);
    let nodes: Vec<&walk::Node> = roots
        .iter()
        .flat_map(|root| std::iter::once(root).chain(root.descendants()))
        .collect();

    // A named function, a method and a property hook inside the body are
    // scopes of their own, so nothing written in one is this local.
    let own: Vec<Span> = nodes
        .iter()
        .filter(|node| matches!(node.kind, "Function" | "Method" | "PropertyHook"))
        .filter(|node| inside(body, node.span) && node.span != body)
        .map(|node| node.span)
        .collect();
    let in_scope = |span: Span| inside(body, span) && !own.iter().any(|skip| inside(*skip, span));

    // An anonymous function reads this body's locals by value. One that binds
    // the same name has a variable of its own under it, and renaming would
    // have to tell the two apart.
    let mut nested: BTreeSet<String> = BTreeSet::new();
    for function in nodes
        .iter()
        .filter(|node| node.kind == "Fn" && in_scope(node.span))
    {
        nested.extend(bound_in(file, tokens, &scopes, function));
    }
    if nested.contains(name) {
        return Err(format!(
            "An anonymous function in this code also declares `${name}`, so `${name}` cannot be \
             renamed here."
        ));
    }

    let spelled = format!("${name}");
    let mut sites = Vec::new();
    for candidate in tokens.iter().filter(|candidate| {
        candidate.kind == TokenKind::Variable
            && in_scope(candidate.span)
            && text_of(file, candidate.span) == spelled
    }) {
        let Some(innermost) = innermost(&nodes, candidate.span) else {
            continue;
        };
        match innermost.kind {
            "Variable" | "LocalDecl" | "Foreach" | "Try" | "Destructure" | "StaticLocal" => {
                sites.push(candidate.span);
            }
            "Global" => {
                return Err(format!(
                    "`${name}` is a global variable here, so it cannot be renamed."
                ));
            }
            _ => {}
        }
    }
    if text_of(file, binding.declared) == spelled {
        sites.push(binding.declared);
    }
    sites.sort_by_key(|span| (span.start, span.end));
    sites.dedup();
    if !sites.contains(&token) {
        return Err(NOTHING.to_owned());
    }
    Ok(Subject {
        at: Site {
            path: entry.to_path_buf(),
            start: token.start,
            end: token.end,
        },
        what: What::Local {
            path: entry.to_path_buf(),
            name: name.to_owned(),
            bindings: bindings.to_vec(),
            nested,
            sites,
        },
    })
}

/// Every name the anonymous function `function` binds: its block body's own
/// bindings, and each parameter, which no child node covers.
fn bound_in(
    file: &SourceFile,
    tokens: &[Token],
    scopes: &[(Span, &[LocalBinding])],
    function: &walk::Node,
) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = scopes
        .iter()
        .filter(|(body, _)| inside(function.span, *body))
        .flat_map(|(_, bindings)| bindings.iter().map(|binding| binding.name.clone()))
        .collect();
    for token in tokens.iter().filter(|token| {
        token.kind == TokenKind::Variable
            && inside(function.span, token.span)
            && !function
                .children
                .iter()
                .any(|child| inside(child.span, token.span))
    }) {
        if let Some(name) = text_of(file, token.span).strip_prefix('$') {
            found.insert(name.to_owned());
        }
    }
    found
}

/// Whether `outer` holds every byte of `inner`.
const fn inside(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// The smallest node that holds `span`, a variable or a declaration before
/// any other node of the same size.
fn innermost<'a>(nodes: &[&'a walk::Node], span: Span) -> Option<&'a walk::Node> {
    nodes
        .iter()
        .filter(|node| inside(node.span, span))
        .min_by_key(|node| {
            (
                node.span.end - node.span.start,
                !matches!(node.kind, "Variable" | "LocalDecl"),
            )
        })
        .copied()
}

/// Whether `name` can be written as a variable, a method, a property, a
/// constant or an enum case: a letter or `_`, then letters, digits and `_`.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && chars.all(|rest| rest == '_' || rest.is_alphanumeric())
}

/// The key a path that may not exist yet is compared under: its directory as
/// [`canonical_key`] spells it, and its own name.
fn new_key(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => canonical_key(parent).join(name),
        _ => path.to_path_buf(),
    }
}

/// `written`, a `require` literal in the file that will be at `from`, written
/// again to name the file that will be at `to`, or `None` when it stays as it
/// is or is no plain literal.
fn rewritten(written: &str, from: &Path, to: &Path) -> Option<String> {
    let quote = written
        .chars()
        .next()
        .filter(|first| matches!(first, '\'' | '"'))?;
    let inner = written.strip_prefix(quote)?.strip_suffix(quote)?;
    // A literal with an escape or an interpolation is not the path it reads
    // as, so it is left alone.
    if inner.contains('\\') || (quote == '"' && inner.contains(['$', '{'])) {
        return None;
    }
    let target = plain(to);
    let path = if Path::new(inner).is_absolute() || inner.starts_with('/') {
        target.to_str()?.to_owned()
    } else {
        let relative = relative(&plain(from.parent()?), &target)?;
        if inner.starts_with("./") && !relative.starts_with("../") {
            format!("./{relative}")
        } else {
            relative
        }
    };
    (path != inner).then(|| format!("{quote}{path}{quote}"))
}

/// `path` without Windows' verbatim `\\?\` prefix, which
/// [`canonical_key`] adds and no program writes.
fn plain(path: &Path) -> PathBuf {
    path.to_str()
        .and_then(|text| text.strip_prefix(r"\\?\"))
        .map_or_else(|| path.to_path_buf(), PathBuf::from)
}

/// `to` relative to the directory `from`, with `/` separators, or `None` when
/// the two share no root.
fn relative(from: &Path, to: &Path) -> Option<String> {
    let from: Vec<Component<'_>> = from.components().collect();
    let to: Vec<Component<'_>> = to.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| same(left, right))
        .count();
    if common == 0 {
        return None;
    }
    let mut parts: Vec<String> = vec!["..".to_owned(); from.len() - common];
    parts.extend(
        to.iter()
            .skip(common)
            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
    );
    Some(parts.join("/"))
}

/// Whether two path components name the same directory, without regard to
/// case on Windows, whose file system ignores it.
fn same(left: &Component<'_>, right: &Component<'_>) -> bool {
    let (left, right) = (
        left.as_os_str().to_string_lossy(),
        right.as_os_str().to_string_lossy(),
    );
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_require_is_written_from_the_requiring_files_directory() {
        let from = Path::new("/w/app/main.nvs");
        assert_eq!(
            rewritten("'lib/user.nvs'", from, Path::new("/w/app/lib/account.nvs")).as_deref(),
            Some("'lib/account.nvs'")
        );
        assert_eq!(
            rewritten("\"./lib/user.nvs\"", from, Path::new("/w/shared/user.nvs")).as_deref(),
            Some("\"../shared/user.nvs\"")
        );
        assert_eq!(
            rewritten("'lib/user.nvs'", from, Path::new("/w/app/lib/user.nvs")),
            None
        );
        assert_eq!(
            rewritten("\"lib/$name.nvs\"", from, Path::new("/w/x.nvs")),
            None
        );
    }

    #[test]
    fn a_name_is_a_letter_or_underscore_then_word_characters() {
        assert!(is_identifier("total"));
        assert!(is_identifier("_count2"));
        assert!(!is_identifier("2count"));
        assert!(!is_identifier("first-name"));
        assert!(!is_identifier(""));
    }
}
