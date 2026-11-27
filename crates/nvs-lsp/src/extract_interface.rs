//! *Extract interface*: a new interface with the public methods of a class,
//! which the class then implements
//! (`rule:tooling/nvs-code-answers-about-the-project`).
//!
//! [`extracted`] takes the class, the interface's name and, optionally, the
//! methods to take. A bare name is in the class's namespace. Without a list,
//! it takes every public method that is not static, except the constructor.
//!
//! # Decision: what it writes
//!
//! - **A new file**, at the path the `autoload` map gives the interface's
//!   name, the first root of its longest prefix, as a type moved to another
//!   namespace finds it (`rule:ide/a-type-moves-with-its-namespace`). An
//!   autoloaded file holds one declaration
//!   (`rule:programs/one-declaration-per-autoloaded-file`), so a class that no
//!   map loads, a name no root reaches and a path where a file already is are
//!   refused.
//! - **Each method's signature** as its declaration writes it, with its `///`
//!   run and without its attributes or body, written as *Implement the
//!   missing methods* writes one (`crate::generate::signature_typed`). Each
//!   type is written for the new file: its short name where the interface's
//!   namespace or a `use` line of the new file reaches it, a new `use` line
//!   where the short name is free, and its qualified name otherwise. It
//!   never writes an alias (`rule:ide/no-refactoring-introduces-an-alias`).
//! - **The class's `implements` list** gains the interface, with a `use`
//!   line where the class's file needs one.
//!
//! No parameter type anywhere changes to the interface. Which uses of the
//! class should take the interface instead is the person's choice.
//!
//! The edit passes `crate::refactor::checked_creating` with the new file, so
//! a method whose signature does not compile in the new file, such as one
//! that writes a type parameter of a generic class, refuses the whole edit.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. One analysis of the
//! class's file, one of each program that reads it, to find the new path,
//! then the check. Nothing is kept.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{SourceFile, SourceMap, Span};
use nvs_hir::QName;
use nvs_stdlib::registry;
use nvs_syntax::ast::{ClassDecl, ClassMember, ClassMemberKind, MethodMember, Modifier, StmtKind};

use crate::definition::{declared_type, resolved_name_in, text_of};
use crate::document::{Documents, analyse_file};
use crate::index::{DeclKind, SymbolIndex};
use crate::move_member::{Files, spell};
use crate::rename::{Rewrite, new_key, sorted_heads};
use crate::{Site, refactor};

/// One level of indentation.
const INDENT: &str = "    ";

/// What extract interface writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extracted {
    /// The edits to existing files: the class's `implements` list and its
    /// `use` lines.
    pub rewrites: Vec<Rewrite>,
    /// The new file's path.
    pub path: PathBuf,
    /// The new file's text.
    pub text: String,
}

/// The interface `name` extracted from the class `class`, with `methods`, or
/// every public method that is not static or the constructor.
///
/// # Errors
///
/// Returns why nothing is written, as the module doc lists it, or the first
/// error the edit adds.
pub fn extracted(
    documents: &Documents,
    index: &SymbolIndex,
    class: &str,
    name: &str,
    methods: Option<&[String]>,
) -> Result<Extracted, String> {
    let declared = index
        .declaration(class)
        .ok_or_else(|| format!("`{class}` is not declared in this project."))?;
    if declared.kind != DeclKind::Class {
        return Err(format!("`{class}` is not a class."));
    }
    let class = declared.symbol.clone();
    let path = declared.site.path.clone();
    let full = full_name(&class, name)?;
    if index.declaration(&full).is_some() {
        return Err(format!(
            "`{full}` already exists, so nothing was changed. Choose another name."
        ));
    }
    let to = new_path(documents, index, &path, &class, &full)?;
    if to.exists() {
        return Err(format!(
            "`{}` already exists, so nothing was changed.",
            to.display()
        ));
    }

    let analysed = analyse_file(documents, &path, 0)
        .ok_or_else(|| format!("The file of `{class}` cannot be read."))?;
    let (stmt, file) = declared_type(&analysed, &QName::parse(&class))
        .ok_or_else(|| format!("The declaration of `{class}` cannot be read."))?;
    let StmtKind::ClassDecl(decl) = &stmt.kind else {
        return Err(format!("`{class}` is not a class."));
    };
    let chosen = chosen(file, decl, methods, &class)?;

    let space = namespace_of(&full).to_owned();
    let mut names = Names {
        index,
        space: space.clone(),
        own: short_of(&full).to_owned(),
        uses: BTreeMap::new(),
    };
    let mut written = Vec::with_capacity(chosen.len());
    for (member, method) in chosen {
        let typed = |span: Span| names_typed(&analysed, file, span);
        let signature = crate::generate::signature_typed(file, method, &typed);
        let signature = names.respelled(&signature);
        let mut block = String::new();
        if let Some(doc) = &member.doc {
            for line in &doc.lines {
                block.push_str(INDENT);
                block.push_str(text_of(file, *line).trim());
                block.push('\n');
            }
        }
        block.push_str(&format!("{INDENT}{signature};\n"));
        written.push(block);
    }
    let text = file_text(&space, &names, short_of(&full), &written);

    let mut files = Files::new(documents);
    let mut heads = Vec::new();
    let named = spell(&mut files, index, &path, decl.span.start, &full, &mut heads)?;
    let (at, added) = match (decl.implements.last(), &decl.extends) {
        (Some(last), _) => (last.span.end, format!(", {named}")),
        (None, Some(parent)) => (parent.span.end, format!(" implements {named}")),
        (None, None) => (decl.name.span.end, format!(" implements {named}")),
    };
    let mut rewrites = vec![Rewrite {
        site: Site {
            path: path.clone(),
            start: at,
            end: at,
        },
        text: added,
    }];
    rewrites.extend(sorted_heads(documents, heads)?);
    let created = [(to.clone(), text.clone())];
    let checked = refactor::checked_creating(documents, index, rewrites, &[], &created)
        .map_err(|broken| broken.refusal())?;
    Ok(Extracted {
        rewrites: checked.rewrites,
        path: to,
        text,
    })
}

/// The interface's full name: `name` without its leading `\` when it is
/// qualified, else `name` in the namespace of `class`.
fn full_name(class: &str, name: &str) -> Result<String, String> {
    let name = name.trim();
    let bare = name.trim_start_matches('\\');
    let valid = !bare.is_empty()
        && bare.split('\\').all(|segment| {
            let mut chars = segment.chars();
            chars
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        });
    if !valid {
        return Err(format!("`{name}` is not a name for an interface."));
    }
    if name.contains('\\') {
        return Ok(bare.to_owned());
    }
    let space = namespace_of(class);
    Ok(if space.is_empty() {
        bare.to_owned()
    } else {
        format!("{space}\\{bare}")
    })
}

/// The path the `autoload` map of every program that loads `path` gives
/// `full`, the first root of its longest prefix.
fn new_path(
    documents: &Documents,
    index: &SymbolIndex,
    path: &Path,
    class: &str,
    full: &str,
) -> Result<PathBuf, String> {
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
                "No `autoload` folder of `{}` contains `{full}`, so the interface has no \
                 file to go to.",
                entry.display()
            ));
        };
        found.entry(new_key(to)).or_default().push(entry);
    }
    if found.len() > 1 {
        let programs: Vec<String> = found
            .values()
            .flatten()
            .map(|entry| format!("`{}`", entry.display()))
            .collect();
        return Err(format!(
            "The programs that load `{class}` would look for `{full}` at different paths: {}.",
            programs.join(", ")
        ));
    }
    found.into_keys().next().ok_or_else(|| {
        format!(
            "No program loads `{class}` through an `autoload` folder, so the interface has no \
             file to go to."
        )
    })
}

/// The methods to write, in the order the class declares them.
fn chosen<'a>(
    file: &SourceFile,
    decl: &'a ClassDecl,
    methods: Option<&[String]>,
    class: &str,
) -> Result<Vec<(&'a ClassMember, &'a MethodMember)>, String> {
    let offered: Vec<(&ClassMember, &MethodMember)> = decl
        .members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Method(method) => Some((member, method)),
            _ => None,
        })
        .filter(|(_, method)| {
            !method.modifiers.iter().any(|modifier| {
                matches!(
                    modifier,
                    Modifier::Static | Modifier::Private | Modifier::Protected
                )
            }) && !text_of(file, method.name).eq_ignore_ascii_case("constructor")
        })
        .collect();
    let Some(wanted) = methods else {
        if offered.is_empty() {
            return Err(format!(
                "`{class}` has no public method that is not static, so there is nothing to \
                 extract."
            ));
        }
        return Ok(offered);
    };
    if wanted.is_empty() {
        return Err("Name at least one method.".to_owned());
    }
    for name in wanted {
        if !offered
            .iter()
            .any(|(_, method)| text_of(file, method.name).eq_ignore_ascii_case(name))
        {
            return Err(format!(
                "`{class}` has no public method `{name}` that is not static, so nothing was \
                 changed."
            ));
        }
    }
    Ok(offered
        .into_iter()
        .filter(|(_, method)| {
            wanted
                .iter()
                .any(|name| text_of(file, method.name).eq_ignore_ascii_case(name))
        })
        .collect())
}

/// The type at `span` with each name in it that resolves to a declared type
/// written fully qualified with a leading `\`. [`Names::respelled`] then
/// writes each one for the new file.
fn names_typed(analysed: &crate::document::Analysed, file: &SourceFile, span: Span) -> String {
    let text = text_of(file, span);
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if word.is_empty() {
            return;
        }
        match resolved_name_in(analysed, file.id(), word, span.start) {
            Some(symbol) if is_type(analysed, &symbol) => {
                out.push('\\');
                out.push_str(&symbol.to_string());
            }
            _ => out.push_str(word),
        }
        word.clear();
    };
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '\\' {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Whether `symbol` names a type the analysis declares or the `Core`
/// registry has.
fn is_type(analysed: &crate::document::Analysed, symbol: &QName) -> bool {
    analysed.module.symbols.get(symbol).is_some()
        || registry::type_names()
            .iter()
            .any(|core| QName::parse(core) == *symbol)
}

/// How the new file writes each type, and the `use` lines that needs.
struct Names<'a> {
    index: &'a SymbolIndex,
    /// The interface's namespace, `""` for the global one.
    space: String,
    /// The interface's own short name, which no `use` line may take.
    own: String,
    /// Each `use` line, by its short name in lower case.
    uses: BTreeMap<String, String>,
}

impl Names<'_> {
    /// `signature` with each `\Full\Name` outside a string literal, as
    /// [`names_typed`] writes a type, written the way the new file names it.
    fn respelled(&mut self, signature: &str) -> String {
        let mut out = String::with_capacity(signature.len());
        let mut quote: Option<char> = None;
        let mut chars = signature.char_indices().peekable();
        while let Some((at, ch)) = chars.next() {
            match quote {
                Some(open) => {
                    out.push(ch);
                    if ch == '\\' {
                        if let Some((_, escaped)) = chars.next() {
                            out.push(escaped);
                        }
                    } else if ch == open {
                        quote = None;
                    }
                }
                None if ch == '\'' || ch == '"' => {
                    quote = Some(ch);
                    out.push(ch);
                }
                None if ch == '\\' => {
                    let after = &signature[at + 1..];
                    let len = after
                        .find(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '\\'))
                        .unwrap_or(after.len());
                    out.push_str(&self.name(&after[..len]));
                    while chars.peek().is_some_and(|(next, _)| *next <= at + len) {
                        chars.next();
                    }
                }
                None => out.push(ch),
            }
        }
        out
    }

    /// How the new file writes the type `full`.
    fn name(&mut self, full: &str) -> String {
        let short = short_of(full);
        if namespace_of(full).eq_ignore_ascii_case(&self.space) {
            return short.to_owned();
        }
        let key = short.to_ascii_lowercase();
        if let Some(used) = self.uses.get(&key) {
            return if used.eq_ignore_ascii_case(full) {
                short.to_owned()
            } else {
                format!("\\{full}")
            };
        }
        let local = if self.space.is_empty() {
            short.to_owned()
        } else {
            format!("{}\\{short}", self.space)
        };
        if !full.contains('\\')
            || short.eq_ignore_ascii_case(&self.own)
            || self.index.declaration(&local).is_some()
        {
            return format!("\\{full}");
        }
        self.uses.insert(key, full.to_owned());
        short.to_owned()
    }
}

/// The new file: its `namespace` line, its sorted `use` lines and the
/// interface with each of `methods`.
fn file_text(space: &str, names: &Names<'_>, short: &str, methods: &[String]) -> String {
    let mut text = String::from("<?nvs\n");
    if !space.is_empty() {
        text.push_str(&format!("namespace {space};\n\n"));
    }
    if !names.uses.is_empty() {
        for full in names.uses.values() {
            text.push_str(&format!("use {full};\n"));
        }
        text.push('\n');
    }
    text.push_str(&format!("interface {short}\n{{\n"));
    text.push_str(&methods.join("\n"));
    text.push_str("}\n");
    let mut map = SourceMap::new();
    let id = map.add(format!("{short}.nvs"), text.clone());
    match nvs_fmt::sorted_imports(map.file(id)) {
        Ok(Some(sorted)) => sorted,
        _ => text,
    }
}

/// The namespace of `symbol`, a fully-qualified name, and `""` for the
/// global namespace.
fn namespace_of(symbol: &str) -> &str {
    symbol.rsplit_once('\\').map_or("", |(space, _)| space)
}

/// The last segment of a qualified name.
fn short_of(full: &str) -> &str {
    full.rsplit_once('\\').map_or(full, |(_, short)| short)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_is_in_the_class_namespace() {
        assert_eq!(
            full_name(r"Shop\Cart", "Priced").as_deref(),
            Ok(r"Shop\Priced")
        );
        assert_eq!(
            full_name(r"Shop\Cart", r"\Billing\Priced").as_deref(),
            Ok(r"Billing\Priced")
        );
        assert_eq!(full_name("Cart", "Priced").as_deref(), Ok("Priced"));
        assert!(full_name(r"Shop\Cart", "a-b").is_err());
        assert!(full_name(r"Shop\Cart", r"Shop\\Priced").is_err());
    }
}
