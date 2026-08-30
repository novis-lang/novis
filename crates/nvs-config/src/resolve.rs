//! The configuration tree, resolved: which file is the root, which files it pulls in, and the one
//! ordered stream they flatten to.
//!
//! [ADR 0103] is the whole of this module's specification. § 1 picks the root, § 2 expands
//! `[[include]]`, § 3 flattens the tree to one sequence where a later assignment wins, § 4 splits
//! replacing from appending, and § 5 resolves every relative path against the file it is written in.
//! What is **not** here is § 6's ownership check, which is a property of the bytes' provenance
//! rather than of the tree's shape and lands with [`Files`]'s disk implementation.
//!
//! **Later wins is only acceptable because every override is recorded.** § 3 states that as an
//! obligation and not a permission: without the record it is the silent-shadowing failure
//! [ADR 0064] refused INI for, in a file that grants capabilities. So the merge does not just
//! overwrite — it carries an [`Origin`] for every value it holds and emits an [`Override`] naming
//! both files whenever one replaces another. A caller that drops [`Resolved::overrides`] on the
//! floor has removed a security property, not a log line.
//!
//! **Both of ADR 0064 § 3's refusals stay per file.** Each file is deserialized into
//! [`Config`] on its own before anything is merged, so an unknown key is refused with *that* file's
//! line under it; the merge itself runs over `toml::Table`, where a key set in two files is an
//! override rather than a duplicate. That is why a file is deserialized twice — once to refuse it,
//! once to merge it — and the second pass is free next to the read.
//!
//! Cost: the whole tree's text and one merged table are held for the length of a boot or a reload,
//! then dropped once the snapshot is built. Nothing here runs per request.
//!
//! [ADR 0064]: ../../../docs/adr/0064-configuration-file-format.md
//! [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use nvs_diagnostics::{Diagnostic, SourceId, SourceMap, code};

use crate::tree::Config;

/// How deep `[[include]]` may nest — ADR 0103 § 2.
///
/// It is also the backstop for a cycle built out of symlinks, which [`same_file`]'s lexical
/// comparison cannot see. § 6's canonicalization is what closes that properly, and until it lands
/// this cap is what stops the recursion.
pub const MAX_INCLUDE_DEPTH: usize = 8;

/// Where a configuration file's bytes come from.
///
/// A trait rather than direct `std::fs` calls for two reasons, and neither is testing for its own
/// sake. § 6's ownership check belongs on the *reader*, so a caller that has one and a caller that
/// does not are two implementations of one interface rather than a flag threaded through the
/// resolver. And § 2's directory order is **mandated** — ascending by filename, because § 3 makes
/// order decide the answer and a capability grant settled by `readdir` order is not a design — so
/// [`list`](Files::list) is allowed to return entries in any order and the sort is this module's.
pub trait Files {
    /// The file's text, or a message describing why not.
    ///
    /// # Errors
    ///
    /// Whatever the underlying reader says; the resolver wraps it in `E0605`.
    fn read(&self, path: &Path) -> Result<String, String>;

    /// Every entry directly inside `dir`, in any order and unfiltered.
    ///
    /// # Errors
    ///
    /// Whatever the underlying reader says; the resolver wraps it in `E0605`.
    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String>;

    /// Whether the path is there at all. Only [`Include::optional`](crate::tree::Include::optional)
    /// and § 1's step 2 ask, and both of them treat absence as an answer rather than a failure.
    fn exists(&self, path: &Path) -> bool;
}

/// The real filesystem.
#[derive(Clone, Copy, Debug, Default)]
pub struct Disk;

impl Files for Disk {
    fn read(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|err| err.to_string())
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let entries = std::fs::read_dir(dir).map_err(|err| err.to_string())?;
        let mut paths = Vec::new();
        for entry in entries {
            paths.push(entry.map_err(|err| err.to_string())?.path());
        }
        Ok(paths)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Where one value in the resolved configuration was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    /// The file, absolute.
    pub path: PathBuf,
    /// That file in the caller's [`SourceMap`], so a report can render the line as well as name it.
    pub source: SourceId,
}

/// One assignment replaced by a later one — ADR 0103 § 3's record, carrying **both** origins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Override {
    /// The dotted key, `limits.hard.memory`.
    pub key: String,
    /// Where the value that lost was written.
    pub replaced: Origin,
    /// Where the value in force was written.
    pub winner: Origin,
}

/// What resolving the tree produced.
#[derive(Clone, Debug)]
pub struct Resolved {
    /// The flattened configuration.
    pub config: Config,
    /// Every file the tree reached, in the order § 3 read them. This is what the boot log prints.
    pub files: Vec<PathBuf>,
    /// Every override, in the order they happened. § 9's `nvs config dump --origin` prints these in
    /// full and the boot log summarizes them; dropping them is not an option (see the module doc).
    pub overrides: Vec<Override>,
}

/// What ADR 0103 § 1's four steps selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Roots {
    /// Step 1 or step 2: files to read, in order, each absolute.
    Files(Vec<PathBuf>),
    /// Step 3: the shipped defaults, which are a complete and valid configuration — capabilities
    /// deny-all, `[mode] default = "production"` — and so are [`Config::default`] plus the defaults
    /// each reader applies, not a failure to find anything.
    Defaults,
}

/// ADR 0103 § 1: every `--config` in the order given, else `./nvs.toml`, else the shipped defaults.
///
/// `flags` are resolved against `cwd` because a path given on the command line means what a shell
/// argument means (§ 5). **Any explicit `--config` disables step 2 entirely**, so an operator naming
/// files never gets a surprise merge with whatever is in the working directory — which is why this
/// does not probe `cwd` when `flags` is non-empty even if every one of them turns out to be missing.
/// A `--config` naming a file that does not exist is a hard refusal, and it is [`resolve`]'s: this
/// function reports what was *asked for*, and optionality is a property a file declares about its
/// own includes and never something argv can assert.
pub fn roots(flags: &[PathBuf], cwd: &Path, files: &dyn Files) -> Roots {
    if !flags.is_empty() {
        return Roots::Files(flags.iter().map(|flag| absolute(cwd, flag)).collect());
    }
    let local = cwd.join("nvs.toml");
    if files.exists(&local) {
        // Exactly this directory, never a walk upward: what makes reading the wrong file a
        // question about one path rather than about an ancestry.
        return Roots::Files(vec![normalize(&local)]);
    }
    Roots::Defaults
}

/// Reads the tree and flattens it — ADR 0103 § 3's one ordered stream.
///
/// Each root's own keys land first, then its includes depth-first in list order, then the next
/// root. A later assignment wins and is recorded.
///
/// # Errors
///
/// One [`Diagnostic`]: a file that cannot be read (`E0605`), an include cycle or a nesting deeper
/// than [`MAX_INCLUDE_DEPTH`] (`E0606`), or anything either of ADR 0064 § 3's per-file refusals
/// catches (`E0601`/`E0604`), which arrives already carrying its own file's line.
pub fn resolve(
    roots: &Roots,
    sources: &mut SourceMap,
    files: &dyn Files,
) -> Result<Resolved, Diagnostic> {
    let mut merge = Merge::default();
    let Roots::Files(paths) = roots else {
        return merge.finish();
    };
    for path in paths {
        read_into(&mut merge, path, sources, files, &mut Vec::new(), 0)?;
    }
    merge.finish()
}

/// One file: its own keys into the merge, then its includes, depth-first in list order.
fn read_into(
    merge: &mut Merge,
    path: &Path,
    sources: &mut SourceMap,
    files: &dyn Files,
    chain: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), Diagnostic> {
    if let Some(cycle) = chain.iter().position(|seen| same_file(seen, path)) {
        return Err(cycle_refusal(&chain[cycle..], path));
    }
    if depth > MAX_INCLUDE_DEPTH {
        return Err(Diagnostic::error(
            code::E_INCLUDE_CYCLE,
            format!("`[[include]]` nested deeper than {MAX_INCLUDE_DEPTH} files"),
        )
        .with_note(chain_note(chain, path))
        .with_help("an include tree this deep is nearly always a cycle a symlink hid"));
    }

    let text = files
        .read(path)
        .map_err(|err| unreadable(path, &err, "the configuration reads it"))?;
    let (source, parsed) =
        crate::file::parse::<Config>(sources, &path.display().to_string(), &text);
    // ADR 0064 § 3's two refusals are per file, so this is where they run: the diagnostic carries
    // this file's own line, before anything of it has been merged into the stream.
    let file = parsed?;
    let table: toml::Table = toml::from_str(&text).map_err(|err| {
        // Unreachable in practice — the typed parse above accepted the same bytes — but a
        // configuration reader is not the place to unwrap on that reasoning.
        Diagnostic::error(code::E_BAD_DIRECTIVE, err.message().to_string())
    })?;

    let origin = Origin {
        path: path.to_path_buf(),
        source,
    };
    merge.absorb(&table, &origin);
    merge.files.push(path.to_path_buf());

    // § 5: a relative path resolves against the directory of the file it is written in.
    let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    chain.push(path.to_path_buf());
    for include in &file.include {
        for target in include_targets(include, &base, path, files)? {
            read_into(merge, &target, sources, files, chain, depth + 1)?;
        }
    }
    chain.pop();
    Ok(())
}

/// The files one `[[include]]` entry names, in the order § 2 mandates.
///
/// `path` is one file; `dir` is every `*.toml` **directly** inside, ascending by byte order of
/// filename and not recursing — the sort is here rather than left to the reader because § 3 makes
/// order decide the answer, and a capability grant settled by directory-entry order is not a design.
fn include_targets(
    include: &crate::tree::Include,
    base: &Path,
    written_in: &Path,
    files: &dyn Files,
) -> Result<Vec<PathBuf>, Diagnostic> {
    let optional = include.optional.unwrap_or(false);
    match (&include.path, &include.dir) {
        (Some(one), None) => {
            let target = absolute(base, Path::new(one));
            if !files.exists(&target) {
                if optional {
                    // § 6: absence is the whole of what `optional` covers. The ownership check on
                    // the directory that would hold it is that section's and lands with it.
                    return Ok(Vec::new());
                }
                return Err(
                    unreadable(&target, "no such file", "an `[[include]]` names it")
                        .with_note(format!("included from `{}`", written_in.display()))
                        .with_help("write `optional = true` if the file is allowed to be absent"),
                );
            }
            Ok(vec![target])
        }
        (None, Some(dir)) => {
            let target = absolute(base, Path::new(dir));
            if !files.exists(&target) {
                if optional {
                    return Ok(Vec::new());
                }
                return Err(
                    unreadable(&target, "no such directory", "an `[[include]]` names it")
                        .with_note(format!("included from `{}`", written_in.display())),
                );
            }
            let mut entries: Vec<PathBuf> = files
                .list(&target)
                .map_err(|err| unreadable(&target, &err, "an `[[include]]` names it"))?
                .into_iter()
                .filter(|entry| entry.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            Ok(entries)
        }
        _ => Err(Diagnostic::error(
            code::E_BAD_DIRECTIVE,
            "an `[[include]]` entry carries `path` or `dir`, never both and never neither"
                .to_string(),
        )
        .with_note(format!("written in `{}`", written_in.display()))),
    }
}

/// `E0605`, phrased so the message says what could not be read and the note says why anyone tried.
fn unreadable(path: &Path, why: &str, who: &str) -> Diagnostic {
    Diagnostic::error(
        code::E_UNREADABLE_CONFIG,
        format!("cannot read `{}`: {why}", path.display()),
    )
    .with_note(format!(
        "{who}, and only an absent `optional` include is allowed to be missing"
    ))
}

/// `E0606` for a cycle, naming the whole chain rather than its last file: the cycle is a property of
/// the path, and an operator who is shown only the repeated file has to rediscover how it was reached.
fn cycle_refusal(cycle: &[PathBuf], repeated: &Path) -> Diagnostic {
    Diagnostic::error(
        code::E_INCLUDE_CYCLE,
        format!(
            "`[[include]]` cycle: `{}` includes itself",
            repeated.display()
        ),
    )
    .with_note(chain_note(cycle, repeated))
}

/// The chain as one arrow-joined line.
fn chain_note(chain: &[PathBuf], last: &Path) -> String {
    let mut note = String::from("the chain is ");
    for path in chain {
        note.push_str(&format!("`{}` -> ", path.display()));
    }
    note.push_str(&format!("`{}`", last.display()));
    note
}

/// The merge state: the table so far, where each of its leaves came from, and the record.
#[derive(Default)]
struct Merge {
    table: toml::Table,
    origins: BTreeMap<String, Origin>,
    overrides: Vec<Override>,
    files: Vec<PathBuf>,
}

impl Merge {
    /// Folds one file's table into the stream, recording what it replaced.
    fn absorb(&mut self, table: &toml::Table, origin: &Origin) {
        merge_table(
            &mut self.table,
            table,
            origin,
            "",
            &mut self.origins,
            &mut self.overrides,
        );
    }

    /// Deserializes the merged table into the typed tree.
    fn finish(self) -> Result<Resolved, Diagnostic> {
        let config = toml::Value::Table(self.table)
            .try_into::<Config>()
            // Every file was typed on its own before it was merged, so nothing new can be unknown
            // here; what can still fail is a key one file wrote as a table and another as a value.
            .map_err(|err| Diagnostic::error(code::E_BAD_DIRECTIVE, err.message().to_string()))?;
        Ok(Resolved {
            config,
            files: self.files,
            overrides: self.overrides,
        })
    }
}

/// ADR 0103 §§ 3 and 4 as one walk: recurse into a table, append an array of tables, replace
/// anything else and say so.
///
/// The replace/append split is decided **structurally** — an array whose entries are all tables is
/// an array of tables — rather than from a list of the five `[[block]]` spellings § 4 names. That is
/// the same distinction TOML itself draws, which is § 4's own argument for the split: appending is
/// what two `[[schedule]]` blocks already mean inside one file, so a sixth such block added by a
/// later ADR gets the right behaviour with nothing here to update. An empty array is ambiguous
/// under that rule and does not need to be: appending nothing and replacing with nothing agree.
fn merge_table(
    dest: &mut toml::Table,
    src: &toml::Table,
    origin: &Origin,
    prefix: &str,
    origins: &mut BTreeMap<String, Origin>,
    overrides: &mut Vec<Override>,
) {
    for (key, value) in src {
        let dotted = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match (dest.get_mut(key), value) {
            (Some(toml::Value::Table(into)), toml::Value::Table(from)) => {
                merge_table(into, from, origin, &dotted, origins, overrides);
            }
            (Some(toml::Value::Array(into)), toml::Value::Array(from))
                if is_array_of_tables(into) && is_array_of_tables(from) =>
            {
                into.extend(from.iter().cloned());
                origins.insert(dotted, origin.clone());
            }
            (Some(slot), _) => {
                if let Some(replaced) = origins.insert(dotted.clone(), origin.clone()) {
                    // Only a value that some file actually wrote is an override. A key seeded by
                    // nothing has nothing to report and no origin to name.
                    overrides.push(Override {
                        key: dotted,
                        replaced,
                        winner: origin.clone(),
                    });
                }
                *slot = value.clone();
            }
            (None, _) => {
                dest.insert(key.clone(), value.clone());
                claim(value, origin, &dotted, origins);
            }
        }
    }
}

/// Records `origin` for every leaf inside a value being inserted for the first time, so that a later
/// file replacing one of them has a `replaced` origin to name.
fn claim(
    value: &toml::Value,
    origin: &Origin,
    dotted: &str,
    origins: &mut BTreeMap<String, Origin>,
) {
    match value {
        toml::Value::Table(table) => {
            for (key, nested) in table {
                claim(nested, origin, &format!("{dotted}.{key}"), origins);
            }
        }
        _ => {
            origins.insert(dotted.to_string(), origin.clone());
        }
    }
}

/// Whether every entry is a table, which is what makes the array an array of tables.
fn is_array_of_tables(array: &[toml::Value]) -> bool {
    !array.is_empty() && array.iter().all(toml::Value::is_table)
}

/// Whether two paths name the same file, lexically.
///
/// **Lexical and not `fs::canonicalize`**, which means a cycle assembled out of symlinks is invisible
/// here and is caught by [`MAX_INCLUDE_DEPTH`] instead. § 6's ownership check has to `stat` every
/// file anyway and canonicalization comes with it; doing it here first would `stat` twice and would
/// still leave the depth cap as the backstop.
fn same_file(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// `path` made absolute against `base` when it is relative — ADR 0103 § 5.
fn absolute(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize(path)
    } else {
        normalize(&base.join(path))
    }
}

/// `.` dropped and `..` resolved lexically, with no filesystem access.
///
/// A `..` with nothing to pop is kept rather than discarded: dropping it would turn `../etc/nvs.toml`
/// into `etc/nvs.toml` and read a different file than the operator wrote.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}
