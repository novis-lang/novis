//! `nvs code` — questions about one project, answered from the editor's index.
//!
//! `rule:tooling/nvs-code-answers-about-the-project` is the surface. Every
//! answer is read through [`nvs_lsp::query`] over one [`nvs_lsp::workspace`],
//! the functions the editor's references, implementation and rename requests
//! call, so this command and the editor never disagree. This module decides
//! nothing about the language: it finds the project, turns a [`Site`] into a
//! position a reader can type, escapes what the project wrote, and renders.
//!
//! **Only `rename` and `move` write**, through [`write_out`], the one write
//! path: it writes only inside the project root, and only files whose bytes
//! have not changed since this call first read them.
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
use nvs_lsp::rename::{self, Rewrite};
use nvs_lsp::{DeclKind, Declaration, Site, Workspace};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, Stmt, StmtKind};
use serde_json::{Map, Value, json};

use crate::agent::{SCHEMA_VERSION, pretty};

/// Answer questions about the Novis project you are in: where a name is
/// used, what declares it, and which type a variable has.
///
/// The project is the folder of the nearest `nvs.toml` at or above the
/// working directory. If there is none, it is the working directory. Every
/// `.nvs` file in it is read. Only `rename` changes files.
///
/// Write a name in full, with its namespace, as `'Shop\Cart::addItem'`.
/// Put it in single quotes, so that the shell keeps the backslashes. A
/// file is a path from the working directory.
// `rule:tooling/nvs-code-answers-about-the-project`.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(subcommand)]
    command: Command,
}

/// `nvs code`'s verbs, each a reader of one workspace index
/// (`rule:tooling/nvs-code-answers-about-the-project`).
#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Print where a name is declared, its signature and its doc comment. For
    /// a `Core` name, print the card that `nvs agent show` prints.
    Show {
        /// The name, as `Shop\Cart`, `Shop\Cart::addItem` or
        /// `Shop\Cart::$total`.
        symbol: String,
        /// Print the answer as one JSON document. An unknown name prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print every place a name is written. The declaration comes first. Then
    /// each `use` line and each use follow, in file and line order.
    ///
    /// Each line has the position, the kind (`declared`, `import` or `use`)
    /// and the line of code.
    Refs {
        /// The name, as `Shop\Cart::addItem`. A `Core` name lists the places
        /// where the project uses it.
        symbol: String,
        /// Print the answer as one JSON document. An unknown name prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print the types that extend or implement a type, or the methods that
    /// override a method.
    ///
    /// These lines say `below`. Lines that say `above` show what the type
    /// extends or implements, or which method the method overrides.
    Impls {
        /// The name of a type or a method, as `Shop\Priced`.
        symbol: String,
        /// Print the answer as one JSON document. An unknown name prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print what a file declares, one line each. The members of a class are
    /// indented under it.
    Outline {
        /// The file, as `src/Cart.nvs`.
        file: String,
        /// Print the answer as one JSON document. An unknown file prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print the type of each variable that a file declares with `var`.
    Types {
        /// The file, as `src/Cart.nvs`. Add `:10` for line 10 only, or
        /// `:10-20` for lines 10 to 20.
        file: String,
        /// Print the answer as one JSON document. An unknown file prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Rename a class, a method, a property, a constant, an enum case or a
    /// variable everywhere it is used, and save the changed files.
    ///
    /// This is the same rename that the editor does. A method is renamed in
    /// every class that overrides it. A class that is alone in a file with
    /// its name also gets a new file name.
    ///
    /// The command prints one line for each changed file. If the rename is
    /// not possible, it prints why, changes nothing and fails.
    Rename {
        /// What to rename. Write a name, as `Shop\Cart::addItem`, or a
        /// position, as `src/Cart.nvs:12:9`. A local variable has no name
        /// outside its function, so use its position.
        target: String,
        /// The new name, as `addProduct`. For a variable or a property, the
        /// `$` is optional.
        new_name: String,
        /// Print each change, and save nothing. Each line has the position,
        /// the old text and the new text.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Move a file or a directory, and change every `require` that names it.
    ///
    /// This is the same file move that the editor does. A directory moves
    /// with everything in it. The command does not rename a class or create
    /// a directory.
    ///
    /// The command prints one line for each changed file and one for the
    /// move. If the move is not possible, it prints why, changes nothing and
    /// fails.
    Move {
        /// The file or directory to move, as `src/Cart.nvs` or `src/Shop`.
        from: String,
        /// The new path, as `src/Basket.nvs`. Nothing may exist there yet,
        /// and its directory must exist.
        to: String,
        /// Print each change and the move, and save nothing. Each line has
        /// the position, the old text and the new text.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved and moved.
        #[arg(long)]
        json: bool,
    },
}

impl Args {
    /// Runs the `nvs code` verb clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        match self.command {
            Command::Show { symbol, json } => show(&symbol, json),
            Command::Refs { symbol, json } => refs(&symbol, json),
            Command::Impls { symbol, json } => impls(&symbol, json),
            Command::Outline { file, json } => outline(&file, json),
            Command::Types { file, json } => types(&file, json),
            Command::Rename {
                target,
                new_name,
                dry_run,
                json,
            } => rename(&target, &new_name, dry_run, json),
            Command::Move {
                from,
                to,
                dry_run,
                json,
            } => move_path(&from, &to, dry_run, json),
        }
    }
}

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

    /// The bytes of `path`, read once, which every later position in it is
    /// counted in. A file that cannot be read is an error.
    fn snapshot(&mut self, path: &Path) -> Result<&str, String> {
        if !self.texts.contains_key(path) {
            let text = std::fs::read_to_string(path).map_err(|err| {
                format!(
                    "`{}` cannot be read: {err}",
                    escaped(&relative(path, &self.cwd))
                )
            })?;
            self.texts.insert(path.to_owned(), text);
        }
        Ok(self.text(path))
    }

    /// `path` as a position prints it.
    fn shown(&self, path: &Path) -> String {
        escaped(&relative(path, &self.cwd))
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

/// `nvs code rename`: the editor's rename of a name or of the symbol at a
/// position, written to disk through [`write_out`], or printed with
/// `--dry-run`.
pub(crate) fn rename(target: &str, new_name: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let renamed = if let Some((file, line, column)) = split_position(target) {
        let Some(path) = project.file(file) else {
            let nearest = project.nearest_files(file);
            return unknown(
                &format!("no file `{}` in this project", escaped(file)),
                &nearest,
                json,
            );
        };
        let at = query::Target::Position {
            path: &path,
            line,
            column,
        };
        query::rename(&project.workspace, at, new_name)
    } else {
        if project.workspace.index().declaration(target).is_none() {
            let nearest = project.nearest_names(target);
            return unknown(
                &format!("no name `{}` in this project", escaped(target)),
                &nearest,
                json,
            );
        }
        query::rename(&project.workspace, query::Target::Name(target), new_name)
    };
    match renamed {
        Ok(renamed) => {
            let moves: Vec<(PathBuf, PathBuf)> = renamed.file.into_iter().collect();
            write_out(&mut project, &renamed.rewrites, &moves, dry_run, json)
        }
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code move`: the editor's file move of a file or a directory, with
/// every `require` edit [`rename::moved`] gives, written to disk through
/// [`write_out`], or printed with `--dry-run`.
///
/// Both paths are judged before anything is computed: `from` exists and is
/// inside the project root, and `to` is inside it too, does not exist, is not
/// inside `from`, and has a directory that exists. A directory is not created
/// and a type is not renamed, as the editor's move does neither (the rule's
/// Declined list).
pub(crate) fn move_path(from: &str, to: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let moves = match judged_move(&project, from, to) {
        Ok(moving) => vec![moving],
        Err((message, nearest)) => return unknown(&message, &nearest, json),
    };
    let workspace = &project.workspace;
    let rewrites = rename::moved(workspace.documents(), workspace.index(), &moves);
    write_out(&mut project, &rewrites, &moves, dry_run, json)
}

/// `from` and `to` as the keys a move is made under, or the message and the
/// nearest files of the first check that fails.
fn judged_move(
    project: &Project,
    from: &str,
    to: &str,
) -> Result<(PathBuf, PathBuf), (String, Vec<String>)> {
    let root = project.workspace.root();
    let refused = |message: String| Err((message, Vec::new()));
    let source = canonical_key(&project.cwd.join(from));
    if std::fs::symlink_metadata(&source).is_err() {
        return Err((
            format!("no file or directory `{}` in this project", escaped(from)),
            project.nearest_files(from),
        ));
    }
    if !inside(root, &source) {
        return refused(format!(
            "`{}` is outside the project, so nothing was changed.",
            escaped(from)
        ));
    }
    let written = project.cwd.join(to);
    let (Some(parent), Some(name)) = (written.parent(), written.file_name()) else {
        return refused(format!(
            "`{}` is not a path that a file can be moved to.",
            escaped(to)
        ));
    };
    if !parent.is_dir() {
        return refused(format!(
            "The directory `{}` does not exist. Create it, then run the command again.",
            project.shown(parent)
        ));
    }
    let target = canonical_key(parent).join(name);
    if !inside(root, &target) {
        return refused(format!(
            "`{}` is outside the project, so nothing was changed.",
            escaped(to)
        ));
    }
    if std::fs::symlink_metadata(&target).is_ok() {
        return refused(format!(
            "`{}` already exists, so nothing was changed.",
            project.shown(&target)
        ));
    }
    if target.starts_with(&source) {
        return refused(format!(
            "`{}` is inside `{}`. A directory cannot be moved into itself.",
            project.shown(&target),
            project.shown(&source)
        ));
    }
    Ok((source, target))
}

/// `path:line:column` split into the file, the line and the column, and
/// `None` for anything else, a name such as `Shop\Cart::MAX` included.
fn split_position(spec: &str) -> Option<(&str, u32, u32)> {
    let (rest, column) = spec.rsplit_once(':')?;
    let (file, line) = rest.rsplit_once(':')?;
    let line: u32 = line.parse().ok()?;
    let column: u32 = column.parse().ok()?;
    (!file.is_empty() && line > 0 && column > 0).then_some((file, line, column))
}

/// One file the write path changes: the bytes the edits were computed
/// against, the bytes it writes, and how many edits make the difference.
struct Change {
    path: PathBuf,
    before: String,
    after: String,
    edits: usize,
}

/// The one write path of `nvs code`, which `rename` and `move` share.
///
/// In this order: every edit is applied in memory, and a file that cannot be
/// read or an edit that does not fit its file writes nothing. Every path and
/// every move's target is checked to be inside the project root. Each file is
/// read again, and one whose bytes differ from the bytes the edits were
/// computed against writes nothing. Then each file is written, and each move
/// is made after all of them. `--dry-run` stops before the second read.
///
/// The bytes the edits were computed against are each file's first read in
/// this call, which is right after the analysis. A crash between two writes
/// leaves the project half changed, and `nvs check` then names every site
/// that no longer resolves. A journal is not kept (the rule's Declined list).
fn write_out(
    project: &mut Project,
    rewrites: &[Rewrite],
    moves: &[(PathBuf, PathBuf)],
    dry_run: bool,
    json: bool,
) -> ExitCode {
    let mut rewrites: Vec<&Rewrite> = rewrites.iter().collect();
    rewrites.sort();
    rewrites.dedup();
    let changes = match planned(project, &rewrites, moves) {
        Ok(changes) => changes,
        Err(message) => return unknown(&message, &[], json),
    };
    let edits: Vec<(Place, String, String)> = rewrites
        .iter()
        .map(|rewrite| {
            let place = project.place(&rewrite.site);
            let old = project
                .text(&rewrite.site.path)
                .get(rewrite.site.start as usize..rewrite.site.end as usize)
                .map(escaped)
                .unwrap_or_default();
            (place, old, escaped(&rewrite.text))
        })
        .collect();
    let shown_moves: Vec<(String, String)> = moves
        .iter()
        .map(|(from, to)| (project.shown(from), project.shown(to)))
        .collect();
    if !dry_run && let Err(message) = committed(project, &changes, moves) {
        return unknown(&message, &[], json);
    }
    if json {
        let edits: Vec<Value> = edits
            .iter()
            .map(|(place, old, new)| {
                let mut record = place.json();
                record.insert("old".into(), Value::from(old.clone()));
                record.insert("new".into(), Value::from(new.clone()));
                Value::Object(record)
            })
            .collect();
        let moved: Vec<Value> = shown_moves
            .iter()
            .map(|(from, to)| json!({ "from": from, "to": to }))
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "edits": edits,
                "moved": moved,
                "written": !dry_run,
            }))
        );
        return ExitCode::SUCCESS;
    }
    if dry_run {
        let width = edits
            .iter()
            .map(|(place, _, _)| place.position().len())
            .max()
            .unwrap_or(0);
        for (place, old, new) in &edits {
            println!("{:<width$}  {old} -> {new}", place.position());
        }
        for (from, to) in &shown_moves {
            println!("move {from} -> {to}");
        }
        return ExitCode::SUCCESS;
    }
    for change in &changes {
        let noun = if change.edits == 1 { "edit" } else { "edits" };
        println!("{}  {} {noun}", project.shown(&change.path), change.edits);
    }
    for (from, to) in &shown_moves {
        println!("moved {from} -> {to}");
    }
    ExitCode::SUCCESS
}

/// Each changed file's bytes before and after `rewrites`, which are sorted,
/// once every path and every move is judged to be inside the project root.
fn planned(
    project: &mut Project,
    rewrites: &[&Rewrite],
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<Change>, String> {
    let root = project.workspace.root().to_owned();
    let outside = |path: &Path| {
        format!(
            "`{}` is outside the project, so nothing was changed.",
            escaped(&relative(path, &root))
        )
    };
    let judged = rewrites
        .iter()
        .map(|rewrite| rewrite.site.path.as_path())
        .chain(
            moves
                .iter()
                .flat_map(|(from, to)| [from.as_path(), to.as_path()]),
        );
    for path in judged {
        if !inside(&root, path) {
            return Err(outside(path));
        }
    }
    let mut changes: Vec<Change> = Vec::new();
    for group in rewrites.chunk_by(|a, b| a.site.path == b.site.path) {
        let path = &group[0].site.path;
        let before = project.snapshot(path)?.to_owned();
        let after = applied(&before, group).ok_or_else(|| {
            format!(
                "An edit does not fit `{}`, so nothing was changed.",
                project.shown(path)
            )
        })?;
        changes.push(Change {
            path: path.clone(),
            before,
            after,
            edits: group.len(),
        });
    }
    Ok(changes)
}

/// Whether `path` is `root` or under it, with no `.` or `..` part that could
/// lead out of it. Every path the index holds is a canonical key, so this
/// fails only on a bug, and it fails closed.
fn inside(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
}

/// `text` with each of `rewrites` applied, which are in source order, and
/// `None` when two overlap or one is not on a character boundary.
fn applied(text: &str, rewrites: &[&Rewrite]) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for rewrite in rewrites {
        out.push_str(text.get(at..rewrite.site.start as usize)?);
        out.push_str(&rewrite.text);
        at = rewrite.site.end as usize;
    }
    out.push_str(text.get(at..)?);
    Some(out)
}

/// The variable a test sets to a file that is changed after the edits are
/// computed and before they are written. Only a debug build reads it, so a
/// release binary has no such hook.
#[cfg(debug_assertions)]
const CHANGE_BEFORE_WRITE: &str = "NVS_TEST_CODE_CHANGE_BEFORE_WRITE";

/// Writes `changes` once no file differs from the bytes they were computed
/// against, and then makes `moves`.
fn committed(
    project: &Project,
    changes: &[Change],
    moves: &[(PathBuf, PathBuf)],
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os(CHANGE_BEFORE_WRITE) {
        let mut bytes = std::fs::read(&path).unwrap_or_default();
        bytes.push(b'\n');
        let _ = std::fs::write(&path, bytes);
    }
    for change in changes {
        let now = std::fs::read(&change.path).unwrap_or_default();
        if now != change.before.as_bytes() {
            return Err(format!(
                "`{}` changed while the command ran, so nothing was changed. Run the command again.",
                project.shown(&change.path)
            ));
        }
    }
    for (_, to) in moves {
        if to.exists() {
            return Err(format!(
                "`{}` already exists, so nothing was changed.",
                project.shown(to)
            ));
        }
    }
    let mut done: Vec<String> = Vec::new();
    let partly = |what: String, done: &[String]| {
        if done.is_empty() {
            what
        } else {
            format!(
                "{what} These files were already changed: {}.",
                done.join(", ")
            )
        }
    };
    for change in changes {
        if let Err(err) = std::fs::write(&change.path, &change.after) {
            let what = format!(
                "`{}` cannot be written: {err}.",
                project.shown(&change.path)
            );
            return Err(partly(what, &done));
        }
        done.push(project.shown(&change.path));
    }
    for (from, to) in moves {
        if let Err(err) = std::fs::rename(from, to) {
            let what = format!(
                "`{}` cannot be moved to `{}`: {err}.",
                project.shown(from),
                project.shown(to)
            );
            return Err(partly(what, &done));
        }
    }
    Ok(())
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
