//! `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s
//! portable single-file executable: `nvs build --compile` on the way out, and
//! the footer check every `nvs` process makes on the way in.
//!
//! ## What is appended
//!
//! § 2's payload is **source, not artifacts**: the entry file, every file its
//! `require` graph statically resolves to, and every file its `autoload` roots
//! declare, as a flat list of `(relative path, length, bytes)`, with no archive
//! format and no compression. § 4 puts a fixed-size footer after it, so the
//! whole appended region is
//!
//! ```text
//! [ host binary ][ manifest ][ footer ]
//!
//! manifest: count: u32
//!           count x ( path_len: u32, path: UTF-8 bytes, len: u64, bytes )
//! footer:   b"NVSB", format_version: u16, manifest_offset: u64, manifest_len: u64
//! ```
//!
//! all little-endian, the footer's 22 bytes at the very end of the file. A path
//! is written with `/` separators whatever built it, because the payload is the
//! artifact that travels between platforms and a `\` in it would resolve on
//! exactly one of them. The entry point is **entry zero**, which is
//! `nvs_hir::resolve_program`'s own order contract carried through
//! unchanged — nothing in the footer names it, so nothing can disagree with it.
//!
//! Neither the PE nor the ELF loader reads past the sections its own headers
//! describe, so these bytes are invisible to both and the copy stays runnable
//! as itself: an `nvs` that finds no footer is the `nvs` it always was.
//!
//! On macOS the copy is ad-hoc signed once the payload is on the end of it,
//! which is `rule:packaging/a-macos-bundle-is-ad-hoc-signed-at-build`. A
//! `codesign --sign -` that does not succeed is a warning and not a failed
//! build: an unsigned bundle is still the artifact the author asked for, and on
//! a machine with no developer tools installed refusing to produce one would be
//! worse than saying so.
//!
//! ## The extensions it carries
//!
//! `rule:packaging/a-nvsx-dependency-embeds-in-the-same-payload`: [`build`] reads the
//! configuration the build would run under (`--config`, else `./nvs.toml`), and appends each
//! `[[extension]]` entry's `.nvsx` to the same list after the source, as an opaque entry named
//! `\0extension/<index>/<file name>`. Then one last entry, `\0extension.toml`, is the entries
//! themselves as a TOML `[[extension]]` array: `path` is the payload name without its prefix,
//! `sha256` and `memory` are as written, and `grants` is as written with every root made absolute
//! against the file that wrote it, so a root means the directory the author named on every
//! machine. No relative path can start with a NUL byte, so no source file can take either name.
//! A file whose bytes do not match its pin refuses the build, before anything is written.
//!
//! At start, [`run`] keeps the entries and their bytes in one process-wide table, each `path` now
//! the payload name joined onto the executable's own path, which can never be a file on disk.
//! [`carry_extensions`] puts them in front of the tree's own `[[extension]]` entries every time
//! the run resolves its configuration, so they load first, and the loader reads their bytes from
//! [`extension_file`]. From there it is the ordinary run: every embedded pin is checked again at
//! start, one digest per entry, and a class declared by both an embedded entry and one of the
//! run's own refuses the start as two entries of one tree would. What it spends: the `.nvsx`
//! files' bytes in the executable, and once more in memory for the life of the process.
//!
//! ## What the shipped binary does with it
//!
//! [`embedded`] is read before clap sees a single argument, because a bundle's
//! `argv` belongs to the *program* rather than to `nvs`: an app whose first
//! argument happens to be `run` or `check` must not have it eaten by a
//! subcommand. On a hit, [`run`] installs the payload as
//! `nvs_diagnostics::embedded`'s byte source and calls the ordinary
//! `nvs run` path — § 4's "the same code path with one different byte source
//! for reads", so there is no second interpreter here to drift from the first.
//!
//! Configuration is that same path's: naming no `--config` leaves the search
//! for `./nvs.toml` in the working directory
//! (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`),
//! which is what a bundled process therefore gets. § 1's single trust domain is
//! what makes reading a file beside the binary harmless — the person running it
//! is the only principal — and the consequence is that a malformed `nvs.toml` in
//! that directory refuses the run as the configuration error it is, before the
//! bundled program is parsed.
//!
//! ## Known gap
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-cli/src/bundle.rs` lists them.

use std::collections::HashSet;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// § 4's magic, first of the footer's 22 bytes.
const MAGIC: &[u8; 4] = b"NVSB";

/// The payload layout this build of `nvs` writes and reads. A mismatch is not
/// a bundle *this* host understands, and is treated as no bundle at all — the
/// binary then runs as an ordinary `nvs`, which is the only behaviour that
/// cannot corrupt anything.
const FORMAT_VERSION: u16 = 1;

/// `MAGIC` + `u16` + `u64` + `u64`.
const FOOTER_LEN: usize = 4 + 2 + 8 + 8;

/// [`FOOTER_LEN`] as a file offset. A `const` and an `expect` rather than a
/// cast because every length in this module is checked against the bytes
/// actually held, and a silently wrapping conversion would be the one place
/// that stopped being true.
fn footer_len() -> u64 {
    u64::try_from(FOOTER_LEN).expect("a 22-byte footer fits in a u64")
}

/// The prefix of an embedded `.nvsx`'s payload name.
const EXTENSION_PREFIX: &str = "\0extension/";

/// The payload name of the embedded `[[extension]]` entries, written last.
const EXTENSION_ENTRIES: &str = "\0extension.toml";

/// One bundled program, as read back out of a host binary.
pub(crate) struct Bundle {
    /// § 2's flat list in payload order, entry file first.
    files: Vec<(String, String)>,
    /// Each embedded `.nvsx`, by its payload name without [`EXTENSION_PREFIX`].
    extensions: Vec<(String, Vec<u8>)>,
    /// The [`EXTENSION_ENTRIES`] text, when the bundle carries extensions.
    entries: Option<String>,
}

impl Bundle {
    /// Splits a payload's entries into source, embedded `.nvsx` files and their entries. `None`
    /// for a source entry that is not UTF-8.
    fn of(raw: Vec<(String, Vec<u8>)>) -> Option<Self> {
        let mut bundle = Self {
            files: Vec::new(),
            extensions: Vec::new(),
            entries: None,
        };
        for (name, bytes) in raw {
            if name == EXTENSION_ENTRIES {
                bundle.entries = Some(String::from_utf8(bytes).ok()?);
            } else if let Some(name) = name.strip_prefix(EXTENSION_PREFIX) {
                bundle.extensions.push((name.to_owned(), bytes));
            } else {
                bundle.files.push((name, String::from_utf8(bytes).ok()?));
            }
        }
        Some(bundle)
    }
}

/// The extensions a running bundle carries: its `[[extension]]` entries, each `path` absolute,
/// and each file's bytes under that path.
struct Embedded {
    /// The entries as TOML tables, in payload order.
    tables: Vec<toml::Value>,
    /// The same entries, typed.
    typed: Vec<nvs_config::tree::Extension>,
    /// Each entry's `path` and the file's bytes.
    files: Vec<(PathBuf, Vec<u8>)>,
}

/// Set once by [`run`] for a bundle that carries extensions, and never in an ordinary `nvs`.
static EMBEDDED: std::sync::OnceLock<Embedded> = std::sync::OnceLock::new();

/// The bytes of the embedded `.nvsx` at `path`, or `None` for a path no embedded entry names.
pub(crate) fn extension_file(path: &Path) -> Option<&'static [u8]> {
    let embedded = EMBEDDED.get()?;
    embedded
        .files
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, bytes)| bytes.as_slice())
}

/// Puts the running bundle's embedded `[[extension]]` entries in front of `resolved`'s own, so
/// they load first. Every origin of an `extension.<index>` key moves by the same count, so a
/// refusal still points at the line that wrote its entry. Does nothing outside a bundle.
pub(crate) fn carry_extensions(resolved: &mut nvs_config::resolve::Resolved) {
    let Some(embedded) = EMBEDDED.get() else {
        return;
    };
    let count = embedded.tables.len();
    let written = resolved
        .table
        .entry("extension")
        .or_insert_with(|| toml::Value::Array(Vec::new()));
    if let toml::Value::Array(written) = written {
        written.splice(0..0, embedded.tables.iter().cloned());
    }
    resolved
        .config
        .extension
        .splice(0..0, embedded.typed.iter().cloned());
    resolved.origins = std::mem::take(&mut resolved.origins)
        .into_iter()
        .map(|(key, origin)| (shifted(&key, count), origin))
        .collect();
}

/// `key` with the index of an `extension.<index>` key raised by `count`, and any other key as it
/// is.
fn shifted(key: &str, count: usize) -> String {
    let Some(rest) = key.strip_prefix("extension.") else {
        return key.to_owned();
    };
    let (index, tail) = rest.split_at(rest.find('.').unwrap_or(rest.len()));
    match index.parse::<usize>() {
        Ok(index) => format!("extension.{}{tail}", index + count),
        Err(_) => key.to_owned(),
    }
}

/// Keeps a bundle's embedded extensions for the run, each entry's `path` joined onto `root`.
fn embed(root: &Path, extensions: Vec<(String, Vec<u8>)>, entries: &str) -> Result<(), String> {
    let mut table: toml::Table = entries
        .parse()
        .map_err(|err| format!("the embedded extension entries do not parse: {err}"))?;
    let Some(toml::Value::Array(written)) = table.remove("extension") else {
        return Err("the embedded extension entries are not an array".to_owned());
    };
    let mut bytes: std::collections::HashMap<String, Vec<u8>> = extensions.into_iter().collect();
    let mut embedded = Embedded {
        tables: Vec::with_capacity(written.len()),
        typed: Vec::with_capacity(written.len()),
        files: Vec::with_capacity(written.len()),
    };
    for mut entry in written {
        let toml::Value::Table(fields) = &mut entry else {
            return Err("an embedded extension entry is not a table".to_owned());
        };
        let Some(name) = fields
            .get("path")
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
        else {
            return Err("an embedded extension entry has no path".to_owned());
        };
        let Some(file) = bytes.remove(&name) else {
            return Err(format!("the embedded extension `{name}` is missing"));
        };
        // The path the loader looks up is the one the entry writes, so both are this one string.
        let path = PathBuf::from(root.join(&name).to_string_lossy().into_owned());
        fields.insert("path".into(), path.to_string_lossy().into_owned().into());
        let typed: nvs_config::tree::Extension = entry
            .clone()
            .try_into()
            .map_err(|err| format!("the embedded extension `{name}` does not read: {err}"))?;
        embedded.tables.push(entry);
        embedded.typed.push(typed);
        embedded.files.push((path, file));
    }
    EMBEDDED
        .set(embedded)
        .map_err(|_| "the embedded extensions were already read".to_owned())
}

/// The payload appended to the running executable, or `None` for an ordinary
/// `nvs`.
///
/// Every failure here answers `None` rather than reporting: a binary whose own
/// tail cannot be read is one that must still run as the compiler it also is,
/// and the alternative is an `nvs` that refuses to start because of something
/// no user asked it to do.
pub(crate) fn embedded() -> Option<Bundle> {
    let exe = std::env::current_exe().ok()?;
    read_payload(&exe).ok().flatten()
}

/// Runs a bundled program: install § 2's payload as the byte source, then the
/// ordinary `nvs run` path over its entry point.
pub(crate) fn run(bundle: Bundle) -> ExitCode {
    let Ok(root) = std::env::current_exe() else {
        eprintln!("error: this executable carries a bundle but cannot name itself");
        return ExitCode::FAILURE;
    };
    if let Some(entries) = &bundle.entries
        && let Err(reason) = embed(&root, bundle.extensions, entries)
    {
        eprintln!("error: this executable's bundle does not read: {reason}");
        return ExitCode::FAILURE;
    }
    let entry = nvs_diagnostics::embedded::install(&root, bundle.files).to_path_buf();
    // Every word past the executable is the bundled program's own, there being
    // no `nvs run` in front of it to claim any: `rule:programs/bundle-trust-domain`'s whole point is
    // that the binary *is* the program, so this is the plainest reading of its
    // command line and the one `rule:tooling/commands-are-compiled`'s `Core\Command::run` matches.
    super::run_run(
        &entry,
        false,
        false,
        None,
        // No `--count` either: a bundle has no command line of its own to ask for one on.
        false,
        // A bundle is a program, and a program answers a request only where
        // something handed it one: there is no `nvs run` in front of this to
        // have carried a `--request`, and a word on this command line is the
        // bundled program's own. It is no connection either, for the same
        // reason: nothing in front of it carried a `--peer` or an `--events`.
        None,
        None,
        None,
        &[],
        std::env::args().skip(1).collect(),
        // Step 3 writes for a **project command**, and a bundle is not one: there is no `nvs run`
        // in front of this to have been typed in a project, and a shipped executable that left a
        // `nvs.toml` in whatever directory a user happened to start it from would be writing into
        // one nobody chose. It runs on the shipped defaults, as it did before that step existed.
        crate::config::Init::Never,
    )
}

/// `nvs build --compile <entry> [-o <out>]` — `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`.
///
/// The program goes through the same front end `check` and `run` do before a
/// byte is written, which is where § 3's static-`require` rule is enforced: a
/// path the walk cannot resolve is already `E_REQUIRE_TARGET_NOT_FOUND`, and a
/// program that does not compile produces no artifact at all rather than one
/// that fails on the user's machine.
///
/// What is collected is the file set a *re-compilation* of this program reads,
/// and the `require` graph is only half of it: a bundled process runs the same
/// front end over the payload, so every file the `autoload` roots declare is
/// frozen in here too, `nvs_hir::AutoloadMap::enumerate`'s answer taken at
/// build time because a root is a real directory on the machine that built the
/// bundle and nothing at all on the machine that runs it
/// (`rule:programs/no-runtime-autoload`: the file graph closes while
/// compiling). A file under a root that no name reaches is carried as well,
/// since that is what the source tree offers `Core\Program::implementing<T>()`
/// and `rule:routing/routes-are-compiled-not-registered`'s route table. What it
/// spends is the source bytes of every `.nvs` file under a declared root, once,
/// in the artifact — a program autoloading a directory it does not use ships
/// that directory.
///
/// The program is typed against the extension set `config` resolves to, and each of its files is
/// embedded after the source (*The extensions it carries* above).
pub(crate) fn build(entry: &Path, out: Option<&Path>, config: &[PathBuf]) -> ExitCode {
    let manifests = match crate::config::extension_set(config, entry) {
        Ok(manifests) => manifests,
        Err(code) => return code,
    };
    // A file an extension's source section carries travels inside its `.nvsx`, under the name
    // the compiler gives it, and has no path on disk. A built-in component's travels inside the
    // host binary the bundle copies.
    let built_in = crate::extensions::with_built_in(Vec::new());
    let carried: HashSet<String> = built_in
        .iter()
        .chain(&manifests)
        .flat_map(|manifest| {
            manifest
                .source
                .iter()
                .map(|file| format!("{}:{}", manifest.class.trim_start_matches('\\'), file.path))
        })
        .collect();
    let checked = match super::front_end_extended(entry, manifests) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let extensions = match extensions_of(config, entry) {
        Ok(extensions) => extensions,
        Err(code) => return code,
    };

    let mut sources: Vec<(PathBuf, String)> = Vec::with_capacity(checked.files.len());
    for file in &checked.files {
        let src = checked.map.file(file.id);
        if src.path().is_none() && carried.contains(src.name().trim_start_matches('\\')) {
            continue;
        }
        let Some(path) = src
            .path()
            .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
        else {
            eprintln!("error: a file in the require graph has no path on disk to bundle");
            return ExitCode::FAILURE;
        };
        sources.push((path, src.text().to_owned()));
    }

    // `enumerate` is sorted by name and canonicalizes what it finds, as the
    // loop above does, so the payload's tail is the same list in the same order
    // on every host that builds it and a file the graph walk already loaded is
    // recognised rather than embedded twice. A program that declared no
    // `autoload` walks no directory here.
    let mut seen: HashSet<PathBuf> = sources.iter().map(|(path, _)| path.clone()).collect();
    for (_, path) in checked.autoload.enumerate() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!(
                    "error: {} is declared by an autoload root and cannot be read to bundle it: {err}",
                    path.display()
                );
                return ExitCode::FAILURE;
            }
        };
        sources.push((path, text));
    }

    let Some(root) = common_root(&sources) else {
        eprintln!(
            "error: the require graph spans more than one filesystem root, so it has no \
             relative layout to bundle"
        );
        return ExitCode::FAILURE;
    };

    let source_count = sources.len();
    let extension_count = extensions.files.len();
    let mut files: Vec<(String, Vec<u8>)> = Vec::with_capacity(sources.len() + extension_count + 1);
    for (path, text) in sources {
        let name = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        files.push((name, text.into_bytes()));
    }
    if let Some(entries) = extensions.entries {
        files.extend(extensions.files);
        files.push((EXTENSION_ENTRIES.to_owned(), entries.into_bytes()));
    }

    let output = match out {
        Some(path) => path.to_path_buf(),
        None => default_output(entry),
    };
    match write_bundle(&files, &output) {
        Ok(()) => {
            // ASCII on purpose: this line is captured by `bun nv try` and by
            // the acceptance check behind it, and a Windows console's legacy
            // code page renders an em dash here as mojibake in both.
            if extension_count == 0 {
                println!("wrote {} ({source_count} source file(s))", output.display());
            } else {
                println!(
                    "wrote {} ({source_count} source file(s), {extension_count} extension(s))",
                    output.display()
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: could not write {}: {err}", output.display());
            ExitCode::FAILURE
        }
    }
}

/// What [`extensions_of`] carries into the payload: each `.nvsx` under its payload name, and the
/// [`EXTENSION_ENTRIES`] text, which is `None` when the configuration lists none.
struct Carried {
    files: Vec<(String, Vec<u8>)>,
    entries: Option<String>,
}

/// Every `.nvsx` the configuration `config` resolves to for `entry` lists, read and checked
/// against its pin, with the entries that load them.
///
/// # Errors
///
/// The exit code after one line naming the file that does not read, whose bytes do not match
/// its pin, or whose grant root is not UTF-8.
fn extensions_of(config: &[PathBuf], entry: &Path) -> Result<Carried, ExitCode> {
    let mut carried = Carried {
        files: Vec::new(),
        entries: None,
    };
    let mut sources = nvs_diagnostics::SourceMap::new();
    // A tree that does not resolve lists no extension, as it does for the compile in `build`.
    let Ok((snapshot, origins)) = crate::config::boot_origins(
        config,
        Some(entry),
        &mut sources,
        crate::config::Init::Never,
    ) else {
        return Ok(carried);
    };
    let mut tables = Vec::with_capacity(snapshot.config.extension.len());
    for (index, written) in snapshot.config.extension.iter().enumerate() {
        let loaded = crate::extensions::entry(index, written, &origins);
        let fail = |reason: String| {
            eprintln!("error: {}: {reason}", loaded.path.display());
            ExitCode::FAILURE
        };
        let bytes = fs::read(&loaded.path)
            .map_err(|err| fail(format!("the extension does not read: {err}")))?;
        let sha256 = nvs_ext::load::pin(&bytes);
        if !sha256.eq_ignore_ascii_case(&loaded.sha256) {
            return Err(fail(format!(
                "the file's sha256 is {sha256}, and the configuration pins {}",
                loaded.sha256
            )));
        }
        let roots = |paths: &[PathBuf]| -> Result<toml::Value, ExitCode> {
            paths
                .iter()
                .map(|path| {
                    path.to_str()
                        .map(|root| toml::Value::String(root.to_owned()))
                        .ok_or_else(|| {
                            fail(format!("the grant root {} is not UTF-8", path.display()))
                        })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(toml::Value::Array)
        };
        let mut grants = toml::Table::new();
        grants.insert("read".into(), roots(&loaded.grants.read)?);
        grants.insert("write".into(), roots(&loaded.grants.write)?);
        grants.insert(
            "connect".into(),
            toml::Value::Array(
                loaded
                    .grants
                    .connect
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        );
        let file_name = loaded.path.file_name().map_or_else(
            || "extension.nvsx".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let name = format!("{index}/{file_name}");
        let mut table = toml::Table::new();
        table.insert("path".into(), name.clone().into());
        table.insert("sha256".into(), loaded.sha256.clone().into());
        if let Some(memory) = &written.memory {
            table.insert("memory".into(), setting(memory));
        }
        table.insert("grants".into(), toml::Value::Table(grants));
        tables.push(toml::Value::Table(table));
        carried
            .files
            .push((format!("{EXTENSION_PREFIX}{name}"), bytes));
    }
    if !tables.is_empty() {
        let mut document = toml::Table::new();
        document.insert("extension".into(), toml::Value::Array(tables));
        carried.entries = Some(document.to_string());
    }
    Ok(carried)
}

/// A written setting as the TOML value that wrote it.
fn setting(setting: &nvs_config::tree::Setting) -> toml::Value {
    use nvs_config::tree::Setting;
    match setting {
        Setting::Bool(value) => toml::Value::Boolean(*value),
        Setting::Integer(value) => toml::Value::Integer(*value),
        Setting::Float(value) => toml::Value::Float(*value),
        Setting::Text(value) => toml::Value::String(value.clone()),
        Setting::List(values) => {
            toml::Value::Array(values.iter().cloned().map(toml::Value::String).collect())
        }
    }
}

/// The deepest directory every bundled file sits under.
///
/// The payload's paths are relative to *something*, and the entry file's own
/// directory is not it: a `require "../lib/db.nvs"` is legal and would escape
/// it. The common ancestor is the one root under which every file in the graph
/// keeps the layout it was written with, which is what makes a bundled
/// `require` resolve to the file the build resolved it to.
fn common_root(sources: &[(PathBuf, String)]) -> Option<PathBuf> {
    let mut root: PathBuf = sources.first()?.0.parent()?.to_path_buf();
    for (path, _) in sources.iter().skip(1) {
        let dir = path.parent()?;
        while !dir.starts_with(&root) {
            if !root.pop() {
                return None;
            }
        }
    }
    Some(root)
}

/// `app.nvs` -> `app` (`app.exe` on Windows), in the working directory.
///
/// The file stem is the only name the compiler has for a program, the same
/// reasoning `run_build`'s OpenAPI half already writes down: nothing in the
/// language declares one.
fn default_output(entry: &Path) -> PathBuf {
    let stem = entry
        .file_stem()
        .map_or_else(|| "app".to_owned(), |s| s.to_string_lossy().into_owned());
    PathBuf::from(if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem
    })
}

/// The manifest bytes for a file list — § 2's flat list, and nothing else.
fn manifest(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(files.len()).unwrap_or(u32::MAX).to_le_bytes());
    for (name, bytes) in files {
        let path = name.as_bytes();
        out.extend_from_slice(&u32::try_from(path.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(path);
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

/// Copies this `nvs` binary, appends the payload, and makes the result
/// runnable.
fn write_bundle(files: &[(String, Vec<u8>)], out: &Path) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let mut host = fs::read(&exe)?;
    // Rebundling from a bundle is § 5's ordinary case — the author reruns the
    // command — so the host is this binary with any payload of its own cut off
    // rather than a second payload stacked on the first.
    if let Ok(Some(offset)) = payload_offset(&exe) {
        host.truncate(usize::try_from(offset).unwrap_or(host.len()));
    }

    let body = manifest(files);
    let manifest_offset = host.len() as u64;
    let manifest_len = body.len() as u64;

    let mut bytes = host;
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&manifest_offset.to_le_bytes());
    bytes.extend_from_slice(&manifest_len.to_le_bytes());

    fs::write(out, &bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(out)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(out, perms)?;
    }
    // § 5: appending after a Mach-O's signature invalidates it, so the result
    // is ad-hoc signed here. A failure is a warning rather than a failed build —
    // this module's *What is appended* says why.
    #[cfg(target_os = "macos")]
    {
        let signed = std::process::Command::new("codesign")
            .args(["--sign", "-", "--force"])
            .arg(out)
            .status();
        match signed {
            Ok(status) if status.success() => {}
            _ => eprintln!(
                "warning: `codesign --sign -` did not succeed; {} is unsigned and macOS \
                 will refuse to run it until it is signed",
                out.display()
            ),
        }
    }
    Ok(())
}

/// The offset the payload starts at in `path`, or `None` if it carries none.
fn payload_offset(path: &Path) -> std::io::Result<Option<u64>> {
    let mut file = fs::File::open(path)?;
    let len = file.seek(SeekFrom::End(0))?;
    if len < footer_len() {
        return Ok(None);
    }
    file.seek(SeekFrom::End(
        -i64::try_from(FOOTER_LEN).expect("a 22-byte footer fits in an i64"),
    ))?;
    let mut footer = [0_u8; FOOTER_LEN];
    file.read_exact(&mut footer)?;
    if &footer[0..4] != MAGIC || u16::from_le_bytes([footer[4], footer[5]]) != FORMAT_VERSION {
        return Ok(None);
    }
    let offset = u64::from_le_bytes(footer[6..14].try_into().expect("eight bytes"));
    let manifest_len = u64::from_le_bytes(footer[14..22].try_into().expect("eight bytes"));
    if offset
        .saturating_add(manifest_len)
        .saturating_add(footer_len())
        != len
    {
        return Ok(None);
    }
    Ok(Some(offset))
}

/// Reads a host binary's payload back into § 2's file list.
///
/// A footer that does not add up, or a manifest that runs off its own end, is
/// `Ok(None)`: the file is then simply not a bundle. Nothing here trusts a
/// length it has not first checked against the bytes it actually holds.
fn read_payload(path: &Path) -> std::io::Result<Option<Bundle>> {
    let Some(offset) = payload_offset(path)? else {
        return Ok(None);
    };
    let mut file = fs::File::open(path)?;
    let len = file.seek(SeekFrom::End(0))?;
    let manifest_len = len - offset - footer_len();
    file.seek(SeekFrom::Start(offset))?;
    let mut body = vec![0_u8; usize::try_from(manifest_len).unwrap_or(0)];
    file.read_exact(&mut body)?;
    Ok(parse_manifest(&body).and_then(Bundle::of))
}

/// § 2's flat list, or `None` for bytes that do not describe one.
fn parse_manifest(body: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    let mut at = 0_usize;
    let mut take = |n: usize| -> Option<&[u8]> {
        let end = at.checked_add(n)?;
        let slice = body.get(at..end)?;
        at = end;
        Some(slice)
    };
    let count = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
    let mut files = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let path_len = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
        let name = String::from_utf8(take(path_len)?.to_vec()).ok()?;
        let text_len = usize::try_from(u64::from_le_bytes(take(8)?.try_into().ok()?)).ok()?;
        files.push((name, take(text_len)?.to_vec()));
    }
    Some(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest round-trips: what § 2 writes is what § 4 reads, entry file
    /// first and every byte of every file intact.
    #[test]
    fn a_manifest_round_trips_its_file_list() {
        let files = vec![
            (
                "app.nvs".to_owned(),
                b"<?nvs\nrequire \"lib/db.nvs\";\n".to_vec(),
            ),
            ("lib/db.nvs".to_owned(), b"<?nvs\nclass Db {}\n".to_vec()),
        ];
        let read = parse_manifest(&manifest(&files)).expect("the bytes just written parse");
        assert_eq!(read, files);
    }

    /// An embedded `.nvsx` and its entries come back apart from the source, the file's bytes
    /// whole even where they are not UTF-8.
    #[test]
    fn a_payload_splits_into_source_and_embedded_extensions() {
        let files = vec![
            ("app.nvs".to_owned(), b"<?nvs\n".to_vec()),
            (
                format!("{EXTENSION_PREFIX}0/shop.nvsx"),
                vec![0, 0x61, 0xff],
            ),
            (EXTENSION_ENTRIES.to_owned(), b"[[extension]]\n".to_vec()),
        ];
        let bundle = parse_manifest(&manifest(&files))
            .and_then(Bundle::of)
            .expect("the bytes just written parse");
        assert_eq!(bundle.files, [("app.nvs".to_owned(), "<?nvs\n".to_owned())]);
        assert_eq!(
            bundle.extensions,
            [("0/shop.nvsx".to_owned(), vec![0, 0x61, 0xff])]
        );
        assert_eq!(bundle.entries.as_deref(), Some("[[extension]]\n"));
    }

    /// An `extension.<index>` origin moves by the count of embedded entries, and every other key
    /// stays.
    #[test]
    fn an_extension_origin_moves_past_the_embedded_entries() {
        assert_eq!(shifted("extension.0.path", 2), "extension.2.path");
        assert_eq!(shifted("extension.11", 1), "extension.12");
        assert_eq!(
            shifted("extension.0.grants.read", 1),
            "extension.1.grants.read"
        );
        assert_eq!(shifted("ext.geo.key", 3), "ext.geo.key");
        assert_eq!(shifted("limits.memory", 3), "limits.memory");
    }

    /// A truncated manifest is `None` and never a panic: the bytes at the tail
    /// of an executable are input, and this is the only reader of them.
    #[test]
    fn a_truncated_manifest_is_refused_rather_than_trusted() {
        let bytes = manifest(&[("a.nvs".to_owned(), b"<?nvs\n".to_vec())]);
        for cut in 1..bytes.len() {
            assert!(
                parse_manifest(&bytes[..cut]).is_none(),
                "{cut} of {} bytes is not a whole manifest",
                bytes.len()
            );
        }
    }
}
