//! A package's external files: each `[[download]]` entry fetched, checked and placed
//! (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`).
//!
//! **The form is checked wherever an entry is read** ([`check_form`]): HTTPS URLs, a hash of 64
//! lowercase hex digits, a size above zero, a `license` that is an SPDX identifier when present,
//! and a path the archive's path rules accept that does not end in `.nvs` and is not under `src/`.
//! [`pack::file_set`] adds the last rule, a path the archive does not carry, so `nvs pkg pack`,
//! the registry's build and a git dependency's pack refuse the same entries.
//!
//! **Every download goes through [`nvs_stdlib::tool_stream`]**, the one TLS stack, with every
//! address of the first host and of each redirect hop judged by [`Outbound`] before anything
//! connects. The stream follows at most three redirects, inside the rule's five, and refuses a
//! hop into plaintext. The read stops one byte past the declared size, the SHA-256 hash is
//! checked before the file is renamed into the cache, and a mismatch tries the next URL.
//!
//! **The cache is keyed by SHA-256** ([`Store`]), under the data folder's `downloads/`, and is
//! opened only once the ownership check the artifact cache uses passes on it
//! (`rule:config/ownership-is-the-trust-boundary`). A file is kept once across projects and
//! versions. Each file is written to a temporary name in its shard and renamed, so a reader
//! never sees half of it.
//!
//! **A placed file is copied from the cache and hashed again while it is copied** ([`place`]), so
//! a cache file somebody changed is found, removed and downloaded again. Nothing is unpacked and
//! nothing runs (`rule:packaging/nothing-runs-before-the-program`).
//!
//! Cost: one copy buffer and one hash state per file while it is fetched or placed, and disk for
//! each file once in the cache and once per package folder. Nothing here runs on the request
//! path.

use std::fs;
use std::io::{self, Read, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rand::RngExt as _;
use sha2::{Digest as _, Sha256};

use std::process::ExitCode;

use super::fetch::{self, App, Failed};
use super::git::Outbound;
use super::index::{Fault, fault};
use super::manifest::{self, Download, Name};
use super::pack;

/// The name a refusal of the download names.
const MEMBER: &str = "nvs pkg";

/// The folder under the package that holds its code, where no download may go.
const SRC: &str = "src";

/// The time every download may take before the declared size is counted.
const TIME_BASE: Duration = Duration::from_secs(60);

/// The slowest rate a download is given time for, in bytes per second: 256 KiB/s.
const SLOWEST_RATE: u64 = 256 * 1024;

/// The largest file `nvs pkg download` reads before an entry declares its size: 4 GiB.
pub(crate) const MAX_NEW: u64 = 4 << 30;

/// The size of one read from the network or the disk.
const STEP: usize = 64 * 1024;

/// Whether each entry of `downloads` is well formed, apart from the files the archive carries,
/// which [`pack::file_set`] checks.
///
/// # Errors
///
/// The first entry's fault, as a sentence that names its path.
pub(crate) fn check_form(downloads: &[Download]) -> Result<(), String> {
    let mut seen: Vec<String> = Vec::new();
    for entry in downloads {
        let path = &entry.path;
        check_path(path)?;
        let folded = path.to_lowercase();
        if seen.contains(&folded) {
            return Err(format!("`[[download]]` names the path `{path}` twice."));
        }
        seen.push(folded);
        if entry.urls.is_empty() {
            return Err(format!(
                "the download `{path}` has no URL. Add one or more to `urls`."
            ));
        }
        if let Some(url) = entry.urls.iter().find(|url| !is_https(url)) {
            return Err(format!(
                "the download `{path}` names `{url}`. Every URL must start with `https://`."
            ));
        }
        if !is_sha256(&entry.sha256) {
            return Err(format!(
                "the download `{path}` has the hash `{}`. A SHA-256 hash is 64 lowercase hex \
                 digits. `nvs pkg download` writes it for you.",
                entry.sha256
            ));
        }
        if entry.size == 0 {
            return Err(format!(
                "the download `{path}` has the size 0. The size must be above zero."
            ));
        }
        if let Some(license) = &entry.license
            && !is_spdx(license)
        {
            return Err(format!(
                "the download `{path}` has the license `{license}`, which is not an SPDX \
                 identifier such as `OFL-1.1` or `MIT`."
            ));
        }
    }
    Ok(())
}

/// The path rules of one entry's `path`: a relative path to one file, written with `/`, that the
/// archive's rules accept, that does not end in `.nvs` and is not under `src/`.
fn check_path(path: &str) -> Result<(), String> {
    let bad = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..");
    if bad {
        return Err(format!(
            "the download path `{path}` must be the path of one file inside the package, \
             written with `/`, such as `fonts/Big.otf`."
        ));
    }
    pack::check_path(path).map_err(|refusal| refusal.to_string())?;
    if path.to_lowercase().ends_with(".nvs") {
        return Err(format!(
            "the download path `{path}` ends in `.nvs`. Code goes under `src/`, in the archive."
        ));
    }
    let first = path.split('/').next().unwrap_or_default();
    if first.eq_ignore_ascii_case(SRC) {
        return Err(format!(
            "the download path `{path}` is under `src/`. Put a downloaded file in another folder."
        ));
    }
    Ok(())
}

/// Whether `url` starts with `https://`, compared without case.
fn is_https(url: &str) -> bool {
    url.get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
        && url.len() > 8
}

/// Whether `text` is a SHA-256 hash: 64 lowercase hex digits.
fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Whether `text` has the form of an SPDX license identifier: letters, digits, `.`, `-` and a
/// final `+`, such as `OFL-1.1`, `GPL-2.0+` or `LicenseRef-Acme`.
fn is_spdx(text: &str) -> bool {
    let body = text.strip_suffix('+').unwrap_or(text);
    !body.is_empty()
        && body.len() <= 64
        && body
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

/// The host of `url`, as the report prints it: what is between `https://` and the next `/`, `?`
/// or `#`, lowercased.
pub(crate) fn host(url: &str) -> String {
    let rest = url.get(8..).unwrap_or_default();
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    rest[..end].to_ascii_lowercase()
}

/// The lowercase hex of `hash`.
fn hex(hash: &[u8]) -> String {
    use std::fmt::Write as _;
    hash.iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            // Writing into a `String` does not fail.
            let _ = write!(text, "{byte:02x}");
            text
        })
}

/// The time a download of `size` bytes may take: [`TIME_BASE`], and one second per
/// [`SLOWEST_RATE`] bytes.
fn time_for(size: u64) -> Duration {
    TIME_BASE.saturating_add(Duration::from_secs(size.div_ceil(SLOWEST_RATE)))
}

/// The SHA-256 cache: one file per hash, at `<dir>/<first two digits>/<hash>`.
#[derive(Debug, Clone)]
pub(crate) struct Store {
    dir: PathBuf,
}

impl Store {
    /// The cache at `dir`, once the ownership check passes on it, or on the nearest folder above
    /// it that exists.
    ///
    /// # Errors
    ///
    /// A folder owned by another account or writable by anybody but its owner.
    pub(crate) fn open(dir: PathBuf) -> Result<Self, Fault> {
        nvs_config::trust::check(crate::cache::existing_root(&dir)).map_err(|untrusted| {
            let help = match &untrusted {
                nvs_config::trust::Untrusted::Breach { help, .. } => format!(" {help}"),
                nvs_config::trust::Untrusted::Unreadable(_) => String::new(),
            };
            fault(format!(
                "the download cache `{}` cannot be used: {}.{help}",
                dir.display(),
                untrusted.message()
            ))
        })?;
        Ok(Self { dir })
    }

    /// The cache under the data folder's `downloads/`.
    ///
    /// # Errors
    ///
    /// No data folder, and [`Self::open`]'s.
    pub(crate) fn current() -> Result<Self, Fault> {
        let data = nvs_config::data::current().ok_or_else(|| {
            fault("there is no data folder to keep downloaded files in. Set `NVS_DATA_DIR`.")
        })?;
        Self::open(data.downloads())
    }

    /// Where the file with the hash `sha256` is kept.
    pub(crate) fn path(&self, sha256: &str) -> PathBuf {
        self.dir.join(sha256.get(..2).unwrap_or("00")).join(sha256)
    }

    /// The cached file of `entry`, downloaded from its URLs in order when the cache does not have
    /// it, each connection allowed only where `outbound` allows.
    ///
    /// # Errors
    ///
    /// Every URL failed: the fault names `package`, the path and what each URL returned.
    pub(crate) fn get(
        &self,
        package: &Name,
        entry: &Download,
        outbound: &Outbound,
    ) -> Result<PathBuf, Fault> {
        let cached = self.path(&entry.sha256);
        if cached.is_file() {
            return Ok(cached);
        }
        let mut failures = Vec::new();
        for url in &entry.urls {
            match self.download(url, entry, outbound) {
                Ok(path) => return Ok(path),
                Err(why) => failures.push(format!("`{url}`: {why}")),
            }
        }
        Err(fault(format!(
            "`{package}` needs the file `{}`, and no URL gave it. {}",
            entry.path,
            failures.join(". ")
        )))
    }

    /// `entry`'s file from `url`, checked and renamed into the cache.
    fn download(
        &self,
        url: &str,
        entry: &Download,
        outbound: &Outbound,
    ) -> Result<PathBuf, String> {
        let mut reply = request(url, entry.size, outbound)?;
        self.keep(&mut reply, entry).map_err(|why| why.to_string())
    }

    /// The bytes `from` gives, written into the cache under `entry`'s hash once their size and
    /// hash match it. A temporary file is written in the shard and renamed into place, and it is
    /// removed on every other way out.
    fn keep(&self, from: &mut dyn Read, entry: &Download) -> io::Result<PathBuf> {
        let cached = self.path(&entry.sha256);
        let shard = cached
            .parent()
            .expect("a cache path always has its shard above it");
        fs::create_dir_all(shard)?;
        let temp = shard.join(format!(".tmp-{:032x}", rand::rng().random::<u128>()));
        let written = copy_checked(from, &temp, entry).and_then(|()| fs::rename(&temp, &cached));
        if written.is_err() {
            // The fault below is the one to report; a file that will not go is a stray temporary.
            let _ = fs::remove_file(&temp);
        }
        written.map(|()| cached)
    }

    /// A new temporary file in the cache's folder, which exists.
    fn temporary(&self) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        Ok(self
            .dir
            .join(format!(".tmp-{:032x}", rand::rng().random::<u128>())))
    }

    /// The file at `url`, of at most [`MAX_NEW`] bytes, downloaded into the cache under the hash
    /// it has, with its size and hash. This is `nvs pkg download`'s read, before an entry pins it.
    ///
    /// # Errors
    ///
    /// A URL that is not `https:`, a refused address, a failed request, a status other than
    /// `200`, a file over [`MAX_NEW`], and a failed write.
    pub(crate) fn first(
        &self,
        url: &str,
        outbound: &Outbound,
    ) -> Result<(PathBuf, u64, String), String> {
        let mut reply = request(url, MAX_NEW, outbound)?;
        let temp = self.temporary().map_err(|err| err.to_string())?;
        let over = format!("the file is larger than {MAX_NEW} bytes");
        let kept = copy_hashed(&mut reply, &temp, MAX_NEW, &over).and_then(|(size, hash)| {
            let cached = self.path(&hash);
            fs::create_dir_all(cached.parent().expect("a cache path has its shard"))?;
            fs::rename(&temp, &cached)?;
            Ok((cached, size, hash))
        });
        if kept.is_err() {
            // The fault below is the one to report; a file that will not go is a stray temporary.
            let _ = fs::remove_file(&temp);
        }
        kept.map_err(|err| err.to_string())
    }

    /// `file`, which somebody obtained in another way, put into the cache when its size and hash
    /// match `entry`.
    ///
    /// # Errors
    ///
    /// A file that cannot be read, a size or a hash that differs, and a failed write.
    pub(crate) fn provide(&self, file: &Path, entry: &Download) -> io::Result<PathBuf> {
        let mut from = fs::File::open(file)?;
        self.keep(&mut from, entry)
    }
}

/// A `GET` of `url`, which must be `https:`, with a reply of at most `size` bytes read past by one
/// at most, connecting only where `outbound` allows.
fn request(url: &str, size: u64, outbound: &Outbound) -> Result<nvs_stdlib::ToolStream, String> {
    if !is_https(url) {
        return Err("the URL does not start with `https://`".to_owned());
    }
    let headers = vec![
        (
            "User-Agent".to_owned(),
            format!("nvs/{}", env!("CARGO_PKG_VERSION")),
        ),
        ("Accept-Encoding".to_owned(), "identity".to_owned()),
    ];
    let bounds = nvs_stdlib::ToolBounds {
        within: time_for(size),
        bytes: size.saturating_add(1),
    };
    let refused = |address| outbound.refused(address);
    let reply = nvs_stdlib::tool_stream(MEMBER, url, headers, None, bounds, &refused)?;
    if reply.status != 200 {
        return Err(format!("the status was {}", reply.status));
    }
    Ok(reply)
}

/// The size and the SHA-256 hash of the file at `path`, read in steps.
///
/// # Errors
///
/// A file that cannot be read.
pub(crate) fn measure(path: &Path) -> io::Result<(u64, String)> {
    let mut from = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buf = vec![0u8; STEP];
    loop {
        let read = from.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        size += u64::try_from(read).unwrap_or(u64::MAX);
    }
    Ok((size, hex(&hasher.finalize())))
}

/// The bytes of `from` written to the new file `to`, and their size and SHA-256 hash. The read
/// stops one byte past `limit`, and `over` is the error then.
fn copy_hashed(
    from: &mut dyn Read,
    to: &Path,
    limit: u64,
    over: &str,
) -> io::Result<(u64, String)> {
    let mut out = fs::File::create_new(to)?;
    let mut hasher = Sha256::new();
    let mut got = 0u64;
    let mut buf = vec![0u8; STEP];
    let ceiling = limit.saturating_add(1);
    loop {
        let left = usize::try_from(ceiling - got).unwrap_or(usize::MAX);
        let want = buf.len().min(left);
        if want == 0 {
            return Err(io::Error::other(over.to_owned()));
        }
        let read = from.read(&mut buf[..want])?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        out.write_all(&buf[..read])?;
        got += u64::try_from(read).unwrap_or(u64::MAX);
    }
    out.sync_all()?;
    Ok((got, hex(&hasher.finalize())))
}

/// The bytes of `from` written to the new file `to`, stopped one byte past `entry.size`, with
/// their size and SHA-256 hash checked against `entry` once the source ends.
fn copy_checked(from: &mut dyn Read, to: &Path, entry: &Download) -> io::Result<()> {
    let over = format!(
        "the file is larger than its declared size of {} bytes",
        entry.size
    );
    let (got, hash) = copy_hashed(from, to, entry.size, &over)?;
    if got != entry.size {
        return Err(io::Error::other(format!(
            "the file has {got} bytes, and its declared size is {} bytes",
            entry.size
        )));
    }
    if hash != entry.sha256 {
        return Err(io::Error::other(format!(
            "the file's SHA-256 hash is {hash}, and the declared hash is {}",
            entry.sha256
        )));
    }
    Ok(())
}

/// `entry`'s file placed at its path under the package folder `dir`, from `store`, downloaded
/// first when the cache does not have it. A file already there with the declared size and hash is
/// kept. A cache file whose bytes no longer match is removed and downloaded again.
///
/// # Errors
///
/// [`Store::get`]'s, and a file that cannot be written.
pub(crate) fn place(
    store: &Store,
    package: &Name,
    entry: &Download,
    dir: &Path,
    outbound: &Outbound,
) -> Result<(), Fault> {
    let target = dir.join(&entry.path);
    if measure(&target).is_ok_and(|(size, hash)| size == entry.size && hash == entry.sha256) {
        return Ok(());
    }
    let failed = |err: io::Error| {
        fault(format!(
            "cannot write `{}` of `{package}` to `{}`: {err}",
            entry.path,
            target.display()
        ))
    };
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(failed)?;
    }
    let mut partial = target.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    for attempt in 0..2 {
        let cached = store.get(package, entry, outbound)?;
        let _ = fs::remove_file(&partial);
        let copied =
            fs::File::open(&cached).and_then(|mut from| copy_checked(&mut from, &partial, entry));
        match copied {
            Ok(()) => return fs::rename(&partial, &target).map_err(failed),
            Err(err) => {
                let _ = fs::remove_file(&partial);
                if attempt == 1 || !cached.is_file() {
                    return Err(failed(err));
                }
                // The cache file changed after it was written: it goes, and the next pass
                // downloads it again.
                fs::remove_file(&cached).map_err(failed)?;
            }
        }
    }
    unreachable!("the second pass returns")
}

/// `nvs pkg download <url> --to <path>`, in the package whose `package.toml` is nearest the
/// working directory.
pub(crate) fn run_download(url: &str, to: &str) -> ExitCode {
    fetch::exit(add_entry(url, to))
}

/// [`run_download`]: the file fetched into the cache and placed at `to`, and its entry appended
/// to `package.toml`'s text, so every comment stays.
fn add_entry(url: &str, to: &str) -> Result<(), Failed> {
    let app = App::open()?;
    let Some(name) = app.manifest.name.clone() else {
        return Err(fault(format!(
            "`{}` has no `[package] name`. Only a package lists downloaded files.",
            app.path.display()
        ))
        .into());
    };
    check_path(to).map_err(fault)?;
    if let Some(had) = app
        .manifest
        .downloads
        .iter()
        .find(|entry| entry.path.eq_ignore_ascii_case(to))
    {
        return Err(fault(format!(
            "`[[download]]` already names `{}`. Remove that entry first to replace it.",
            had.path
        ))
        .into());
    }
    let store = Store::current()?;
    let (_, size, sha256) = store
        .first(url, &app.outbound)
        .map_err(|why| fault(format!("cannot download `{url}`: {why}")))?;
    let quoted = |text: &str| toml::Value::String(text.to_owned()).to_string();
    let original = std::fs::read_to_string(&app.path)
        .map_err(|err| fault(format!("cannot read `{}`: {err}", app.path.display())))?;
    let mut text = original.clone();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!(
        "\n[[download]]\npath = {}\nurls = [{}]\nsha256 = \"{sha256}\"\nsize = {size}\n",
        quoted(to),
        quoted(url)
    ));
    let manifest = app.reread(&text)?;
    check_form(&manifest.downloads).map_err(fault)?;
    let entry = manifest
        .downloads
        .iter()
        .find(|entry| entry.path == to)
        .expect("the entry was just written");
    place(&store, &name, entry, &app.dir, &app.outbound)?;
    fetch::replace_file(&app.path, &text)?;
    println!(
        "added [[download]] `{to}`: {size} bytes from {}, SHA-256 {sha256}",
        host(url)
    );
    println!("note: add `license = \"<SPDX identifier>\"` to the entry if the file has a license");
    Ok(())
}

/// `nvs pkg provide <file>`, for the application whose `package.toml` is nearest the working
/// directory.
pub(crate) fn run_provide(file: &Path) -> ExitCode {
    fetch::exit(provide(file))
}

/// [`run_provide`]: `file` put into the cache when a package in `package.lock` lists a file with
/// its size and hash. The package's folder under `vendor/` names the entries, so a fetch that
/// stopped at a dead URL has already written it.
fn provide(file: &Path) -> Result<(), Failed> {
    let app = App::open()?;
    let (size, sha256) =
        measure(file).map_err(|err| fault(format!("cannot read `{}`: {err}", file.display())))?;
    let Some(lock) = &app.old else {
        return Err(fault(format!(
            "`{}` has no `{}`. Run `nvs pkg fetch` first.",
            app.dir.display(),
            super::lock::FILE
        ))
        .into());
    };
    for locked in &lock.packages {
        let path = app
            .dir
            .join(fetch::VENDOR)
            .join(locked.name.slug())
            .join(nvs_config::grants::FILE);
        if !path.is_file() {
            continue;
        }
        let mut sources = nvs_diagnostics::SourceMap::new();
        let Ok(manifest) = manifest::read(&path, &mut sources) else {
            continue;
        };
        let Some(entry) = manifest
            .downloads
            .iter()
            .find(|entry| entry.size == size && entry.sha256 == sha256)
        else {
            continue;
        };
        Store::current()?
            .provide(file, entry)
            .map_err(|err| fault(format!("cannot keep `{}`: {err}", file.display())))?;
        println!(
            "`{}` is now in the download cache as `{}` of `{}`. Run `nvs pkg fetch` to place it.",
            file.display(),
            entry.path,
            locked.name
        );
        return Ok(());
    }
    Err(fault(format!(
        "no package in `{}` lists a file with the size {size} and the SHA-256 hash {sha256}. Only \
         a file a package lists can be provided.",
        super::lock::FILE
    ))
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An entry for the bytes `body`, at `path`.
    fn entry(path: &str, body: &[u8]) -> Download {
        Download {
            path: path.to_owned(),
            // Loopback, which the default table refuses before anything connects.
            urls: vec!["https://127.0.0.1/file".to_owned()],
            sha256: hex(&Sha256::digest(body)),
            size: u64::try_from(body.len()).expect("a test body fits"),
            license: Some("OFL-1.1".to_owned()),
        }
    }

    /// Each rule of the form refuses its entry, and a good entry passes.
    #[test]
    fn the_form_refuses_each_bad_entry() {
        let good = entry("fonts/Big.otf", b"font");
        check_form(std::slice::from_ref(&good)).expect("a good entry passes");
        type Change = fn(&mut Download);
        let cases: [(&str, Change); 9] = [
            ("`https://`", |e| {
                e.urls = vec!["http://example.com/f".to_owned()]
            }),
            ("has no URL", |e| e.urls.clear()),
            ("64 lowercase hex", |e| e.sha256 = "AB".repeat(32)),
            ("above zero", |e| e.size = 0),
            ("SPDX", |e| e.license = Some("MIT OR Apache".to_owned())),
            ("written with `/`", |e| e.path = "../outside.otf".to_owned()),
            ("ends in `.nvs`", |e| e.path = "fonts/Code.nvs".to_owned()),
            ("under `src/`", |e| e.path = "src/Big.otf".to_owned()),
            ("written with `/`", |e| e.path = "/abs.otf".to_owned()),
        ];
        for (said, change) in cases {
            let mut bad = good.clone();
            change(&mut bad);
            let why = check_form(&[bad]).expect_err(said);
            assert!(why.contains(said), "{said}: {why}");
        }
        let twice = check_form(&[good.clone(), good]).expect_err("a path twice");
        assert!(twice.contains("twice"), "{twice}");
    }

    /// A file of the right size and hash is kept in the cache, one past the size is stopped, and
    /// a wrong hash leaves nothing behind.
    #[test]
    fn the_cache_keeps_only_a_matching_file() {
        let scratch = nvs_repo::scratch("download-cache");
        let store = Store::open(scratch.join("downloads")).expect("the scratch folder is trusted");
        let good = entry("fonts/Big.otf", b"font");
        let kept = store
            .keep(&mut &b"font"[..], &good)
            .expect("a matching file is kept");
        assert_eq!(kept, store.path(&good.sha256));
        assert_eq!(fs::read(&kept).expect("it is there"), b"font");

        let longer = store
            .keep(&mut &b"fonts"[..], &entry("a", b"xxxx"))
            .expect_err("one byte past the size");
        assert!(longer.to_string().contains("larger"), "{longer}");
        let wrong = entry("a", b"other");
        let mismatch = store
            .keep(&mut &b"wrong"[..], &wrong)
            .expect_err("a wrong hash");
        assert!(mismatch.to_string().contains("SHA-256"), "{mismatch}");
        assert!(!store.path(&wrong.sha256).exists());
        let shard = store.path(&wrong.sha256);
        let left = fs::read_dir(shard.parent().expect("a shard"))
            .map(Iterator::count)
            .unwrap_or(0);
        assert_eq!(left, 0, "no temporary file stays");
    }

    /// A placed file is copied from the cache, and a cache file somebody changed is found.
    #[test]
    fn a_changed_cache_file_is_not_placed() {
        let scratch = nvs_repo::scratch("download-place");
        let store = Store::open(scratch.join("downloads")).expect("the scratch folder is trusted");
        let good = entry("fonts/Big.otf", b"font");
        store
            .keep(&mut &b"font"[..], &good)
            .expect("a matching file is kept");
        let name = Name::checked("Acme\\Fonts").expect("a name");
        let dir = scratch.join("pkg");
        place(&store, &name, &good, &dir, &Outbound::default()).expect("it is placed");
        assert_eq!(
            fs::read(dir.join("fonts/Big.otf")).expect("placed"),
            b"font"
        );

        fs::remove_file(dir.join("fonts/Big.otf")).expect("the test removes it");
        fs::write(store.path(&good.sha256), b"evil").expect("the test changes the cache");
        // The changed file is removed, and its URL, which the address table refuses, is tried.
        let fault = place(&store, &name, &good, &dir, &Outbound::default())
            .expect_err("the changed file is not placed");
        assert!(fault.0.contains("no URL gave it"), "{fault}");
        assert!(!dir.join("fonts/Big.otf").exists());
        assert!(!store.path(&good.sha256).exists());
    }
}
