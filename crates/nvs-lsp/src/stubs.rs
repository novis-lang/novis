//! The stub tree: one generated `.nvs` file per `Core` class, enum, interface
//! and compiler attribute, which is what a jump to a `Core` name opens.
//!
//! A `Core` name has no declaration in any source file — the registry is Rust
//! data, and `nvs_hir` resolves every `Core\…` without a symbol — so until this
//! module existed nothing under `Core` was a place an editor could go. A
//! stub is that place: a declaration with an empty body and its real
//! signature, carrying the same reference card hover renders
//! (`rule:core-api/reference-card`), written to disk once per server version.
//! Written rather than served as a virtual document because a `file:` location
//! works unchanged in every client and a cursor *inside* a stub keeps every
//! answer this server gives (`rule:ide/one-server-two-thin-clients`,
//! `rule:ide/the-stub-tree-is-where-core-is-declared`).
//!
//! **One generator, three readers.** [`tree`] renders every file once per
//! process, and [`Stubs`] writes it where a client asked, `nvs stubs --out`
//! writes it where a person asked, and `crate::definition` reads the **line
//! table** the same render produced — `(qualified name, member) → line` — so
//! no stub is ever parsed to find out where a name in it is. A file the
//! render spells is a file the table knows the lines of, by construction.
//!
//! **A stub declares `Core`, which user source may not.** The parser reports
//! `E0217` on its `namespace` line and keeps going, so every declaration under
//! it is in the tree an editor asks about; the server publishes no diagnostic
//! for a document under the tree's directory, that one included, because a
//! stub's empty bodies do not type-check either and nothing in it is the
//! reader's to fix.
//!
//! **A stub is written, never repaired.** [`Stubs::locate`] writes the whole
//! tree when its stamp is not the running build's or the file asked for is
//! missing, and otherwise touches nothing; a file somebody edited is written
//! over the next time the stamp changes, which the header comment in every
//! file says. Each file is made read-only after it is written, which is as
//! much as a file on disk can say about itself.
//!
//! **Registry prose is rendered as it is.** A card written in this
//! repository's own voice reads that way in the stub too; rewriting a card is
//! the registry's business and not this renderer's. What this module writes
//! itself — the header — is written for the person who opened the file.
//!
//! **What it spends:** the rendered tree once per process, on the first `Core`
//! jump — a few hundred kilobytes of text, held for the process's life so the
//! line table is a lookup — and the same again on disk per server version.

use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use nvs_stdlib::registry::{self, Const, CoreClass, CoreEnum, CoreMethod, CoreTy};
use nvs_syntax::Keyword;
use nvs_types::derive::{self, AttributeDoc};

use crate::hover::reference_card;

/// The name of the file that records which render a directory holds.
const STAMP: &str = ".stamp";

/// Where one declaration is inside its stub: the line and the column its
/// name starts at, and the name's length — all in bytes, which is also every
/// other encoding's count because a stub's declaration lines are ASCII.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    /// 0-based.
    pub line: u32,
    /// 0-based, from the start of the line.
    pub character: u32,
    /// The declared name's length.
    pub length: u32,
}

/// One rendered stub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StubFile {
    /// Its path under the tree's root, `/`-separated — `Core/Http/Method.nvs`.
    pub path: String,
    /// The whole file.
    pub text: String,
}

/// Every stub, and where every declaration in them is.
#[derive(Debug)]
pub struct Tree {
    /// The files, in registry order: classes, enums, interfaces, attributes.
    pub files: Vec<StubFile>,
    /// What this render is, for [`Stubs`] to tell a directory it wrote from
    /// one an earlier build wrote.
    stamp: String,
    /// `(qualified name, member)` → the file it is in and the line.
    lines: HashMap<(String, Option<String>), (usize, Line)>,
}

impl Tree {
    /// Where `member` of `name` — or `name` itself, for `None` — is declared:
    /// the stub's path under the root, and the line. `None` for a name the
    /// registry does not hold.
    #[must_use]
    pub fn line(&self, name: &str, member: Option<&str>) -> Option<(&str, Line)> {
        let (file, line) = self
            .lines
            .get(&(name.to_owned(), member.map(str::to_owned)))?;
        Some((self.files.get(*file)?.path.as_str(), *line))
    }
}

/// The tree, rendered once per process.
pub fn tree() -> &'static Tree {
    static TREE: OnceLock<Tree> = OnceLock::new();
    TREE.get_or_init(render)
}

/// Renders every stub from the registry and the attribute roster.
#[must_use]
pub fn render() -> Tree {
    let mut tree = Tree {
        files: Vec::new(),
        stamp: String::new(),
        lines: HashMap::new(),
    };
    for class in registry::CLASSES {
        let (text, lines) = class_stub(class);
        tree.add(class.name, text, lines);
    }
    for core in registry::ENUMS {
        let (text, lines) = enum_stub(core);
        tree.add(core.name, text, lines);
    }
    for name in registry::DERIVE_INTERFACES {
        let (text, lines) = interface_stub(name);
        tree.add(name, text, lines);
    }
    for doc in derive::ATTRIBUTE_DOCS {
        // An attribute whose name is also a class — `Core\Test`,
        // `Core\Command` — is declared in the class's file, where
        // [`class_stub`] wrote its card into the class's own.
        if registry::class(doc.name).is_some() {
            continue;
        }
        let (text, lines) = attribute_stub(doc);
        tree.add(doc.name, text, lines);
    }
    let mut hasher = DefaultHasher::new();
    for file in &tree.files {
        file.path.hash(&mut hasher);
        file.text.hash(&mut hasher);
    }
    tree.stamp = format!("{} {:016x}\n", env!("CARGO_PKG_VERSION"), hasher.finish());
    tree
}

impl Tree {
    fn add(&mut self, name: &str, text: String, lines: Vec<(Option<String>, Line)>) {
        let index = self.files.len();
        self.files.push(StubFile {
            path: format!("{}.nvs", name.replace('\\', "/")),
            text,
        });
        for (member, line) in lines {
            self.lines.insert((name.to_owned(), member), (index, line));
        }
    }
}

/// The directory a stub tree lives in, and the writes that keep it there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stubs {
    dir: PathBuf,
}

impl Stubs {
    /// The tree at `dir`, written or not.
    #[must_use]
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Where the tree goes when no client named a directory: this account's
    /// cache directory, one subdirectory per server version, so a bare
    /// `nvs lsp` still answers a jump. `%LOCALAPPDATA%\novis\stubs\<version>`
    /// on Windows, `$XDG_CACHE_HOME`'s or `~/.cache`'s `novis/stubs/<version>`
    /// elsewhere — the layout `nvs-cli`'s opcache uses, beside it. `None` on
    /// an account with none of those set.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        #[cfg(windows)]
        let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        #[cfg(not(windows))]
        let root = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
        Some(
            root?
                .join("novis")
                .join("stubs")
                .join(env!("CARGO_PKG_VERSION")),
        )
    }

    /// The directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Whether `path` is a file of this tree — which is what makes a document
    /// one the server publishes no diagnostics for: a stub's bodies are empty
    /// and do not type-check, and nothing in it is the reader's to fix.
    ///
    /// Both sides are canonicalized where they exist, because a client spells
    /// a path the way it opened it and the directory may have been reached
    /// through a link.
    #[must_use]
    pub fn holds(&self, path: &Path) -> bool {
        if path.starts_with(&self.dir) {
            return true;
        }
        match (fs::canonicalize(&self.dir), fs::canonicalize(path)) {
            (Ok(dir), Ok(path)) => path.starts_with(dir),
            _ => false,
        }
    }

    /// Where `member` of `name` is declared on disk, writing the tree first
    /// when the directory holds another build's render or not this file.
    ///
    /// `None` for a name the registry does not hold, and for a tree that could
    /// not be written — a jump that opens nothing, which is what the cursor got
    /// before this module existed.
    #[must_use]
    pub fn locate(&self, name: &str, member: Option<&str>) -> Option<(PathBuf, Line)> {
        let tree = tree();
        let (relative, line) = tree.line(name, member)?;
        let path = self.dir.join(relative);
        let current =
            fs::read_to_string(self.dir.join(STAMP)).is_ok_and(|stamp| stamp == tree.stamp);
        if !current || !path.is_file() {
            write_to(&self.dir).ok()?;
        }
        Some((path, line))
    }
}

/// Writes every stub under `dir`, replacing what is there, and answers the
/// paths written in the order they were written.
///
/// A file already there is made writable before it is replaced and every file
/// is made read-only after, so a tree written twice lands the second time too.
/// The stamp is written last, so a directory a failed write left behind
/// carries no stamp and is written again at the next jump.
///
/// # Errors
///
/// The first write that fails, with nothing rolled back: the next
/// [`Stubs::locate`] writes the whole tree again.
pub fn write_to(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let tree = tree();
    let mut written = Vec::with_capacity(tree.files.len());
    for file in &tree.files {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        set_readonly(&path, false);
        fs::write(&path, &file.text)?;
        set_readonly(&path, true);
        written.push(path);
    }
    let stamp = dir.join(STAMP);
    set_readonly(&stamp, false);
    fs::write(&stamp, &tree.stamp)?;
    Ok(written)
}

/// Makes every file under `dir` writable again, for a caller about to remove
/// the tree — `remove_dir_all` refuses a read-only file on Windows.
pub fn unlock(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            unlock(&path);
        } else {
            set_readonly(&path, false);
        }
    }
}

/// Sets or clears the read-only bit on one file, and says nothing about a file
/// that is not there: the write that follows reports that itself.
fn set_readonly(path: &Path, readonly: bool) {
    if let Ok(metadata) = fs::metadata(path) {
        let mut permissions = metadata.permissions();
        #[allow(
            clippy::permissions_set_readonly_false,
            reason = "clearing the bit this module set is the point, on a file it wrote"
        )]
        permissions.set_readonly(readonly);
        let _ = fs::set_permissions(path, permissions);
    }
}

/// The text of one stub as it is built, with the line it is on.
struct Writer {
    text: String,
    line: u32,
}

impl Writer {
    /// A file's head: the header comment and the namespace `name` is declared
    /// in.
    fn open(name: &str) -> Self {
        let mut writer = Self {
            text: String::new(),
            line: 0,
        };
        writer.push(&format!(
            "<?nvs\n// This file is generated by `nvs stubs` for Novis {}. Your editor reads it to\n\
             // explain `{name}`. Do not edit this file: the next `nvs stubs` run writes it again.\n\n",
            env!("CARGO_PKG_VERSION")
        ));
        if let Some((namespace, _)) = name.rsplit_once('\\') {
            writer.push(&format!("namespace {namespace};\n\n"));
        }
        writer
    }

    fn push(&mut self, text: &str) {
        self.line += u32::try_from(text.matches('\n').count()).unwrap_or(u32::MAX);
        self.text.push_str(text);
    }

    /// Writes `name` and answers where it landed.
    fn name(&mut self, name: &str) -> Line {
        let start = self.text.rfind('\n').map_or(0, |at| at + 1);
        let line = Line {
            line: self.line,
            character: u32::try_from(self.text.len() - start).unwrap_or(u32::MAX),
            length: u32::try_from(name.len()).unwrap_or(u32::MAX),
        };
        self.text.push_str(name);
        line
    }

    /// A `///` run, indented by `indent`, from Markdown that may span lines.
    fn doc(&mut self, indent: &str, text: &str) {
        for line in text.trim().lines() {
            let line = line.trim_end();
            if line.is_empty() {
                self.push(&format!("{indent}///\n"));
            } else {
                self.push(&format!("{indent}/// {line}\n"));
            }
        }
    }
}

/// The short name of a qualified one — `Method` of `Core\Http\Method`.
fn short(name: &str) -> &str {
    name.rsplit('\\').next().unwrap_or(name)
}

/// One class: its card, then every constant, every static member, the
/// constructor `new` reaches where there is one, and every instance member,
/// each under its own card.
fn class_stub(class: &CoreClass) -> (String, Vec<(Option<String>, Line)>) {
    let mut writer = Writer::open(class.name);
    let mut lines = Vec::new();
    let mut card = class.doc.map_or(String::new(), |doc| doc.short.to_owned());
    if let Some(doc) = derive::attribute_doc(class.name) {
        // The class is also an attribute, and this file is where both are
        // declared: the attribute's card joins the class's.
        if !card.is_empty() {
            card.push_str("\n\n");
        }
        card.push_str(&format!(
            "Written above {} as `{}`: {}",
            doc.site,
            doc.spelled(),
            doc.short
        ));
    }
    writer.doc("", &card);
    writer.push("final class ");
    lines.push((None, writer.name(short(class.name))));
    writer.push("\n{\n");
    let mut first = true;
    for constant in class.constants {
        separate(&mut writer, &mut first);
        writer.doc("    ", constant.desc);
        writer.push(&format!("    public const {} ", stub_ty(&constant.ty)));
        lines.push((Some(constant.name.to_owned()), writer.name(constant.name)));
        writer.push(&format!(" = {};\n", spell_const(&constant.value)));
    }
    for member in class.methods {
        separate(&mut writer, &mut first);
        lines.push((
            Some(member.name.to_owned()),
            method(&mut writer, member, "public static function "),
        ));
    }
    if let Some(constructor) = registry::constructor_of(class.name) {
        separate(&mut writer, &mut first);
        lines.push((
            Some(constructor.name.to_owned()),
            method(&mut writer, constructor, "public function "),
        ));
    }
    for member in class.instance {
        separate(&mut writer, &mut first);
        lines.push((
            Some(member.name.to_owned()),
            method(&mut writer, member, "public function "),
        ));
    }
    writer.push("}\n");
    (writer.text, lines)
}

/// A blank line between two members, and none in front of the first.
fn separate(writer: &mut Writer, first: &mut bool) {
    if !*first {
        writer.push("\n");
    }
    *first = false;
}

/// One member as a declaration with an empty body, under its reference card,
/// and where its name landed.
fn method(writer: &mut Writer, member: &CoreMethod, keyword: &str) -> Line {
    if let Some(doc) = member.doc {
        writer.doc("    ", &reference_card(member, doc));
    }
    writer.push(&format!("    {keyword}"));
    let line = writer.name(member.name);
    let ret = if member.name == "constructor" {
        String::new()
    } else {
        format!(": {}", stub_ty(&member.return_ty))
    };
    writer.push(&format!("({}){ret} {{}}\n", params(member)));
    line
}

/// A member's parameter list as a declaration writes it: each positional
/// parameter with its type and name, a default on each optional one, a
/// variadic tail as `T ...$rest`, and a trailing options bag as a shape-typed
/// parameter under [`registry::OPTIONS_NAME`].
fn params(member: &CoreMethod) -> String {
    let positional = member.positional();
    let required = positional.len().saturating_sub(member.defaults.len());
    let mut parts: Vec<String> = positional
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let name = member.names.get(index).copied().unwrap_or("");
            match ty {
                CoreTy::Variadic(elem) => format!("{} ...${name}", stub_ty(elem)),
                _ => match index
                    .checked_sub(required)
                    .and_then(|at| member.defaults.get(at))
                {
                    Some(default) => format!("{} ${name} = {}", stub_ty(ty), spell_const(default)),
                    None => format!("{} ${name}", stub_ty(ty)),
                },
            }
        })
        .collect();
    if let Some(options) = member.options() {
        parts.push(format!(
            "{} ${}",
            stub_ty(&CoreTy::Options(options)),
            registry::OPTIONS_NAME
        ));
    }
    parts.join(", ")
}

/// A registry type as a declaration writes it: [`CoreTy::spelled`], which is
/// the spelling `nvs meta` prints, with the one spelling that is prose rather
/// than syntax written as the type it stands for.
fn stub_ty(ty: &CoreTy) -> String {
    match ty {
        // "A shape of callables" has no closed spelling — its keys are the
        // caller's — so the type a program can write for it is the erased one.
        CoreTy::ShapeOfCallables(_) => "object".to_owned(),
        // A key that is a reserved word — `default` — is one a shape *type*
        // cannot spell, though a literal writes it freely; the bag is declared
        // erased and its card still names every key.
        CoreTy::Options(options) if options.iter().any(|option| reserved(option.name)) => {
            "object".to_owned()
        }
        _ => ty.spelled(),
    }
}

/// Whether `name` is one of the grammar's reserved words.
fn reserved(name: &str) -> bool {
    Keyword::from_lowercase(&name.to_ascii_lowercase()).is_some()
}

/// A constant as a declaration writes it.
///
/// A scalar is its literal, a string is quoted, a case is its qualified
/// spelling, and a built instance is the call that builds it — the same
/// readings `nvs meta` prints, spelled so that a parser accepts each: an
/// omitted option's marker has no written form and stands as `null`, and a
/// `bytes` value is written as the string of its bytes, which is the one
/// spelling for it the grammar has.
fn spell_const(value: &Const) -> String {
    match value {
        Const::Null | Const::NeverWritten => "null".to_owned(),
        Const::Bool(value) => value.to_string(),
        Const::Int(value) => value.to_string(),
        Const::Uint(value) => value.to_string(),
        Const::Float(value) => format!("{value:?}"),
        Const::Str(text) => quoted(text),
        Const::Bytes(bytes) => quoted(&String::from_utf8_lossy(bytes)),
        Const::EmptyArray => "[]".to_owned(),
        Const::EnumCase(owner, case) => format!("{owner}::{case}"),
        Const::Built { symbol, args } => {
            let call = registry::CLASSES
                .iter()
                .find_map(|class| {
                    class
                        .members()
                        .find(|member| member.symbol == *symbol)
                        .map(|member| format!("{}::{}", class.name, member.name))
                })
                .unwrap_or_else(|| (*symbol).to_owned());
            let args: Vec<String> = args.iter().map(spell_const).collect();
            format!("{call}({})", args.join(", "))
        }
        _ => "null".to_owned(),
    }
}

/// `text` as a double-quoted literal.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One enum: its card, then every case at its value, each under its line.
fn enum_stub(core: &CoreEnum) -> (String, Vec<(Option<String>, Line)>) {
    let mut writer = Writer::open(core.name);
    let mut lines = Vec::new();
    if let Some(doc) = core.doc {
        writer.doc("", doc.short);
    }
    writer.push("enum ");
    lines.push((None, writer.name(short(core.name))));
    writer.push(": int\n{\n");
    for (case, value) in core.cases {
        let desc = core
            .doc
            .and_then(|doc| doc.cases.iter().find(|written| written.name == *case))
            .map_or("", |written| written.desc);
        writer.doc("    ", desc);
        writer.push("    ");
        lines.push((Some((*case).to_owned()), writer.name(case)));
        writer.push(&format!(" = {value},\n"));
    }
    writer.push("}\n");
    (writer.text, lines)
}

/// One of the interfaces a derive attribute stands for: a name a class may
/// write in an `implements` clause, and nothing a program calls.
fn interface_stub(name: &str) -> (String, Vec<(Option<String>, Line)>) {
    let mut writer = Writer::open(name);
    writer.doc(
        "",
        "A class carrying the matching `Derive` attribute implements this interface. You may \
         write it in an `implements` clause, and you do not have to.",
    );
    writer.push("interface ");
    let line = writer.name(short(name));
    writer.push("\n{\n}\n");
    (writer.text, vec![(None, line)])
}

/// One compiler attribute that is not also a class: the shape its payload
/// satisfies, declared the way a userland attribute is
/// (`rule:attributes/attach-sites-and-forms`), under its card.
fn attribute_stub(doc: &AttributeDoc) -> (String, Vec<(Option<String>, Line)>) {
    let mut writer = Writer::open(doc.name);
    writer.doc(
        "",
        &format!(
            "{}\n\nWritten above {} as `{}`.",
            doc.short,
            doc.site,
            doc.spelled()
        ),
    );
    writer.push("type ");
    let line = writer.name(doc.short_name());
    let shape = if doc.fields.is_empty() {
        "object".to_owned()
    } else {
        doc.shape()
    };
    writer.push(&format!(" = {shape};\n"));
    (writer.text, vec![(None, line)])
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};

    use super::*;

    /// A stub that does not parse is a bug in the generator, not in the
    /// registry: every file is handed to the parser and must come back clean.
    ///
    /// Clean but for the one refusal a stub earns by construction: it declares
    /// `namespace Core`, which user source may not
    /// (`rule:core-api/reserved-namespace`, `E0217`). The parser reports that
    /// and keeps going, every declaration under it is in the tree, and the
    /// server publishes nothing for a stub in any case.
    #[test]
    fn every_stub_parses() {
        let mut failed = Vec::new();
        for file in &tree().files {
            let mut map = SourceMap::new();
            let id = map.add(file.path.clone(), file.text.clone());
            let mut diags = Diagnostics::new();
            let _ = nvs_syntax::parse_file(map.file(id), &mut diags);
            let first = diags
                .iter()
                .find(|diagnostic| diagnostic.code != Some(code::E_RESERVED_CORE_NAMESPACE));
            if let Some(first) = first {
                failed.push(format!("{}: {}", file.path, first.message));
            }
        }
        assert!(
            failed.is_empty(),
            "stubs that do not parse:\n{}",
            failed.join("\n")
        );
    }

    /// Every name the registry and the attribute roster hold has a line, and
    /// so does every member of each: the table is what a jump reads, and a
    /// name it lacks is a jump that opens nothing.
    #[test]
    fn every_core_name_and_member_has_a_line() {
        let tree = tree();
        let mut missing = Vec::new();
        for class in registry::CLASSES {
            if tree.line(class.name, None).is_none() {
                missing.push(class.name.to_owned());
            }
            for member in class.members() {
                if tree.line(class.name, Some(member.name)).is_none() {
                    missing.push(format!("{}::{}", class.name, member.name));
                }
            }
            for constant in class.constants {
                if tree.line(class.name, Some(constant.name)).is_none() {
                    missing.push(format!("{}::{}", class.name, constant.name));
                }
            }
        }
        for core in registry::ENUMS {
            if tree.line(core.name, None).is_none() {
                missing.push(core.name.to_owned());
            }
            for (case, _) in core.cases {
                if tree.line(core.name, Some(case)).is_none() {
                    missing.push(format!("{}::{case}", core.name));
                }
            }
        }
        for doc in derive::ATTRIBUTE_DOCS {
            if tree.line(doc.name, None).is_none() {
                missing.push(doc.name.to_owned());
            }
        }
        assert!(missing.is_empty(), "names with no line: {missing:?}");
    }

    /// A line names the declaration it claims to: the text at the recorded
    /// position is the name.
    #[test]
    fn every_line_lands_on_its_name() {
        let tree = tree();
        for ((name, member), (file, line)) in &tree.lines {
            let text = &tree.files[*file].text;
            let row = text
                .lines()
                .nth(line.line as usize)
                .expect("a recorded line exists");
            let at = line.character as usize;
            let found = &row[at..at + line.length as usize];
            let expected = member.as_deref().unwrap_or_else(|| short(name));
            assert_eq!(
                found, expected,
                "{name}::{member:?} in {}",
                tree.files[*file].path
            );
        }
    }

    /// One small file, verbatim, so a change to the spelling is a visible
    /// diff rather than a surprise in an editor.
    #[test]
    fn the_method_enum_is_spelled_exactly_so() {
        let tree = tree();
        let file = tree
            .files
            .iter()
            .find(|file| file.path == "Core/Http/Method.nvs")
            .expect("the Method enum has a stub");
        let expected = "<?nvs\n\
// This file is generated by `nvs stubs` for Novis {version}. Your editor reads it to\n\
// explain `Core\\Http\\Method`. Do not edit this file: the next `nvs stubs` run writes it again.\n\
\n\
namespace Core\\Http;\n\
\n\
/// The closed set of HTTP verbs a `#[Route]` may be declared under and a request may carry — eight of them, safe ones first so that the four the CSRF check covers are the contiguous tail from `Post` on; `CONNECT` is deliberately absent.\n\
enum Method: int\n\
{\n\
\x20   /// Reads a resource; safe, so no CSRF check applies.\n\
\x20   Get = 0,\n\
\x20   /// `Get` without a response body; safe.\n\
\x20   Head = 1,\n\
\x20   /// Asks what a resource supports; safe.\n\
\x20   Options = 2,\n\
\x20   /// Echoes the request back; safe.\n\
\x20   Trace = 3,\n\
\x20   /// Submits data and may change state; the first of the CSRF-covered tail.\n\
\x20   Post = 4,\n\
\x20   /// Replaces a resource; CSRF-covered.\n\
\x20   Put = 5,\n\
\x20   /// Changes a resource in place; CSRF-covered.\n\
\x20   Patch = 6,\n\
\x20   /// Removes a resource; CSRF-covered.\n\
\x20   Delete = 7,\n\
}\n"
        .replace("{version}", env!("CARGO_PKG_VERSION"));
        assert_eq!(file.text, expected);
    }

    /// A tree is written once per render: the second locate finds the stamp
    /// and writes nothing, and a missing file brings the whole tree back.
    #[test]
    fn a_tree_is_written_once_and_rewritten_when_a_file_is_missing() {
        let dir = std::env::temp_dir().join(format!("nvs-stubs-test-{}", std::process::id()));
        unlock(&dir);
        let _ = fs::remove_dir_all(&dir);
        let stubs = Stubs::at(dir.clone());
        let (path, line) = stubs
            .locate(r"Core\Http\Method", Some("Post"))
            .expect("a case has a line");
        assert!(path.ends_with(Path::new("Core/Http/Method.nvs")));
        assert_eq!(line.line, 18);
        assert!(stubs.holds(&path));
        let stamped = fs::metadata(dir.join(STAMP))
            .expect("a stamp")
            .modified()
            .ok();
        let _ = stubs.locate(r"Core\Str", Some("length"));
        assert_eq!(
            fs::metadata(dir.join(STAMP))
                .expect("a stamp")
                .modified()
                .ok(),
            stamped
        );
        set_readonly(&path, false);
        fs::remove_file(&path).expect("a stub can be removed");
        let (again, _) = stubs
            .locate(r"Core\Http\Method", None)
            .expect("the tree is written again");
        assert!(again.is_file());
        assert!(
            fs::metadata(&again)
                .expect("a stub")
                .permissions()
                .readonly()
        );
        unlock(&dir);
        let _ = fs::remove_dir_all(&dir);
    }
}
