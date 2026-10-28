//! A root git dependency: one exact commit of a repository, written out and packed as a
//! registry archive is (`rule:packaging/a-git-dependency-is-root-only`).
//!
//! **Only the commit's tree is read**, never its history, and only the paths the package's archive
//! can carry are written: `package.toml`, the files at the root, everything under `src/`, and the
//! `[package] files` list (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
//! [`pack::file_set`] then reads that directory exactly as `nvs pkg pack` reads a package, so a git
//! package's digest is the digest of the same bytes a registry would serve. A symlink or a
//! submodule at one of those paths is refused, as `nvs pkg pack` refuses a symlink, and every
//! other path of the repository is never written.
//!
//! **A `file:` remote is opened where it is**, with gix's isolated options, so no configuration
//! file, hook or helper program of that repository or of this user is read or run. It reaches only
//! the user's own disk. An `https:` remote is fetched at the commit alone, at depth 1, into a
//! new bare repository in a scratch folder. `gix` speaks the protocol, and every request it makes
//! goes through [`nvs_stdlib::tool_stream`], the TLS stack `Core\Http\Client` uses. Each address
//! the host resolves to is judged by [`Outbound`] before anything connects, so an internal host
//! is refused unless the operator's `[capabilities.net] internal` key names it.
//!
//! **Two limits bound the clone** ([`Limits`]): the bytes received and read, and the time since
//! the clone started. Every request runs inside what is left of the time, every body byte
//! received counts against the size, and both are checked again before each object is read.
//!
//! Cost: the archive in memory, at most [`pack::MAX_ARCHIVE_BYTES`], one `POST` body `gix` writes
//! (its list of wants, a few KiB), and the written files and fetched pack in scratch folders that
//! are removed on every way out. Nothing here runs on the request path.

use std::cell::{Cell, RefCell};
use std::fs;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use gix::bstr::ByteSlice as _;
use gix::error::ErrorExt;
use gix_transport::client::blocking_io::http;
use nvs_diagnostics::SourceMap;

use super::index::{Fault, fault};
use super::manifest::{self, Manifest};
use super::pack;

/// The limits a clone runs inside.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// The most bytes the clone may read, counted over every object.
    pub(crate) bytes: u64,
    /// The longest the clone may take.
    pub(crate) time: Duration,
}

/// The limits of `rule:packaging/the-registry-builds-the-archive-from-git`: 512 MiB, and five
/// minutes, which fetches the size limit at about 2 MiB/s.
pub(crate) const LIMITS: Limits = Limits {
    bytes: 512 * 1024 * 1024,
    time: Duration::from_secs(300),
};

/// A cloned package: its manifest, its archive and the archive's digest.
#[derive(Debug)]
pub(crate) struct Cloned {
    pub(crate) manifest: Manifest,
    pub(crate) archive: Vec<u8>,
    pub(crate) digest: blake3::Hash,
}

/// The addresses an `https:` clone may connect to. Every address in the outbound address table
/// is refused (`rule:security/net-address-policy`), except the ones the operator's
/// `[capabilities.net] internal` key names.
#[derive(Debug, Clone, Default)]
pub(crate) struct Outbound {
    /// The application's written `[capabilities]`, or `None` where it writes none.
    written: Option<nvs_config::tree::Capabilities>,
}

impl Outbound {
    /// The addresses `written`'s `[capabilities.net] internal` key excepts from the table.
    pub(crate) fn new(written: Option<nvs_config::tree::Capabilities>) -> Self {
        Self { written }
    }

    /// Why the clone may not connect to `address`, or `None` where it may.
    fn refused(&self, address: IpAddr) -> Option<String> {
        let range = match &self.written {
            Some(caps) => caps.address_refused(address),
            None => nvs_config::capability::denied_by_default(address),
        }?;
        Some(format!(
            "resolves to `{address}`, which is {range}. To allow it, write `{}` under \
             `[capabilities.net]`.",
            nvs_config::capability::internal_line(address)
        ))
    }
}

/// The package at `url`'s commit `rev`, written out under a fresh folder in `scratch` and packed.
/// A relative `file:` URL resolves against `base`, and an `https:` one connects only where
/// `outbound` allows.
///
/// # Errors
///
/// A URL that is neither `file:` nor `https:`, a repository that cannot be opened or fetched, a
/// host `outbound` refuses, a commit the repository does not have, a clone over `limits`, a
/// symlink or a submodule where the archive reads, a package that does not pack, and a scratch
/// folder that cannot be written.
pub(crate) fn clone(
    url: &str,
    rev: &str,
    base: &Path,
    scratch: &Path,
    limits: Limits,
    outbound: &Outbound,
) -> Result<Cloned, Fault> {
    let started = Instant::now();
    let id = gix::ObjectId::from_hex(rev.as_bytes())
        .map_err(|err| fault(format!("`{rev}` is not a commit hash: {err}")))?;
    let folder = Folder::new(scratch)?;
    // The bare repository an `https:` remote is fetched into, beside the folder that is packed.
    let store;
    let repo = if let Some(path) = url.strip_prefix("file:") {
        gix::open_opts(base.join(path), gix::open::Options::isolated())
            .map_err(|err| fault(format!("cannot open the repository `{url}`: {err}")))?
    } else if url.starts_with("https:") {
        store = Folder::new(scratch)?;
        fetched(url, id, &store.0, limits, outbound, started)?
    } else {
        return Err(fault(format!("`{url}` is not a `file:` or `https:` URL")));
    };
    let mut walk = Walk {
        repo: &repo,
        url,
        limits,
        started,
        read: 0,
    };
    let manifest = walk.write(id, &folder.0)?;
    let entries = pack::file_set(&folder.0, &manifest)
        .map_err(|refusal| fault(format!("`{url}` at `{rev}` does not pack: {refusal}")))?;
    let mut archive = Vec::new();
    let digest = pack::write(&entries, &mut archive)
        .map_err(|refusal| fault(format!("`{url}` at `{rev}` does not pack: {refusal}")))?;
    Ok(Cloned {
        manifest,
        archive,
        digest,
    })
}

/// A scratch folder of its own, removed when it is dropped.
struct Folder(PathBuf);

impl Folder {
    /// A new, empty folder under `scratch`, named after this process and a counter.
    fn new(scratch: &Path) -> Result<Self, Fault> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = scratch.join(format!("nvs-git-{}-{n}", std::process::id()));
        // A folder left by an earlier process with the same id is not this clone's.
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)
            .map_err(|err| fault(format!("cannot make `{}`: {err}", path.display())))?;
        Ok(Self(path))
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        // A folder that cannot be removed is left for the user's temporary-file cleanup.
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What an `https:` clone's refusals name.
const MEMBER: &str = "nvs pkg";

/// The commit `id` of the `https:` repository `url`, fetched with no history into a new bare
/// repository at `dir`, inside what is left of `limits` since `started`.
///
/// The repository is opened with isolated options, so no configuration file of this user is
/// read. A `401` is a fault like any other status, so no credential helper runs.
fn fetched(
    url: &str,
    id: gix::ObjectId,
    dir: &Path,
    limits: Limits,
    outbound: &Outbound,
    started: Instant,
) -> Result<gix::Repository, Fault> {
    let failed = |err: &dyn std::error::Error| fault(format!("cannot fetch `{url}`: {}", chain(err)));
    let repo = gix::ThreadSafeRepository::init_opts(
        dir,
        gix::create::Kind::Bare,
        gix::create::Options::default(),
        gix::open::Options::isolated(),
    )
    .map_err(|err| failed(&err))?
    .to_thread_local();
    let hex = id.to_hex().to_string();
    let remote = repo
        .remote_at(url)
        .map_err(|err| failed(&err))?
        .with_fetch_tags(gix::remote::fetch::Tags::None)
        .with_refspecs([hex.as_str()], gix::remote::Direction::Fetch)
        .map_err(|err| failed(&err))?;
    let location = gix::url::parse(url.as_bytes().as_bstr()).map_err(|err| failed(&err))?;
    let link = Rc::new(Link {
        outbound: outbound.clone(),
        limits,
        started,
        received: Cell::new(0),
    });
    let transport = http::connect_http(
        Checked(link),
        location,
        gix::protocol::transport::Protocol::V2,
        false,
    );
    remote
        .to_connection_with_transport(transport)
        .prepare_fetch(gix::progress::Discard, gix::remote::ref_map::Options::default())
        .map_err(|err| failed(&err))?
        .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(NonZeroU32::MIN))
        .receive(gix::progress::Discard, &AtomicBool::new(false))
        .map_err(|err| failed(&err))?;
    Ok(repo)
}

/// `err` and every error that caused it, joined, so the cause a reader can act on is printed.
fn chain(err: &dyn std::error::Error) -> String {
    let mut text = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        let said = cause.to_string();
        if !text.contains(&said) {
            text = format!("{text}: {said}");
        }
        source = cause.source();
    }
    text
}

/// What every request of one `https:` clone shares: where it may connect, its limits, and the
/// body bytes received so far.
struct Link {
    outbound: Outbound,
    limits: Limits,
    started: Instant,
    received: Cell<u64>,
}

impl Link {
    /// One request to `url` through [`nvs_stdlib::tool_stream`], inside what is left of the
    /// limits: the reply's header lines and its body. A status outside `2xx` is an error.
    fn send(
        self: &Rc<Self>,
        url: &str,
        headers: Vec<(String, String)>,
        body: Option<(&str, Vec<u8>)>,
    ) -> io::Result<(Vec<u8>, Counted)> {
        let within = self
            .limits
            .time
            .checked_sub(self.started.elapsed())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| self.too_slow())?;
        let bounds = nvs_stdlib::ToolBounds {
            within,
            bytes: self.limits.bytes.saturating_sub(self.received.get()),
        };
        let refused = |address| self.outbound.refused(address);
        let reply = nvs_stdlib::tool_stream(MEMBER, url, headers, body, bounds, &refused)
            .map_err(|why| self.failed(why))?;
        if !(200..300).contains(&reply.status) {
            return Err(io::Error::other(format!(
                "`{url}` answered with the status {}",
                reply.status
            )));
        }
        let head = reply
            .headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\n"))
            .collect::<String>()
            .into_bytes();
        Ok((
            head,
            Counted {
                link: Rc::clone(self),
                stream: reply,
            },
        ))
    }

    /// `why` a request failed, or the time limit where the limit is what ended it.
    fn failed(&self, why: String) -> io::Error {
        if self.started.elapsed() >= self.limits.time {
            return self.too_slow();
        }
        io::Error::other(why)
    }

    /// The time limit's error.
    fn too_slow(&self) -> io::Error {
        io::Error::other(format!(
            "the clone took longer than {} seconds, so it was stopped",
            self.limits.time.as_secs()
        ))
    }
}

/// A reply's body, with every byte read counted against the clone's size limit.
struct Counted {
    link: Rc<Link>,
    stream: nvs_stdlib::ToolStream,
}

impl Read for Counted {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self
            .stream
            .read(buf)
            .map_err(|err| self.link.failed(err.to_string()))?;
        let total = self
            .link
            .received
            .get()
            .saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        self.link.received.set(total);
        if total > self.link.limits.bytes {
            return Err(io::Error::other(format!(
                "the clone is larger than {} bytes, so it was stopped",
                self.link.limits.bytes
            )));
        }
        Ok(read)
    }
}

/// `gix`'s HTTP transport over [`nvs_stdlib::tool_stream`]: the one TLS stack, with every
/// address checked against [`Outbound`] (the standing decision *A git clone runs inside fixed
/// limits*).
struct Checked(Rc<Link>);

/// One request and its reply. A `GET` is sent at once. A `POST` is sent when its reply is first
/// read, because `gix` writes the body after [`http::Http::post`] returns.
struct Exchange {
    link: Rc<Link>,
    state: State,
}

/// Where an [`Exchange`] is.
enum State {
    /// A `POST` whose body `gix` is still writing.
    Unsent {
        url: String,
        headers: Vec<(String, String)>,
        content_type: String,
        body: Vec<u8>,
    },
    /// The reply's header lines, and its body until a reader takes it.
    Sent {
        head: Vec<u8>,
        body: Option<Counted>,
    },
    /// Why the request failed.
    Failed(String),
}

impl Exchange {
    /// Sends the request where it is not sent yet.
    fn send(&mut self) -> io::Result<()> {
        let state = std::mem::replace(&mut self.state, State::Failed(String::new()));
        self.state = match state {
            State::Unsent {
                url,
                headers,
                content_type,
                body,
            } => match self.link.send(&url, headers, Some((&content_type, body))) {
                Ok((head, body)) => State::Sent {
                    head,
                    body: Some(body),
                },
                Err(err) => State::Failed(err.to_string()),
            },
            other => other,
        };
        match &self.state {
            State::Failed(why) => Err(io::Error::other(why.clone())),
            _ => Ok(()),
        }
    }
}

/// A reply's header lines, `Name: value` each, read once the request is sent.
struct Head(Rc<RefCell<Exchange>>, Option<Cursor<Vec<u8>>>);

impl Head {
    fn lines(&mut self) -> io::Result<&mut Cursor<Vec<u8>>> {
        if self.1.is_none() {
            let mut exchange = self.0.borrow_mut();
            exchange.send()?;
            let head = match &exchange.state {
                State::Sent { head, .. } => head.clone(),
                _ => Vec::new(),
            };
            drop(exchange);
            self.1 = Some(Cursor::new(head));
        }
        Ok(self.1.get_or_insert_with(|| Cursor::new(Vec::new())))
    }
}

impl Read for Head {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.lines()?.read(buf)
    }
}

impl BufRead for Head {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.lines()?.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        if let Some(lines) = &mut self.1 {
            lines.consume(amount);
        }
    }
}

/// A reply's body, taken from its [`Exchange`] once the request is sent.
struct Body(Rc<RefCell<Exchange>>, Option<BufReader<Counted>>);

impl Body {
    fn reader(&mut self) -> io::Result<&mut BufReader<Counted>> {
        if self.1.is_none() {
            let mut exchange = self.0.borrow_mut();
            exchange.send()?;
            let body = match &mut exchange.state {
                State::Sent { body, .. } => body.take(),
                _ => None,
            };
            drop(exchange);
            self.1 = body.map(BufReader::new);
        }
        self.1
            .as_mut()
            .ok_or_else(|| io::Error::other("the reply's body was already read"))
    }
}

impl Read for Body {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader()?.read(buf)
    }
}

impl BufRead for Body {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader()?.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        if let Some(reader) = &mut self.1 {
            reader.consume(amount);
        }
    }
}

/// The body `gix` writes for a `POST`, kept until the request is sent.
struct Upload(Rc<RefCell<Exchange>>);

impl Write for Upload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.0.borrow_mut().state {
            State::Unsent { body, .. } => {
                body.extend_from_slice(buf);
                Ok(buf.len())
            }
            _ => Err(io::Error::other("the request was already sent")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// `gix`'s header lines as names and values, with the content type apart.
fn split(
    headers: impl IntoIterator<Item = impl AsRef<str>>,
) -> (Vec<(String, String)>, Option<String>) {
    let mut content_type = None;
    let mut named = Vec::new();
    for line in headers {
        let Some((name, value)) = line.as_ref().split_once(':') else {
            continue;
        };
        let (name, value) = (name.trim(), value.trim());
        if name.eq_ignore_ascii_case("content-type") {
            content_type = Some(value.to_owned());
        } else {
            named.push((name.to_owned(), value.to_owned()));
        }
    }
    (named, content_type)
}

impl http::Http for Checked {
    type Headers = Head;
    type ResponseBody = Body;
    type PostBody = Upload;

    fn get(
        &mut self,
        url: &str,
        _base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> gix::error::Result<http::GetResponse<Head, Body>> {
        let (headers, _) = split(headers);
        let (head, body) = self.0.send(url, headers, None).map_err(ErrorExt::raise)?;
        let exchange = Rc::new(RefCell::new(Exchange {
            link: Rc::clone(&self.0),
            state: State::Sent {
                head,
                body: Some(body),
            },
        }));
        Ok(http::GetResponse {
            headers: Head(Rc::clone(&exchange), None),
            body: Body(exchange, None),
        })
    }

    fn post(
        &mut self,
        url: &str,
        _base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        _body: http::PostBodyDataKind,
    ) -> gix::error::Result<http::PostResponse<Head, Body, Upload>> {
        let (headers, content_type) = split(headers);
        let exchange = Rc::new(RefCell::new(Exchange {
            link: Rc::clone(&self.0),
            state: State::Unsent {
                url: url.to_owned(),
                headers,
                content_type: content_type.unwrap_or_default(),
                body: Vec::new(),
            },
        }));
        Ok(http::PostResponse {
            post_body: Upload(Rc::clone(&exchange)),
            headers: Head(Rc::clone(&exchange), None),
            body: Body(exchange, None),
        })
    }

    fn configure(&mut self, _config: &dyn std::any::Any) -> gix::error::Result {
        Ok(())
    }
}

/// One commit's tree being written out, with the bytes read so far.
struct Walk<'a> {
    repo: &'a gix::Repository,
    url: &'a str,
    limits: Limits,
    started: Instant,
    read: u64,
}

impl Walk<'_> {
    /// The commit `id`'s `package.toml`, read, and every path the archive can carry written
    /// under `dir`.
    fn write(&mut self, id: gix::ObjectId, dir: &Path) -> Result<Manifest, Fault> {
        let url = self.url;
        let commit = self
            .repo
            .find_commit(id)
            .map_err(|err| fault(format!("`{url}` has no commit `{id}`: {err}")))?;
        let tree = commit
            .tree_id()
            .map_err(|err| fault(format!("cannot read the commit `{id}` of `{url}`: {err}")))?
            .detach();
        let root = self.entries(tree)?;
        let Some(toml) = root
            .iter()
            .find(|(name, _, _)| name == nvs_config::grants::FILE)
        else {
            return Err(fault(format!(
                "`{url}` has no `{}` at the commit `{id}`",
                nvs_config::grants::FILE
            )));
        };
        let text = self.blob(toml, nvs_config::grants::FILE)?;
        let text = String::from_utf8(text).map_err(|_| {
            fault(format!(
                "the `{}` of `{url}` is not UTF-8",
                nvs_config::grants::FILE
            ))
        })?;
        let path = dir.join(nvs_config::grants::FILE);
        let manifest =
            manifest::parse(&mut SourceMap::new(), &path, &text).map_err(|diagnostic| {
                fault(format!(
                    "the `{}` of `{url}` does not read: {}",
                    nvs_config::grants::FILE,
                    diagnostic.message
                ))
            })?;
        for entry in &root {
            self.place(entry, "", dir, &manifest.files)?;
        }
        Ok(manifest)
    }

    /// The entries of the tree `id`: each name, its mode and its object.
    fn entries(
        &mut self,
        id: gix::ObjectId,
    ) -> Result<Vec<(String, gix::object::tree::EntryMode, gix::ObjectId)>, Fault> {
        let data = self.read(id, "a folder")?;
        let tree = gix::objs::TreeRef::from_bytes(&data, gix::hash::Kind::Sha1)
            .map_err(|err| fault(format!("cannot read a folder of `{}`: {err}", self.url)))?;
        tree.entries
            .iter()
            .map(|entry| {
                let name = entry.filename.to_str().map_err(|_| {
                    fault(format!("`{}` has a file name that is not UTF-8", self.url))
                })?;
                Ok((name.to_owned(), entry.mode, entry.oid.to_owned()))
            })
            .collect()
    }

    /// `entry`, at `prefix`, written under `dir` when the archive can carry it, and each entry
    /// of a folder the archive reads.
    fn place(
        &mut self,
        (name, mode, id): &(String, gix::object::tree::EntryMode, gix::ObjectId),
        prefix: &str,
        dir: &Path,
        files: &[String],
    ) -> Result<(), Fault> {
        let path = format!("{prefix}{name}");
        let in_src = path == "src" || path.starts_with("src/");
        let listed = files.contains(&path);
        let folder = format!("{path}/");
        let leads = files.iter().any(|file| file.starts_with(&folder));
        let at_root = prefix.is_empty();
        if mode.is_tree() {
            if !in_src && !leads {
                return Ok(());
            }
            check(&path, self.url)?;
            fs::create_dir_all(dir.join(&path))
                .map_err(|err| fault(format!("cannot write `{path}`: {err}")))?;
            for entry in self.entries(*id)? {
                self.place(&entry, &format!("{path}/"), dir, files)?;
            }
            return Ok(());
        }
        if !in_src && !listed && !at_root {
            return Ok(());
        }
        if mode.is_link() {
            return Err(fault(format!(
                "`{path}` in `{}` is a symlink. A package's files must be regular files.",
                self.url
            )));
        }
        if mode.is_commit() {
            return Err(fault(format!(
                "`{path}` in `{}` is a submodule. A package's files must be in its own repository.",
                self.url
            )));
        }
        check(&path, self.url)?;
        let bytes = self.blob(&(name.clone(), *mode, *id), &path)?;
        fs::write(dir.join(&path), bytes)
            .map_err(|err| fault(format!("cannot write `{path}`: {err}")))
    }

    /// The contents of the blob `entry`, at `path`.
    fn blob(
        &mut self,
        (_, _, id): &(String, gix::object::tree::EntryMode, gix::ObjectId),
        path: &str,
    ) -> Result<Vec<u8>, Fault> {
        self.read(*id, path)
    }

    /// The object `id`'s data, once the limits allow it. `what` names it in a fault.
    fn read(&mut self, id: gix::ObjectId, what: &str) -> Result<Vec<u8>, Fault> {
        let url = self.url;
        if self.started.elapsed() > self.limits.time {
            return Err(fault(format!(
                "the clone of `{url}` took longer than {} seconds, so it was stopped",
                self.limits.time.as_secs()
            )));
        }
        let header = self
            .repo
            .find_header(id)
            .map_err(|err| fault(format!("cannot read {what} of `{url}`: {err}")))?;
        self.read = self.read.saturating_add(header.size());
        if self.read > self.limits.bytes {
            return Err(fault(format!(
                "the clone of `{url}` is larger than {} bytes, so it was stopped",
                self.limits.bytes
            )));
        }
        let object = self
            .repo
            .find_object(id)
            .map_err(|err| fault(format!("cannot read {what} of `{url}`: {err}")))?;
        Ok(object.detach().data)
    }
}

/// A fault when `path`, from the repository at `url`, is not a path every platform can write.
fn check(path: &str, url: &str) -> Result<(), Fault> {
    pack::check_path(path).map_err(|refusal| fault(format!("`{url}`: {refusal}")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A repository at `dir` with one commit of `files`, each a path and its contents. The
    /// commit's hash.
    pub(crate) fn repository(dir: &Path, files: &[(&str, &[u8])]) -> String {
        let repo = gix::init_bare(dir).expect("the test makes a repository");
        let mut editor = repo
            .edit_tree(gix::ObjectId::empty_tree(gix::hash::Kind::Sha1))
            .expect("the test edits a tree");
        for (path, contents) in files {
            let blob = repo.write_blob(contents).expect("the test writes a file");
            editor
                .upsert(*path, gix::object::tree::EntryKind::Blob, blob)
                .expect("the test adds a file");
        }
        let tree = editor.write().expect("the test writes the tree");
        let person = gix::actor::Signature {
            name: "Test".into(),
            email: "test@example.com".into(),
            time: gix::date::Time::new(0, 0),
        };
        let mut committer = gix::date::parse::TimeBuf::default();
        let mut author = gix::date::parse::TimeBuf::default();
        let commit = repo
            .new_commit_as(
                person.to_ref(&mut committer),
                person.to_ref(&mut author),
                "one",
                tree,
                std::iter::empty::<gix::ObjectId>(),
            )
            .expect("the test commits");
        commit.id.to_hex().to_string()
    }

    /// A package `Acme\Git` with a class, a license and one file the archive does not carry.
    pub(crate) fn package(dir: &Path, extra: &[(&str, &[u8])]) -> String {
        let mut files: Vec<(&str, &[u8])> = vec![
            (
                "package.toml",
                b"[package]\nname = 'Acme\\Git'\nversion = \"1.0.0\"\n",
            ),
            ("LICENSE", b"MIT\n"),
            (
                "src/Main.nvs",
                b"<?nvs\nnamespace Acme\\Git;\n\nfinal class Main\n{\n}\n",
            ),
            ("docs/notes.txt", b"not in the archive\n"),
        ];
        files.extend_from_slice(extra);
        repository(dir, &files)
    }

    /// The clone of a `file:` remote is the package at that commit, packed: only the archive's
    /// files, with the digest the same directory packs to.
    #[test]
    fn a_file_remote_is_cloned_at_its_exact_commit() {
        let scratch = nvs_repo::scratch("git-exact");
        let rev = package(&scratch.join("repo"), &[]);
        let url = format!("file:{}", scratch.join("repo").display());
        let cloned = clone(&url, &rev, &scratch, &scratch, LIMITS, &Outbound::default()).expect("it clones");
        assert_eq!(
            cloned.manifest.name.as_ref().map(ToString::to_string),
            Some("Acme\\Git".to_owned())
        );
        let paths: Vec<String> = tar::Archive::new(cloned.archive.as_slice())
            .entries()
            .expect("the archive reads")
            .map(|entry| {
                entry
                    .expect("an entry reads")
                    .path()
                    .expect("a path reads")
                    .display()
                    .to_string()
            })
            .collect();
        assert_eq!(paths, ["LICENSE", "package.toml", "src/Main.nvs"]);
        assert_eq!(cloned.digest, blake3::hash(&cloned.archive));
        let again = clone(&url, &rev, &scratch, &scratch, LIMITS, &Outbound::default()).expect("it clones again");
        assert_eq!(again.digest, cloned.digest);
    }

    /// A commit the repository does not have is a fault that names it.
    #[test]
    fn a_missing_commit_is_refused() {
        let scratch = nvs_repo::scratch("git-missing");
        package(&scratch.join("repo"), &[]);
        let url = format!("file:{}", scratch.join("repo").display());
        let rev = "1".repeat(40);
        let fault = clone(&url, &rev, &scratch, &scratch, LIMITS, &Outbound::default()).expect_err("it is refused");
        assert!(fault.0.contains(&rev), "{fault}");
    }

    /// A repository over the size limit stops the clone.
    #[test]
    fn an_over_size_repository_is_stopped() {
        let scratch = nvs_repo::scratch("git-size");
        let big = vec![b'x'; 64 * 1024];
        let rev = package(&scratch.join("repo"), &[("src/Big.nvs", &big)]);
        let url = format!("file:{}", scratch.join("repo").display());
        let limits = Limits {
            bytes: 32 * 1024,
            ..LIMITS
        };
        let fault = clone(&url, &rev, &scratch, &scratch, limits, &Outbound::default()).expect_err("it is stopped");
        assert!(fault.0.contains("larger than 32768 bytes"), "{fault}");
    }

    /// An `https:` remote on loopback, with no `internal` exception written, is refused before
    /// anything connects, and the fault names the line that would allow it.
    #[test]
    fn an_internal_host_is_refused() {
        let scratch = nvs_repo::scratch("git-internal");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the test listens");
        listener
            .set_nonblocking(true)
            .expect("the test's listener does not block");
        let url = format!(
            "https://127.0.0.1:{}/acme/git",
            listener.local_addr().expect("it has an address").port()
        );
        let fault = clone(
            &url,
            &"1".repeat(40),
            &scratch,
            &scratch,
            LIMITS,
            &Outbound::default(),
        )
        .expect_err("it is refused");
        assert!(fault.0.contains("resolves to `127.0.0.1`"), "{fault}");
        assert!(fault.0.contains("internal = [\"127.0.0.1\"]"), "{fault}");
        assert_eq!(
            listener.accept().map(|_| ()).map_err(|err| err.kind()),
            Err(io::ErrorKind::WouldBlock),
            "nothing connected"
        );
    }

    /// A host that accepts the connection and never answers is stopped by the time limit. The
    /// operator's `internal` exception lets the clone reach loopback.
    #[test]
    fn a_silent_host_is_stopped_by_the_time_limit() {
        let scratch = nvs_repo::scratch("git-silent");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the test listens");
        let port = listener.local_addr().expect("it has an address").port();
        let (done, ended) = std::sync::mpsc::channel::<()>();
        let host = std::thread::spawn(move || {
            // The connection is held open, and silent, until the test ends.
            let held = listener.accept();
            let _ = ended.recv();
            drop(held);
        });
        let outbound = Outbound::new(Some(nvs_config::tree::Capabilities {
            net: Some(nvs_config::tree::CapNet {
                internal: Some(nvs_config::Setting::List(vec!["127.0.0.1".to_owned()])),
                ..Default::default()
            }),
            ..Default::default()
        }));
        let limits = Limits {
            time: Duration::from_secs(1),
            ..LIMITS
        };
        let fault = clone(
            &format!("https://127.0.0.1:{port}/acme/git"),
            &"1".repeat(40),
            &scratch,
            &scratch,
            limits,
            &outbound,
        )
        .expect_err("it is stopped");
        drop(done);
        host.join().expect("the test's host ends");
        assert!(fault.0.contains("took longer than 1 seconds"), "{fault}");
    }
}
