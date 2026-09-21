//! `rule:http-server/a-request-resolves-in-five-steps`'s five steps: the request that arrived, resolved against the
//! table [`nvs_config::mount`] expanded at boot.
//!
//! ```text
//! 0. [server] health_path && the request path is exactly it  -> the probe answers
//! 1. longest match:  host mounts by prefix  ->  host-less mounts by prefix  ->  404
//! 2. strip the prefix
//! 3. [server] static  &&  the remainder is an existing non-.nvs file under the mount root  -> serve it
//! 4. dispatch == "path"  &&  the remainder is an existing .nvs file under the mount root   -> run it
//! 5. otherwise                                                                             -> run the mount's entry
//! ```
//!
//! **Step 0 is § 5's probe and it is not one of § 4's steps** — it runs ahead of
//! all of them, so no mount can shadow it and no application can answer it. That
//! ordering is what the endpoint is *for*: it reports that the process is alive
//! even when the application fails to compile, and a probe a mount could take
//! over would fail exactly when the answer matters and produce a restart loop no
//! restart fixes. It is off unless `[server] health_path` names a path
//! ([`nvs_config::server::health_path`], which is also where a path no request
//! could carry is refused), it checks nothing but its own liveness — a probe that
//! pinged the database would turn one slow database into a simultaneous outage
//! across every instance — and it reports no version and no build.
//!
//! **The table is the whole of what a request may reach, and it is already
//! literal.** § 2's rule is kept by [`nvs_config::mount::expand`], which walked
//! every `scan` glob against the disk before this server bound a socket; what
//! this module does is *select* one of its rows. Steps 3 and 4 are the one place
//! a remainder meets the filesystem at all, they run only where the two
//! `[server]` switches turn them on, and § 4 makes them development's `try_files
//! $uri /index.nvs` rather than a path construction: the remainder is refused
//! before it is joined ([`under`] below), the join is checked canonically
//! against the mount root afterwards, and a production deployment
//! (`dispatch = "entry"`, `static = false`) never runs either of them.
//!
//! # Decision: an unwritten switch is the closed one, not development's
//!
//! `rule:config/a-startup-default-is-never-flipped`
//! gives `dispatch` and `static` different defaults per mode — `path` and
//! on in development, `entry` and off in production. [`Table::new`] takes both
//! as decided values and [`Table::from_config`] reads only what was *written*,
//! defaulting to the production pair. Resolving a mode's defaults is the mode
//! slice's, and until it lands the absent value is the fail-closed one: a
//! deployment that gets development's dispatch by accident executes `.nvs` files
//! nobody enumerated, which is exactly what § 2 exists to prevent, while one that
//! gets production's by accident answers every request with its entry — a
//! feature missing, not a rule broken.
//!
//! # What a remainder may be
//!
//! A URL path segment is percent-decoded first and then has to *spell a name*:
//! empty, `.`, `..` and anything holding a separator, a NUL or a `:` are refused
//! outright rather than repaired, which is
//! `rule:errors/ambiguous-input-refused`'s
//! direction applied to the one string in this crate that comes from the peer.
//! Refused means the *step* does not apply, so the request falls through to step
//! 5 and the mount's entry answers it — a traversal attempt reaches the
//! application as a path it can 404 by itself, and reaches the filesystem not at
//! all. The decode runs before the check, so `%2e%2e%2f` is the same refusal
//! `../` is; a `%` that is not two hex digits is refused rather than taken
//! literally.
//!
//! Step 3 has one addition to that, and it is lexical too: a remainder ending in
//! `/` names a directory below the mount root, and § 4's **sole default
//! document** `index.html` is looked for inside it ([`default_document`]). Never
//! at the mount root, and never for step 4 — that is [`crate::statics`]'s docs
//! § *Decision*.
//!
//! **What it spends**, per `rule:programs/memory-priority`:
//! nothing per request that outlives it. A selection borrows its mount from the
//! table and owns one `PathBuf` — the file steps 3-5 chose — and steps 3 and 4
//! cost one `stat` and one `canonicalize` each, only where their switch is on.
//! The table itself is one allocation per configuration generation, shared by
//! every request and never rebuilt on the request path.
//!

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hyper::Request;
use hyper::body::Incoming;
use hyper::header::HOST;
use nvs_config::mount::Mounted;
use nvs_config::tree::Config;
use nvs_runtime::Inbound;

/// Which of § 4's two dispatch readings is in force.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dispatch {
    /// `dispatch = "entry"` — step 4 does not run, and every request the mount
    /// matched runs its entry. The default here, for this module's docs
    /// § *Decision*.
    #[default]
    Entry,
    /// `dispatch = "path"` — an existing `.nvs` file under the mount root runs
    /// itself, which is the `try_files $uri /index.nvs` every PHP application
    /// already deploys under.
    Path,
}

/// The one question § 4 steps 3 and 4 ask of a filesystem.
///
/// A trait for the reason [`nvs_config::resolve::Files`] is one: the cases below
/// assert containment against a symlink, and planting one is a privileged
/// operation on Windows. It is not that trait because the question is not that
/// question — a router asks for an existing **file**, canonical, and a directory
/// answers `None`, since § 4 serves an exact file and never a listing.
pub trait Existing {
    /// The canonical path of the regular file at `path`, or `None` for anything
    /// that is not one — absent, a directory, or unreadable.
    fn file(&self, path: &Path) -> Option<PathBuf>;
}

/// The real filesystem.
#[derive(Clone, Copy, Debug, Default)]
pub struct OnDisk;

impl Existing for OnDisk {
    /// `nvs_config::trust::canonical` rather than [`std::fs::canonicalize`],
    /// because a mount's own root was canonicalized by that function and a
    /// Windows verbatim `\\?\` prefix on one side of a `starts_with` is a
    /// containment check that answers `false` for every path in the tree.
    fn file(&self, path: &Path) -> Option<PathBuf> {
        if !std::fs::metadata(path).is_ok_and(|meta| meta.is_file()) {
            return None;
        }
        nvs_config::trust::canonical(path).ok()
    }
}

/// The mount table a running server selects from, with the two `[server]`
/// switches § 4 steps 3 and 4 are gated on.
#[derive(Clone, Debug)]
pub struct Table {
    /// The expanded rows, in [`nvs_config::mount::expand`]'s order. Step 1 scans
    /// them: a table is a handful of mounts, and a request that walked a map
    /// would still have to walk it twice for host and host-less.
    mounts: Vec<Mounted>,
    /// § 4 step 4's switch.
    dispatch: Dispatch,
    /// § 4 step 3's switch.
    serve_static: bool,
    /// § 5's probe path, or `None` where this server reserves none. Step 0, and
    /// the module docs own why it is not a step of § 4's.
    health: Option<String>,
}

impl Table {
    /// The table, with both switches already decided and no probe reserved —
    /// [`with_health`](Table::with_health) is § 5's other half.
    #[must_use]
    pub fn new(mounts: Vec<Mounted>, dispatch: Dispatch, serve_static: bool) -> Self {
        Self {
            mounts,
            dispatch,
            serve_static,
            health: None,
        }
    }

    /// The same table, reserving `health` from every mount in it.
    ///
    /// Taken after the fact rather than as a fourth constructor argument because
    /// it is the one part of a table that is *not* about routing to an
    /// application: a table with no probe is the default state and the one every
    /// case that is about § 4 wants.
    #[must_use]
    pub fn with_health(mut self, health: Option<String>) -> Self {
        self.health = health;
        self
    }

    /// The two switches as `[server]` *wrote* them, over the production pair —
    /// this module's docs § *Decision* is why an unwritten one is not
    /// development's.
    #[must_use]
    pub fn from_config(mounts: Vec<Mounted>, config: &Config) -> Self {
        let server = config.server.as_ref();
        let dispatch = match server.and_then(|server| server.dispatch.as_deref()) {
            Some("path") => Dispatch::Path,
            _ => Dispatch::Entry,
        };
        let serve_static = server
            .and_then(|server| server.serve_static)
            .unwrap_or(false);
        // A path this server could not answer on is refused at boot by
        // `nvs_config::server::validate`, so the only tree that reaches here
        // with one is one nothing validated — and reading that as *no probe* is
        // this module's § *Decision* applied to a third switch: an unreadable
        // one reserves nothing rather than reserving something nobody wrote.
        let health = nvs_config::server::health_path(config, &BTreeMap::new())
            .ok()
            .flatten();
        Self::new(mounts, dispatch, serve_static).with_health(health)
    }

    /// Every mount this table holds, which is § 2's executable set in full.
    #[must_use]
    pub fn mounts(&self) -> &[Mounted] {
        &self.mounts
    }

    /// Step 0 and then § 4's five steps, over one arrived request.
    ///
    /// The host is the `Host` header's without its port, falling back to the
    /// request target's own authority for the absolute-form target h1 still
    /// allows. § 6's forwarded walk ([`crate::forwarded`]) does not reach this
    /// value and never will: that section reads **no** `X-Forwarded-Host` and
    /// generates no absolute URL from `Host`, because deriving an origin from a
    /// header is host-header injection and `rule:routing/matching-is-not-dispatching` already makes
    /// `Core\Router::url` answer with a path. What a trusted proxy may assert
    /// is the client address and the scheme, and neither is read here.
    #[must_use]
    pub fn select(&self, request: &Request<Incoming>, disk: &dyn Existing) -> Option<Resolved<'_>> {
        let host = request
            .headers()
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .map(named)
            .or_else(|| request.uri().host());
        self.resolve(host, request.uri().path(), disk)
    }

    /// Step 0 and § 4's five steps over a host and a path, which is
    /// [`select`](Table::select) with the request already taken apart.
    ///
    /// `path` is the request target's path alone — a query string is not part of
    /// what selects a mount, and no step below ever sees one. The probe is
    /// therefore matched against the path exactly and on every host: § 5 reserves
    /// **one** URL, and a probe that answered `/healthz?x=1` but not `/healthz`,
    /// or answered only on the host of whichever mount happened to cover it,
    /// would be a different endpoint on each deployment.
    #[must_use]
    pub fn resolve(
        &self,
        host: Option<&str>,
        path: &str,
        disk: &dyn Existing,
    ) -> Option<Resolved<'_>> {
        // 0. Ahead of step 1, so that no mount can shadow it — including a mount
        // at `/`, which covers every path there is.
        if self.health.as_deref() == Some(path) {
            return Some(Resolved::Health);
        }
        // 1.
        let mount = self.matched(host, path)?;
        // 2.
        let remainder = strip(&mount.prefix, path);
        // 3 and 4, in that order, and each only where its switch is on. One
        // extension test read from either end: § 4's own last sentence is that a
        // `.nvs` is never served as source, so step 3 wants the file that is not
        // one and step 4 wants the file that is.
        let under_root = |wanted_nvs: bool| {
            under(&mount.root, remainder, disk).filter(|file| is_nvs(file) == wanted_nvs)
        };
        let what = if self.serve_static
            && let Some(file) =
                under_root(false).or_else(|| default_document(mount, remainder, disk))
        {
            What::Static(file)
        } else if self.dispatch == Dispatch::Path
            && let Some(file) = under_root(true)
        {
            What::Run(file)
        } else {
            // 5.
            What::Run(mount.entry.clone())
        };
        Some(Resolved::Mounted(Selection {
            mount,
            what,
            path: remainder.into(),
        }))
    }

    /// Step 1: the longest prefix among the mounts answering on `host`, and then
    /// the longest among the host-less ones.
    ///
    /// The two passes are ordered rather than merged because § 4 says so, and
    /// because merging them would let a host-less `/blog` outrank a host mount's
    /// `/` — which is a request answered by the wrong tenant, not a longer
    /// match. A prefix matches on a **segment boundary**: `/blog` selects
    /// `/blog` and `/blog/post`, and never `/blogger`.
    fn matched(&self, host: Option<&str>, path: &str) -> Option<&Mounted> {
        let longest = |on_host: bool| {
            self.mounts
                .iter()
                .filter(|mount| match (&mount.host, host) {
                    (Some(mounted), Some(asked)) => on_host && mounted.eq_ignore_ascii_case(asked),
                    (None, _) => !on_host,
                    (Some(_), None) => false,
                })
                .filter(|mount| covers(&mount.prefix, path))
                .max_by_key(|mount| mount.prefix.len())
        };
        longest(true).or_else(|| longest(false))
    }
}

/// What the table made of one request: § 5's probe, or the mount § 4 selected.
///
/// Two variants rather than a third [`What`], because a probe hit has no mount
/// to carry — it is answered before step 1 has run, so there is nothing yet for
/// a [`Selection`] to borrow. A caller that only asks about applications takes
/// [`Resolved::selection`] and is back to § 4 alone; one that serves requests
/// has to say what it does with the probe, which is the point of making it a
/// variant instead of a flag beside the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved<'a> {
    /// Step 0: `[server] health_path` names exactly this path. The answer is
    /// `200` while the server accepts and `503` while it drains, with an empty
    /// body — a status and nothing else, since § 5's probe reports no version
    /// and runs no dependency check.
    Health,
    /// Steps 1 to 5: this mount, and what it says the request is.
    Mounted(Selection<'a>),
}

impl<'a> Resolved<'a> {
    /// The mount § 4 chose, or `None` for a probe hit.
    #[must_use]
    pub fn selection(self) -> Option<Selection<'a>> {
        match self {
            Self::Health => None,
            Self::Mounted(selection) => Some(selection),
        }
    }
}

/// One request's answer to § 4: which mount it selected, and what that mount
/// says the request is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection<'a> {
    /// The row step 1 chose. Its `prefix` and `captures` are `rule:routing/a-request-reads-its-mount`'s
    /// answer and cross onto the request through [`carry`]; its `origin` is what
    /// `Core\Router::urlAbsolute` prepends, and crosses onto the isolate that
    /// answers the request rather than onto its carrier, because an origin is a
    /// fact about the deployment and not about the request
    /// (`rule:routing/an-origin-is-per-mount-and-checked-at-boot`).
    pub mount: &'a Mounted,
    /// Steps 3, 4 and 5's outcome.
    pub what: What,
    /// Step 2's remainder: the request path with `mount.prefix` taken off the
    /// front, always beginning with `/` and still percent-encoded.
    ///
    /// Carried rather than left to the caller to recompute, because it is what
    /// `nvs_runtime::Inbound` holds as the request's path — a mounted
    /// application is written against its own root and never learns the prefix
    /// it was deployed under — and a second stripping at the door would be a
    /// second answer to step 2.
    ///
    /// Owned, at one short allocation per request: the remainder borrows the
    /// request target, which outlives neither the table this borrows from nor
    /// the [`Reply`](crate::Reply) the caller builds out of it, and threading a
    /// second lifetime through the whole table to save it would buy nothing
    /// `rule:programs/memory-priority` ranks above
    /// simplicity.
    pub path: Box<str>,
}

/// What § 4 decided the request is, once a mount had been selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum What {
    /// Step 3: this exact file's bytes, under `[server] static`. Never a
    /// directory and never a `.nvs`.
    Static(PathBuf),
    /// Steps 4 and 5: this file, run as `rule:security/isolate-shares-nothing`'s isolate. Step 4 is the
    /// remainder itself under `dispatch = "path"`, step 5 is the mount's entry,
    /// and both are a path the table or the disk answered for rather than one
    /// assembled from request bytes.
    ///
    Run(PathBuf),
}

/// An authority's host, without the port a client is entitled to send with it.
///
/// A `Host` header carries `name:port` and a mount's `host` is a name, so a
/// comparison against the header as it arrived would never match the very
/// deployment § 3's `host = "{1}.example.com"` is written for — a development
/// server on `:8123`. A bracketed IPv6 literal keeps its colons, since the port
/// is the one after the `]`.
fn named(authority: &str) -> &str {
    let after = authority.rfind(']').map_or(0, |at| at + 1);
    match authority[after..].find(':') {
        Some(at) => &authority[..after + at],
        None => authority,
    }
}

/// Whether `prefix` covers `path` on a segment boundary.
fn covers(prefix: &str, path: &str) -> bool {
    if prefix == "/" {
        return true;
    }
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Step 2: `path` with `prefix` taken off the front, always beginning with `/`.
///
/// A mount at `/blog` answers `/blog` with the remainder `/`, which is the same
/// remainder `/blog/` has: a module is reached at its own root by either
/// spelling, and § 4's steps 3 and 4 find nothing under either, so both run the
/// entry. That is what makes a module relocatable — the prefix is the only thing
/// that moved.
fn strip<'a>(prefix: &str, path: &'a str) -> &'a str {
    if prefix == "/" {
        return path;
    }
    match &path[prefix.len()..] {
        "" => "/",
        rest => rest,
    }
}

/// The remainder resolved under `root`, or `None` when it does not name an
/// existing file inside it.
///
/// Two independent checks, and the second is not the first done again. The
/// lexical one refuses a segment that does not spell a name, so nothing
/// traversing is ever *built*; the canonical one refuses a path that resolves
/// outside `root` anyway, which is the symlink the lexical half cannot see. § 2
/// is kept by both being true rather than by either.
///
/// A third refuses a remainder the disk holds **in another case**. A
/// case-insensitive filesystem finds `style.css` for `/STYLE.CSS`, and the
/// canonical path it returns is the entry's own spelling, so the two are
/// compared and a difference that is only case is a file that is not there —
/// what a case-sensitive disk already says. `rule:programs/path-case` is the
/// same comparison for a `require`, and it costs the same nothing: both paths
/// are already in hand.
fn under(root: &Path, remainder: &str, disk: &dyn Existing) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    let mut segments = 0_usize;
    for segment in remainder.split('/').filter(|segment| !segment.is_empty()) {
        let name = decode(segment)?;
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':', '\0']) {
            return None;
        }
        path.push(name);
        segments += 1;
    }
    if segments == 0 {
        // The remainder is the mount's own root, which names a directory and
        // never a file. Steps 3 and 4 both decline, and step 5 answers.
        return None;
    }
    let file = disk.file(&path)?;
    (file.starts_with(root) && !differs_only_in_case(&file, &path)).then_some(file)
}

/// Whether `resolved` is `asked` written in another case, and in nothing else.
///
/// A path that resolved through a link differs from what was asked in more than
/// case, and the containment check is what answers for it. The exception is a
/// link whose own name is its target's in another case, which is refused with
/// the rest: nothing here can tell it from the filesystem folding the name.
fn differs_only_in_case(resolved: &Path, asked: &Path) -> bool {
    fn folded(text: &str) -> impl Iterator<Item = char> + '_ {
        text.chars().flat_map(char::to_lowercase)
    }
    resolved != asked && folded(&resolved.to_string_lossy()).eq(folded(&asked.to_string_lossy()))
}

/// § 4's sole default document, or `None` where there is not one to serve.
///
/// `index.html` and nothing else, for a remainder that spells a directory
/// *below* the mount root — `/docs/` finds `docs/index.html` — and never for the
/// mount root itself, which is [`crate::statics`]'s docs § *Decision*: a mount
/// whose root holds both `index.nvs` and an `index.html` would otherwise stop
/// running its own application the moment `static` was turned on.
///
/// The trailing `/` is the whole test, which keeps this lexical: a remainder
/// that does not end in one is a *file* the peer named, and step 3 already
/// declined it. There is no redirect from `/docs` to `/docs/` because § 4 has no
/// spelling for one — that request reaches step 5 and the application answers
/// it. The join goes through [`under`] like every other, so a default document
/// is subject to the same two containment checks.
fn default_document(mount: &Mounted, remainder: &str, disk: &dyn Existing) -> Option<PathBuf> {
    if !remainder.ends_with('/') || remainder.split('/').all(str::is_empty) {
        return None;
    }
    under(&mount.root, &format!("{remainder}index.html"), disk)
}

/// One path segment with its percent escapes resolved, or `None` when it is not
/// a segment this server will look at.
///
/// A `%` that is not followed by two hex digits is a refusal rather than a
/// literal `%`, and bytes that do not spell UTF-8 are one too: both are
/// `rule:errors/ambiguous-input-refused`'s direction, and the alternative in each case is a filename the
/// operator did not write.
fn decode(segment: &str) -> Option<String> {
    if !segment.contains('%') {
        return Some(segment.to_string());
    }
    let raw = segment.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        if raw[at] == b'%' {
            let hex = raw.get(at + 1..at + 3)?;
            let text = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(text, 16).ok()?);
            at += 3;
        } else {
            out.push(raw[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// § 4's last sentence, as a predicate: **a `.nvs` file is never served as
/// source**, under any dispatch, from any mount. Step 3 excludes what this
/// answers `true` for and step 4 requires it, so the two are one extension test
/// read from either end.
fn is_nvs(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("nvs"))
}

/// Records `rule:routing/a-request-reads-its-mount`'s mount on the carrier the selected program will
/// answer: the prefix step 2 stripped, and § 3's glob captures of the row that
/// selected it.
///
/// **Nothing here searches.** Both are fields of the [`Mounted`] that
/// [`Table::resolve`] already chose, exactly as
/// [`crate::route::csrf_required`] reads a field of the match rather than
/// matching a second time — and for the same reason: the request was resolved
/// once, before any application code, and § 7 is a *reading* of that answer.
///
/// This is where the door's half of § 7 ends. What the two facts become for a
/// program is `Core\Request::mount()`'s, one crate above this one, and that is
/// also where § 7's `tainted` is declared: a qualifier is a fact about a
/// registry row's signature and there is no such thing to write here.
///
/// Called for every request that reached a mount at all, which is every request
/// but § 5's health probe — that one is answered ahead of step 1, so no mount
/// ever claims it and no carrier is built for it.
///
pub fn carry(mount: &Mounted, inbound: &mut Inbound) {
    // **The prefix as step 2 stripped it, not as the block wrote it.** `/` is
    // the one prefix that comes off nothing — [`strip`]'s first branch hands
    // the path straight back — so the mount at the root answers `""`, which is
    // the same answer a request whose door stripped nothing gives. Writing the
    // literal `/` instead would put it in front of every link a default
    // deployment builds, since `Core\Router::url` prepends this field to a path
    // that already begins with one.
    let stripped = if mount.prefix == "/" {
        ""
    } else {
        &mount.prefix
    };
    inbound.set_mount(stripped, &mount.captures);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// The files the cases describe, with a canonical name for a link.
    #[derive(Default)]
    struct Fake {
        files: Vec<PathBuf>,
        links: BTreeMap<PathBuf, PathBuf>,
    }

    /// A path written the way an ADR writes one, as the host spells it.
    fn p(path: &str) -> PathBuf {
        PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    impl Fake {
        fn with(paths: &[&str]) -> Self {
            Self {
                files: paths.iter().map(|path| p(path)).collect(),
                links: BTreeMap::new(),
            }
        }

        /// `link` is another name for `target`, resolved as a path walks it.
        fn linking(mut self, link: &str, target: &str) -> Self {
            self.links.insert(p(link), p(target));
            self
        }
    }

    impl Existing for Fake {
        fn file(&self, path: &Path) -> Option<PathBuf> {
            let mut out = PathBuf::new();
            for component in path.components() {
                out.push(component.as_os_str());
                if let Some(target) = self.links.get(&out) {
                    out.clone_from(target);
                }
            }
            self.files.contains(&out).then_some(out)
        }
    }

    /// The same described filesystem, read the way a **boot** reads one, so that
    /// a case can expand § 3's globs over exactly the tree its requests then
    /// probe through [`Existing`] above.
    ///
    /// It mirrors `nvs-config`'s own `tests/mount.rs` reader, which is where what
    /// each of these methods means is pinned. The one difference from the
    /// [`Existing`] half is that `list` reports directories as well as files: a
    /// glob's `*` is a directory listing, and a reader that returned only files
    /// would expand nothing.
    impl nvs_config::resolve::Files for Fake {
        fn trust(&self, path: &Path) -> Result<PathBuf, nvs_config::trust::Untrusted> {
            self.canonical(path)
                .map_err(nvs_config::trust::Untrusted::Unreadable)
        }

        fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
            let mut out = PathBuf::new();
            for component in path.components() {
                match component {
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        out.pop();
                    }
                    other => {
                        out.push(other.as_os_str());
                        if let Some(target) = self.links.get(&out) {
                            out.clone_from(target);
                        }
                    }
                }
            }
            if self.exists(&out) {
                Ok(out)
            } else {
                Err("no such file or directory".to_string())
            }
        }

        fn read(&self, _path: &Path) -> Result<String, String> {
            Err("no case here reads a mounted file".to_string())
        }

        fn read_bytes(&self, _path: &Path) -> Result<Vec<u8>, String> {
            Err("no case here reads a mounted file".to_string())
        }

        fn exposure(&self, _path: &Path) -> Option<String> {
            None
        }

        fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
            let mut out: Vec<PathBuf> = Vec::new();
            for path in &self.files {
                let Ok(rest) = path.strip_prefix(dir) else {
                    continue;
                };
                let Some(first) = rest.components().next() else {
                    continue;
                };
                let child = dir.join(first.as_os_str());
                if !out.contains(&child) {
                    out.push(child);
                }
            }
            Ok(out)
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.iter().any(|file| file.starts_with(path))
        }
    }

    /// One mount, as [`nvs_config::mount::expand`] would have produced it.
    fn mount(prefix: &str, host: Option<&str>, entry: &str) -> Mounted {
        let entry = p(entry);
        Mounted {
            prefix: prefix.to_string(),
            host: host.map(str::to_string),
            root: entry
                .parent()
                .expect("an entry has a directory")
                .to_path_buf(),
            entry,
            origin: None,
            captures: Vec::new(),
        }
    }

    /// § 4's five steps, asserted **in order** and not one at a time.
    ///
    /// The order is the whole rule: every request below matches the same mount
    /// and differs only in which step answers it, so a reading that ran step 4
    /// before step 3 serves `style.css` by running it, and one that skipped
    /// straight to step 5 answers all five lines identically while still looking
    /// right on any single one.
    #[test]
    fn a_request_resolves_through_the_five_steps_in_order() {
        let fs = Fake::with(&[
            "/www/public/index.nvs",
            "/www/public/style.css",
            "/www/public/admin.nvs",
            "/www/public/assets/logo.png",
        ])
        .linking("/www/public/away", "/secret");
        let table = Table::new(
            vec![mount("/", None, "/www/public/index.nvs")],
            Dispatch::Path,
            true,
        );
        let what = |path: &str| {
            table
                .resolve(None, path, &fs)
                .and_then(Resolved::selection)
                .expect("the root mount matches everything")
                .what
        };

        // 3: an existing non-`.nvs` file under the mount root, at either depth.
        assert_eq!(what("/style.css"), What::Static(p("/www/public/style.css")));
        assert_eq!(
            what("/assets/logo.png"),
            What::Static(p("/www/public/assets/logo.png"))
        );
        // 4: an existing `.nvs` file runs itself, and is never served as source
        // — which is the same assertion read from the other side, since step 3
        // ran first and declined.
        assert_eq!(what("/admin.nvs"), What::Run(p("/www/public/admin.nvs")));
        // 5: everything else is the mount's entry, including a route the
        // application owns and a file that is not there.
        assert_eq!(what("/orders/17"), What::Run(p("/www/public/index.nvs")));
        assert_eq!(what("/missing.css"), What::Run(p("/www/public/index.nvs")));
        // And the mount root itself, which names a directory: never a listing.
        assert_eq!(what("/"), What::Run(p("/www/public/index.nvs")));

        // Nothing a peer can spell reaches a path outside the mount root: the
        // lexical refusal and the canonical one are asserted together, because
        // a reading holding only the first mounts the symlink.
        for escape in ["/../secret", "/%2e%2e/secret", "/away", "/%zz", "/a%2fb"] {
            assert_eq!(
                what(escape),
                What::Run(p("/www/public/index.nvs")),
                "for {escape:?}"
            );
        }

        // The same table with both switches off is § 4's production sequence:
        // match, strip, run the entry, with steps 3 and 4 not running at all.
        let closed = Table::new(
            vec![mount("/", None, "/www/public/index.nvs")],
            Dispatch::Entry,
            false,
        );
        for path in ["/style.css", "/admin.nvs", "/orders/17"] {
            assert_eq!(
                closed
                    .resolve(None, path, &fs)
                    .and_then(Resolved::selection)
                    .expect("the root mount matches everything")
                    .what,
                What::Run(p("/www/public/index.nvs")),
                "for {path:?}"
            );
        }
    }

    /// Steps 3 and 4 over a disk that folds case, which is what Windows and
    /// macOS are: the file is found for any spelling, and only the entry's own
    /// spelling selects it. Every other one is step 5, as it is on a disk that
    /// never found the file at all — so a URL cannot work on a developer's
    /// machine and be a `404` on the server.
    #[test]
    fn a_remainder_in_another_case_is_a_file_that_is_not_there() {
        // A folded lookup is a link from the asked spelling to the entry's own,
        // which is exactly what `canonicalize` reports on such a disk.
        let fs = Fake::with(&[
            "/www/public/index.nvs",
            "/www/public/style.css",
            "/www/public/admin.nvs",
            "/www/public/assets/logo.png",
        ])
        .linking("/www/public/STYLE.CSS", "/www/public/style.css")
        .linking("/www/public/Admin.nvs", "/www/public/admin.nvs")
        .linking("/www/public/Assets", "/www/public/assets")
        // A link that is a different name, and not a different case, still serves.
        .linking("/www/public/theme.css", "/www/public/style.css");
        let table = Table::new(
            vec![mount("/", None, "/www/public/index.nvs")],
            Dispatch::Path,
            true,
        );
        let what = |path: &str| {
            table
                .resolve(None, path, &fs)
                .and_then(Resolved::selection)
                .expect("the root mount matches everything")
                .what
        };

        for folded in ["/STYLE.CSS", "/Admin.nvs", "/Assets/logo.png"] {
            assert_eq!(
                what(folded),
                What::Run(p("/www/public/index.nvs")),
                "for {folded:?}"
            );
        }
        assert_eq!(what("/style.css"), What::Static(p("/www/public/style.css")));
        assert_eq!(what("/theme.css"), What::Static(p("/www/public/style.css")));
        assert_eq!(what("/admin.nvs"), What::Run(p("/www/public/admin.nvs")));
    }

    /// Step 2, and what it buys: the same module answers the same remainders
    /// wherever it is mounted, and a host mount outranks a host-less one.
    #[test]
    fn a_prefix_is_stripped_and_the_module_is_relocatable() {
        let fs = Fake::with(&[
            "/www/blog/public/index.nvs",
            "/www/blog/public/style.css",
            "/www/shop/public/index.nvs",
            "/www/blogger/public/index.nvs",
        ]);
        let table = Table::new(
            vec![
                mount("/blog", None, "/www/blog/public/index.nvs"),
                mount("/blogger", None, "/www/blogger/public/index.nvs"),
                mount("/", None, "/www/shop/public/index.nvs"),
                mount("/", Some("blog.example.com"), "/www/blog/public/index.nvs"),
            ],
            Dispatch::Path,
            true,
        );
        let at = |host: Option<&str>, path: &str| {
            let selected = table
                .resolve(host, path, &fs)
                .and_then(Resolved::selection)
                .expect("a mount matches");
            (selected.mount.prefix.clone(), selected.what)
        };

        // The prefix is stripped, so the module's own `/style.css` is what it
        // sees whichever prefix it was reached through — and whether it was
        // reached by prefix or by host at all.
        let relocated = [
            at(None, "/blog/style.css"),
            at(Some("blog.example.com"), "/style.css"),
        ];
        for (_, what) in &relocated {
            assert_eq!(*what, What::Static(p("/www/blog/public/style.css")));
        }
        // Step 1's two passes are ordered: the host mount wins over the
        // host-less `/`, which a merged longest-prefix scan would have given to
        // the shop.
        assert_eq!(relocated[1].0, "/");
        assert_eq!(
            at(Some("blog.example.com"), "/anything").1,
            What::Run(p("/www/blog/public/index.nvs"))
        );
        // A host nothing mounts falls through to the host-less pass rather than
        // to a 404.
        assert_eq!(
            at(Some("other.example.com"), "/blog/style.css").1,
            What::Static(p("/www/blog/public/style.css"))
        );

        // Longest wins, and it wins on a segment boundary: `/blogger` is its own
        // module and not a path inside `/blog`.
        assert_eq!(at(None, "/blog").0, "/blog");
        assert_eq!(at(None, "/blogger").0, "/blogger");
        assert_eq!(at(None, "/blogging").0, "/");
        // A mount reached at its own root answers with its entry by either
        // spelling of it.
        assert_eq!(
            at(None, "/blog").1,
            What::Run(p("/www/blog/public/index.nvs"))
        );
        assert_eq!(
            at(None, "/blog/").1,
            What::Run(p("/www/blog/public/index.nvs"))
        );

        // Step 1's third arrow: with no mount at the root, a path nothing covers
        // is nothing this server can answer.
        let sparse = Table::new(
            vec![mount("/blog", None, "/www/blog/public/index.nvs")],
            Dispatch::Path,
            true,
        );
        assert!(sparse.resolve(None, "/shop", &fs).is_none());
        assert!(sparse.resolve(None, "/blog/style.css", &fs).is_some());
    }

    /// Step 0 against the table most able to swallow it: a mount at `/` with both
    /// switches on, a host mount beside it, and the probe pointed at a path that
    /// exists on disk as a static file.
    ///
    /// Every line is the same request answered twice — once with the probe
    /// reserved and once without — because a case that only asserted the hit
    /// would pass against a table that answered `Health` to everything, and one
    /// that only asserted the misses would pass against a probe wired to nothing.
    #[test]
    fn the_health_path_answers_ahead_of_every_mount() {
        let fs = Fake::with(&["/www/public/index.nvs", "/www/public/healthz"]);
        let mounts = || {
            vec![
                mount("/", None, "/www/public/index.nvs"),
                mount("/", Some("blog.example.com"), "/www/public/index.nvs"),
            ]
        };
        let table = Table::new(mounts(), Dispatch::Path, true).with_health(Some("/healthz".into()));
        let off = Table::new(mounts(), Dispatch::Path, true);

        // The hit, on the host-less mount and on the host one alike: § 5 reserves
        // one URL from the whole server and not one per tenant.
        for host in [None, Some("blog.example.com"), Some("other.example.com")] {
            assert_eq!(
                table.resolve(host, "/healthz", &fs),
                Some(Resolved::Health),
                "for {host:?}"
            );
        }
        // And step 3 does not get there first, though `/www/public/healthz` is a
        // file the static policy would happily have sent: step 0 is ahead of the
        // steps, not the first of them.
        assert_eq!(
            off.resolve(None, "/healthz", &fs)
                .and_then(Resolved::selection)
                .expect("the root mount matches everything")
                .what,
            What::Static(p("/www/public/healthz")),
            "the fixture no longer distinguishes the two orders"
        );

        // Exactly one URL. A prefix of it, an extension of it and the directory
        // spelling of it are the application's, or a probe would take paths from
        // a mount that nobody reserved.
        for path in ["/healthz/", "/healthz/live", "/healthzz", "/health"] {
            assert_eq!(
                table.resolve(None, path, &fs).and_then(Resolved::selection),
                off.resolve(None, path, &fs).and_then(Resolved::selection),
                "for {path:?}"
            );
        }

        // Off is the default, and it is the whole difference: the same table with
        // no probe reserved routes the probe's own path to the application.
        assert_eq!(
            off.resolve(None, "/healthz", &fs),
            table
                .clone()
                .with_health(None)
                .resolve(None, "/healthz", &fs)
        );

        // And the tree is where it comes from, read the way § 5 writes it.
        let config = Config {
            server: Some(nvs_config::tree::Server {
                health_path: Some("/healthz".into()),
                ..nvs_config::tree::Server::default()
            }),
            ..Config::default()
        };
        assert_eq!(
            Table::from_config(mounts(), &config).resolve(None, "/healthz", &fs),
            Some(Resolved::Health)
        );
    }

    /// `rule:routing/a-request-reads-its-mount` on the carrier: one table, two tenants, and each request
    /// reaching its program with the row that claimed it.
    ///
    /// **The captures are the load-bearing half.** `prefix` alone is
    /// `mountPrefix`, which § 7 replaces precisely for stating half the fact, so
    /// the two mounts here differ in nothing else: same entry shape, same
    /// remainder, same everything a reading could accidentally answer from. One
    /// that carried the table's first row, or the written `"/{1}"` before
    /// [`nvs_config::mount::expand`] substituted it, still prints plausibly on
    /// either line alone.
    ///
    /// The stripped path is asserted **equal across the two**, which is what
    /// `rule:http-server/a-mount-table-expands-at-boot` bought and what § 7 exists to give back: an application
    /// written against its own root sees `/orders/17` under both mounts, and the
    /// only thing telling it which tenant it is serving is the capture.
    #[test]
    fn a_mounts_captures_reach_the_request_the_handler_answers() {
        let fs = Fake::with(&["/srv/acme/index.nvs", "/srv/globex/index.nvs"]);
        // `scan = "/srv/*/index.nvs"` with `prefix = "/{1}"`, as the boot
        // expansion leaves it: the glob is gone, the row is literal, and the
        // capture that produced it stands beside the prefix it was substituted
        // into.
        let tenant = |name: &str| Mounted {
            captures: vec![name.to_owned()],
            ..mount(&format!("/{name}"), None, &format!("/srv/{name}/index.nvs"))
        };
        let table = Table::new(
            vec![tenant("acme"), tenant("globex")],
            Dispatch::Entry,
            false,
        );
        let carried = |path: &str| {
            let selected = table
                .resolve(None, path, &fs)
                .and_then(Resolved::selection)
                .expect("a tenant claims requests under its own prefix");
            let mut inbound = Inbound::new("GET", &selected.path, "");
            carry(selected.mount, &mut inbound);
            inbound
        };
        let words = |inbound: &Inbound| {
            inbound
                .mount_captures()
                .iter()
                .map(Box::as_ref)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };

        let acme = carried("/acme/orders/17");
        let globex = carried("/globex/orders/17");
        assert_eq!(words(&acme), ["acme"]);
        assert_eq!(words(&globex), ["globex"]);
        assert_eq!(acme.mount_prefix(), "/acme");
        assert_eq!(globex.mount_prefix(), "/globex");
        assert_eq!(acme.path(), "/orders/17");
        assert_eq!(globex.path(), acme.path());

        // And a carrier nothing mounted answers the empty pair rather than
        // guessing: a CLI program and a test both build one, and § 7's `prefix`
        // is what was stripped.
        let unmounted = Inbound::new("GET", "/orders/17", "");
        assert_eq!(unmounted.mount_prefix(), "");
        assert!(words(&unmounted).is_empty());
    }

    /// `rule:http-server/a-path-is-never-derived-from-a-url`'s governing rule, as a **set equality** rather than as a list
    /// of refusals: the paths a booted server can execute are exactly the entries
    /// [`nvs_config::mount::expand`] enumerated, no more and no fewer.
    ///
    /// A refusal case pins one spelling, and there is always another spelling.
    /// This sweeps a generated corpus instead — every one- and two-segment path
    /// over an alphabet holding the fixture's own names, `..` and its encoding,
    /// an encoded separator, a bad escape, the empty segment and a symlink out of
    /// the tree, asked on every host in the table and on one in no mount — and
    /// compares the **set** of files [`Table::resolve`] ever answers `Run` with
    /// against the expanded one. Both directions are the assertion: an answer
    /// outside the set is § 2 broken, and an entry the sweep never reaches means
    /// the equality held over a corpus too thin to have shown it.
    ///
    /// The equality is stated at the production pair — what [`Table::from_config`]
    /// reads from a tree writing neither switch, and what `rule:config/a-startup-default-is-never-flipped` gives
    /// `production`. Development's pair is asserted separately and more weakly,
    /// because § 4 step 4 *deliberately* widens the set to the `.nvs` files inside
    /// a mount root: what is checked there is where the widening stops — never a
    /// `.nvs` elsewhere under `[server] root`, and never one a symlink points at
    /// outside it.
    #[test]
    fn the_executable_path_set_after_boot_equals_the_expanded_mount_table() {
        let fs = Fake::with(&[
            "/www/blog/public/index.nvs",
            "/www/blog/public/style.css",
            "/www/blog/public/admin.nvs",
            // A `.nvs` under `[server] root` and outside every mount root: no
            // switch may reach it, because no mount points at it.
            "/www/blog/src/Post.nvs",
            "/www/shop/public/index.nvs",
            // A directory the glob finds no entry in, so it is not a mount.
            "/www/notes/README.md",
            // Outside `[server] root` altogether, reachable only through the link.
            "/secret/pwned.nvs",
        ])
        .linking("/www/blog/public/away", "/secret");

        // Two blocks over one glob: every module reachable by prefix and by host,
        // which is § 3's two spellings of a capture and gives step 1 both of its
        // passes something to choose between.
        let config: Config = toml::from_str(
            "[server]\nroot = \"/www\"\n\n\
             [[server.mount]]\nscan = \"*/public/index.nvs\"\nprefix = \"/{1}\"\n\n\
             [[server.mount]]\nscan = \"*/public/index.nvs\"\nhost = \"{1}.example.com\"\n",
        )
        .expect("the fixture tree deserializes");
        let mounts = nvs_config::mount::expand(&config, &BTreeMap::new(), &fs)
            .unwrap_or_else(|why| panic!("the boot refused the fixture: {}", why.message));
        assert_eq!(mounts.len(), 4, "two modules, each by prefix and by host");
        let enumerated: BTreeSet<PathBuf> = mounts.iter().map(|one| one.entry.clone()).collect();
        assert_eq!(enumerated.len(), 2, "four rows naming two entry files");

        let segments = [
            "blog",
            "shop",
            "notes",
            "public",
            "src",
            "index.nvs",
            "index.NVS",
            "admin.nvs",
            "style.css",
            "Post.nvs",
            "pwned.nvs",
            "README.md",
            "away",
            "secret",
            "..",
            "%2e%2e",
            ".",
            "",
            "%2f",
            "a%2fb",
            "%zz",
        ];
        let mut corpus: Vec<String> = vec!["/".to_string(), "/healthz".to_string()];
        for one in segments {
            corpus.push(format!("/{one}"));
            corpus.push(format!("/{one}/"));
            for two in segments {
                corpus.push(format!("/{one}/{two}"));
                // The same pairs again below a prefix that matches, so that step
                // 2 has stripped something before steps 3 to 5 see them.
                corpus.push(format!("/blog/{one}/{two}"));
            }
        }
        let hosts = [
            None,
            Some("blog.example.com"),
            // A host mount is matched case-insensitively, and a host in no mount
            // falls through to step 1's second pass.
            Some("BLOG.example.com"),
            Some("shop.example.com"),
            Some("nobody.example.com"),
        ];
        let sweep = |table: &Table| {
            let mut run: BTreeSet<PathBuf> = BTreeSet::new();
            let mut sent: BTreeSet<PathBuf> = BTreeSet::new();
            for host in hosts {
                for path in &corpus {
                    let Some(selected) =
                        table.resolve(host, path, &fs).and_then(Resolved::selection)
                    else {
                        continue;
                    };
                    match selected.what {
                        What::Run(file) => run.insert(file),
                        What::Static(file) => sent.insert(file),
                    };
                }
            }
            (run, sent)
        };

        // § 2, whole: what the server can run after boot *is* the expanded table.
        let (booted, sent) = sweep(&Table::from_config(mounts.clone(), &config));
        assert_eq!(booted, enumerated, "the executable set is the mount table");
        assert!(sent.is_empty(), "a tree writing no `static` serves no file");

        // Development's pair, where step 4 widens the set on purpose.
        let roots: Vec<PathBuf> = mounts.iter().map(|one| one.root.clone()).collect();
        let (widened, sent) = sweep(&Table::new(mounts.clone(), Dispatch::Path, true));
        for file in &widened {
            assert!(is_nvs(file), "steps 4 and 5 answer a `.nvs`: {file:?}");
            assert_eq!(
                fs.file(file).as_ref(),
                Some(file),
                "a file on disk: {file:?}"
            );
            assert!(
                roots.iter().any(|root| file.starts_with(root)),
                "inside a mount root: {file:?}"
            );
        }
        assert!(
            widened.is_superset(&enumerated) && widened.contains(&p("/www/blog/public/admin.nvs")),
            "step 4 adds the `.nvs` files inside a root, and step 5 still answers"
        );
        // The four ways out of a root the corpus spells, all four of them closed:
        // `..`, its encoding, an encoded separator, and the symlink.
        assert!(
            !widened.contains(&p("/www/blog/src/Post.nvs")),
            "a `.nvs` under `[server] root` and outside every mount root"
        );
        assert!(
            !widened.contains(&p("/secret/pwned.nvs")),
            "a `.nvs` the symlink points at outside the tree"
        );
        // And § 4's last sentence, over the same sweep: static serving reaches
        // the same roots and never hands back a `.nvs` as source.
        for file in &sent {
            assert!(
                !is_nvs(file),
                "a `.nvs` is never served as source: {file:?}"
            );
            assert!(
                roots.iter().any(|root| file.starts_with(root)),
                "inside a mount root: {file:?}"
            );
        }
        assert!(sent.contains(&p("/www/blog/public/style.css")));
    }

    /// M7's path traversal suite — `rule:http-server/a-path-is-never-derived-from-a-url` and § 4 step 3, one row per
    /// published technique rather than one case per file.
    ///
    /// The suite is run at the **widest** reading of the table, `dispatch =
    /// "path"` with `[server] static` on, because that is the only pair under
    /// which a remainder reaches the filesystem at all: production answers every
    /// row with the entry without looking, so a suite asserted there would pass
    /// against a step 3 that had no containment check in it.
    ///
    /// **Every row's expected answer is the mount's entry**, which is § 4 step 5.
    /// That single expectation is the whole rule: a traversal attempt is not
    /// repaired into a neighbouring file and is not refused with a status of its
    /// own — it reaches the application as a path the application can 404, and
    /// reaches the filesystem not at all. It also asserts the static half by
    /// construction, since a row that escaped would have to answer
    /// [`What::Static`] to be sent: [`crate::statics::send`] is handed the
    /// `PathBuf` this table chose and never sees a request target, so there is no
    /// second place for a spelling to be re-derived.
    #[test]
    fn the_path_traversal_suite_passes() {
        let fs = Fake::with(&[
            "/www/public/index.nvs",
            "/www/public/style.css",
            "/www/public/admin.nvs",
            "/www/public/assets/logo.png",
            // The three targets every row below is trying to reach: a file
            // beside the mount root, a `.nvs` inside `[server] root` but outside
            // the root, and a file outside the tree altogether.
            "/www/private/secrets.env",
            "/www/src/Post.nvs",
            "/etc/passwd",
        ])
        .linking("/www/public/away", "/www/private")
        .linking("/www/public/assets/up", "/etc");
        let entry = p("/www/public/index.nvs");
        let table = Table::new(
            vec![
                mount("/", None, "/www/public/index.nvs"),
                mount("/blog", None, "/www/public/index.nvs"),
            ],
            Dispatch::Path,
            true,
        );

        // Each row is `(the technique, the target it is written against)`.
        let suite = [
            ("dot-dot, plainly", "/../private/secrets.env"),
            ("dot-dot, twice", "/../../etc/passwd"),
            (
                "dot-dot below a real directory",
                "/assets/../../private/secrets.env",
            ),
            (
                "dot-dot after the prefix was stripped",
                "/blog/../../etc/passwd",
            ),
            (
                "dot-dot behind an existing file",
                "/style.css/../../etc/passwd",
            ),
            ("percent-encoded dots", "/%2e%2e/private/secrets.env"),
            (
                "percent-encoded dots, upper case",
                "/%2E%2E/private/secrets.env",
            ),
            ("one dot encoded, one not", "/.%2e/private/secrets.env"),
            ("the separator encoded too", "/%2e%2e%2fprivate/secrets.env"),
            ("dots and separator in one segment", "/..%2f..%2fetc/passwd"),
            (
                "double encoding, decoded once",
                "/%252e%252e/private/secrets.env",
            ),
            ("the `....//` padding", "/....//private/secrets.env"),
            (
                "a path parameter after the dots",
                "/..;/private/secrets.env",
            ),
            ("a backslash as the separator", "/..\\private\\secrets.env"),
            ("an encoded backslash", "/..%5cprivate%5csecrets.env"),
            ("an encoded separator alone", "/%2fetc%2fpasswd"),
            ("a NUL in the name", "/style.css%00.txt"),
            ("a drive letter", "/C:/Windows/win.ini"),
            ("a UNC share", "//server/share/passwd"),
            ("an absolute target", "/%2f%2fetc%2fpasswd"),
            ("a symlink out of the mount root", "/away/secrets.env"),
            ("a symlink out of the tree", "/assets/up/passwd"),
            (
                "a symlink reached through dots",
                "/assets/../away/secrets.env",
            ),
            ("a `.nvs` outside the mount root", "/../src/Post.nvs"),
            ("a bare `..` as the whole remainder", "/.."),
            ("a directory walk, as a default document", "/../private/"),
        ];
        for (technique, path) in suite {
            let what = table
                .resolve(None, path, &fs)
                .and_then(Resolved::selection)
                .expect("the root mount covers every path")
                .what;
            assert_eq!(
                what,
                What::Run(entry.clone()),
                "{technique}, spelled {path:?}, must answer § 4 step 5"
            );
        }

        // The suite is only worth its length if the same table serves and runs
        // the files it is supposed to: a reading that refused every remainder
        // would pass all 26 rows above and nothing here.
        let what = |path: &str| {
            table
                .resolve(None, path, &fs)
                .and_then(Resolved::selection)
                .expect("the root mount covers every path")
                .what
        };
        assert_eq!(what("/style.css"), What::Static(p("/www/public/style.css")));
        assert_eq!(
            what("/assets/logo.png"),
            What::Static(p("/www/public/assets/logo.png"))
        );
        assert_eq!(what("/admin.nvs"), What::Run(p("/www/public/admin.nvs")));
        assert_eq!(
            what("/blog/style.css"),
            What::Static(p("/www/public/style.css")),
            "and through the prefix the rows above tried to escape from"
        );
    }
}
