//! [ADR 0097] § 4's five steps: the request that arrived, resolved against the
//! table [`nvs_config::mount`] expanded at boot.
//!
//! ```text
//! 1. longest match:  host mounts by prefix  ->  host-less mounts by prefix  ->  404
//! 2. strip the prefix
//! 3. [server] static  &&  the remainder is an existing non-.nvs file under the mount root  -> serve it
//! 4. dispatch == "path"  &&  the remainder is an existing .nvs file under the mount root   -> run it
//! 5. otherwise                                                                             -> run the mount's entry
//! ```
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
//! [ADR 0091](../../../docs/adr/0091-development-and-production-are-the-two-modes.md)
//! § 3a gives `dispatch` and `static` different defaults per mode — `path` and
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
//! [ADR 0095](../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)'s
//! direction applied to the one string in this crate that comes from the peer.
//! Refused means the *step* does not apply, so the request falls through to step
//! 5 and the mount's entry answers it — a traversal attempt reaches the
//! application as a path it can 404 by itself, and reaches the filesystem not at
//! all. The decode runs before the check, so `%2e%2e%2f` is the same refusal
//! `../` is; a `%` that is not two hex digits is refused rather than taken
//! literally.
//!
//! **What it spends**, per [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md):
//! nothing per request that outlives it. A selection borrows its mount from the
//! table and owns one `PathBuf` — the file steps 3-5 chose — and steps 3 and 4
//! cost one `stat` and one `canonicalize` each, only where their switch is on.
//! The table itself is one allocation per configuration generation, shared by
//! every request and never rebuilt on the request path.
//!
//! [ADR 0097]: ../../../docs/adr/0097-development-server-and-proxied-origin.md

use std::path::{Path, PathBuf};

use hyper::Request;
use hyper::body::Incoming;
use hyper::header::HOST;
use nvs_config::mount::Mounted;
use nvs_config::tree::Config;

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
}

impl Table {
    /// The table, with both switches already decided.
    #[must_use]
    pub fn new(mounts: Vec<Mounted>, dispatch: Dispatch, serve_static: bool) -> Self {
        Self {
            mounts,
            dispatch,
            serve_static,
        }
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
        Self::new(mounts, dispatch, serve_static)
    }

    /// Every mount this table holds, which is § 2's executable set in full.
    #[must_use]
    pub fn mounts(&self) -> &[Mounted] {
        &self.mounts
    }

    /// § 4's five steps over one arrived request.
    ///
    /// The host is the `Host` header's without its port, falling back to the
    /// request target's own authority for the absolute-form target h1 still
    /// allows. § 6's forwarded header walk is a later slice and would only ever
    /// *replace* this value: nothing downstream of here reads a header.
    #[must_use]
    pub fn select(
        &self,
        request: &Request<Incoming>,
        disk: &dyn Existing,
    ) -> Option<Selection<'_>> {
        let host = request
            .headers()
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .map(named)
            .or_else(|| request.uri().host());
        self.resolve(host, request.uri().path(), disk)
    }

    /// § 4's five steps over a host and a path, which is [`select`](Table::select)
    /// with the request already taken apart.
    ///
    /// `path` is the request target's path alone — a query string is not part of
    /// what selects a mount, and no step below ever sees one.
    #[must_use]
    pub fn resolve(
        &self,
        host: Option<&str>,
        path: &str,
        disk: &dyn Existing,
    ) -> Option<Selection<'_>> {
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
            && let Some(file) = under_root(false)
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
        Some(Selection { mount, what })
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

/// One request's answer to § 4: which mount it selected, and what that mount
/// says the request is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection<'a> {
    /// The row step 1 chose. Its `origin` is what `Core\Router::urlAbsolute`
    /// prepends and its `captures` are what `Core\Request::mount()` answers, both
    /// of which are the request-context slice's to read off it.
    pub mount: &'a Mounted,
    /// Steps 3, 4 and 5's outcome.
    pub what: What,
}

/// What § 4 decided the request is, once a mount had been selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum What {
    /// Step 3: this exact file's bytes, under `[server] static`. Never a
    /// directory and never a `.nvs`.
    Static(PathBuf),
    /// Steps 4 and 5: this file, run as [ADR 0006]'s isolate. Step 4 is the
    /// remainder itself under `dispatch = "path"`, step 5 is the mount's entry,
    /// and both are a path the table or the disk answered for rather than one
    /// assembled from request bytes.
    ///
    /// [ADR 0006]: ../../../docs/adr/0006-isolated-script-execution.md
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
    file.starts_with(root).then_some(file)
}

/// One path segment with its percent escapes resolved, or `None` when it is not
/// a segment this server will look at.
///
/// A `%` that is not followed by two hex digits is a refusal rather than a
/// literal `%`, and bytes that do not spell UTF-8 are one too: both are
/// ADR 0095's direction, and the alternative in each case is a filename the
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

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
                    .expect("the root mount matches everything")
                    .what,
                What::Run(p("/www/public/index.nvs")),
                "for {path:?}"
            );
        }
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
            let selected = table.resolve(host, path, &fs).expect("a mount matches");
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
}
