//! The completion files: every `.json` file under a `.novis/completion/` folder, loaded into one table
//! keyed by the parameter each value is offered at.
//!
//! `rule:ide/completion-files-offer-values-at-named-parameters` is the rule, ADR 0242 and ADR 0243
//! the reasoning. This module finds the files, reads them, keeps them current and says what is wrong
//! with them. It offers nothing itself: the completion, hover, definition and diagnostic arms read
//! [`CompletionFiles`] and never a file or a directory, which is how
//! `rule:ide/completion-offers-only-what-the-compiler-derived` can name this table as the one source
//! of values the compiler does not derive.
//!
//! # The walk
//!
//! [`CompletionFiles::load`] walks each workspace root once. It enters `vendor/`, so a library can
//! ship values for its own parameters, and skips `target`, `node_modules` and every dot-folder except
//! `.novis`. Under a `.novis/completion/` folder it takes every `.json` file in every subfolder. It
//! lists directories and reads only those files. It is not `crate::index`'s walk, which skips
//! `vendor/` and every dot-folder, and the two share no skip list.
//!
//! [`CompletionFiles::refresh`] is the watcher's half: one path, read again only when its
//! modification time or size changed, added when it is new and dropped when it is gone. A path the
//! walk would not have found, because a folder above its `.novis` is one the walk skips or because
//! no workspace root holds it, is ignored, so a watcher event and the walk agree on what is a
//! completion file.
//!
//! # The format
//!
//! Every shape is parsed with `deny_unknown_fields`, so an unknown field is a finding that names it
//! and the fields allowed there. A file with any finding of that kind contributes nothing. The
//! fields are ADR 0242 § 3's and ADR 0243's: `location` and `replacement` on a value, `when` and
//! `strict` on an attachment, and a list written as `{ "separator", "values" }`.
//!
//! Each piece is parsed from its own slice of the file (`serde_json::value::RawValue`), so a
//! finding about one attachment or one value points at that attachment or value and not at the top
//! of the file.
//!
//! # The table
//!
//! Sets merge by name and values by `value`, the first one read kept, files in path order and
//! entries in written order. An attachment's values are its set's merged values followed by its own
//! `values`, with the same rule. The table keeps every attachment for a parameter, each with its
//! `when` and `strict`, because which ones apply is decided at the call.
//!
//! Questions the records leave open, decided here:
//!
//! - **A separator belongs to the value.** A list's separator is copied onto each value it lists, so
//!   a set merged from two files keeps each value's own separator, and a list without one is offered
//!   whole.
//! - **`method` must be written `Class::method`.** One without `::` is a field of the wrong form, so
//!   the file contributes nothing. A leading `\` on the class is dropped, since the checker's names
//!   carry none.
//! - **An attachment gives `set`, `values` or both.** One that gives neither is a field of the wrong
//!   form.
//! - **A `location` whose file is missing is dropped**, so definition has no answer for that value.
//!   The value keeps every other field.
//! - **A symbolic link to a folder is not followed**, so a link cannot make the walk loop.
//! - **A relative link in `documentation` becomes a `file:` URI** under the file's folder
//!   ([`Value::markdown`]), because a client resolves a relative target against nothing it knows.
//!   A target with a scheme, an absolute path or a fragment is kept as written. VS Code's hover
//!   shows an image linked this way: it was checked by hand with the release server, a value whose
//!   `documentation` links an image relative to its completion file, and an installed VS Code.
//!
//! # The checks against the index
//!
//! [`CompletionFiles::check`] compares each attachment with what `crate::index::SymbolIndex`
//! declares, and the server runs it after the load, after a refresh and after every change to the
//! index. A class the index does not declare is a hint and nothing more is checked for it. A method
//! is looked up on the class the attachment names, so an inherited method is a finding, as the rule
//! says. Two more questions are decided here:
//!
//! - **An attachment with a warning is taken out of the table** by the check, and comes back at
//!   the next check that finds nothing wrong with it, since each check merges the files again
//!   first. That is what makes one whose `when` names a missing parameter contribute nothing,
//!   whatever a call's other arguments are. An attachment to an undeclared class stays, because
//!   its class may come from `vendor/`.
//! - **A parameter takes a string unless its declared type says otherwise.** The type is read from
//!   its text: one whose members are all of a kind that is not a string, such as `int`, `bool` or
//!   an array, is a finding. A named type is not, because it may be an alias of a string type, and
//!   nor is a parameter with no type.
//!
//! Memory: every loaded value is held for the life of the session, with its folder shared by every
//! value of one file, and every file's text, for the server to place its findings.

use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroU32;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use lsp_types::CompletionItemKind;
use nvs_diagnostics::{Code, code};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::value::RawValue;

use crate::index::{DeclKind, Declaration, SymbolIndex};

/// The folder a project keeps its tool data in.
const NOVIS: &str = ".novis";

/// The kind of tool data this module reads, as `.novis`'s subfolder.
const COMPLETION: &str = "completion";

/// The glob the client watches, which is ADR 0242 § 2's.
pub const WATCHED: &str = "**/.novis/completion/**/*.json";

/// Every completion file the workspace holds, and the table they load into.
#[derive(Debug, Default)]
pub struct CompletionFiles {
    /// The workspace roots, which decide what is a completion file and what is under `vendor/`.
    roots: Vec<PathBuf>,
    /// Every file loaded, in path order, which is the order they merge in.
    files: BTreeMap<PathBuf, File>,
    /// Each set's merged values.
    sets: FxHashMap<String, Vec<Arc<Value>>>,
    /// The attachments, by declaring class, then method, then parameter.
    table: FxHashMap<String, FxHashMap<String, FxHashMap<String, Vec<Attachment>>>>,
    /// What the merge found wrong with an attachment, by the file that wrote it.
    merged_findings: BTreeMap<PathBuf, Vec<Finding>>,
    /// What the last [`CompletionFiles::check`] found wrong with an attachment, by the file that
    /// wrote it.
    checked_findings: BTreeMap<PathBuf, Vec<Finding>>,
}

/// One loaded file.
#[derive(Debug)]
struct File {
    /// The modification time and size it was read at.
    stamp: Stamp,
    /// Whether a folder above it is named `vendor`, which keeps its findings off the screen.
    vendored: bool,
    /// Its text, which a finding's span is a range of.
    text: String,
    /// What it says, or nothing when it has a finding of the wrong-form kind.
    read: Read,
}

/// What a file is compared by when the watcher reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

/// What one file contributes, and what is wrong with it.
#[derive(Debug, Default)]
struct Read {
    /// Its sets, in written order.
    sets: Vec<NamedSet>,
    /// Its attachments, in written order.
    attachments: Vec<Written>,
    /// Its own findings, which need no other file to decide.
    findings: Vec<Finding>,
}

/// A set's name and the values one file lists for it.
type NamedSet = (String, Vec<Arc<Value>>);

/// One attachment as its file wrote it, before the sets are merged into it.
#[derive(Debug)]
struct Written {
    class: String,
    method: String,
    parameter: String,
    set: Option<String>,
    values: Vec<Arc<Value>>,
    when: Option<When>,
    strict: bool,
    span: Span,
}

/// A byte range in a completion file's text.
pub type Span = (usize, usize);

/// What happened to a path [`CompletionFiles::refresh`] was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reload {
    /// The path is not a completion file the walk would have found.
    Ignored,
    /// Its modification time and size are the ones it was read at, or it is gone and was never read.
    Unchanged,
    /// It was read, because it is new or because it changed.
    Read,
    /// It is gone, and what it contributed is gone with it.
    Dropped,
}

/// One value offered at a parameter.
///
/// Every field but the last three is one field of an LSP completion item, as ADR 0242 § 3 maps it.
#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    /// The text the item inserts.
    pub value: String,
    /// The item's label, which is [`Self::value`] when absent.
    pub label: Option<String>,
    /// `labelDetails.detail`.
    pub label_detail: Option<String>,
    /// `labelDetails.description`.
    pub label_description: Option<String>,
    /// The item's icon, `value` when absent.
    pub kind: CompletionItemKind,
    /// The popup's heading.
    pub title: Option<String>,
    /// The popup's body, untrusted Markdown.
    pub documentation: Option<String>,
    /// Whether the item carries the `Deprecated` tag.
    pub deprecated: bool,
    /// `sortText`.
    pub sort_text: Option<String>,
    /// `filterText`.
    pub filter_text: Option<String>,
    /// `preselect`.
    pub preselect: bool,
    /// The file and 1-based line that define the value, the file resolved against [`Self::folder`].
    pub location: Option<(PathBuf, NonZeroU32)>,
    /// The value that replaces this one, kept only on a deprecated value.
    pub replacement: Option<String>,
    /// The separator of the list the value was written in, which completion splits it on.
    pub separator: Option<char>,
    /// The folder of the completion file that declares the value, which a relative link in
    /// [`Self::documentation`] resolves against.
    pub folder: Arc<Path>,
}

impl Value {
    /// A value no completion file declares: one member of a parameter's string literal union,
    /// offered as `value` with its text as its label and no other field.
    #[must_use]
    pub fn member(value: String) -> Self {
        Self {
            value,
            label: None,
            label_detail: None,
            label_description: None,
            kind: CompletionItemKind::VALUE,
            title: None,
            documentation: None,
            deprecated: false,
            sort_text: None,
            filter_text: None,
            preselect: false,
            location: None,
            replacement: None,
            separator: None,
            folder: Arc::from(Path::new("")),
        }
    }

    /// [`Self::documentation`], with each relative link and image target rewritten as a `file:`
    /// URI under [`Self::folder`]. A target with a scheme, an absolute path or a `#` fragment is
    /// left as written.
    #[must_use]
    pub fn markdown(&self) -> Option<String> {
        let text = self.documentation.as_deref()?;
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(at) = rest.find("](") {
            let (before, after) = rest.split_at(at + 2);
            out.push_str(before);
            let end = after.find([')', ' ']).unwrap_or(after.len());
            let (target, tail) = after.split_at(end);
            let relative = !(target.is_empty()
                || target.starts_with(['/', '\\', '#', '<'])
                || target.contains(':'));
            match relative
                .then(|| crate::document::uri_of(&self.folder.join(target)))
                .flatten()
            {
                Some(uri) => out.push_str(uri.as_str()),
                None => out.push_str(target),
            }
            rest = tail;
        }
        out.push_str(rest);
        Some(out)
    }
}

/// An attachment's condition on another argument of the same call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct When {
    /// The other parameter, without its `$`.
    pub parameter: String,
    /// The strings its argument must equal one of.
    pub equals: Vec<String>,
}

/// One attachment of values to a parameter, after the sets are merged into it.
#[derive(Clone, Debug)]
pub struct Attachment {
    /// Its set's values and then its own, each `value` once.
    pub values: Vec<Arc<Value>>,
    /// When it applies only for a given value of another argument.
    pub when: Option<When>,
    /// Whether it asks for a literal at its parameter to be checked.
    pub strict: bool,
    /// The file that wrote it.
    pub file: PathBuf,
    /// Where in that file.
    pub span: Span,
}

/// What a finding is about, one diagnostic code each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingKind {
    /// The file is not JSON, or a field is unknown or of the wrong form. The file contributes nothing.
    Invalid,
    /// An attachment names a set no file defines.
    MissingSet,
    /// A value's `location` names a file that does not exist.
    MissingLocation,
    /// A value that is not deprecated names a `replacement`.
    ReplacementNotDeprecated,
    /// An attachment names a method the class does not declare, or a parameter the method does not
    /// have, in `parameter` or in `when`.
    MissingMember,
    /// An attachment names a parameter that takes no string.
    NotAString,
    /// An attachment names a class the workspace index does not declare. A hint, because the class
    /// may come from `vendor/`.
    UndeclaredClass,
}

impl FindingKind {
    /// The diagnostic code the server publishes this kind with.
    #[must_use]
    pub const fn code(self) -> Code {
        match self {
            Self::Invalid => code::W_COMPLETION_FILE_INVALID,
            Self::MissingSet => code::W_COMPLETION_SET_MISSING,
            Self::MissingLocation => code::W_COMPLETION_LOCATION_MISSING,
            Self::ReplacementNotDeprecated => code::W_COMPLETION_REPLACEMENT_NOT_DEPRECATED,
            Self::MissingMember => code::W_COMPLETION_MEMBER_MISSING,
            Self::NotAString => code::W_COMPLETION_PARAMETER_NOT_A_STRING,
            Self::UndeclaredClass => code::W_COMPLETION_CLASS_UNDECLARED,
        }
    }

    /// Whether the server publishes this kind as a hint. Every other kind is a warning.
    #[must_use]
    pub const fn is_hint(self) -> bool {
        matches!(self, Self::UndeclaredClass)
    }
}

/// One thing wrong with a completion file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// What is wrong.
    pub kind: FindingKind,
    /// Where, in the file's text.
    pub span: Span,
    /// The sentence the editor shows.
    pub message: String,
}

impl CompletionFiles {
    /// Walks `roots` and reads every completion file under them.
    pub fn load(roots: &[PathBuf]) -> Self {
        let mut found = Vec::new();
        for root in roots {
            walk(root, &mut found);
        }
        let mut loaded = Self {
            roots: roots.to_vec(),
            ..Self::default()
        };
        for path in found {
            if let Some(vendored) = loaded.place(&path)
                && let Some(stamp) = stamp_of(&path)
            {
                let (text, read) = read(&path);
                loaded.files.insert(
                    path,
                    File {
                        stamp,
                        vendored,
                        text,
                        read,
                    },
                );
            }
        }
        loaded.merge();
        loaded
    }

    /// Reads `path` again if it changed since it was read, adds it if it is new and drops it if it
    /// is gone.
    pub fn refresh(&mut self, path: &Path) -> Reload {
        let Some(vendored) = self.place(path) else {
            return Reload::Ignored;
        };
        let now = stamp_of(path);
        let held = self.files.get(path).map(|file| file.stamp);
        let reload = match (now, held) {
            (None, None) => return Reload::Unchanged,
            (Some(now), Some(held)) if now == held => return Reload::Unchanged,
            (None, Some(_)) => {
                self.files.remove(path);
                Reload::Dropped
            }
            (Some(stamp), _) => {
                let (text, read) = read(path);
                self.files.insert(
                    path.to_owned(),
                    File {
                        stamp,
                        vendored,
                        text,
                        read,
                    },
                );
                Reload::Read
            }
        };
        self.merge();
        reload
    }

    /// Every completion file loaded, in path order.
    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.files.keys().map(PathBuf::as_path)
    }

    /// The attachments for `parameter` of `class::method`, `class` being the class that declares the
    /// method, in the order they merge in.
    pub fn attachments(&self, class: &str, method: &str, parameter: &str) -> &[Attachment] {
        self.table
            .get(class)
            .and_then(|methods| methods.get(method))
            .and_then(|parameters| parameters.get(parameter))
            .map_or(&[], Vec::as_slice)
    }

    /// The values offered at `parameter` of `class::method` in a call whose other arguments are
    /// `others`, each one's parameter name with its text when it is one string literal.
    ///
    /// An attachment without `when` applies at every call. One with `when` applies only when
    /// `others` gives its parameter a literal equal to one of its strings, so a left-out argument
    /// or one that is not a literal applies none. The values follow the attachments' order, and a
    /// repeated `value` keeps the first one.
    pub fn values_at(
        &self,
        class: &str,
        method: &str,
        parameter: &str,
        others: &[(String, Option<String>)],
    ) -> Vec<Arc<Value>> {
        let mut values = Vec::new();
        for attachment in self.attachments(class, method, parameter) {
            let applies = attachment.when.as_ref().is_none_or(|when| {
                others.iter().any(|(name, text)| {
                    *name == when.parameter
                        && text.as_ref().is_some_and(|text| when.equals.contains(text))
                })
            });
            if applies {
                extend_once(&mut values, &attachment.values);
            }
        }
        values
    }

    /// The merged values of the set `name`.
    pub fn set(&self, name: &str) -> Option<&[Arc<Value>]> {
        self.sets.get(name).map(Vec::as_slice)
    }

    /// The text of the loaded file at `path`, which its findings' spans are ranges of.
    pub fn text(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(|file| file.text.as_str())
    }

    /// Every finding to show, by file, in path order, a file with none included. A file under
    /// `vendor/` is a library's to fix, so it is not among them.
    pub fn findings(&self) -> impl Iterator<Item = (&Path, Vec<&Finding>)> {
        self.files
            .iter()
            .filter(|(_, file)| !file.vendored)
            .map(|(path, file)| {
                let merged = self.merged_findings.get(path).into_iter().flatten();
                let checked = self.checked_findings.get(path).into_iter().flatten();
                (
                    path.as_path(),
                    file.read
                        .findings
                        .iter()
                        .chain(merged)
                        .chain(checked)
                        .collect(),
                )
            })
    }

    /// Checks every attachment outside `vendor/` against the classes, methods and parameters
    /// `index` declares, and returns whether what it found differs from the last check.
    ///
    /// The findings need the index, so they are made here and not when a file is read, and the
    /// server calls this again whenever the index changes. With no attachment to check it reads
    /// nothing; otherwise it reads each of the index's declarations once.
    pub fn check(&mut self, index: &SymbolIndex) -> bool {
        let written = || {
            self.files
                .iter()
                .filter(|(_, file)| !file.vendored)
                .flat_map(|(path, file)| file.read.attachments.iter().map(move |at| (path, at)))
        };
        let mut wanted: FxHashSet<String> = FxHashSet::default();
        for (_, at) in written() {
            wanted.insert(at.class.clone());
            wanted.insert(format!("{}::{}", at.class, at.method));
        }
        let mut declared: FxHashMap<&str, &Declaration> = FxHashMap::default();
        if !wanted.is_empty() {
            for path in index.files() {
                for declaration in index.declarations_in(path) {
                    if wanted.contains(&declaration.symbol) {
                        declared.entry(&declaration.symbol).or_insert(declaration);
                    }
                }
            }
        }
        let mut checked: BTreeMap<PathBuf, Vec<Finding>> = BTreeMap::new();
        for (path, at) in written() {
            for (kind, message) in check_attachment(at, &declared) {
                checked.entry(path.clone()).or_default().push(Finding {
                    kind,
                    span: at.span,
                    message,
                });
            }
        }
        // Rebuilt first, so an attachment a previous check took out comes back once nothing is
        // wrong with it.
        self.merge();
        let dead: FxHashSet<(&Path, Span)> = checked
            .iter()
            .flat_map(|(path, findings)| findings.iter().map(move |finding| (path, finding)))
            .filter(|(_, finding)| finding.kind != FindingKind::UndeclaredClass)
            .map(|(path, finding)| (path.as_path(), finding.span))
            .collect();
        if !dead.is_empty() {
            for attachments in self.table.values_mut().flat_map(FxHashMap::values_mut) {
                for list in attachments.values_mut() {
                    list.retain(|at| !dead.contains(&(at.file.as_path(), at.span)));
                }
            }
        }
        let changed = checked != self.checked_findings;
        self.checked_findings = checked;
        changed
    }

    /// Whether `path` is a completion file the walk would find, and if so whether it is under
    /// `vendor/`.
    fn place(&self, path: &Path) -> Option<bool> {
        if !path.extension().is_some_and(|ext| ext == "json") {
            return None;
        }
        let root = self.roots.iter().find(|root| path.starts_with(root))?;
        let names: Vec<&str> = path
            .strip_prefix(root)
            .ok()?
            .components()
            .map(|part| match part {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect::<Option<_>>()?;
        let at = names
            .windows(2)
            .position(|pair| pair == [NOVIS, COMPLETION])?;
        let above = &names[..at];
        if above.iter().any(|name| skipped(name)) {
            return None;
        }
        Some(above.contains(&"vendor"))
    }

    /// Rebuilds the sets and the table from every file, in path order.
    fn merge(&mut self) {
        self.sets.clear();
        self.table.clear();
        self.merged_findings.clear();
        for file in self.files.values() {
            for (name, values) in &file.read.sets {
                let merged = self.sets.entry(name.clone()).or_default();
                extend_once(merged, values);
            }
        }
        for (path, file) in &self.files {
            for written in &file.read.attachments {
                let mut values = Vec::new();
                if let Some(name) = &written.set {
                    match self.sets.get(name) {
                        Some(set) => extend_once(&mut values, set),
                        None => {
                            self.merged_findings
                                .entry(path.clone())
                                .or_default()
                                .push(Finding {
                                    kind: FindingKind::MissingSet,
                                    span: written.span,
                                    message: format!(
                                        "No completion file defines the set `{name}`."
                                    ),
                                })
                        }
                    }
                }
                extend_once(&mut values, &written.values);
                self.table
                    .entry(written.class.clone())
                    .or_default()
                    .entry(written.method.clone())
                    .or_default()
                    .entry(written.parameter.clone())
                    .or_default()
                    .push(Attachment {
                        values,
                        when: written.when.clone(),
                        strict: written.strict,
                        file: path.clone(),
                        span: written.span,
                    });
            }
        }
    }
}

/// What is wrong with the attachment `at`, given the declarations of its class and its method that
/// the index holds.
fn check_attachment(
    at: &Written,
    declared: &FxHashMap<&str, &Declaration>,
) -> Vec<(FindingKind, String)> {
    let Written { class, method, .. } = at;
    if !declared.contains_key(class.as_str()) {
        return vec![(
            FindingKind::UndeclaredClass,
            format!(
                "The workspace does not declare the class `{class}`. If it comes from `vendor/`, \
                 this is expected."
            ),
        )];
    }
    let Some(declaration) = declared
        .get(format!("{class}::{method}").as_str())
        .filter(|declaration| declaration.kind == DeclKind::Method)
    else {
        return vec![(
            FindingKind::MissingMember,
            format!("`{class}` does not declare the method `{method}`."),
        )];
    };
    let parameter = |name: &str| {
        declaration
            .parameters
            .iter()
            .find(|parameter| parameter.name == name)
    };
    let mut found = Vec::new();
    match parameter(&at.parameter) {
        None => found.push((
            FindingKind::MissingMember,
            format!("`{class}::{method}` has no parameter `${}`.", at.parameter),
        )),
        Some(parameter) if !parameter.takes_a_string => found.push((
            FindingKind::NotAString,
            format!(
                "The parameter `${}` of `{class}::{method}` does not take a string, so these \
                 values are never offered.",
                at.parameter
            ),
        )),
        Some(_) => {}
    }
    if let Some(when) = &at.when
        && parameter(&when.parameter).is_none()
    {
        found.push((
            FindingKind::MissingMember,
            format!(
                "`{class}::{method}` has no parameter `${}`, so this attachment never applies.",
                when.parameter
            ),
        ));
    }
    found
}

/// Appends each of `more` whose `value` `into` does not have yet.
fn extend_once(into: &mut Vec<Arc<Value>>, more: &[Arc<Value>]) {
    for value in more {
        if !into.iter().any(|held| held.value == value.value) {
            into.push(Arc::clone(value));
        }
    }
}

/// Whether the walk skips a folder named `name`. `.novis` is the one dot-folder it enters.
fn skipped(name: &str) -> bool {
    (name.starts_with('.') && name != NOVIS) || name == "target" || name == "node_modules"
}

/// Every `.json` file under a `.novis/completion/` folder below `dir`.
///
/// A folder this process cannot list is skipped, as `crate::index`'s walk skips one: a workspace
/// with one unreadable folder in it still has its other completion files.
fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == NOVIS {
            json_files(&entry.path().join(COMPLETION), found);
        } else if !skipped(name) {
            walk(&entry.path(), found);
        }
    }
}

/// Every `.json` file in `dir` and its subfolders.
fn json_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            json_files(&path, found);
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            found.push(path);
        }
    }
}

/// `path`'s modification time and size, or nothing when it is gone.
fn stamp_of(path: &Path) -> Option<Stamp> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then(|| Stamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

/// What the completion file at `path` says.
fn read(path: &Path) -> (String, Read) {
    let folder: Arc<Path> = Arc::from(path.parent().unwrap_or(path));
    match fs::read_to_string(path) {
        Ok(text) => {
            let read = Reader {
                text: &text,
                folder,
                findings: Vec::new(),
            }
            .file();
            (text, read)
        }
        Err(error) => (
            String::new(),
            Read {
                findings: vec![Finding {
                    kind: FindingKind::Invalid,
                    span: (0, 0),
                    message: format!("This file could not be read as UTF-8 text: {error}."),
                }],
                ..Read::default()
            },
        ),
    }
}

/// A wrong-form finding, which drops the whole file.
#[derive(Debug)]
struct Wrong(Finding);

/// One file's parse, which collects the findings that do not drop it.
struct Reader<'t> {
    text: &'t str,
    folder: Arc<Path>,
    findings: Vec<Finding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileShape<'a> {
    #[serde(rename = "$schema", default)]
    _schema: Option<IgnoredAny>,
    #[serde(default, borrow)]
    sets: BTreeMap<String, &'a RawValue>,
    #[serde(default, borrow)]
    parameters: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListShape<'a> {
    separator: SeparatorShape,
    #[serde(borrow)]
    values: Vec<&'a RawValue>,
}

#[derive(Clone, Copy, Deserialize)]
enum SeparatorShape {
    #[serde(rename = ".")]
    Dot,
    #[serde(rename = "/")]
    Slash,
    #[serde(rename = ":")]
    Colon,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ValueShape {
    value: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    label_detail: Option<String>,
    #[serde(default)]
    label_description: Option<String>,
    #[serde(default)]
    kind: Option<KindShape>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    documentation: Option<String>,
    #[serde(default)]
    deprecated: bool,
    #[serde(default)]
    sort_text: Option<String>,
    #[serde(default)]
    filter_text: Option<String>,
    #[serde(default)]
    preselect: bool,
    #[serde(default)]
    location: Option<LocationShape>,
    #[serde(default)]
    replacement: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationShape {
    file: String,
    line: NonZeroU32,
}

/// The protocol's completion item kinds, by the camelCase names a file writes them with.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum KindShape {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentShape<'a> {
    method: String,
    parameter: String,
    #[serde(default)]
    set: Option<String>,
    #[serde(default, borrow)]
    values: Option<&'a RawValue>,
    #[serde(default)]
    when: Option<WhenShape>,
    #[serde(default)]
    strict: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WhenShape {
    parameter: String,
    equals: EqualsShape,
}

#[derive(Deserialize)]
#[serde(untagged, expecting = "a string or a list of strings")]
enum EqualsShape {
    One(String),
    Many(Vec<String>),
}

impl<'t> Reader<'t> {
    /// The whole file, or only the finding that drops it.
    fn file(mut self) -> Read {
        match self.contents() {
            Ok((sets, attachments)) => Read {
                sets,
                attachments,
                findings: self.findings,
            },
            Err(Wrong(finding)) => Read {
                findings: vec![finding],
                ..Read::default()
            },
        }
    }

    fn contents(&mut self) -> Result<(Vec<NamedSet>, Vec<Written>), Wrong> {
        let text = self.text;
        let shape: FileShape<'_> = self.parse(text)?;
        let mut sets = Vec::new();
        for (name, list) in shape.sets {
            sets.push((name, self.list(list)?));
        }
        let mut attachments = Vec::new();
        for raw in shape.parameters {
            attachments.push(self.attachment(raw)?);
        }
        Ok((sets, attachments))
    }

    fn attachment(&mut self, raw: &RawValue) -> Result<Written, Wrong> {
        let span = self.span_of(raw);
        let shape: AttachmentShape<'_> = self.parse(raw.get())?;
        let Some((class, method)) = shape.method.rsplit_once("::") else {
            return Err(self.wrong(
                span,
                format!(
                    "`method` is `{}`. Write it as `Class::method`, with the class's whole name.",
                    shape.method
                ),
            ));
        };
        if shape.set.is_none() && shape.values.is_none() {
            return Err(self.wrong(
                span,
                "An attachment needs `set`, `values` or both.".to_owned(),
            ));
        }
        let values = match shape.values {
            Some(list) => self.list(list)?,
            None => Vec::new(),
        };
        Ok(Written {
            class: class.trim_start_matches('\\').to_owned(),
            method: method.to_owned(),
            parameter: shape.parameter,
            set: shape.set,
            values,
            when: shape.when.map(|when| When {
                parameter: when.parameter,
                equals: match when.equals {
                    EqualsShape::One(one) => vec![one],
                    EqualsShape::Many(many) => many,
                },
            }),
            strict: shape.strict,
            span,
        })
    }

    /// A list, written as an array of values or as `{ "separator", "values" }`.
    fn list(&mut self, raw: &RawValue) -> Result<Vec<Arc<Value>>, Wrong> {
        let (separator, values) = if raw.get().starts_with('[') {
            (None, self.parse::<Vec<&RawValue>>(raw.get())?)
        } else if raw.get().starts_with('{') {
            let shape: ListShape<'_> = self.parse(raw.get())?;
            let separator = match shape.separator {
                SeparatorShape::Dot => '.',
                SeparatorShape::Slash => '/',
                SeparatorShape::Colon => ':',
            };
            (Some(separator), shape.values)
        } else {
            return Err(self.wrong(
                self.span_of(raw),
                "A list is an array of values, or an object with `separator` and `values`."
                    .to_owned(),
            ));
        };
        let mut list = Vec::new();
        for value in values {
            list.push(Arc::new(self.value(value, separator)?));
        }
        Ok(list)
    }

    /// One value, written as a string or as an object.
    fn value(&mut self, raw: &RawValue, separator: Option<char>) -> Result<Value, Wrong> {
        let span = self.span_of(raw);
        let shape = if raw.get().starts_with('"') {
            ValueShape {
                value: self.parse(raw.get())?,
                label: None,
                label_detail: None,
                label_description: None,
                kind: None,
                title: None,
                documentation: None,
                deprecated: false,
                sort_text: None,
                filter_text: None,
                preselect: false,
                location: None,
                replacement: None,
            }
        } else if raw.get().starts_with('{') {
            self.parse(raw.get())?
        } else {
            return Err(self.wrong(
                span,
                "A value is a string, or an object with a `value` field.".to_owned(),
            ));
        };
        let location = shape.location.and_then(|location| {
            let file = self.folder.join(&location.file);
            if fs::metadata(&file).is_ok() {
                Some((file, location.line))
            } else {
                self.findings.push(Finding {
                    kind: FindingKind::MissingLocation,
                    span,
                    message: format!(
                        "The `location` of `{}` names `{}`, and that file does not exist.",
                        shape.value, location.file
                    ),
                });
                None
            }
        });
        let replacement = match shape.replacement {
            Some(_) if !shape.deprecated => {
                self.findings.push(Finding {
                    kind: FindingKind::ReplacementNotDeprecated,
                    span,
                    message: format!(
                        "`{}` has a `replacement` but is not deprecated, so the replacement is \
                         not used. Add `\"deprecated\": true`, or remove `replacement`.",
                        shape.value
                    ),
                });
                None
            }
            replacement => replacement,
        };
        Ok(Value {
            value: shape.value,
            label: shape.label,
            label_detail: shape.label_detail,
            label_description: shape.label_description,
            kind: shape.kind.map_or(CompletionItemKind::VALUE, kind_of),
            title: shape.title,
            documentation: shape.documentation,
            deprecated: shape.deprecated,
            sort_text: shape.sort_text,
            filter_text: shape.filter_text,
            preselect: shape.preselect,
            location,
            replacement,
            separator,
            folder: Arc::clone(&self.folder),
        })
    }

    /// `slice`, a part of this file's text, parsed as `T`; a failure is a wrong-form finding at the
    /// place serde stopped.
    fn parse<'a, T: Deserialize<'a>>(&self, slice: &'a str) -> Result<T, Wrong> {
        serde_json::from_str(slice).map_err(|error| {
            let at = self.offset_of(slice) + offset_in(slice, error.line(), error.column());
            let message = format!("This file is not used until it is fixed: {error}.");
            Wrong(Finding {
                kind: FindingKind::Invalid,
                span: (at, at),
                message,
            })
        })
    }

    fn wrong(&self, span: Span, message: String) -> Wrong {
        Wrong(Finding {
            kind: FindingKind::Invalid,
            span,
            message: format!("This file is not used until it is fixed: {message}"),
        })
    }

    /// Where `raw` sits in this file's text.
    fn span_of(&self, raw: &RawValue) -> Span {
        let start = self.offset_of(raw.get());
        (start, start + raw.get().len())
    }

    /// The byte offset of `slice`, which borrows from this file's text, within it.
    fn offset_of(&self, slice: &str) -> usize {
        (slice.as_ptr() as usize)
            .saturating_sub(self.text.as_ptr() as usize)
            .min(self.text.len())
    }
}

/// The byte offset of serde's 1-based `line` and `column` in `text`.
fn offset_in(text: &str, line: usize, column: usize) -> usize {
    let start: usize = text
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum();
    (start + column.saturating_sub(1)).min(text.len())
}

const fn kind_of(kind: KindShape) -> CompletionItemKind {
    match kind {
        KindShape::Text => CompletionItemKind::TEXT,
        KindShape::Method => CompletionItemKind::METHOD,
        KindShape::Function => CompletionItemKind::FUNCTION,
        KindShape::Constructor => CompletionItemKind::CONSTRUCTOR,
        KindShape::Field => CompletionItemKind::FIELD,
        KindShape::Variable => CompletionItemKind::VARIABLE,
        KindShape::Class => CompletionItemKind::CLASS,
        KindShape::Interface => CompletionItemKind::INTERFACE,
        KindShape::Module => CompletionItemKind::MODULE,
        KindShape::Property => CompletionItemKind::PROPERTY,
        KindShape::Unit => CompletionItemKind::UNIT,
        KindShape::Value => CompletionItemKind::VALUE,
        KindShape::Enum => CompletionItemKind::ENUM,
        KindShape::Keyword => CompletionItemKind::KEYWORD,
        KindShape::Snippet => CompletionItemKind::SNIPPET,
        KindShape::Color => CompletionItemKind::COLOR,
        KindShape::File => CompletionItemKind::FILE,
        KindShape::Reference => CompletionItemKind::REFERENCE,
        KindShape::Folder => CompletionItemKind::FOLDER,
        KindShape::EnumMember => CompletionItemKind::ENUM_MEMBER,
        KindShape::Constant => CompletionItemKind::CONSTANT,
        KindShape::Struct => CompletionItemKind::STRUCT,
        KindShape::Event => CompletionItemKind::EVENT,
        KindShape::Operator => CompletionItemKind::OPERATOR,
        KindShape::TypeParameter => CompletionItemKind::TYPE_PARAMETER,
    }
}
