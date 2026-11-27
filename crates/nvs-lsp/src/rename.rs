//! Rename: a member at every site the index holds it, a local at every site
//! its body writes it, and the `require` paths and doc comment file targets a
//! moved file changes.
//!
//! The three requests `textDocument/prepareRename`, `textDocument/rename` and
//! `workspace/willRenameFiles` are answered here as [`Rewrite`]s or as a
//! refusal message, and nothing here is protocol: `crate::server` reads the
//! parameters, converts positions and builds the `WorkspaceEdit`, and the
//! `.lspt` runner renders the same rewrites.
//!
//! # What is renamed
//!
//! - **A type** — a class, an interface, an enum, a type alias declared
//!   outside a class, or an `extend` block's own name — at its declaration, at every occurrence the index holds
//!   and in every `use` line that imports it. Only the last segment changes, so
//!   `\Shop\User` and `use Shop\User;` keep their namespace. Before any edit,
//!   every file the index holds is read for the old name as a name or at the
//!   end of a string, and a place the index holds no site at refuses the
//!   rename: such a place is a site the edits would miss. A type declared
//!   alone in a file named after it, `User.nvs` for `User`, moves that file
//!   to the new name too, because the name is how an `autoload` finds it.
//!   Each `require` literal that names the file is written again as [`moved`]
//!   writes it, and a file already at the new path refuses the rename.
//!   A qualified new name, `Shop\Billing\Cart`, moves the type to that
//!   namespace instead ([`namespaced`]): its `namespace` line, its `use`
//!   lines, its qualified sites, the `use` lines a bare name now needs, and
//!   its file, to the path the `autoload` map gives the new name.
//! - **An owned type** — a type alias or an enum a class, an interface or an
//!   enum declares (`rule:types/class-scoped-alias`,
//!   `rule:enums/class-scoped-enum`) — at its declaration, at every
//!   `Owner::Name` the index holds and at every bare `Name` inside its
//!   owner's body. Only the name after `::` changes, and no file moves.
//! - **A member** — a method, a property, a class constant or an enum case —
//!   at its declaration and at every occurrence the index holds
//!   ([`SymbolIndex::declaration`], [`SymbolIndex::occurrences`]). A method
//!   takes its whole override family with it, closed over
//!   [`SymbolIndex::overridden`] and [`SymbolIndex::overriders`] in both
//!   directions, so no override stops overriding. A member an `extend` block
//!   adds is the block's (`Block::method`), and every call of it is a site,
//!   on whatever receiver it was written. A property site keeps or
//!   drops the `$` the way its own text has it: `public string $name` and
//!   `User::$count` write it, `$u->name` does not.
//! - **A property's compared name** — a string literal one of the property
//!   callbacks of its class compares with its name parameter
//!   (`rule:classes/a-compared-property-name-is-a-use`) — is an occurrence the
//!   index holds, so it is renamed with the property, between its quotes.
//!   [`callback_uses`] refuses the rename while a callback of the class or a
//!   class below it writes the old name another way, or writes its name
//!   parameter.
//! - **A local** of a function, a method or a script body: every `$name` the
//!   declaring body writes as a variable read, a declaration, a `foreach`,
//!   `catch`, destructuring or `static` binding, the binding's own
//!   declaration included. A nested function or method is a scope of its own
//!   and is skipped. An anonymous function reads the body's locals by value,
//!   so a capture inside one is renamed with the rest.
//! - **A parameter of a method**, as a local, and also its `@param` name in
//!   the method's `///` run and the `name:` of each named argument at every
//!   call the index holds for that method ([`named_arguments`]). A cursor on
//!   the `$name` of the `@param` line renames the parameter too. An override
//!   keeps its own parameter names, because a named argument binds to the
//!   method the call resolved to.
//!
//! - **A field of an `attribute` declaration**, at its declaration and at
//!   every key a payload writes for it. A read of the field on a retrieval's
//!   result is not a site the index holds, so a file that writes `->field`
//!   where the index holds no use refuses the rename ([`unresolved_reads`]).
//! - **A `@see` target or an `@throws` name** that resolved to a type or a
//!   member, as one more site of it ([`SymbolIndex::mentions_of`]). Only the
//!   renamed segment changes, so `@see Shop\User::greet` keeps its class
//!   when `greet` is renamed. A cursor on one renames what it names.
//!
//! # What is refused, each with a message the editor shows
//!
//! A cursor on no symbol; `$this`; a type
//! whose name is written somewhere the index holds no site; a parameter of a
//! constructor or of an anonymous function, and a promoted parameter, because
//! a call may pass it by name at a place the index does not hold; a method's
//! parameter where a call the index holds cannot be read; a local an anonymous
//! function inside the body declares
//! again; a symbol whose declaration the index does not hold, which is every
//! `Core` name and every file outside the workspace; a method that overrides
//! one declared outside the workspace; and a new name that is no identifier
//! or that clashes. A clash is a refusal and never an alias
//! (`rule:ide/no-refactoring-introduces-an-alias`): a member's new name may not
//! be declared already on any class of the family, above it or below it; a
//! local's may not be bound already in its body; and a type's may not be a
//! built-in type, the name of another type in the workspace, or a name any
//! file with a site already writes. An owned type's may not be a built-in
//! type, a name its owner already gives a constant, an enum case, an enum or
//! an alias, or a name its owner's file already writes.
//!
//! # A moved file
//!
//! [`moved`] reads each `require` edge the graph walk recorded
//! (`nvs_hir::requires::Loaded::requires`, the edge `crate::links` underlines),
//! in every graph that read a moved file, and writes the literal again so it
//! names the file at its new place: relative to the requiring file's new
//! directory with `/` separators when it was relative, absolute when it was
//! absolute, and in the same quotes. Each `@see` file target and `@example`
//! path the index holds ([`SymbolIndex::linked`]) is written again the same
//! way, its `#` anchor kept, and from the index alone.
//!
//! # What it costs
//!
//! Nothing is kept. A rename reads the index the server already holds and the
//! text of each file a site is in, once per request. A parameter's rename also
//! parses each file that calls its method once, per request. A file move
//! analyses each entry point that read a moved file once, per request, and
//! walks the index's doc comment file targets once. It reads the text of a
//! file only where a tag in it names a moved path.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, SourceId, SourceMap, Span, canonical_key};
use nvs_hir::QName;
use nvs_syntax::ast::{
    CallArgs, ClassMember, ClassMemberKind, Expr, ExprKind, FnBody, MemberName, Stmt, StmtKind,
};
use nvs_syntax::visit::{StmtChild, each_child_expr, each_child_of_stmt};
use nvs_syntax::{Token, TokenKind, parse_file, tokenize};
use nvs_types::LocalBinding;

use crate::definition::{covers, text_of};
use crate::document::{Analysed, Documents, analyse_file};
use crate::index::{DeclKind, Site, SymbolIndex, symbol_at};
use crate::locals;

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
/// The edit and the file move are checked before they are returned
/// ([`crate::refactor::checked`]): a rename that would add an error is refused
/// with that error.
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
    let Renamed { rewrites, file } = planned(documents, index, analysed, offset, new_name)?;
    let moves: Vec<(PathBuf, PathBuf)> = file.iter().cloned().collect();
    let checked = crate::refactor::checked(documents, index, rewrites, &moves)
        .map_err(|broken| broken.refusal())?;
    Ok(Renamed {
        rewrites: checked.rewrites,
        file,
    })
}

/// [`rename()`] before its check.
fn planned(
    documents: &Documents,
    index: &SymbolIndex,
    analysed: &Analysed,
    offset: BytePos,
    new_name: &str,
) -> Result<Renamed, String> {
    let subject = subject(index, analysed, offset)?;
    if new_name.contains('\\') {
        let What::Type { symbol, old, sites } = subject.what else {
            return Err(
                "Only a class, an interface, an enum or a type alias can take a name with a \
                 namespace."
                    .to_owned(),
            );
        };
        return namespaced(documents, index, &symbol, &old, new_name, &sites);
    }
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
        What::Owned {
            symbol,
            owner,
            old,
            sites,
        } => {
            if new == old {
                return Err("The new name is the same as the old one.".to_owned());
            }
            owned(documents, index, &symbol, &owner, &old, new, &sites).map(|rewrites| Renamed {
                rewrites,
                file: None,
            })
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
            if kind == DeclKind::Field {
                unresolved_reads(documents, index, &family, &old)?;
            }
            if kind == DeclKind::Property {
                callback_uses(documents, index, &family, &old)?;
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
            calls,
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
            let mut rewrites: Vec<Rewrite> = sites
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
            if let Some(method) = calls {
                rewrites.extend(named_arguments(documents, index, &method, &name, new)?);
                rewrites.sort();
                rewrites.dedup();
            }
            Ok(Renamed {
                rewrites,
                file: None,
            })
        }
    }
}

/// Every edit to a `require` literal, a `@see` file target and an `@example`
/// path that moving each `(old, new)` path makes necessary, in file and then
/// source order.
///
/// A path that is a directory moves every file under it. The edits are keyed
/// by each file's current path, because a client applies them before it moves
/// anything. A doc comment tag is rewritten from the index alone
/// ([`SymbolIndex::linked`]), and its `#` anchor stays as it is. A move that
/// no `require` edge and no tag names analyses nothing and reads no file.
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
    let roots: Vec<(PathBuf, PathBuf)> = moves
        .iter()
        .map(|(old, new)| (canonical_key(old), new_key(new)))
        .collect();
    let new_place = |path: &Path| -> Option<PathBuf> {
        roots.iter().find_map(|(old, new)| {
            let rest = path.strip_prefix(old).ok()?;
            Some(if rest.as_os_str().is_empty() {
                new.clone()
            } else {
                new.join(rest)
            })
        })
    };

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

    let mut texts = Texts::new(documents);
    for linked in index.linked() {
        let (from, to) = (new_place(&linked.site.path), new_place(&linked.target));
        if from.is_none() && to.is_none() {
            continue;
        }
        let Some(written) = texts.text(&linked.site) else {
            continue;
        };
        let Some(text) = rewritten_path(
            &written,
            from.as_deref().unwrap_or(&linked.site.path),
            to.as_deref().unwrap_or(&linked.target),
        ) else {
            continue;
        };
        found.push(Rewrite {
            site: linked.site.clone(),
            text,
        });
    }
    found.sort();
    found.dedup();
    found
}

/// What a rename at one cursor is about, and the name the cursor is on.
struct Subject {
    /// The bytes of the name under the cursor.
    at: Site,
    /// What a rename there changes.
    what: What,
}

/// The four things a rename changes.
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
    /// A type alias or an enum that a class, an interface or an enum declares.
    Owned {
        /// `Owner::Name`, spelled as the index spells it.
        symbol: String,
        /// The fully-qualified name of the type that declares it.
        owner: String,
        /// The name after `::`, which is the name a rename changes.
        old: String,
        /// Its declaration and every occurrence.
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
        /// The method a parameter belongs to, spelled as the index spells it,
        /// whose calls pass it by name. `None` for every other local.
        calls: Option<String>,
    },
}

/// What the cursor at `offset` would rename, or why nothing there can be.
fn subject(index: &SymbolIndex, analysed: &Analysed, offset: BytePos) -> Result<Subject, String> {
    let file = analysed.map.file(analysed.entry);
    let entry = file.path().map(canonical_key).ok_or(NOTHING)?;
    let tokens = tokenize(file, &mut Diagnostics::new());
    // A `@see` target or an `@throws` name is in a comment, so no token
    // covers it either. Its site is the one the index holds for it.
    if let Some(symbol) = crate::index::doc_symbol_at(analysed, offset) {
        let here = index
            .mentions_in(&entry)
            .iter()
            .find(|mention| {
                mention.symbol == symbol
                    && mention.site.start <= offset
                    && offset <= mention.site.end
            })
            .ok_or(NOTHING)?;
        return member(index, &symbol, here.site.clone());
    }
    // The `$name` of an `@param` line is in a comment, so no token covers it.
    if let Some((target, param)) = locals::param_tag_at(analysed, offset) {
        let written = text_of(file, param.name);
        return local(
            analysed,
            file,
            &tokens,
            &entry,
            param.name,
            target,
            &written[1..],
        );
    }
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
        return local(
            analysed,
            file,
            &tokens,
            &entry,
            token.span,
            token.span,
            &written[1..],
        );
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
        DeclKind::Class
            | DeclKind::Interface
            | DeclKind::Enum
            | DeclKind::TypeAlias
            | DeclKind::Extend
    ) {
        if let Some((owner, old)) = symbol.rsplit_once("::") {
            return Ok(Subject {
                at,
                what: What::Owned {
                    symbol: symbol.to_owned(),
                    owner: owner.to_owned(),
                    old: old.to_owned(),
                    sites: sites_of(index, &[symbol.to_owned()]),
                },
            });
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

/// Every declaration, occurrence and doc comment tag of each of `symbols`,
/// sorted.
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
                .chain(
                    index
                        .mentions_of(symbol)
                        .into_iter()
                        .map(|mentioned| mentioned.site.clone()),
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

/// An error when a file the index holds reads a member named `old` with `->`
/// or `?->`, and the index holds no use there.
///
/// Such a read is on a value whose type has no class: an attribute's payload
/// from a retrieval, or another shape. The checker records no link from that
/// read to the attribute declaration, so a rename cannot tell whether the read
/// is of the field being renamed. A read of a class property is a use the
/// index holds, and it does not refuse.
fn unresolved_reads(
    documents: &Documents,
    index: &SymbolIndex,
    family: &[String],
    old: &str,
) -> Result<(), String> {
    let symbol = family.first().map_or(old, String::as_str);
    let mut texts = Texts::new(documents);
    for path in index.files() {
        let Some(file) = texts.file(path) else {
            continue;
        };
        let held = index.occurrences_in(path);
        let tokens = tokenize(file, &mut Diagnostics::new());
        let read = tokens.windows(2).find(|pair| {
            let [arrow, name] = pair else {
                return false;
            };
            matches!(arrow.kind, TokenKind::Arrow | TokenKind::NullsafeArrow)
                && name.kind == TokenKind::Ident
                && text_of(file, name.span) == old
                && !held.iter().any(|occurrence| {
                    occurrence.site.start <= name.span.start && name.span.end <= occurrence.site.end
                })
        });
        if let Some([_, name]) = read {
            // `line_col` counts from 0, and an editor shows lines from 1.
            let (line, _) = file.line_col(name.span.start);
            return Err(format!(
                "`{}` reads `->{old}` on line {}. Rename cannot tell if that value comes from \
                 this attribute, so `{symbol}` cannot be renamed.",
                path.file_name().map_or_else(
                    || plain(path).display().to_string(),
                    |name| name.to_string_lossy().into_owned()
                ),
                line + 1
            ));
        }
    }
    Ok(())
}

/// An error when a property callback of a class in `family`, or of a class
/// below it, writes `old` as a literal in a way the index does not follow
/// (`rule:classes/a-compared-property-name-is-a-use`).
///
/// That is a literal the callback does not compare with its name parameter,
/// or any literal while the body writes that parameter. A compared literal is
/// a site the index holds, and the rename changes it. Every method named
/// after a callback is read, whether or not its class implements the
/// interface, so a doubtful place refuses.
fn callback_uses(
    documents: &Documents,
    index: &SymbolIndex,
    family: &[String],
    old: &str,
) -> Result<(), String> {
    let symbol = family.first().map_or(old, String::as_str);
    let mut classes = BTreeSet::new();
    for member in family {
        let Some((owner, _)) = member.rsplit_once("::") else {
            continue;
        };
        let mut pending = vec![owner.to_owned()];
        while let Some(current) = pending.pop() {
            if classes.insert(current.clone()) {
                pending.extend(
                    index
                        .subtypes(&current)
                        .into_iter()
                        .map(|below| below.symbol.clone()),
                );
            }
        }
    }
    let mut texts = Texts::new(documents);
    for class in &classes {
        let Some(declared) = index.declaration(class) else {
            continue;
        };
        let path = declared.site.path.clone();
        let Some(file) = texts.file(&path) else {
            continue;
        };
        let stmts = parse_file(file, &mut Diagnostics::new());
        let Some(members) = class_members(&stmts, declared.site.start) else {
            continue;
        };
        for (method, _, names) in nvs_hir::compared_names::callbacks_in(members, file.text()) {
            let named = |span: &&Span| text_of(file, **span) == old;
            let place = if names.assigned.is_some() {
                names.compared.iter().chain(&names.other).find(named)
            } else {
                names.other.iter().find(named)
            };
            let Some(place) = place else {
                continue;
            };
            let callback = text_of(file, method.name);
            let file_name = path.file_name().map_or_else(
                || plain(&path).display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            // `line_col` counts from 0, and an editor shows lines from 1.
            let line = file.line_col(place.start).0 + 1;
            return Err(if names.assigned.is_some() {
                format!(
                    "`{callback}` in `{file_name}` changes its first parameter, so rename cannot \
                     tell what `'{old}'` on line {line} names. `{symbol}` cannot be renamed."
                )
            } else {
                format!(
                    "`{callback}` in `{file_name}` uses `'{old}'` on line {line}, but not in a \
                     comparison with the property name. So `{symbol}` cannot be renamed."
                )
            });
        }
    }
    Ok(())
}

/// The members of the class whose name starts at `name` in `stmts`, inside a
/// namespace block or not.
fn class_members(stmts: &[Stmt], name: BytePos) -> Option<&[ClassMember]> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(decl) => class_members(&decl.body.as_ref()?.stmts, name),
        StmtKind::ClassDecl(decl) if decl.name.span.start == name => Some(decl.members.as_slice()),
        _ => None,
    })
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
    written_already(&mut texts, symbol, new, sites)?;
    missed_sites(&mut texts, index, symbol, old, sites)?;

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
            return Err(cannot_change(symbol, &site.path));
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
        refuse_taken(from, to, symbol, new)?;
        rewrites.extend(moved(documents, index, &[(from.clone(), to.clone())]));
        rewrites.sort();
        rewrites.dedup();
    }
    Ok(Renamed { rewrites, file })
}

/// The refusal for a site whose text is neither the old name nor a qualified
/// name that ends with it.
fn cannot_change(symbol: &str, path: &Path) -> String {
    format!(
        "`{symbol}` is written in `{}` in a way that rename cannot change, so it cannot be \
         renamed.",
        plain(path).display()
    )
}

/// Refuses a move of `from` to `to` while another file is at `to`.
fn refuse_taken(from: &Path, to: &Path, symbol: &str, new: &str) -> Result<(), String> {
    // `User.nvs` to `user.nvs` names the same file on a disk that ignores
    // case, and that file is the one being moved.
    let same_file = from.parent() == to.parent()
        && from
            .file_name()
            .zip(to.file_name())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right));
    if to.exists() && !same_file {
        return Err(format!(
            "`{}` already exists, so `{symbol}` cannot be renamed to `{new}`.",
            plain(to).display()
        ));
    }
    Ok(())
}

/// Refuses a type's new short name `new` while a file with a site of it
/// already writes that name, since a site there would then name two things.
fn written_already(
    texts: &mut Texts,
    symbol: &str,
    new: &str,
    sites: &[Site],
) -> Result<(), String> {
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
    Ok(())
}

/// Refuses a type's rename while any file the index holds writes its short
/// name `old`, as a name or at the end of a string, where the index holds no
/// site: the edits would miss that place.
fn missed_sites(
    texts: &mut Texts,
    index: &SymbolIndex,
    symbol: &str,
    old: &str,
    sites: &[Site],
) -> Result<(), String> {
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
    Ok(())
}

/// The edits that move `symbol`, a type whose short name is `old`, to the
/// namespace `written` names, and the move of its file
/// (`rule:ide/a-type-moves-with-its-namespace`).
///
/// `written` is a qualified name, `Billing\Cart` or `\Billing\Invoice`. A name
/// in the type's own namespace is an ordinary rename. Only a type that is its
/// file's sole declaration moves. Its file moves to the path that the
/// `autoload` map of every program that loads it gives the new name
/// ([`autoloaded_path`]), and a file only a `require` reaches keeps its path.
///
/// Each `use` line of the type names the new name, and one in a file of the
/// new namespace is deleted. A qualified site names the new name. A file that
/// wrote the bare name through the old namespace gets a `use` line. Inside the
/// moved file, the `namespace` line names the new namespace, each type of the
/// old namespace it wrote bare gets a `use` line, and each `use` line that
/// names the new namespace is deleted. Every `use` block an edit touches is
/// sorted as `nvs fmt` sorts it ([`headed`]).
fn namespaced(
    documents: &Documents,
    index: &SymbolIndex,
    symbol: &str,
    old: &str,
    written: &str,
    sites: &[Site],
) -> Result<Renamed, String> {
    let full = written.strip_prefix('\\').unwrap_or(written);
    if full.split('\\').any(|segment| !is_identifier(segment)) {
        return Err(format!("`{written}` is not a valid name."));
    }
    let old_space = namespace_of(symbol);
    let (space, new) = full.rsplit_once('\\').unwrap_or(("", full));
    if space == old_space {
        if new == old {
            return Err("The new name is the same as the old one.".to_owned());
        }
        return types(documents, index, symbol, old, new, sites);
    }
    if space.is_empty() || old_space.is_empty() {
        return Err(format!(
            "A type can only move from one namespace to another, so `{symbol}` cannot be \
             renamed to `{written}`."
        ));
    }
    if BUILT_IN.iter().any(|it| it.eq_ignore_ascii_case(new)) {
        return Err(format!(
            "`{new}` is a built-in type, so a type cannot be renamed to it."
        ));
    }
    if let Some(clash) = index
        .all_declarations()
        .find(|declared| declared.symbol.eq_ignore_ascii_case(full))
    {
        return Err(format!(
            "`{}` already exists, so `{symbol}` cannot be renamed to `{full}`.",
            clash.symbol
        ));
    }
    let declared = index
        .declaration(symbol)
        .ok_or_else(|| outside(index, symbol))?;
    let path = declared.site.path.clone();
    if !index.sole_kinds().any(|(file, _)| file == path) {
        return Err(format!(
            "`{symbol}` is not the only declaration in its file. Move it to its own file first."
        ));
    }
    let to = autoloaded_path(documents, index, &path, symbol, full)?;
    if let Some(to) = &to {
        refuse_taken(&path, to, symbol, full)?;
    }

    let mut texts = Texts::new(documents);
    if new != old {
        written_already(&mut texts, symbol, new, sites)?;
    }
    missed_sites(&mut texts, index, symbol, old, sites)?;

    let imports = index.imports_of(symbol);
    // The edits to `use` and `namespace` lines, which [`headed`] sorts, and
    // every other edit.
    let mut heads: Vec<Rewrite> = Vec::new();
    let mut rewrites: Vec<Rewrite> = Vec::new();
    // Each file that needs `use` lines: the first place it needs one, and the
    // names.
    let mut wanted: BTreeMap<PathBuf, (BytePos, BTreeSet<String>)> = BTreeMap::new();
    for site in sites {
        let text = texts
            .text(site)
            .ok_or_else(|| format!("`{}` could not be read.", plain(&site.path).display()))?;
        let replace = |text: String| Rewrite {
            site: site.clone(),
            text,
        };
        if *site == declared.site {
            if new != old {
                rewrites.push(replace(new.to_owned()));
            }
            continue;
        }
        if imports.iter().any(|import| import.site == *site) {
            let file = texts
                .file(&site.path)
                .ok_or_else(|| format!("`{}` could not be read.", plain(&site.path).display()))?;
            let shape = Shape::of(file);
            if shape.namespace_at(site.start) == space {
                if let Some(line) = shape.uses.iter().find(|line| line.covers(site.start)) {
                    heads.push(line.deletion(&site.path));
                }
            } else {
                let lead = if text.starts_with('\\') { "\\" } else { "" };
                heads.push(replace(format!("{lead}{full}")));
            }
            continue;
        }
        if text.contains('\\') {
            // A name with a `\` always starts at the root, so it is the full
            // name, as a `@see` target or an `@throws` name is too.
            let lead = if text.starts_with('\\') { "\\" } else { "" };
            rewrites.push(replace(format!("{lead}{full}")));
            continue;
        }
        if text != old {
            return Err(cannot_change(symbol, &site.path));
        }
        if new != old {
            rewrites.push(replace(new.to_owned()));
        }
        if site.path == path || imports.iter().any(|import| import.site.path == site.path) {
            continue;
        }
        let entry = wanted
            .entry(site.path.clone())
            .or_insert_with(|| (site.start, BTreeSet::new()));
        entry.0 = entry.0.min(site.start);
        entry.1.insert(full.to_owned());
    }

    // The moved file: its `namespace` line, its old neighbours, and each `use`
    // line that now names its own namespace.
    let neighbours: Vec<(String, Site)> = index
        .occurrences_in(&path)
        .iter()
        .map(|used| (used.symbol.clone(), used.site.clone()))
        .chain(
            index
                .mentions_in(&path)
                .iter()
                .map(|mentioned| (mentioned.symbol.clone(), mentioned.site.clone())),
        )
        .filter(|(named, _)| {
            !named.contains("::")
                && named != symbol
                && namespace_of(named) == old_space
                && index.declaration(named).is_some()
        })
        .collect();
    let mut bare: BTreeSet<String> = BTreeSet::new();
    for (named, site) in neighbours {
        if texts.text(&site).is_some_and(|text| !text.contains('\\')) {
            bare.insert(named);
        }
    }
    let file = texts
        .file(&path)
        .ok_or_else(|| format!("`{}` could not be read.", plain(&path).display()))?;
    let shape = Shape::of(file);
    let line = shape
        .spaces
        .iter()
        .find(|it| it.start <= declared.site.start && declared.site.start < it.end)
        .and_then(|it| it.name.as_ref())
        .ok_or_else(|| {
            format!(
                "`{}` has no `namespace` line, so `{symbol}` cannot move.",
                plain(&path).display()
            )
        })?;
    heads.push(Rewrite {
        site: Site {
            path: path.clone(),
            start: line.1.start,
            end: line.1.end,
        },
        text: space.to_owned(),
    });
    for line in &shape.uses {
        bare.remove(&line.name);
        if namespace_of(&line.name) == space {
            heads.push(line.deletion(&path));
        }
    }
    if !bare.is_empty() {
        let entry = wanted
            .entry(path.clone())
            .or_insert_with(|| (declared.site.start, BTreeSet::new()));
        entry.0 = entry.0.min(declared.site.start);
        entry.1.extend(bare);
    }

    for (file_path, (at, names)) in wanted {
        let file = texts
            .file(&file_path)
            .ok_or_else(|| format!("`{}` could not be read.", plain(&file_path).display()))?;
        let stmts = parse_file(file, &mut Diagnostics::new());
        let place = nvs_hir::import_site(&stmts, file, at).ok_or_else(|| {
            format!(
                "`{}` has no place for a `use` line, so `{symbol}` cannot move.",
                plain(&file_path).display()
            )
        })?;
        let names: Vec<QName> = names.iter().map(|name| QName::parse(name)).collect();
        heads.push(Rewrite {
            site: Site {
                path: file_path,
                start: place.at,
                end: place.at,
            },
            text: place.use_lines(names.iter()),
        });
    }

    rewrites.extend(headed(&mut texts, heads)?);
    let file = to.map(|to| (path, to));
    if let Some((from, to)) = &file {
        rewrites.extend(moved(documents, index, &[(from.clone(), to.clone())]));
    }
    rewrites.sort();
    rewrites.dedup();
    Ok(Renamed { rewrites, file })
}

/// The namespace of `symbol`, a fully-qualified name, and `""` for the global
/// namespace.
fn namespace_of(symbol: &str) -> &str {
    symbol.rsplit_once('\\').map_or("", |(space, _)| space)
}

/// The path the `autoload` map of every program that loads `path` gives
/// `full`, the first root of its longest prefix, or `None` where no program
/// loads `path` through its map, since only a `require` reaches that file.
///
/// Each entry point that read `path` is analysed once.
///
/// # Errors
///
/// A program that would lose the type, because no root of its map reaches the
/// new name, and programs whose maps give different paths.
fn autoloaded_path(
    documents: &Documents,
    index: &SymbolIndex,
    path: &Path,
    symbol: &str,
    full: &str,
) -> Result<Option<PathBuf>, String> {
    let name = QName::parse(full);
    let mut found: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for entry in index.readers(path) {
        let Some(analysed) = analyse_file(documents, &entry, 0) else {
            continue;
        };
        if !analysed.autoload.claims(path) {
            continue;
        }
        let probe = analysed.autoload.resolve(&name);
        let Some(to) = probe.tried.first() else {
            return Err(format!(
                "No `autoload` root of `{}` reaches `{full}`, so that program would lose \
                 `{symbol}`.",
                plain(&entry).display()
            ));
        };
        found.entry(new_key(to)).or_default().push(entry);
    }
    if found.len() > 1 {
        let programs: Vec<String> = found
            .values()
            .flatten()
            .map(|entry| format!("`{}`", plain(entry).display()))
            .collect();
        return Err(format!(
            "The programs that load `{symbol}` would find `{full}` at different paths: {}.",
            programs.join(", ")
        ));
    }
    Ok(found.into_keys().next())
}

/// `heads`, the edits to the `use` and `namespace` lines of each file, as one
/// edit per file whose `use` block is then sorted with `nvs fmt`'s sort, so a
/// format after the rename moves nothing.
///
/// An insertion that falls inside a deleted line goes where that line was. A
/// file that does not parse after the edits is left unsorted, and the check
/// that follows names its error.
fn headed(texts: &mut Texts, mut heads: Vec<Rewrite>) -> Result<Vec<Rewrite>, String> {
    heads.sort();
    heads.dedup();
    let mut out = Vec::new();
    for group in heads.chunk_by(|a, b| a.site.path == b.site.path) {
        let path = &group[0].site.path;
        let file = texts
            .file(path)
            .ok_or_else(|| format!("`{}` could not be read.", plain(path).display()))?;
        let before = file.text().to_owned();
        let name = file.name().to_owned();
        let mut edits: Vec<(usize, usize, String)> = group
            .iter()
            .map(|it| {
                (
                    it.site.start as usize,
                    it.site.end as usize,
                    it.text.clone(),
                )
            })
            .collect();
        // An insertion goes after a `use` line. When that line is deleted, the
        // inserted lines replace it, after the line breaks it was deleted
        // with, so the blank lines around the block stay as they were.
        let inserted: Vec<usize> = (0..edits.len())
            .filter(|&at| edits[at].0 == edits[at].1)
            .collect();
        for at in inserted {
            let place = edits[at].0;
            let Some(gone) = (0..edits.len()).find(|&other| {
                let (start, end, text) = &edits[other];
                text.is_empty() && *start < place && place <= *end
            }) else {
                continue;
            };
            let (start, end) = (edits[gone].0, edits[gone].1);
            let breaks = before[start..end]
                .bytes()
                .take_while(|byte| matches!(byte, b'\n' | b'\r'))
                .count();
            let lines = edits[at].2.trim_start_matches(['\n', '\r']).to_owned();
            edits[gone].2 = format!("{}{lines}", &before[start..start + breaks]);
            edits[at].2.clear();
        }
        edits.retain(|(start, end, text)| start != end || !text.is_empty());
        edits.sort_by_key(|(start, end, _)| (*start, *end));
        let mut after = String::with_capacity(before.len());
        let mut at = 0;
        for (start, end, text) in edits {
            let piece = before.get(at..start).ok_or_else(|| {
                format!(
                    "Two edits overlap in `{}`, so nothing was changed.",
                    plain(path).display()
                )
            })?;
            after.push_str(piece);
            after.push_str(&text);
            at = end;
        }
        after.push_str(before.get(at..).unwrap_or_default());
        let mut map = SourceMap::new();
        let id = map.add(name, after);
        let sorted = match nvs_fmt::sorted_imports(map.file(id)) {
            Ok(Some(sorted)) => sorted,
            _ => map.file(id).text().to_owned(),
        };
        if sorted == before {
            continue;
        }
        let (start, end) = crate::organize::changed(&before, &sorted);
        let to = |at: usize| BytePos::try_from(at).unwrap_or(BytePos::MAX);
        out.push(Rewrite {
            site: Site {
                path: path.clone(),
                start: to(start),
                end: to(end),
            },
            text: sorted[start..sorted.len() - (before.len() - end)].to_owned(),
        });
    }
    Ok(out)
}

/// The `namespace` and `use` lines of one file, read off its statements.
struct Shape {
    /// Each namespace declaration: where it is in force and its name.
    spaces: Vec<Space>,
    /// Each `use` line, at the top level and in a bracketed namespace.
    uses: Vec<UseLine>,
}

/// One namespace declaration.
struct Space {
    /// Where it is in force: a bracketed block, or from a statement to the next
    /// statement-form declaration.
    start: BytePos,
    end: BytePos,
    /// Its name and the span of the name, or `None` for the global form.
    name: Option<(String, Span)>,
}

/// One `use` line.
struct UseLine {
    /// The name it imports, without a leading `\`.
    name: String,
    /// The statement's bytes.
    start: BytePos,
    end: BytePos,
    /// The bytes that delete it: the statement and the line break before it.
    gone: (BytePos, BytePos),
}

impl UseLine {
    fn covers(&self, at: BytePos) -> bool {
        self.start <= at && at < self.end
    }

    fn deletion(&self, path: &Path) -> Rewrite {
        Rewrite {
            site: Site {
                path: path.to_path_buf(),
                start: self.gone.0,
                end: self.gone.1,
            },
            text: String::new(),
        }
    }
}

impl Shape {
    fn of(file: &SourceFile) -> Self {
        let stmts = parse_file(file, &mut Diagnostics::new());
        let mut shape = Self {
            spaces: Vec::new(),
            uses: Vec::new(),
        };
        for stmt in &stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(decl) => {
                    let name = decl
                        .name
                        .as_ref()
                        .map(|name| (text_of(file, name.span).to_owned(), name.span));
                    if let Some(open) = shape.spaces.last_mut().filter(|it| it.end == BytePos::MAX)
                    {
                        open.end = stmt.span.start;
                    }
                    match &decl.body {
                        Some(block) => {
                            shape.spaces.push(Space {
                                start: block.span.start,
                                end: block.span.end,
                                name,
                            });
                            for inner in &block.stmts {
                                if let StmtKind::UseDecl(used) = &inner.kind {
                                    shape.uses.push(UseLine::of(file, used));
                                }
                            }
                        }
                        None => shape.spaces.push(Space {
                            start: stmt.span.start,
                            end: BytePos::MAX,
                            name,
                        }),
                    }
                }
                StmtKind::UseDecl(used) => shape.uses.push(UseLine::of(file, used)),
                _ => {}
            }
        }
        shape
    }

    /// The namespace in force at `at`, and `""` for the global namespace.
    fn namespace_at(&self, at: BytePos) -> &str {
        self.spaces
            .iter()
            .find(|it| it.start <= at && at < it.end)
            .and_then(|it| it.name.as_ref())
            .map_or("", |(name, _)| name.trim_start_matches('\\'))
    }
}

impl UseLine {
    fn of(file: &SourceFile, used: &nvs_syntax::ast::UseDecl) -> Self {
        let text = file.text().as_bytes();
        let (start, end) = (used.span.start, used.span.end);
        let line_break_before = |at: usize| -> Option<usize> {
            if at >= 2 && text.get(at - 2..at) == Some(b"\r\n") {
                Some(at - 2)
            } else if at >= 1 && text.get(at - 1) == Some(&b'\n') {
                Some(at - 1)
            } else {
                None
            }
        };
        let after = text.get(end as usize..).unwrap_or_default();
        let blank_after =
            after.is_empty() || after.starts_with(b"\n\n") || after.starts_with(b"\r\n\r\n");
        let at = start as usize;
        let from = match line_break_before(at) {
            // A line alone between two blank lines takes one of them with it,
            // so no two blank lines are left.
            Some(before) if blank_after => line_break_before(before).unwrap_or(before),
            Some(before) => before,
            None => at,
        };
        Self {
            name: text_of(file, used.path.span)
                .trim_start_matches('\\')
                .to_owned(),
            start,
            end,
            gone: (BytePos::try_from(from).unwrap_or(start), end),
        }
    }
}

/// The edits that rename `symbol`, an alias or an enum that `owner` declares,
/// from `old` to `new`, after three checks.
///
/// The new name may not be a built-in type. The owner may not declare a
/// constant, an enum case, an enum or an alias of that name already, because
/// one body gives a name to one thing (`E0304`). And the owner's file may not
/// write the new name already, because a bare site inside the owner's body
/// would then name two things. Only the last segment of a site changes, and no
/// file moves: an owned name has no file of its own.
fn owned(
    documents: &Documents,
    index: &SymbolIndex,
    symbol: &str,
    owner: &str,
    old: &str,
    new: &str,
    sites: &[Site],
) -> Result<Vec<Rewrite>, String> {
    if BUILT_IN.iter().any(|it| it.eq_ignore_ascii_case(new)) {
        return Err(format!(
            "`{new}` is a built-in type, so a type cannot be renamed to it."
        ));
    }
    if let Some(clash) = index.declaration(&format!("{owner}::{new}")) {
        return Err(format!(
            "`{owner}` already has {} named `{new}`.",
            clash.kind.describe()
        ));
    }
    let mut texts = Texts::new(documents);
    if let Some(declared) = index.declaration(owner) {
        let path = &declared.site.path;
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

    let mut rewrites = Vec::new();
    for site in sites {
        let written = texts
            .text(site)
            .ok_or_else(|| format!("`{}` could not be read.", plain(&site.path).display()))?;
        let text = if written == old {
            new.to_owned()
        } else if let Some(prefix) = written.strip_suffix(old).filter(|it| it.ends_with("::")) {
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
    Ok(rewrites)
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

/// The local subject for the variable `$name` written at `token`, with the
/// refusals a rename makes on top of [`crate::locals::of`]'s sites. `at` is
/// the site under the cursor: `token` itself, or the `@param` name of the
/// parameter `token` declares.
fn local(
    analysed: &Analysed,
    file: &SourceFile,
    tokens: &[Token],
    entry: &Path,
    token: Span,
    at: Span,
    name: &str,
) -> Result<Subject, String> {
    let Some(found) = locals::of(analysed, file, tokens, token, name) else {
        return Err(format!(
            "`${name}` is not declared here, so it cannot be renamed."
        ));
    };
    let calls = if found.is_parameter() {
        let Some((method, param, _)) = found.parameter else {
            return Err(format!(
                "`${name}` is a parameter of an anonymous function, and a call can pass it by \
                 name. It cannot be renamed."
            ));
        };
        if !param.modifiers.is_empty() {
            return Err(format!(
                "`${name}` is also a property of the class, so it cannot be renamed here."
            ));
        }
        if text_of(file, method.name) == CONSTRUCTOR {
            return Err(format!(
                "`${name}` is a parameter of a constructor, and `new` can pass it by name. It \
                 cannot be renamed."
            ));
        }
        Some(symbol_at(analysed, method.name.start).ok_or(NOTHING)?)
    } else {
        None
    };
    if found.nested.contains(name) {
        return Err(format!(
            "An anonymous function in this code also declares `${name}`, so `${name}` cannot be \
             renamed here."
        ));
    }
    if found.global {
        return Err(format!(
            "`${name}` is a global variable here, so it cannot be renamed."
        ));
    }
    if !found.sites.contains(&at) {
        return Err(NOTHING.to_owned());
    }
    Ok(Subject {
        at: Site {
            path: entry.to_path_buf(),
            start: at.start,
            end: at.end,
        },
        what: What::Local {
            path: entry.to_path_buf(),
            name: name.to_owned(),
            bindings: found.bindings.to_vec(),
            nested: found.nested,
            sites: found.sites,
            calls,
        },
    })
}

/// The edit to the `name:` of each argument named `old` at every call the
/// index holds for `method`, read off each call's own arguments in a parse of
/// its file.
///
/// # Errors
///
/// A call the index holds and the parse does not find refuses the rename,
/// because its argument names are a site the edits would miss.
fn named_arguments(
    documents: &Documents,
    index: &SymbolIndex,
    method: &str,
    old: &str,
    new: &str,
) -> Result<Vec<Rewrite>, String> {
    let mut texts = Texts::new(documents);
    let mut rewrites = Vec::new();
    for occurrence in index.occurrences(method) {
        let site = &occurrence.site;
        let unread = || {
            format!(
                "A call to `{method}` in `{}` cannot be read, so its parameters cannot be renamed.",
                plain(&site.path).display()
            )
        };
        let file = texts.file(&site.path).ok_or_else(unread)?;
        let stmts = parse_file(file, &mut Diagnostics::new());
        let Some(args) = call_in_stmts(&stmts, site.start, site.end) else {
            return Err(unread());
        };
        let CallArgs::List(args) = args else {
            continue;
        };
        for name in args.iter().filter_map(|arg| arg.name) {
            if text_of(file, name) == old {
                rewrites.push(Rewrite {
                    site: Site {
                        path: site.path.clone(),
                        start: name.start,
                        end: name.end,
                    },
                    text: new.to_owned(),
                });
            }
        }
    }
    Ok(rewrites)
}

/// The arguments of the method call in `stmts` whose method name is written
/// from `start` to `end`.
fn call_in_stmts(stmts: &[Stmt], start: BytePos, end: BytePos) -> Option<&CallArgs> {
    stmts
        .iter()
        .filter(|stmt| stmt.span.start <= start && end <= stmt.span.end)
        .find_map(|stmt| call_in_stmt(stmt, start, end))
}

/// [`call_in_stmts`] over one statement: a type's method bodies, a
/// namespace's statements, and what any other statement runs.
fn call_in_stmt(stmt: &Stmt, start: BytePos, end: BytePos) -> Option<&CallArgs> {
    match &stmt.kind {
        StmtKind::NamespaceDecl(decl) => call_in_stmts(&decl.body.as_ref()?.stmts, start, end),
        StmtKind::ClassDecl(decl) => call_in_members(&decl.members, start, end),
        StmtKind::InterfaceDecl(decl) => call_in_members(&decl.members, start, end),
        StmtKind::EnumDecl(decl) => call_in_members(&decl.members, start, end),
        _ => {
            let mut found = None;
            each_child_of_stmt(stmt, &mut |child| {
                if found.is_none() {
                    found = match child {
                        StmtChild::Stmt(inner) => {
                            call_in_stmts(std::slice::from_ref(inner), start, end)
                        }
                        StmtChild::Expr(expr) => call_in_expr(expr, start, end),
                    };
                }
            });
            found
        }
    }
}

/// [`call_in_stmts`] over the method bodies among `members`.
fn call_in_members(members: &[ClassMember], start: BytePos, end: BytePos) -> Option<&CallArgs> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) => call_in_stmts(&method.body.as_ref()?.stmts, start, end),
        _ => None,
    })
}

/// [`call_in_stmts`] over one expression, an anonymous function's body
/// included.
fn call_in_expr(expr: &Expr, start: BytePos, end: BytePos) -> Option<&CallArgs> {
    if !(expr.span.start <= start && end <= expr.span.end) {
        return None;
    }
    match &expr.kind {
        ExprKind::MethodCall {
            method: MemberName::Ident(name),
            args,
            ..
        }
        | ExprKind::StaticCall {
            method: MemberName::Ident(name),
            args,
            ..
        } if name.start == start && name.end == end => return Some(args),
        ExprKind::Fn(function) => {
            return match &function.body {
                FnBody::Expr(body) => call_in_expr(body, start, end),
                FnBody::Block(body) => call_in_stmts(&body.stmts, start, end),
            };
        }
        _ => {}
    }
    let mut found = None;
    each_child_expr(expr, &mut |child| {
        if found.is_none() {
            found = call_in_expr(child, start, end);
        }
    });
    found
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

/// The key a path that may not exist yet is compared under: its nearest
/// existing ancestor as [`canonical_key`] spells it, and the parts below it
/// ([`nvs_hir::requires::planned_key`]).
pub(crate) fn new_key(path: &Path) -> PathBuf {
    nvs_hir::requires::planned_key(path).unwrap_or_else(|| path.to_path_buf())
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
    rewritten_path(inner, from, to).map(|path| format!("{quote}{path}{quote}"))
}

/// `written`, a path as a `require` literal's text or a doc comment tag
/// writes it in the file that will be at `from`, written again to name the
/// file that will be at `to`, or `None` when it stays as it is.
fn rewritten_path(written: &str, from: &Path, to: &Path) -> Option<String> {
    let target = plain(to);
    let path = if Path::new(written).is_absolute() || written.starts_with('/') {
        target.to_str()?.to_owned()
    } else {
        let relative = relative(&plain(from.parent()?), &target)?;
        if written.starts_with("./") && !relative.starts_with("../") {
            format!("./{relative}")
        } else {
            relative
        }
    };
    (path != written).then_some(path)
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
