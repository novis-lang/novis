//! `nvs pkg pack` — a package directory as the archive whose BLAKE3 digest is its identity
//! (`rule:packaging/a-package-is-its-digest`).
//!
//! **What goes in** is `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`'s
//! allowlist, read by [`file_set`]: every `.nvs` file under `src/`, `package.toml`, the license and
//! readme files at the root, and the `[package] files` list. Nothing else is read, so a new file in
//! the directory stays out until somebody lists it. A symlink anywhere the set is read from is
//! refused rather than followed or skipped, and so is an `nvs.toml`
//! (`rule:packaging/a-package-cannot-reach-its-host`).
//!
//! **Every path is checked to work on every platform** ([`check_path`]), and the set as a whole
//! against case-only twins and the two limits, [`MAX_FILES`] and [`MAX_BYTES`]. Those are
//! constants of the format, which the registry's build and the fetch read from here.
//!
//! **The bytes depend on the files' paths and contents alone** ([`write`]). The archive is an
//! uncompressed `ustar` tar, entries sorted by path bytes, and every header field this code does
//! not derive from the file is fixed: mode `0644`, owner and group `0` with no names, time `0`.
//! The `tar` crate writes the format (`rule:packaging/somebody-elses-specification-is-a-crate`),
//! and every field it writes is set here, so its defaults never reach the bytes. The same
//! directory therefore packs to the same digest on every platform, whatever the files' times,
//! owners or modes, and whatever order the file system lists them in.
//!
//! Cost: the archive streams from the files to its output through the hash, so memory holds the
//! list of paths and one copy buffer. Nothing here runs on the request path.

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{Renderer, SourceMap};

use super::manifest::{self, Manifest};

/// The most bytes an archive's files may add up to, counted before any compression.
pub(crate) const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The most files an archive may hold.
pub(crate) const MAX_FILES: usize = 5_000;

/// The most bytes an archive within [`MAX_FILES`] and [`MAX_BYTES`] can have: a 512-byte header
/// and up to 511 bytes of padding per file, and the two zero blocks at the end. A fetch reads no
/// byte past it.
pub(crate) const MAX_ARCHIVE_BYTES: u64 = MAX_BYTES + MAX_FILES as u64 * (512 + 511) + 1024;

/// The longest path inside an archive, in UTF-8 bytes. It leaves room under the Windows path
/// limit for the folder a package is fetched into.
pub(crate) const MAX_PATH_BYTES: usize = 160;

/// The folder whose `.nvs` files are the package's code.
const SRC: &str = "src";

/// The configuration file a package may never carry.
const SERVER_CONFIG: &str = "nvs.toml";

/// Names Windows reserves for devices, with any extension.
const RESERVED_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// The characters no path component may contain, beside the control characters.
const RESERVED_CHARS: &[char] = &['<', '>', ':', '"', '\\', '|', '?', '*'];

/// One file of the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The path inside the archive, `/`-separated and relative to the package root.
    pub(crate) path: String,
    /// The file's size in bytes when it was listed.
    pub(crate) size: u64,
    /// Where the file is on disk.
    pub(crate) source: PathBuf,
}

/// Why a directory does not pack. The text is the message the user reads.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Refusal(String);

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A refusal with `message` as its text.
fn refuse(message: impl Into<String>) -> Refusal {
    Refusal(message.into())
}

/// `nvs pkg pack <dir>`: the archive written to `out`, or to `<vendor>-<name>[-<version>].tar`
/// in the working directory, and every file, the total and the digest printed.
pub(crate) fn run(dir: &Path, out: Option<&Path>) -> ExitCode {
    let path = dir.join(nvs_config::grants::FILE);
    let mut sources = SourceMap::new();
    let manifest = match manifest::read(&path, &mut sources) {
        Ok(manifest) => manifest,
        Err(diagnostic) => {
            let mut err = io::stderr().lock();
            // Standard error that cannot be written to has nobody to tell.
            let _ = Renderer::new().render(&diagnostic, &sources, &mut err);
            return ExitCode::FAILURE;
        }
    };
    match pack(dir, &manifest, out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            ExitCode::FAILURE
        }
    }
}

/// [`run`] once the manifest has read.
fn pack(dir: &Path, manifest: &Manifest, out: Option<&Path>) -> Result<(), Refusal> {
    let Some(name) = &manifest.name else {
        return Err(refuse(
            "`package.toml` has no `[package] name`, so this directory is an application. \
             Only a package can be packed.",
        ));
    };
    let entries = file_set(dir, manifest)?;
    let target = out.map_or_else(
        || {
            let mut file = name.slug().replace('/', "-");
            if let Some(version) = manifest.version {
                file.push_str(&format!("-{version}"));
            }
            PathBuf::from(format!("{file}.tar"))
        },
        Path::to_path_buf,
    );
    let digest = write_file(&entries, &target)?;
    let total: u64 = entries.iter().map(|entry| entry.size).sum();
    let mut stdout = io::stdout().lock();
    // Standard output that cannot be written to has nobody to tell.
    for entry in &entries {
        let _ = writeln!(stdout, "{:>10}  {}", entry.size, entry.path);
    }
    let _ = writeln!(stdout, "{total:>10}  total, {} files", entries.len());
    let version = manifest
        .version
        .map_or_else(String::new, |version| format!(" {version}"));
    let _ = writeln!(
        stdout,
        "{}{version}  blake3:{}",
        name.slug(),
        digest.to_hex()
    );
    let _ = writeln!(stdout, "wrote {}", target.display());
    Ok(())
}

/// The archive of `entries` at `target`, written beside it first and renamed into place, so a
/// failed pack leaves no partial archive under the name.
fn write_file(entries: &[Entry], target: &Path) -> Result<blake3::Hash, Refusal> {
    let mut temporary = target.as_os_str().to_owned();
    temporary.push(".partial");
    let temporary = PathBuf::from(temporary);
    let result = fs::File::create(&temporary)
        .map_err(|err| refuse(format!("cannot create `{}`: {err}", temporary.display())))
        .and_then(|file| {
            let mut file = io::BufWriter::new(file);
            let digest = write(entries, &mut file)?;
            file.into_inner()
                .map_err(io::IntoInnerError::into_error)
                .and_then(|file| file.sync_all())
                .map_err(|err| refuse(format!("cannot write `{}`: {err}", temporary.display())))?;
            Ok(digest)
        })
        .and_then(|digest| {
            fs::rename(&temporary, target)
                .map_err(|err| refuse(format!("cannot write `{}`: {err}", target.display())))?;
            Ok(digest)
        });
    if result.is_err() {
        // The partial file is the only thing to clean up, and a failure to remove it does not
        // change what the user is told.
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Every file the archive of the package at `dir` carries, sorted by path bytes, each path and
/// the set checked.
///
/// # Errors
///
/// A missing license file, a listed file that is missing, is not a regular file, ends in `.nvs`
/// or is already in the default set, a symlink, an `nvs.toml`, a path [`check_path`] refuses,
/// two paths equal ignoring case, or a set over [`MAX_FILES`] or [`MAX_BYTES`].
pub(crate) fn file_set(dir: &Path, manifest: &Manifest) -> Result<Vec<Entry>, Refusal> {
    let mut entries = Vec::new();
    entries.push(regular(dir, nvs_config::grants::FILE)?);
    let mut licensed = false;
    for item in read_dir(dir, "")? {
        let (name, kind) = item?;
        let Some(license) = root_file(&name) else {
            continue;
        };
        if kind.is_dir() {
            continue;
        }
        entries.push(regular(dir, &name)?);
        licensed |= license;
    }
    if !licensed {
        return Err(refuse(
            "the package has no license file. Add a file named `LICENSE` or `COPYING` \
             (such as `LICENSE-MIT`) next to `package.toml`.",
        ));
    }
    if dir.join(SRC).exists() {
        source_files(dir, SRC, &mut entries)?;
    }
    let defaults = entries.len();
    for listed in &manifest.files {
        listed_form(listed)?;
        check_path(listed)?;
        let folded = listed.to_lowercase();
        if folded.ends_with(".nvs") {
            return Err(refuse(format!(
                "`files` lists `{listed}`, a `.nvs` file. Code goes under `src/`, and every \
                 `.nvs` file there is packed without being listed."
            )));
        }
        if let Some(twin) = entries
            .iter()
            .find(|entry| entry.path.to_lowercase() == folded)
        {
            let what = if entries[..defaults].contains(twin) {
                "is packed without being listed"
            } else {
                "is listed twice"
            };
            return Err(refuse(format!("`files` lists `{listed}`, which {what}.")));
        }
        entries.push(regular(dir, listed)?);
    }
    super::download::check_form(&manifest.downloads).map_err(refuse)?;
    if let Some(carried) = manifest.downloads.iter().find(|download| {
        let folded = download.path.to_lowercase();
        entries
            .iter()
            .any(|entry| entry.path.to_lowercase() == folded)
    }) {
        return Err(refuse(format!(
            "the download path `{}` is a file the archive already contains.",
            carried.path
        )));
    }
    check_set(&mut entries)?;
    Ok(entries)
}

/// Whether the root file `name` is packed, and if it is, whether it is a license file.
fn root_file(name: &str) -> Option<bool> {
    let lower = name.to_ascii_lowercase();
    let stem = |word: &str| lower == word || lower.starts_with(&format!("{word}."));
    if lower.starts_with("license") || lower.starts_with("copying") {
        Some(true)
    } else if stem("readme") || stem("notice") {
        Some(false)
    } else {
        None
    }
}

/// Every `.nvs` file under `relative`, a folder of the package at `dir`, added to `entries`.
fn source_files(dir: &Path, relative: &str, entries: &mut Vec<Entry>) -> Result<(), Refusal> {
    for item in read_dir(dir, relative)? {
        let (name, kind) = item?;
        let path = format!("{relative}/{name}");
        if kind.is_symlink() {
            return Err(symlink(&path));
        }
        if kind.is_dir() {
            source_files(dir, &path, entries)?;
        } else if name.ends_with(".nvs") {
            entries.push(regular(dir, &path)?);
        }
    }
    Ok(())
}

/// The UTF-8 name and kind of each entry of `relative`, a folder of the package at `dir`.
fn read_dir(
    dir: &Path,
    relative: &str,
) -> Result<impl Iterator<Item = Result<(String, fs::FileType), Refusal>>, Refusal> {
    let folder = dir.join(relative);
    let shown = if relative.is_empty() { "." } else { relative };
    let listing = fs::read_dir(&folder)
        .map_err(|err| refuse(format!("cannot read the folder `{shown}`: {err}")))?;
    let shown = shown.to_owned();
    Ok(listing.map(move |item| {
        let item =
            item.map_err(|err| refuse(format!("cannot read the folder `{shown}`: {err}")))?;
        let kind = item
            .file_type()
            .map_err(|err| refuse(format!("cannot read the folder `{shown}`: {err}")))?;
        let name = item.file_name().into_string().map_err(|name| {
            refuse(format!(
                "the folder `{shown}` contains `{}`, a name that is not UTF-8.",
                name.to_string_lossy()
            ))
        })?;
        Ok((name, kind))
    }))
}

/// The entry for `relative`, which must be a regular file of the package at `dir`, reached
/// through no symlink.
fn regular(dir: &Path, relative: &str) -> Result<Entry, Refusal> {
    let mut on_disk = dir.to_path_buf();
    let mut parts = relative.split('/').peekable();
    while let Some(part) = parts.next() {
        on_disk.push(part);
        let meta = fs::symlink_metadata(&on_disk).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                refuse(format!("`{relative}` does not exist."))
            } else {
                refuse(format!("cannot read `{relative}`: {err}"))
            }
        })?;
        if meta.file_type().is_symlink() {
            return Err(symlink(relative));
        }
        if parts.peek().is_none() {
            if !meta.is_file() {
                return Err(refuse(format!("`{relative}` is not a regular file.")));
            }
            return Ok(Entry {
                path: relative.to_owned(),
                size: meta.len(),
                source: on_disk,
            });
        }
    }
    Err(refuse(format!("`{relative}` is not a regular file.")))
}

/// The refusal of a symlink at `path`.
fn symlink(path: &str) -> Refusal {
    refuse(format!(
        "`{path}` is a symbolic link. A package can only contain regular files."
    ))
}

/// A `files` line is a relative path to one file, written with `/`.
fn listed_form(listed: &str) -> Result<(), Refusal> {
    let bad = listed.is_empty()
        || listed.starts_with('/')
        || listed.contains('\\')
        || listed
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..");
    if bad {
        return Err(refuse(format!(
            "`files` lists `{listed}`. Each line must be the path of one file inside the \
             package, written with `/`, such as `templates/error.html`."
        )));
    }
    Ok(())
}

/// Whether `path` works as a file path on every platform.
///
/// # Errors
///
/// The reason, as a sentence that names the path: it is not NFC, it is over
/// [`MAX_PATH_BYTES`], or a component is a reserved Windows name, contains a control character
/// or one of `<>:"\|?*`, or ends in a dot or a space.
pub(crate) fn check_path(path: &str) -> Result<(), Refusal> {
    if !unicode_normalization::is_nfc(path) {
        return Err(refuse(format!(
            "the path `{path}` is not in Unicode NFC form. Rename the file with its characters \
             composed."
        )));
    }
    if path.len() > MAX_PATH_BYTES {
        return Err(refuse(format!(
            "the path `{path}` is {} bytes long. The limit is {MAX_PATH_BYTES} bytes.",
            path.len()
        )));
    }
    for part in path.split('/') {
        let stem = part.split('.').next().unwrap_or(part).to_ascii_lowercase();
        if RESERVED_NAMES.contains(&stem.as_str()) {
            return Err(refuse(format!(
                "the path `{path}` uses `{part}`, a name Windows reserves for a device."
            )));
        }
        if let Some(c) = part
            .chars()
            .find(|c| c.is_control() || RESERVED_CHARS.contains(c))
        {
            return Err(refuse(format!(
                "the path `{path}` contains the character {c:?}, which Windows does not allow \
                 in a file name."
            )));
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return Err(refuse(format!(
                "the path `{path}` has a name that ends in a dot or a space. Windows removes \
                 that character."
            )));
        }
    }
    Ok(())
}

/// `entries` sorted by path bytes, each path checked, with no case-only twins, no `nvs.toml`,
/// and within both limits.
fn check_set(entries: &mut [Entry]) -> Result<(), Refusal> {
    entries.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    for entry in entries.iter() {
        check_path(&entry.path)?;
        let file = entry.path.rsplit('/').next().unwrap_or(&entry.path);
        if file.eq_ignore_ascii_case(SERVER_CONFIG) {
            return Err(refuse(format!(
                "`{}` is a server configuration file. A package may not contain one: only the \
                 person who runs the server configures it.",
                entry.path
            )));
        }
    }
    let mut folded: Vec<(String, &str)> = entries
        .iter()
        .map(|entry| (entry.path.to_lowercase(), entry.path.as_str()))
        .collect();
    folded.sort();
    if let Some(pair) = folded.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(refuse(format!(
            "`{}` and `{}` differ only in case. Some file systems cannot store both.",
            pair[0].1, pair[1].1
        )));
    }
    check_limits(entries)
}

/// `entries` hold at most [`MAX_FILES`] files of at most [`MAX_BYTES`] together.
fn check_limits(entries: &[Entry]) -> Result<(), Refusal> {
    if entries.len() > MAX_FILES {
        return Err(refuse(format!(
            "the package has {} files. The limit is {MAX_FILES}.",
            entries.len()
        )));
    }
    let total: u64 = entries.iter().map(|entry| entry.size).sum();
    if total > MAX_BYTES {
        return Err(refuse(format!(
            "the package's files are {total} bytes together. The limit is {MAX_BYTES} bytes \
             (16 MiB). A larger file can be an external download."
        )));
    }
    Ok(())
}

/// The archive of `entries`, which [`file_set`] sorted and checked, written to `out`, and the
/// BLAKE3 digest of the bytes written.
///
/// # Errors
///
/// A file that cannot be read or changed size since it was listed, a path the `ustar` header
/// cannot store, or a failed write.
pub(crate) fn write(entries: &[Entry], out: &mut impl Write) -> Result<blake3::Hash, Refusal> {
    let mut hashed = Hashed {
        inner: out,
        hasher: blake3::Hasher::new(),
    };
    let mut builder = tar::Builder::new(&mut hashed);
    for entry in entries {
        let mut header = tar::Header::new_ustar();
        header.set_path(&entry.path).map_err(|err| {
            refuse(format!(
                "the path `{}` does not fit the archive's header: {err}",
                entry.path
            ))
        })?;
        header.set_size(entry.size);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        let file = fs::File::open(&entry.source)
            .map_err(|err| refuse(format!("cannot read `{}`: {err}", entry.path)))?;
        let mut exact = Exact {
            inner: file,
            left: entry.size,
        };
        builder
            .append(&header, &mut exact)
            .map_err(|err| refuse(format!("cannot pack `{}`: {err}", entry.path)))?;
        let mut more = [0_u8; 1];
        if exact.inner.read(&mut more).unwrap_or(0) != 0 {
            return Err(changed(&entry.path));
        }
    }
    builder
        .finish()
        .map_err(|err| refuse(format!("cannot write the archive: {err}")))?;
    drop(builder);
    Ok(hashed.hasher.finalize())
}

/// The refusal of a file whose size changed while it was packed.
fn changed(path: &str) -> Refusal {
    refuse(format!(
        "`{path}` changed while it was packed. Run the command again."
    ))
}

/// A writer that hashes every byte it passes on.
struct Hashed<'a, W: Write> {
    inner: &'a mut W,
    hasher: blake3::Hasher,
}

impl<W: Write> Write for Hashed<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A reader of exactly `left` more bytes, and an error if the file ends sooner, so a header's
/// size always matches the data after it.
struct Exact {
    inner: fs::File,
    left: u64,
}

impl Read for Exact {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 {
            return Ok(0);
        }
        let room = usize::try_from(self.left)
            .unwrap_or(usize::MAX)
            .min(buf.len());
        let read = self.inner.read(&mut buf[..room])?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the file is shorter than when it was listed",
            ));
        }
        self.left -= read as u64;
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(count: usize, size: u64) -> Vec<Entry> {
        (0..count)
            .map(|i| Entry {
                path: format!("src/F{i}.nvs"),
                size,
                source: PathBuf::new(),
            })
            .collect()
    }

    #[test]
    fn the_file_limit_holds_and_one_more_is_refused() {
        assert!(check_limits(&entries(MAX_FILES, 0)).is_ok());
        let refusal = check_limits(&entries(MAX_FILES + 1, 0)).unwrap_err();
        assert!(refusal.0.contains("5001 files"), "{refusal}");
    }

    #[test]
    fn the_byte_limit_holds_and_one_byte_more_is_refused() {
        let mut set = entries(2, MAX_BYTES / 2);
        assert!(check_limits(&set).is_ok());
        set[1].size += 1;
        let refusal = check_limits(&set).unwrap_err();
        assert!(refusal.0.contains("16777217 bytes"), "{refusal}");
    }

    #[test]
    fn a_portable_path_passes() {
        for path in [
            "src/Acme/Http/Client.nvs",
            "templates/Übersicht.html",
            "LICENSE-MIT",
        ] {
            assert_eq!(check_path(path), Ok(()), "{path}");
        }
        assert_eq!(check_path(&"a".repeat(MAX_PATH_BYTES)), Ok(()));
    }

    #[test]
    fn each_path_rule_refuses() {
        let decomposed = "templates/U\u{308}bersicht.html";
        let long = "a".repeat(MAX_PATH_BYTES + 1);
        for path in [
            decomposed,
            long.as_str(),
            "assets/CON",
            "assets/nul.txt",
            "Com1/a.txt",
            "assets/a:b.txt",
            "assets/a?.txt",
            "assets/a*.txt",
            "assets/a|b.txt",
            "assets/a<b.txt",
            "assets/a\"b.txt",
            "assets/tab\there.txt",
            "assets/name.",
            "assets/name ",
            "folder./a.txt",
        ] {
            assert!(check_path(path).is_err(), "accepted {path:?}");
        }
    }

    #[test]
    fn a_name_that_only_starts_like_a_reserved_one_passes() {
        for path in ["console.txt", "auxiliary/a.txt", "com10.txt", "lpt.txt"] {
            assert_eq!(check_path(path), Ok(()), "{path}");
        }
    }

    #[test]
    fn a_files_line_must_be_one_relative_path() {
        for line in [
            "",
            "/abs.txt",
            "a//b.txt",
            "./a.txt",
            "../a.txt",
            "a/../b.txt",
            "a\\b.txt",
        ] {
            assert!(listed_form(line).is_err(), "accepted {line:?}");
        }
        assert!(listed_form("templates/error.html").is_ok());
    }

    #[test]
    fn root_files_are_the_license_readme_and_notice() {
        assert_eq!(root_file("LICENSE"), Some(true));
        assert_eq!(root_file("license-mit"), Some(true));
        assert_eq!(root_file("COPYING.txt"), Some(true));
        assert_eq!(root_file("README.md"), Some(false));
        assert_eq!(root_file("Notice"), Some(false));
        assert_eq!(root_file("README-dev.md"), None);
        assert_eq!(root_file("CHANGELOG.md"), None);
        assert_eq!(root_file("nvs.toml"), None);
    }
}
