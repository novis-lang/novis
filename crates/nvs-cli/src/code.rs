//! `nvs code` — questions about one project, answered from the editor's index.
//!
//! `rule:tooling/nvs-code-answers-about-the-project` is the surface. Every
//! answer is read through [`nvs_lsp::query`] over one [`nvs_lsp::workspace`],
//! the functions the editor's references and implementation requests call, so
//! this command and the editor never disagree. This module decides nothing
//! about the language: it finds the project, turns a [`Site`] into a position
//! a reader can type, escapes what the project wrote, and renders.
//!
//! A call costs one analysis per `.nvs` file under the project root, and
//! nothing is kept after it: there is no cache and no daemon, because either
//! is a second copy of the project that can be stale.
//!
//! **A position** is `path:line:column`, the path relative to the working
//! directory with `/` between its parts, the line and the column counted from
//! 1 and the column in characters. JSON carries the byte `start` and `end`
//! beside it. **Text from the project** — a source line, a doc comment, a
//! signature, a type, a path — has every control character but tab (and the
//! line breaks of a doc comment) written as `\u{..}`, in both renderings,
//! because a file's bytes reach a terminal or an agent's context here.
//!
//! The JSON follows `rule:ide/check-json-is-the-diagnostic-record-as-a-document`
//! as `nvs agent --json` does: `schemaVersion` at the root, every key present,
//! and a failure prints `error` and `nearest` on standard output while the
//! exit status is 1.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::ops::RangeInclusive;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{SourceFile, canonical_key};
use nvs_hir::members::edit_distance;
use nvs_lsp::query::{self, Outline};
use nvs_lsp::{DeclKind, Declaration, Site, Workspace};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, Stmt, StmtKind};
use serde_json::{Map, Value, json};

use crate::agent::{SCHEMA_VERSION, pretty};

/// How many names or files an unknown argument lists as nearest.
const NEAREST: usize = 5;

/// The working directory, the project around it and the workspace built over
/// that project, with each file's text read once.
struct Project {
    cwd: PathBuf,
    workspace: Workspace,
    texts: HashMap<PathBuf, String>,
}

impl Project {
    /// Finds the project and indexes it, and reports on standard error how
    /// many of its files have errors.
    fn open() -> Result<Self, ExitCode> {
        let cwd = std::env::current_dir().map_err(|err| {
            eprintln!("error: the working directory cannot be read: {err}");
            ExitCode::FAILURE
        })?;
        let cwd = canonical_key(&cwd);
        let root = project_root(&cwd);
        let workspace = nvs_lsp::workspace(&root);
        match workspace.index().files_with_errors() {
            0 => {}
            1 => eprintln!(
                "warning: 1 file in this project has errors. A use inside it may be missing."
            ),
            n => eprintln!(
                "warning: {n} files in this project have errors. A use inside them may be missing."
            ),
        }
        Ok(Self {
            cwd,
            workspace,
            texts: HashMap::new(),
        })
    }

    /// The text of `path`, read once, and empty for a file that cannot be read.
    fn text(&mut self, path: &Path) -> &str {
        self.texts
            .entry(path.to_owned())
            .or_insert_with(|| std::fs::read_to_string(path).unwrap_or_default())
    }

    /// `site` as a reader types it, and the source line it is on.
    fn place(&mut self, site: &Site) -> Place {
        let path = escaped(&relative(&site.path, &self.cwd));
        let text = self.text(&site.path);
        let before = text.get(..site.start as usize).unwrap_or_default();
        let line_start = before.rfind('\n').map_or(0, |at| at + 1);
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |at| line_start + at);
        Place {
            path,
            line: before.matches('\n').count() + 1,
            column: before[line_start..].chars().count() + 1,
            start: site.start,
            end: site.end,
            source: escaped(
                text[line_start..line_end]
                    .trim_start()
                    .trim_end_matches('\r'),
            ),
        }
    }

    /// A file argument, resolved against the working directory, when the
    /// project indexed it.
    fn file(&self, written: &str) -> Option<PathBuf> {
        let path = canonical_key(&self.cwd.join(written));
        self.workspace.index().holds(&path).then_some(path)
    }

    /// The project's files nearest to `written`, as positions print them.
    fn nearest_files(&self, written: &str) -> Vec<String> {
        let wanted = written.replace('\\', "/");
        let mut ranked: Vec<(usize, String)> = self
            .workspace
            .index()
            .files()
            .map(|path| {
                let shown = relative(path, &self.cwd);
                (edit_distance(&wanted, &shown), escaped(&shown))
            })
            .collect();
        ranked.sort();
        ranked
            .into_iter()
            .take(NEAREST)
            .map(|(_, shown)| shown)
            .collect()
    }

    /// The project's names nearest to `symbol`.
    fn nearest_names(&self, symbol: &str) -> Vec<String> {
        query::nearest(self.workspace.index(), symbol, NEAREST)
            .into_iter()
            .map(escaped)
            .collect()
    }
}

/// Where something is written, as a reader types it.
struct Place {
    path: String,
    line: usize,
    column: usize,
    start: u32,
    end: u32,
    /// The whole source line, its indentation removed.
    source: String,
}

impl Place {
    fn position(&self) -> String {
        format!("{}:{}:{}", self.path, self.line, self.column)
    }

    /// The position's keys of a JSON record.
    fn json(&self) -> Map<String, Value> {
        let mut out = Map::new();
        out.insert("path".into(), Value::from(self.path.clone()));
        out.insert("line".into(), Value::from(self.line));
        out.insert("column".into(), Value::from(self.column));
        out.insert("start".into(), Value::from(self.start));
        out.insert("end".into(), Value::from(self.end));
        out
    }
}

/// `nvs code show`: where a name is declared, its signature and its doc
/// comment, or a `Core` name's card as `nvs agent show` prints it.
pub(crate) fn show(symbol: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let card = query::declaration(&project.workspace, symbol)
        .map(|card| (card.declaration.clone(), card.doc));
    let Some((declared, doc)) = card else {
        if crate::agent::has_card(symbol) {
            return crate::agent::show(symbol, json);
        }
        let nearest = project.nearest_names(symbol);
        return unknown(
            &format!("no name `{}` in this project", escaped(symbol)),
            &nearest,
            json,
        );
    };
    let place = project.place(&declared.site);
    let signature = signature(&project.workspace, &declared).map(|text| escaped(&text));
    let doc = doc.as_deref().map(escaped_lines);
    if json {
        let mut out = Map::new();
        out.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
        out.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
        out.insert("kind".into(), Value::from(kind(declared.kind)));
        out.extend(place.json());
        out.insert(
            "signature".into(),
            signature.map_or(Value::Null, Value::from),
        );
        out.insert("doc".into(), doc.map_or(Value::Null, Value::from));
        print!("{}", pretty(Value::Object(out)));
        return ExitCode::SUCCESS;
    }
    println!(
        "{}  {}, declared at {}",
        escaped(&declared.symbol),
        kind(declared.kind),
        place.position()
    );
    if let Some(signature) = signature {
        println!("{signature}");
    }
    if let Some(doc) = doc {
        println!("{doc}");
    }
    ExitCode::SUCCESS
}

/// `nvs code refs`: the declaration, then every import and use, in path and
/// then line order.
pub(crate) fn refs(symbol: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let found: Vec<(&'static str, Site)> =
        query::references(project.workspace.index(), symbol, true)
            .iter()
            .map(|found| (found.kind.label(), found.site.clone()))
            .collect();
    if found.is_empty() && !known(&project, symbol) {
        let nearest = project.nearest_names(symbol);
        return unknown(
            &format!("no name `{}` in this project", escaped(symbol)),
            &nearest,
            json,
        );
    }
    let rows: Vec<(&str, Place)> = found
        .iter()
        .map(|(kind, site)| (*kind, project.place(site)))
        .collect();
    if json {
        let references: Vec<Value> = rows
            .iter()
            .map(|(kind, place)| {
                let mut record = place.json();
                record.insert("kind".into(), Value::from(*kind));
                record.insert("text".into(), Value::from(place.source.clone()));
                Value::Object(record)
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "symbol": escaped(symbol),
                "references": references,
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (kind, place) in &rows {
        println!("{:<width$}  {kind:<8}  {}", place.position(), place.source);
    }
    ExitCode::SUCCESS
}

/// `nvs code impls`: what sits below and above a type or a method.
pub(crate) fn impls(symbol: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let found = query::implementations(project.workspace.index(), symbol);
    let rows: Vec<(&str, Declaration)> = found
        .below
        .iter()
        .map(|declared| ("below", (*declared).clone()))
        .chain(
            found
                .above
                .iter()
                .map(|declared| ("above", (*declared).clone())),
        )
        .collect();
    if rows.is_empty() && !known(&project, symbol) {
        let nearest = project.nearest_names(symbol);
        return unknown(
            &format!("no name `{}` in this project", escaped(symbol)),
            &nearest,
            json,
        );
    }
    let placed: Vec<(&str, &Declaration, Place)> = rows
        .iter()
        .map(|(relation, declared)| (*relation, declared, project.place(&declared.site)))
        .collect();
    if json {
        let record = |declared: &Declaration, place: &Place| {
            let mut record = place.json();
            record.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
            record.insert("kind".into(), Value::from(kind(declared.kind)));
            Value::Object(record)
        };
        let side = |wanted: &str| -> Vec<Value> {
            placed
                .iter()
                .filter(|(relation, _, _)| *relation == wanted)
                .map(|(_, declared, place)| record(declared, place))
                .collect()
        };
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "symbol": escaped(symbol),
                "below": side("below"),
                "above": side("above"),
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = placed
        .iter()
        .map(|(_, _, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (relation, declared, place) in &placed {
        println!(
            "{:<width$}  {relation}  {}",
            place.position(),
            escaped(&declared.symbol)
        );
    }
    ExitCode::SUCCESS
}

/// `nvs code outline`: what a file declares, members under their owner.
pub(crate) fn outline(file: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let Some(path) = project.file(file) else {
        let nearest = project.nearest_files(file);
        return unknown(
            &format!("no file `{}` in this project", escaped(file)),
            &nearest,
            json,
        );
    };
    let index = project.workspace.index();
    let entries: Vec<(usize, Declaration)> = flatten(&query::outline(index, &path), 0);
    let rows: Vec<(usize, &Declaration, Place)> = entries
        .iter()
        .map(|(depth, declared)| (*depth, declared, project.place(&declared.site)))
        .collect();
    if json {
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "path": escaped(&relative(&path, &project.cwd)),
                "declarations": nested_json(&rows, 0, &mut 0),
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, _, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (depth, declared, place) in &rows {
        println!(
            "{:<width$}  {}{}  {}",
            place.position(),
            "  ".repeat(*depth),
            kind(declared.kind),
            escaped(&declared.symbol)
        );
    }
    ExitCode::SUCCESS
}

/// An outline in source order, each declaration with how deep it sits.
fn flatten(outline: &[Outline<'_>], depth: usize) -> Vec<(usize, Declaration)> {
    outline
        .iter()
        .flat_map(|entry| {
            std::iter::once((depth, entry.declaration.clone()))
                .chain(flatten(&entry.members, depth + 1))
        })
        .collect()
}

/// The flattened rows from `*next` on that sit at `depth`, each with the rows
/// below it under `members`.
fn nested_json(
    rows: &[(usize, &Declaration, Place)],
    depth: usize,
    next: &mut usize,
) -> Vec<Value> {
    let mut out = Vec::new();
    while let Some((at, declared, place)) = rows.get(*next) {
        if *at != depth {
            break;
        }
        *next += 1;
        let mut record = place.json();
        record.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
        record.insert("kind".into(), Value::from(kind(declared.kind)));
        record.insert(
            "members".into(),
            Value::Array(nested_json(rows, depth + 1, next)),
        );
        out.push(Value::Object(record));
    }
    out
}

/// `nvs code types`: each variable declared with `var` in a file or a range
/// of its lines, with the type the checker inferred.
pub(crate) fn types(spec: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let (file, lines) = split_lines(spec);
    let typed = project
        .file(file)
        .and_then(|path| query::types(&project.workspace, &path, lines).map(|typed| (path, typed)));
    let Some((path, typed)) = typed else {
        let nearest = project.nearest_files(file);
        return unknown(
            &format!("no file `{}` in this project", escaped(file)),
            &nearest,
            json,
        );
    };
    let rows: Vec<(String, String, Place)> = typed
        .iter()
        .map(|binding| {
            (
                escaped(&binding.name),
                escaped(&binding.ty),
                project.place(&binding.site),
            )
        })
        .collect();
    if json {
        let records: Vec<Value> = rows
            .iter()
            .map(|(name, ty, place)| {
                let mut record = place.json();
                record.insert("name".into(), Value::from(name.clone()));
                record.insert("type".into(), Value::from(ty.clone()));
                Value::Object(record)
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "path": escaped(&relative(&path, &project.cwd)),
                "types": records,
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, _, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (name, ty, place) in &rows {
        println!("{:<width$}  {name}: {ty}", place.position());
    }
    ExitCode::SUCCESS
}

/// `file:10` or `file:10-20` split into the file and its lines, and the whole
/// file where no line range is written.
fn split_lines(spec: &str) -> (&str, RangeInclusive<u32>) {
    let whole = (spec, 1..=u32::MAX);
    let Some((file, lines)) = spec.rsplit_once(':') else {
        return whole;
    };
    let range = match lines.split_once('-') {
        Some((first, last)) => first.parse().ok().zip(last.parse().ok()),
        None => lines.parse().ok().map(|line| (line, line)),
    };
    range.map_or(whole, |(first, last)| (file, first..=last))
}

/// Whether `symbol` names something even though the project has no answer for
/// it: a `Core` name the project does not use.
fn known(project: &Project, symbol: &str) -> bool {
    project.workspace.index().declaration(symbol).is_some() || crate::agent::has_card(symbol)
}

/// The failure for a name or a file the project does not have.
fn unknown(error: &str, nearest: &[String], json: bool) -> ExitCode {
    if json {
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "error": error,
                "nearest": nearest,
            }))
        );
        return ExitCode::FAILURE;
    }
    eprintln!("error: {error}");
    if !nearest.is_empty() {
        eprintln!("nearest:");
        for name in nearest {
            eprintln!("  {name}");
        }
    }
    ExitCode::FAILURE
}

/// The word a kind is printed as: `method`, `enum case`.
fn kind(kind: DeclKind) -> &'static str {
    let described = kind.describe();
    described
        .split_once(' ')
        .map_or(described, |(_, noun)| noun)
}

/// The signature `nvs meta` writes for a method or a property, read from a
/// fresh parse of the file that declares it, and `None` for anything else.
fn signature(workspace: &Workspace, declared: &Declaration) -> Option<String> {
    if !matches!(declared.kind, DeclKind::Method | DeclKind::Property) {
        return None;
    }
    let analysed = nvs_lsp::analyse_file(workspace.documents(), &declared.site.path, 0)?;
    let loaded = analysed.loaded.iter().find(|loaded| {
        analysed
            .map
            .file(loaded.id)
            .path()
            .is_some_and(|path| canonical_key(path) == declared.site.path)
    })?;
    member_signature(&loaded.stmts, analysed.map.file(loaded.id), &declared.site)
}

/// The signature of the member whose name `site` covers, in `stmts`.
fn member_signature(stmts: &[Stmt], src: &SourceFile, site: &Site) -> Option<String> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(decl) => decl
            .body
            .as_ref()
            .and_then(|block| member_signature(&block.stmts, src, site)),
        StmtKind::ClassDecl(decl) => in_members(&decl.members, src, site),
        StmtKind::InterfaceDecl(decl) => in_members(&decl.members, src, site),
        StmtKind::EnumDecl(decl) => in_members(&decl.members, src, site),
        _ => None,
    })
}

/// [`member_signature`] over one body, an enum inside it included.
fn in_members(members: &[ClassMember], src: &SourceFile, site: &Site) -> Option<String> {
    let covers = |span: nvs_diagnostics::Span| span.start < site.end && site.start < span.end;
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) if covers(method.name) => {
            Some(crate::meta::method_signature(method, src))
        }
        ClassMemberKind::Property(property) if covers(property.name) => {
            Some(crate::meta::property_signature(property, src))
        }
        ClassMemberKind::Enum(decl) => in_members(&decl.members, src, site),
        _ => None,
    })
}

/// The project root: the directory of the nearest `nvs.toml` at or above
/// `cwd`, and `cwd` itself when there is none.
fn project_root(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|dir| dir.join("nvs.toml").is_file())
        .unwrap_or(cwd)
        .to_owned()
}

/// `path` relative to `cwd`, with `..` for each level above it and `/`
/// between the parts. Both are canonical keys.
fn relative(path: &Path, cwd: &Path) -> String {
    let path_parts: Vec<Component<'_>> = path.components().collect();
    let cwd_parts: Vec<Component<'_>> = cwd.components().collect();
    let shared = path_parts
        .iter()
        .zip(&cwd_parts)
        .take_while(|(a, b)| a == b)
        .count();
    if shared == 0 {
        return path.to_string_lossy().into_owned();
    }
    let mut parts: Vec<String> = vec!["..".to_owned(); cwd_parts.len() - shared];
    parts.extend(
        path_parts[shared..]
            .iter()
            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

/// `text` with every control character but tab written as `\u{..}`.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() && ch != '\t' {
            let _ = write!(out, "\\u{{{:x}}}", u32::from(ch));
        } else {
            out.push(ch);
        }
    }
    out
}

/// [`escaped`] over each line of a doc comment, the line breaks kept.
fn escaped_lines(text: &str) -> String {
    text.split('\n').map(escaped).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_range_is_split_off_the_file_and_a_drive_letter_is_not() {
        assert_eq!(split_lines("src/Cart.nvs:3-9"), ("src/Cart.nvs", 3..=9));
        assert_eq!(split_lines("src/Cart.nvs:4"), ("src/Cart.nvs", 4..=4));
        assert_eq!(split_lines("src/Cart.nvs"), ("src/Cart.nvs", 1..=u32::MAX));
        assert_eq!(
            split_lines(r"C:\src\Cart.nvs"),
            (r"C:\src\Cart.nvs", 1..=u32::MAX)
        );
    }

    #[test]
    fn a_control_character_is_escaped_and_a_tab_is_not() {
        assert_eq!(escaped("a\u{1b}[31m\tb\r"), "a\\u{1b}[31m\tb\\u{d}");
    }
}
