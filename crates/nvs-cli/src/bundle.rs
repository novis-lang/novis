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
//!           count x ( path_len: u32, path: UTF-8 bytes, text_len: u64, text: bytes )
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

/// One bundled program, as read back out of a host binary.
pub(crate) struct Bundle {
    /// § 2's flat list in payload order, entry file first.
    files: Vec<(String, String)>,
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
pub(crate) fn build(entry: &Path, out: Option<&Path>) -> ExitCode {
    let checked = match super::front_end(entry) {
        Ok(checked) => checked,
        Err(code) => return code,
    };

    let mut sources: Vec<(PathBuf, String)> = Vec::with_capacity(checked.files.len());
    for file in &checked.files {
        let src = checked.map.file(file.id);
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

    let mut files: Vec<(String, String)> = Vec::with_capacity(sources.len());
    for (path, text) in sources {
        let name = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        files.push((name, text));
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
            println!(
                "wrote {} ({} source file(s))",
                output.display(),
                files.len()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: could not write {}: {err}", output.display());
            ExitCode::FAILURE
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
fn manifest(files: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(files.len()).unwrap_or(u32::MAX).to_le_bytes());
    for (name, text) in files {
        let path = name.as_bytes();
        out.extend_from_slice(&u32::try_from(path.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(path);
        out.extend_from_slice(&(text.len() as u64).to_le_bytes());
        out.extend_from_slice(text.as_bytes());
    }
    out
}

/// Copies this `nvs` binary, appends the payload, and makes the result
/// runnable.
fn write_bundle(files: &[(String, String)], out: &Path) -> std::io::Result<()> {
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
    Ok(parse_manifest(&body).map(|files| Bundle { files }))
}

/// § 2's flat list, or `None` for bytes that do not describe one.
fn parse_manifest(body: &[u8]) -> Option<Vec<(String, String)>> {
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
        let text = String::from_utf8(take(text_len)?.to_vec()).ok()?;
        files.push((name, text));
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
                "<?nvs\nrequire \"lib/db.nvs\";\n".to_owned(),
            ),
            ("lib/db.nvs".to_owned(), "<?nvs\nclass Db {}\n".to_owned()),
        ];
        let read = parse_manifest(&manifest(&files)).expect("the bytes just written parse");
        assert_eq!(read, files);
    }

    /// A truncated manifest is `None` and never a panic: the bytes at the tail
    /// of an executable are input, and this is the only reader of them.
    #[test]
    fn a_truncated_manifest_is_refused_rather_than_trusted() {
        let bytes = manifest(&[("a.nvs".to_owned(), "<?nvs\n".to_owned())]);
        for cut in 1..bytes.len() {
            assert!(
                parse_manifest(&bytes[..cut]).is_none(),
                "{cut} of {} bytes is not a whole manifest",
                bytes.len()
            );
        }
    }
}
