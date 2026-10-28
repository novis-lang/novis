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
//! **Each package is unpacked into `vendor/<vendor>/<name>/`** ([`unpack`]), its slug as a path.
//! Every entry of the archive is a regular file at a path [`pack::check_path`] accepts, within
//! [`pack::MAX_FILES`] and [`pack::MAX_BYTES`], so a hostile archive writes nothing outside its
//! folder. The folder is written beside its place and renamed into it, so a failed fetch leaves
//! the old folder whole. `fetch` keeps a folder whose files already pack to the locked digest, so
//! a second fetch reads no archive. `vendor` writes every folder again from its archive. A folder
//! of a package the old lock had and the graph no longer needs is removed; nothing else under
//! `vendor/` is touched, because a person may keep hand-vendored code there.
//!
//! **`vendor/packages.nvs` is generated from the lock**: one `autoload` line per package, in slug
//! order, so the same lock writes the same bytes and a change to the graph is a line in a diff.
//!
//! **One grant line per package is added to `[grants]`** ([`with_grants`]): `'Acme\Http\*' = []`,
//! the subtree form, because a package's code is its namespace and every namespace below it
//! (`rule:security/grants-are-keyed-on-a-namespace`). The line grants nothing, and its comment
//! names what the package requests. A package that already has a key naming its namespace, in
//! either form, gets no line, so a line a person edited is never rewritten. The lines are added
//! to the file's text, so every comment and every other line stays as it was written.
//!
//! **No code of a package runs** (`rule:packaging/nothing-runs-before-the-program`): the command
//! reads archives and writes files, and compiles nothing.
//!
//! Cost: one archive in memory at a time, at most [`pack::MAX_ARCHIVE_BYTES`], beside what
//! resolution holds. Nothing here runs on the request path.

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{Renderer, SourceMap};

use super::git::{self, Outbound};
use super::index::{Capabilities, Fault, fault};
use super::lock::{self, Lock};
use super::manifest::{self, Manifest};
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
    match fetch(mode, locked) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failed::Fault(fault)) => {
            eprintln!("error: {fault}");
            ExitCode::FAILURE
        }
        Err(Failed::Reported(code)) => code,
    }
}

/// Why the command stopped.
enum Failed {
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

/// [`run`], returning what stopped it.
fn fetch(mode: Mode, locked: bool) -> Result<(), Failed> {
    let cwd = std::env::current_dir()
        .map_err(|err| fault(format!("cannot read the working directory: {err}")))?;
    let Some(path) = manifest::find(&cwd) else {
        return Err(fault(format!(
            "there is no `{}` in `{}` or a folder above it. Write one with a `[dependencies]` \
             table.",
            nvs_config::grants::FILE,
            cwd.display()
        ))
        .into());
    };
    let mut sources = SourceMap::new();
    let manifest = manifest::read(&path, &mut sources).map_err(|diagnostic| {
        // Standard error that cannot be written to has nobody to tell.
        let _ = Renderer::new().render(&diagnostic, &sources, &mut io::stderr().lock());
        Failed::Reported(ExitCode::FAILURE)
    })?;
    let app = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    let outbound = Outbound::new(crate::config::written_capabilities().map_err(Failed::Reported)?);
    let data = nvs_config::data::current();
    let keep = data.map(nvs_config::data::Folder::registry);
    let lock_path = app.join(lock::FILE);
    let old = Lock::read(&lock_path)?;
    if locked && old.is_none() {
        return Err(fault(format!(
            "`--locked` uses the versions in {}, and `{}` has none. Run the command without \
             `--locked` to write it.",
            lock::FILE,
            app.display()
        ))
        .into());
    }
    let mut registries = Registries::new(&manifest, keep.as_deref()).reaching(outbound);
    if let Some(data) = data {
        // A git dependency is written out under the data folder, beside every other file Novis
        // keeps, and removed once it is packed.
        registries = registries.clones_in(&data.tmp(), git::LIMITS);
    }
    let graph = resolve::resolve(&manifest, &mut registries, old.as_ref())?;
    if let Some(old) = &old {
        old.check(&graph, locked)?;
    }
    for crossing in &graph.crossings {
        eprintln!("note: {crossing}");
    }
    let vendor = app.join(VENDOR);
    let written = write_packages(mode, &graph, &mut registries, &vendor)?;
    if let Some(old) = &old {
        remove_unneeded(old, &graph, &vendor)?;
    }
    let new = Lock::of(&graph);
    write_if_changed(&vendor.join(AUTOLOADS), &autoloads(&new))?;
    if !locked && old.as_ref() != Some(&new) {
        new.write(&lock_path)?;
    }
    let added = add_grants(&path, &manifest, &new)?;
    for line in &added {
        println!("added to [grants]: {line}");
    }
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
        let archive = match registries.cloned(name) {
            Some(archive) => archive.to_vec(),
            None => registries
                .registry(name)?
                .archive(name, release.version, &release.digest)?,
        };
        unpack(&archive, &dir).map_err(|err| {
            fault(format!(
                "cannot write `{name}` {} to `{}`: {}",
                release.version,
                dir.display(),
                err.0
            ))
        })?;
        written += 1;
    }
    Ok(written)
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
    let failed = |err: io::Error| fault(format!("cannot write `{}`: {err}", path.display()));
    let text = fs::read_to_string(path).map_err(failed)?;
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    fs::write(&partial, with_grants(&text, &lines)).map_err(failed)?;
    fs::rename(&partial, path).map_err(failed)?;
    Ok(lines)
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
    let mut out: Vec<&str> = text.lines().collect();
    let header = |line: &str| {
        let line = line.trim();
        line.strip_prefix('[')
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '['))
    };
    let grants = out.iter().position(|line| {
        let line = line.trim();
        line.strip_prefix("[grants]")
            .is_some_and(|rest| rest.trim().is_empty() || rest.trim().starts_with('#'))
    });
    let added: Vec<&str> = lines.iter().map(String::as_str).collect();
    match grants {
        Some(at) => {
            let end = out[at + 1..]
                .iter()
                .position(|line| header(line))
                .map_or(out.len(), |next| at + 1 + next);
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
            out.push("[grants]");
            out.extend(added);
        }
    }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut written = out.join(newline);
    written.push_str(newline);
    written
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
