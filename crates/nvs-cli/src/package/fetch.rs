//! `nvs pkg fetch` and `nvs pkg vendor`: the resolved graph written under `vendor/`, with
//! `vendor/packages.nvs` and one grant line per package
//! (`rule:packaging/a-fetched-package-is-an-autoload-line`,
//! `rule:security/package-authority-is-granted-one-line-at-a-time`).
//!
//! **The command resolves, checks and locks**, then writes. It reads the application's
//! `package.toml`, found by walking up from the working directory, resolves its graph
//! ([`resolve::resolve`]), checks the result against `package.lock` ([`Lock::check`]), and writes
//! the lock again when the graph changed. With `--locked` a package, version or entry the lock
//! does not match is an error, and the lock is never written.
//!
//! **Each package is unpacked into `vendor/<vendor>/<name>/`** ([`unpack`]), its slug as a path,
//! so its folder does not move when its version does.
//! Every entry of the archive is a regular file at a path [`pack::check_path`] accepts, within
//! [`pack::MAX_FILES`] and [`pack::MAX_BYTES`], so a hostile archive writes nothing outside its
//! folder. The folder is written beside its place and renamed into it, so a failed fetch leaves
//! the old folder whole. `fetch` keeps a folder whose files already pack to the locked digest, so
//! a second fetch reads no archive. `vendor` writes every folder again from its archive, for a
//! tree a person changed by hand. A folder
//! of a package the old lock had and the graph no longer needs is removed; nothing else under
//! `vendor/` is touched, because a person may keep hand-vendored code there.
//!
//! **`vendor/packages.nvs` is generated from the lock**: one `autoload` line per package, in slug
//! order, so the same lock writes the same bytes and a change to the graph is a line in a diff.
//!
//! **`vendor/package-files.toml` lists each package's own files** ([`package_files`]): the paths
//! its archive carries ([`pack::file_set`]) and the paths it downloads, relative to its folder.
//! `nvs_config::package` reads it, and code in a package's namespace may read a file it lists
//! with no grant (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
//!
//! **One grant line per package is added to `[grants]`** ([`with_grants`]): `'Acme\Http\*' = []`,
//! the subtree form, because a package's code is its namespace and every namespace below it
//! (`rule:security/grants-are-keyed-on-a-namespace`). The line grants nothing, and its comment
//! names what the package requests. A package that already has a key naming its namespace, in
//! either form, gets no line, so a line a person edited is never rewritten. The lines are added
//! to the file's text, so every comment and every other line stays as it was written.
//!
//! **Each `[[download]]` file is placed at its path in its package's folder** ([`download::place`]),
//! from the SHA-256 cache, downloaded first when the cache does not have it
//! (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`). A folder `vendor` writes
//! again gets its files again from the cache.
//!
//! **No code of a package runs** (`rule:packaging/nothing-runs-before-the-program`): the command
//! reads archives and writes files, and compiles nothing.
//!
//! Cost: one archive in memory at a time, at most [`pack::MAX_ARCHIVE_BYTES`], beside what
//! resolution holds. `nvs pkg add` and `update` read each package's `[[download]]` entries for
//! their report before anything is written ([`files_before`]), so they keep the archive of every
//! package whose folder does not already hold it until it is written, and no archive is read
//! twice. Nothing here runs on the request path.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{Renderer, SourceMap};

use super::download::{self, Files};
use super::git::{self, Outbound};
use super::index::{Capabilities, Fault, fault};
use super::lock::{self, Lock};
use super::manifest::{self, Download, Manifest, Name};
use super::pack;
use super::resolve::{self, Graph, Registries};

/// The folder the packages are written into, beside the application's `package.toml`.
pub(crate) const VENDOR: &str = "vendor";

/// The generated file a program `require`s, under [`VENDOR`].
pub(crate) const AUTOLOADS: &str = "packages.nvs";

/// Which command runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// `nvs pkg fetch`: a folder whose files pack to the locked digest is kept.
    Fetch,
    /// `nvs pkg vendor`: every folder is written again from its archive.
    Vendor,
}

/// `nvs pkg fetch` or `nvs pkg vendor`, in the working directory.
pub(crate) fn run(mode: Mode, locked: bool) -> ExitCode {
    exit(fetch(mode, locked))
}

/// The exit code of a package command that ended with `result`, with its fault printed.
pub(crate) fn exit(result: Result<(), Failed>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failed::Fault(fault)) => {
            eprintln!("error: {fault}");
            ExitCode::FAILURE
        }
        Err(Failed::Reported(code)) => code,
    }
}

/// Why the command stopped.
pub(crate) enum Failed {
    /// A fault the command prints.
    Fault(Fault),
    /// A failure already printed, such as a diagnostic.
    Reported(ExitCode),
}

impl From<Fault> for Failed {
    fn from(fault: Fault) -> Self {
        Self::Fault(fault)
    }
}

/// The application a package command works on: its `package.toml`, read, and its lock.
pub(crate) struct App {
    /// The `package.toml`'s path.
    pub(crate) path: PathBuf,
    /// The folder it is in.
    pub(crate) dir: PathBuf,
    pub(crate) manifest: Manifest,
    /// `package.lock`, if the folder has one.
    pub(crate) old: Option<Lock>,
    /// The addresses an `https:` git dependency or a download may be fetched from.
    pub(crate) outbound: Outbound,
}

impl App {
    /// The application whose `package.toml` is the nearest one to the working directory.
    ///
    /// # Errors
    ///
    /// No `package.toml`, one the reader refuses, which is printed, an unreadable configuration
    /// tree, and a lock that cannot be read.
    pub(crate) fn open() -> Result<Self, Failed> {
        let cwd = std::env::current_dir()
            .map_err(|err| fault(format!("cannot read the working directory: {err}")))?;
        Self::from(&cwd)
    }

    /// The application whose `package.toml` is the nearest one to `cwd`.
    ///
    /// # Errors
    ///
    /// As [`App::open`].
    fn from(cwd: &Path) -> Result<Self, Failed> {
        let Some(path) = manifest::find(cwd) else {
            return Err(fault(format!(
                "there is no `{}` in `{}` or a folder above it. Write one with a \
                 `[dependencies]` table.",
                nvs_config::grants::FILE,
                cwd.display()
            ))
            .into());
        };
        let mut sources = SourceMap::new();
        let manifest = manifest::read(&path, &mut sources).map_err(|d| reported(&d, &sources))?;
        let dir = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        let outbound =
            Outbound::new(crate::config::package_network(None).map_err(Failed::Reported)?);
        let old = Lock::read(&dir.join(lock::FILE))?;
        Ok(Self {
            path,
            dir,
            manifest,
            old,
            outbound,
        })
    }

    /// The registries `manifest` names, reaching what the tree allows, with each git dependency
    /// written out under the data folder.
    pub(crate) fn registries<'a>(&self, manifest: &'a Manifest) -> Registries<'a> {
        let data = nvs_config::data::current();
        let keep = data.map(nvs_config::data::Folder::registry);
        let registries = Registries::new(manifest, keep.as_deref()).reaching(self.outbound.clone());
        match data {
            // A git dependency is written out under the data folder, beside every other file
            // Novis keeps, and removed once it is packed.
            Some(data) => registries.clones_in(&data.tmp(), git::LIMITS),
            None => registries,
        }
    }

    /// `text` parsed as this application's `package.toml`, which is not written yet.
    ///
    /// # Errors
    ///
    /// A refusal of the reader, which is printed.
    pub(crate) fn reread(&self, text: &str) -> Result<Manifest, Failed> {
        let mut sources = SourceMap::new();
        manifest::parse(&mut sources, &self.path, text).map_err(|d| reported(&d, &sources))
    }
}

/// `diagnostic` printed on standard error, as a failure already reported.
fn reported(diagnostic: &nvs_diagnostics::Diagnostic, sources: &SourceMap) -> Failed {
    // Standard error that cannot be written to has nobody to tell.
    let _ = Renderer::new().render(diagnostic, sources, &mut io::stderr().lock());
    Failed::Reported(ExitCode::FAILURE)
}

/// [`run`], returning what stopped it.
fn fetch(mode: Mode, locked: bool) -> Result<(), Failed> {
    let app = App::open()?;
    if locked && app.old.is_none() {
        return Err(fault(format!(
            "`--locked` uses the versions in {}, and `{}` has none. Run the command without \
             `--locked` to write it.",
            lock::FILE,
            app.dir.display()
        ))
        .into());
    }
    let mut registries = app.registries(&app.manifest);
    let graph = resolve::resolve(&app.manifest, &mut registries, app.old.as_ref())?;
    if let Some(old) = &app.old {
        old.check(&graph, locked)?;
    }
    for crossing in &graph.crossings {
        eprintln!("note: {crossing}");
    }
    land(&app, &app.manifest, mode, locked, &graph, &mut registries)
}

/// `graph`, resolved from `manifest` and checked against the lock, written: every package under
/// `vendor/`, `vendor/packages.nvs`, the lock unless `locked`, and the grant lines.
///
/// # Errors
///
/// An archive that cannot be read or written, and a file that cannot be written.
pub(crate) fn land(
    app: &App,
    manifest: &Manifest,
    mode: Mode,
    locked: bool,
    graph: &Graph,
    registries: &mut Registries<'_>,
) -> Result<(), Failed> {
    let vendor = app.dir.join(VENDOR);
    let written = write_packages(mode, graph, registries, &vendor)?;
    if let Some(old) = &app.old {
        remove_unneeded(old, graph, &vendor)?;
    }
    let (files, sets) = files_in(graph, &vendor)?;
    let new = Lock::of(graph).with_hosts(&files);
    write_if_changed(&vendor.join(AUTOLOADS), &autoloads(&new))?;
    write_if_changed(
        &vendor.join(nvs_config::package::FILES),
        &package_files(&sets)?,
    )?;
    if !locked && app.old.as_ref() != Some(&new) {
        new.write(&app.dir.join(lock::FILE))?;
    }
    let added = add_grants(&app.path, manifest, &new)?;
    for line in &added {
        println!("added to [grants]: {line}");
    }
    // Last, so a file whose URLs all fail leaves the lock written, and `nvs pkg provide` finds
    // its entry there.
    place_downloads(graph, &files, &vendor, &app.outbound)?;
    println!(
        "{} package(s), {written} written to {VENDOR}/",
        graph.packages.len()
    );
    Ok(())
}

/// Every package of `graph` written under `vendor`, and how many folders were written.
fn write_packages(
    mode: Mode,
    graph: &Graph,
    registries: &mut Registries<'_>,
    vendor: &Path,
) -> Result<usize, Fault> {
    let mut written = 0;
    for selected in &graph.packages {
        let (name, release) = (&selected.name, &selected.release);
        let dir = vendor.join(name.slug());
        if mode == Mode::Fetch && holds(&dir, &release.digest) {
            continue;
        }
        let archive = registries.archive(selected)?;
        unpack(archive, &dir).map_err(|err| {
            fault(format!(
                "cannot write `{name}` {} to `{}`: {}",
                release.version,
                dir.display(),
                err.0
            ))
        })?;
        registries.release(name);
        written += 1;
    }
    Ok(written)
}

/// Each package's name and the paths of its own files, which its code may read with no grant:
/// what its archive carries and what it downloads, relative to its folder, sorted.
type Sets = BTreeMap<String, Vec<String>>;

/// The `[[download]]` entries of every package in `graph`, read from its folder under `vendor`,
/// each package's entries checked by [`download::check_form`], and each package's [`Sets`] line.
///
/// # Errors
///
/// A folder's `package.toml` that cannot be read, a folder [`pack::file_set`] refuses, and an
/// entry the form check refuses.
fn files_in(graph: &Graph, vendor: &Path) -> Result<(Files, Sets), Fault> {
    let mut files = Files::new();
    let mut sets = Sets::new();
    for selected in &graph.packages {
        let dir = vendor.join(selected.name.slug());
        let path = dir.join(nvs_config::grants::FILE);
        let manifest = manifest::read(&path, &mut SourceMap::new())
            .map_err(|diagnostic| fault(format!("`{}`: {}", path.display(), diagnostic.message)))?;
        let mut set: Vec<String> = pack::file_set(&dir, &manifest)
            .map_err(|why| fault(format!("`{}`: {why}", dir.display())))?
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        set.extend(manifest.downloads.iter().map(|entry| entry.path.clone()));
        set.sort();
        set.dedup();
        sets.insert(selected.name.namespace().to_owned(), set);
        add_files(&mut files, &selected.name, manifest.downloads)?;
    }
    Ok((files, sets))
}

/// The text of `vendor/package-files.toml`: one line per package of `sets`, which
/// `nvs_config::package` reads onto each package.
///
/// # Errors
///
/// A set TOML cannot write, which no path the archive's rules allow is.
fn package_files(sets: &Sets) -> Result<String, Fault> {
    let lines = toml::to_string(sets)
        .map_err(|err| fault(format!("cannot write the package files: {err}")))?;
    Ok(format!(
        "# Generated by `nvs pkg` from {}. Do not edit.\n\
         # The files each package's own code may read with no grant.\n{lines}",
        lock::FILE
    ))
}

/// The `[[download]]` entries of every package in `graph` before anything is written, for the
/// report: read from a folder under `vendor` that packs to the package's digest, and from the
/// package's archive otherwise. [`Registries`] keeps each archive it reads, so [`land`] does not
/// read it again.
///
/// # Errors
///
/// An archive that cannot be read, one with no `package.toml` this client reads, and an entry
/// the form check refuses.
pub(crate) fn files_before(
    graph: &Graph,
    registries: &mut Registries<'_>,
    vendor: &Path,
) -> Result<Files, Fault> {
    let mut files = Files::new();
    for selected in &graph.packages {
        let name = &selected.name;
        let dir = vendor.join(name.slug());
        let path = dir.join(nvs_config::grants::FILE);
        let mut sources = SourceMap::new();
        let read = if holds(&dir, &selected.release.digest) {
            manifest::read(&path, &mut sources)
        } else {
            let text = manifest_text(registries.archive(selected)?)
                .map_err(|why| fault(format!("the archive of `{name}`: {why}")))?;
            manifest::parse(&mut sources, &path, &text)
        };
        let manifest = read
            .map_err(|diagnostic| fault(format!("`{}`: {}", path.display(), diagnostic.message)))?;
        add_files(&mut files, name, manifest.downloads)?;
    }
    Ok(files)
}

/// `downloads`, checked by [`download::check_form`], kept in `files` under `name` when there are
/// any.
fn add_files(files: &mut Files, name: &Name, downloads: Vec<Download>) -> Result<(), Fault> {
    if downloads.is_empty() {
        return Ok(());
    }
    download::check_form(&downloads).map_err(|why| fault(format!("`{name}`: {why}")))?;
    files.insert(name.slug(), downloads);
    Ok(())
}

/// The text of the `package.toml` in `archive`.
fn manifest_text(archive: &[u8]) -> Result<String, String> {
    let mut tar = tar::Archive::new(archive);
    for entry in tar.entries().map_err(|err| err.to_string())? {
        let mut entry = entry.map_err(|err| err.to_string())?;
        if entry.path_bytes().as_ref() != nvs_config::grants::FILE.as_bytes() {
            continue;
        }
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|err| format!("its `{}`: {err}", nvs_config::grants::FILE))?;
        return Ok(text);
    }
    Err(format!("it has no `{}`", nvs_config::grants::FILE))
}

/// Each file of `files` placed at its path in its package's folder under `vendor`
/// ([`download::place`]). The cache is opened only when a package has a file.
fn place_downloads(
    graph: &Graph,
    files: &Files,
    vendor: &Path,
    outbound: &Outbound,
) -> Result<(), Fault> {
    if files.is_empty() {
        return Ok(());
    }
    let store = download::Store::current()?;
    for selected in &graph.packages {
        let slug = selected.name.slug();
        let Some(entries) = files.get(&slug) else {
            continue;
        };
        let dir = vendor.join(&slug);
        for entry in entries {
            download::place(&store, &selected.name, entry, &dir, outbound)?;
        }
    }
    Ok(())
}

/// Whether the package folder `dir` packs to `digest`, so its files are the locked ones.
fn holds(dir: &Path, digest: &blake3::Hash) -> bool {
    let path = dir.join(nvs_config::grants::FILE);
    if !path.is_file() {
        return false;
    }
    let Ok(manifest) = manifest::read(&path, &mut SourceMap::new()) else {
        return false;
    };
    pack::file_set(dir, &manifest)
        .and_then(|entries| pack::write(&entries, &mut io::sink()))
        .is_ok_and(|got| got == *digest)
}

/// `archive` unpacked into the folder `dir`, replacing what was there. The files are written
/// into a folder beside `dir` first, which is removed on every way out.
///
/// # Errors
///
/// An entry that is not a regular file, a path [`pack::check_path`] refuses or with an empty
/// part, an archive over [`pack::MAX_FILES`] or [`pack::MAX_BYTES`], and a failed write.
pub(crate) fn unpack(archive: &[u8], dir: &Path) -> Result<(), Fault> {
    let mut partial = dir.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    let failed = |err: io::Error| fault(err.to_string());
    if partial.exists() {
        fs::remove_dir_all(&partial).map_err(failed)?;
    }
    fs::create_dir_all(&partial).map_err(failed)?;
    let unpacked = unpack_into(archive, &partial).and_then(|()| {
        if dir.exists() {
            fs::remove_dir_all(dir).map_err(failed)?;
        }
        fs::rename(&partial, dir).map_err(failed)
    });
    if unpacked.is_err() {
        // The fault below is the one to report; a folder that will not go is left for the next run.
        let _ = fs::remove_dir_all(&partial);
    }
    unpacked
}

/// [`unpack`]'s walk over the archive's entries, writing each file under `into`.
fn unpack_into(archive: &[u8], into: &Path) -> Result<(), Fault> {
    let bad = |detail: String| fault(format!("the archive is not valid: {detail}"));
    let mut tar = tar::Archive::new(archive);
    let (mut files, mut bytes) = (0usize, 0u64);
    for entry in tar.entries().map_err(|err| bad(err.to_string()))? {
        let mut entry = entry.map_err(|err| bad(err.to_string()))?;
        let path_bytes = entry.path_bytes().into_owned();
        let path =
            String::from_utf8(path_bytes).map_err(|_| bad("a path is not UTF-8".to_owned()))?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            return Err(bad(format!("`{path}` is not a regular file")));
        }
        if path.split('/').any(str::is_empty) {
            return Err(bad(format!("`{path}` has an empty part")));
        }
        pack::check_path(&path).map_err(|refusal| bad(refusal.to_string()))?;
        let size = entry.size();
        files += 1;
        bytes += size;
        if files > pack::MAX_FILES || bytes > pack::MAX_BYTES {
            return Err(bad(format!(
                "it has more than {} files or {} bytes",
                pack::MAX_FILES,
                pack::MAX_BYTES
            )));
        }
        let target = into.join(&path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| fault(err.to_string()))?;
        }
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|err| bad(format!("`{path}`: {err}")))?;
        io::copy(&mut (&mut entry).take(size), &mut out).map_err(|err| fault(err.to_string()))?;
    }
    Ok(())
}

/// The folder of each package `old` locked and `graph` no longer needs, removed, and its vendor's
/// folder too once it is empty.
fn remove_unneeded(old: &Lock, graph: &Graph, vendor: &Path) -> Result<(), Fault> {
    for entry in &old.packages {
        let slug = entry.name.slug();
        if graph.packages.iter().any(|s| s.name.slug() == slug) {
            continue;
        }
        let dir = vendor.join(&slug);
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .map_err(|err| fault(format!("cannot remove `{}`: {err}", dir.display())))?;
        }
        if let Some(parent) = dir.parent() {
            // A vendor's folder that still has another package in it stays.
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
}

/// `vendor/packages.nvs`'s text for `lock`: one `autoload` line per package, in slug order.
pub(crate) fn autoloads(lock: &Lock) -> String {
    let mut text = format!(
        "<?nvs\n// Generated by `nvs pkg` from {}. Do not edit.\n",
        lock::FILE
    );
    for entry in &lock.packages {
        text.push_str(&format!(
            "autoload '{}' from './{}/src';\n",
            entry.name,
            entry.name.slug()
        ));
    }
    text
}

/// `text` written to `path` unless the file already has it, so an unchanged file keeps its time.
fn write_if_changed(path: &Path, text: &str) -> Result<(), Fault> {
    if fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    let failed = |err: io::Error| fault(format!("cannot write `{}`: {err}", path.display()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(failed)?;
    }
    fs::write(path, text).map_err(failed)
}

/// One grant line added to the `package.toml` at `path` for each package of `lock` that has no
/// key naming its namespace, and the lines added.
fn add_grants(path: &Path, manifest: &Manifest, lock: &Lock) -> Result<Vec<String>, Fault> {
    let lines: Vec<String> = lock
        .packages
        .iter()
        .filter(|entry| {
            !manifest
                .grants
                .lines()
                .iter()
                .any(|grant| grant.namespace() == entry.name.namespace())
        })
        .map(|entry| grant_line(entry.name.namespace(), &entry.capabilities))
        .collect();
    if lines.is_empty() {
        return Ok(lines);
    }
    let text = fs::read_to_string(path)
        .map_err(|err| fault(format!("cannot read `{}`: {err}", path.display())))?;
    replace_file(path, &with_grants(&text, &lines))?;
    Ok(lines)
}

/// The file at `path` replaced by `text`, written beside it first and renamed into place, so a
/// failed write leaves the old file whole.
pub(crate) fn replace_file(path: &Path, text: &str) -> Result<(), Fault> {
    let failed = |err: io::Error| fault(format!("cannot write `{}`: {err}", path.display()));
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    fs::write(&partial, text).map_err(failed)?;
    fs::rename(&partial, path).map_err(failed)
}

/// The grant line for `namespace`, granting nothing, with what the package requests as its
/// comment.
fn grant_line(namespace: &str, requests: &Capabilities) -> String {
    let mut line = format!("'{namespace}\\*' = []");
    let mut asks = Vec::new();
    if !requests.required.is_empty() {
        asks.push(format!("required: {}", requests.required.join(", ")));
    }
    if !requests.optional.is_empty() {
        asks.push(format!("optional: {}", requests.optional.join(", ")));
    }
    if !asks.is_empty() {
        line.push_str(&format!("  # {}", asks.join("; ")));
    }
    line
}

/// `text` with `lines` added at the end of its `[grants]` table, or in a new `[grants]` table at
/// the end of the file when it has none. Every other line is kept as written.
pub(crate) fn with_grants(text: &str, lines: &[String]) -> String {
    with_lines(text, "grants", lines)
}

/// Whether `line` starts a table: `[name]` or `[[name]]`.
fn is_header(line: &str) -> bool {
    line.trim()
        .strip_prefix('[')
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '['))
}

/// The lines of `table` in `lines`: the index of its header and the index just past its end.
pub(crate) fn table_span(lines: &[&str], table: &str) -> Option<(usize, usize)> {
    let at = lines.iter().position(|line| {
        line.trim()
            .strip_prefix(&format!("[{table}]"))
            .is_some_and(|rest| rest.trim().is_empty() || rest.trim().starts_with('#'))
    })?;
    let end = lines[at + 1..]
        .iter()
        .position(|line| is_header(line))
        .map_or(lines.len(), |next| at + 1 + next);
    Some((at, end))
}

/// `text` with its lines replaced by `out`, in the newline `text` uses.
pub(crate) fn joined(text: &str, out: &[&str]) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut written = out.join(newline);
    written.push_str(newline);
    written
}

/// `text` with `lines` added at the end of its `[<table>]`, or in a new table at the end of the
/// file when it has none. Every other line is kept as written.
pub(crate) fn with_lines(text: &str, table: &str, lines: &[String]) -> String {
    let mut out: Vec<&str> = text.lines().collect();
    let added: Vec<&str> = lines.iter().map(String::as_str).collect();
    let header = format!("[{table}]");
    match table_span(&out, table) {
        Some((at, end)) => {
            // The table's last line that is not blank, so the blank lines before the next table
            // stay after the added lines.
            let last = (at + 1..end)
                .rev()
                .find(|&i| !out[i].trim().is_empty())
                .map_or(at + 1, |i| i + 1);
            out.splice(last..last, added);
        }
        None => {
            if out.last().is_some_and(|line| !line.trim().is_empty()) {
                out.push("");
            }
            out.push(&header);
            out.extend(added);
        }
    }
    joined(text, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grant_line_goes_at_the_end_of_the_grants_table() {
        let text = "[dependencies]\n'Acme\\Http' = \"1.0.0\"\n\n[grants]\n# mine\n'App' = []\n\n[registries]\nAcme = { url = \"file:r\" }\n";
        let got = with_grants(text, &["'Acme\\Http\\*' = []".to_owned()]);
        assert_eq!(
            got,
            "[dependencies]\n'Acme\\Http' = \"1.0.0\"\n\n[grants]\n# mine\n'App' = []\n'Acme\\Http\\*' = []\n\n[registries]\nAcme = { url = \"file:r\" }\n"
        );
    }

    #[test]
    fn a_file_without_a_grants_table_gets_one() {
        let got = with_grants("[dependencies]\n", &["'Acme\\Http\\*' = []".to_owned()]);
        assert_eq!(got, "[dependencies]\n\n[grants]\n'Acme\\Http\\*' = []\n");
    }

    #[test]
    fn an_archive_entry_outside_its_folder_is_refused() {
        let scratch = nvs_repo::scratch("fetch-unpack-escape");
        for path in ["../escape.nvs", "/abs.nvs", "src//a.nvs"] {
            let mut bytes = Vec::new();
            {
                let mut builder = tar::Builder::new(&mut bytes);
                let mut header = tar::Header::new_ustar();
                // `set_path` refuses `..`, so the name is written into the header's bytes.
                header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
                header.set_size(1);
                header.set_entry_type(tar::EntryType::Regular);
                header.set_cksum();
                builder
                    .append(&header, &b"x"[..])
                    .expect("the entry writes");
                builder.finish().expect("the archive ends");
            }
            let dir = scratch.join("pkg");
            let err = unpack(&bytes, &dir).expect_err(path);
            assert!(err.0.contains("not valid"), "{path}: {}", err.0);
            assert!(
                !dir.exists() && !scratch.join("escape.nvs").exists(),
                "{path}"
            );
        }
    }
}
