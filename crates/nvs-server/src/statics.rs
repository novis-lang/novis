//! `rule:http-server/a-request-resolves-in-five-steps`'s static file policy: the bytes a [`What::Static`] selection
//! is, and the four headers that decide whether the peer needs them.
//!
//! [`crate::mount`] answered *which* file; this module answers *with* it. The
//! two are deliberately separate — selection is where § 2's rule lives and it
//! touches a filesystem only to ask whether a path is a file, while sending is
//! where a response is shaped and it reads bytes — but they are one policy, and
//! § 4's own paragraph is why: **static serving is the same in both
//! deployments, because a second policy is a second security model.** Nothing
//! here takes a mode, a [`Dispatch`](crate::mount::Dispatch) or a `[server]`
//! switch as an argument, so there is no development reading of freshness and
//! no production one; the switch decides *whether* step 3 runs, never what it
//! does.
//!
//! What § 4 states, in the order [`send`] answers it:
//!
//! - **The exact file, never a listing.** A directory answers nothing here —
//!   [`crate::mount::Existing::file`] refused it a step earlier — and a `.nvs`
//!   file is never served as source, which is [`crate::mount`]'s extension test
//!   read from step 3's end.
//! - **`Cache-Control: no-cache` with a strong `ETag` over `(size,
//!   mtime_nanos)`**, and `If-None-Match` against it. One validator, exact, and
//!   specifically not `Last-Modified`: a one-second granularity serves stale
//!   bytes for two edits inside the same second, which is the failure a
//!   development server hits most.
//! - **One `Range`, and a multi-range refused.** § 4 honours a single range;
//!   anything else — two ranges, a unit that is not `bytes`, a spec that does
//!   not parse, a range the file cannot satisfy — is a `416` carrying
//!   `Content-Range: bytes */len`, never a silent `200` with the whole body.
//!   That is `rule:errors/ambiguous-input-refused`'s direction: serving something other than what was asked
//!   for is repairing the request, and a client that meant to resume would write
//!   the wrong bytes to disk on the strength of it.
//! - **A fixed extension table**, with `application/octet-stream` for an unknown
//!   one — which `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s
//!   `nosniff` renders inert rather than leaving to a browser to guess.
//!
//! # Decision: the default document is not the mount's root
//!
//! § 4 makes `index.html` the sole default document, and [`crate::mount`]
//! applies it to a remainder that spells a directory *below* the mount root —
//! `/docs/` finds `docs/index.html` — but never to the mount root itself, where
//! step 5's entry is what a mount means. A mount whose root holds both
//! `index.nvs` and an `index.html` would otherwise stop running its own
//! application the moment `static` was turned on, which is a development server
//! answering with the wrong program rather than a policy anyone chose. That
//! rule is one lexical test on the remainder and lives beside the other one, in
//! [`crate::mount`]'s docs § *What a remainder may be*.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`: one buffer
//! per in-flight static request, holding exactly the bytes that response
//! carries — the whole file, or the one range that was asked for. Nothing is
//! cached between requests, so it is O(in-flight) and not O(files served): a
//! second request for the same asset reads it again. That is a deliberate
//! spend of latency for simplicity at this stage, and a cheap one — a development server's
//! asset traffic is a handful of files on a local disk the OS has already
//! cached — and the place a byte cache would go is behind [`Source`], where the
//! [`OnDisk`] implementation is the only thing that would change.
//!
//! [`What::Static`]: crate::mount::What::Static

use std::ffi::OsStr;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;
use std::time::SystemTime;

use hyper::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_RANGE, CONTENT_TYPE, ETAG, HeaderMap, HeaderValue,
    IF_NONE_MATCH, RANGE,
};
use hyper::{Response, StatusCode};

use crate::mount::OnDisk;
use crate::serve::{Answer, Reply};

/// What a file has to answer before any of it is sent: `rule:http-server/a-request-resolves-in-five-steps`'s
/// `ETag` is over these two numbers and nothing else.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stat {
    /// The file's length in bytes, which is also what a `Range` is resolved
    /// against.
    pub len: u64,
    /// The modification time in nanoseconds since the epoch, negative before
    /// it. Nanoseconds rather than seconds is § 4's own choice, and the whole
    /// reason the validator is not `Last-Modified`.
    pub mtime_nanos: i128,
}

/// The two questions this module asks of a filesystem.
///
/// A trait for the reason [`crate::mount::Existing`] is one: a policy asserted
/// against a real directory is a test that needs a disk, a clock the case does
/// not control and a `mtime` it cannot set twice in the same second — which is
/// exactly the edge the `ETag` exists for. [`OnDisk`] is the production
/// implementation and the cases below script the other one.
pub trait Source {
    /// The size and modification time of the regular file at `path`, or `None`
    /// for anything that is not one.
    fn stat(&self, path: &Path) -> Option<Stat>;

    /// Exactly `len` bytes of `path` starting at `at`, or `None` if that many
    /// are not there to be read.
    fn read(&self, path: &Path, at: u64, len: u64) -> Option<Vec<u8>>;
}

// Each stat and read is recorded when `NVS_FOOTPRINT_LOG` names a log, so a check that serves a
// static file is selected again when that file changes.
impl Source for OnDisk {
    fn stat(&self, path: &Path) -> Option<Stat> {
        nvs_footprint::exists(path);
        let meta = std::fs::metadata(path).ok()?;
        if !meta.is_file() {
            return None;
        }
        Some(Stat {
            len: meta.len(),
            mtime_nanos: meta.modified().ok().map_or(0, nanos),
        })
    }

    fn read(&self, path: &Path, at: u64, len: u64) -> Option<Vec<u8>> {
        nvs_footprint::file(path);
        let mut file = std::fs::File::open(path).ok()?;
        if at > 0 {
            file.seek(SeekFrom::Start(at)).ok()?;
        }
        let mut bytes = vec![0_u8; usize::try_from(len).ok()?];
        file.read_exact(&mut bytes).ok()?;
        Some(bytes)
    }
}

/// The request headers [`send`] reads, copied out of a request that still has
/// them — the one home of which headers this policy is a function of.
///
/// A static selection hands [`send`] the request itself and needs none of this.
/// A **program's** file body does: `Core\Response::sendFile` declares one only
/// while the request runs, by which time the request is the isolate's, so
/// [`crate::serve`] has to decide before the handler what it may still be asked
/// for. Copying the two named here rather than the whole map is what keeps that
/// decision free — a request that sent neither leaves an empty map, which
/// allocates nothing, and one that sent both pays two header values on a
/// response that is about to read a file.
///
/// A header this policy comes to read is added here in the same edit, and a
/// stale copy of the list cannot exist elsewhere: nothing but [`send`] reads a
/// request here, and nothing but this builds what it reads.
#[must_use]
pub fn asked(headers: &HeaderMap) -> HeaderMap {
    let mut kept = HeaderMap::new();
    for name in [IF_NONE_MATCH, RANGE] {
        if let Some(value) = headers.get(&name) {
            kept.insert(name, value.clone());
        }
    }
    kept
}

/// `rule:http-server/a-request-resolves-in-five-steps`'s static policy over one selected file.
///
/// `headers` are the request's, which is all of a request this reads: a method
/// is `hyper`'s to honour — it drops the body of a `HEAD` while keeping the
/// headers this built — and a path was already turned into `file` by
/// [`crate::mount`], so nothing here goes near request bytes.
///
/// A file that vanished between selection and this call answers `404` rather
/// than falling back to the mount's entry: re-entering § 4 at step 5 from here
/// would be a second resolution of the same request, and the file genuinely is
/// not there. A file that is there and unreadable is a `500`, since that is a
/// deployment's problem and not the peer's.
///
#[must_use]
pub fn send(file: &Path, headers: &HeaderMap, source: &dyn Source) -> Reply {
    let Some(stat) = source.stat(file) else {
        return Reply::status(StatusCode::NOT_FOUND);
    };
    let tag = etag(stat);
    let mut response = Response::new(Answer::empty());
    // On every answer this function gives, including the two that carry no
    // body: a `304` states the validator it matched, and a `416` is still an
    // answer about this representation.
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response.headers_mut().insert(ETAG, ascii(&tag));

    if unchanged(headers, &tag) {
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        return Reply::Done(response);
    }
    let Some(wanted) = asked_for(headers.get(RANGE), stat.len) else {
        *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
        response
            .headers_mut()
            .insert(CONTENT_RANGE, ascii(&format!("bytes */{}", stat.len)));
        return Reply::Done(response);
    };
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(mime(file)));
    let Some(bytes) = source.read(file, wanted.at, wanted.len) else {
        return Reply::status(StatusCode::INTERNAL_SERVER_ERROR);
    };
    if wanted.partial {
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        let last = wanted.at + wanted.len - 1;
        response.headers_mut().insert(
            CONTENT_RANGE,
            ascii(&format!("bytes {}-{last}/{}", wanted.at, stat.len)),
        );
    }
    *response.body_mut() = Answer::new(bytes);
    Reply::Done(response)
}

/// § 4's strong validator: `(size, mtime_nanos)` and nothing else, so two
/// writes of the same length inside the same second are two different tags.
fn etag(stat: Stat) -> String {
    format!("\"{:x}-{:x}\"", stat.len, stat.mtime_nanos)
}

/// Whether `If-None-Match` names the tag this file has now.
///
/// A list is honoured because a client is entitled to send one, `*` matches any
/// representation that exists, and a `W/` prefix is stripped before the
/// comparison — RFC 7232 § 3.2 says `If-None-Match` compares weakly, and our own
/// tag is strong, so the weak spelling of it is the same representation.
fn unchanged(headers: &HeaderMap, tag: &str) -> bool {
    let Some(asked) = headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    asked.split(',').map(str::trim).any(|candidate| {
        candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == tag
    })
}

/// Which bytes of a file of `len` the request asked for, or `None` for a
/// `Range` this server refuses — which is every one it cannot honour exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wanted {
    at: u64,
    len: u64,
    /// Whether the answer is a `206` over a `Range` rather than the whole file.
    partial: bool,
}

/// § 4's range policy: no header is the whole file, one `bytes=` spec is that
/// spec, and anything else is refused.
///
/// The refusals are one arm each on purpose — a comma, a unit that is not
/// `bytes`, a bound that does not parse, an inverted pair, a suffix of zero and
/// a start past the end all answer `None`, and the caller turns that into the
/// single `416` § 4 states. An `end` past the last byte is the one repair the
/// RFC requires rather than forbids, and it is a clamp of a range that was
/// satisfiable, not a substitution of a different answer.
fn asked_for(header: Option<&HeaderValue>, len: u64) -> Option<Wanted> {
    let Some(header) = header else {
        return Some(Wanted {
            at: 0,
            len,
            partial: false,
        });
    };
    let spec = header.to_str().ok()?.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (first, last) = spec.split_once('-')?;
    let (first, last) = (first.trim(), last.trim());
    let (at, end) = match (first.is_empty(), last.is_empty()) {
        (true, true) => return None,
        // `bytes=-N`: the last N bytes, and never more than the file holds.
        (true, false) => {
            let suffix: u64 = last.parse().ok()?;
            if suffix == 0 {
                return None;
            }
            (len.saturating_sub(suffix), len.checked_sub(1)?)
        }
        // `bytes=N-`: from N to the end.
        (false, true) => (first.parse().ok()?, len.checked_sub(1)?),
        (false, false) => {
            let at: u64 = first.parse().ok()?;
            let end: u64 = last.parse().ok()?;
            if end < at {
                return None;
            }
            (at, end.min(len.checked_sub(1)?))
        }
    };
    if at > end {
        return None;
    }
    Some(Wanted {
        at,
        len: end - at + 1,
        partial: true,
    })
}

/// § 4's fixed extension table, matched case-insensitively, with
/// `application/octet-stream` for everything it does not name.
///
/// Fixed is the point: a table read from configuration is a per-deployment
/// content type, which is the second policy § 4 refuses. A `charset` is stated
/// wherever the type is text, because a browser that guesses one is a browser
/// that can be made to guess `UTF-7`.
fn mime(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

/// A header value this module wrote itself, which is hex digits, digits and
/// punctuation — never a byte that arrived from the peer.
fn ascii(text: &str) -> HeaderValue {
    HeaderValue::from_str(text).expect("this module writes only ASCII header values")
}

/// A `SystemTime` as nanoseconds since the epoch, negative before it.
fn nanos(time: SystemTime) -> i128 {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => i128::try_from(after.as_nanos()).unwrap_or(i128::MAX),
        Err(before) => -i128::try_from(before.duration().as_nanos()).unwrap_or(i128::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mount::{Dispatch, Existing, Table, What};
    use nvs_config::mount::Mounted;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A path written the way an ADR writes one, as the host spells it.
    fn p(path: &str) -> PathBuf {
        PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    #[test]
    fn the_disk_records_each_stat_and_read() {
        // The test binary itself: a file that is certainly there, inside this package's build.
        let manifest = std::env::current_exe().expect("the test binary's path");
        let (bytes, lines) = nvs_footprint::capture(|| {
            let stat = OnDisk.stat(&manifest).expect("the manifest is a file");
            OnDisk.read(&manifest, 0, stat.len.min(4))
        });
        assert!(bytes.is_some());
        let shown = nvs_footprint::shown(&manifest);
        assert_eq!(
            lines,
            [format!("exists\t{shown}"), format!("file\t{shown}")]
        );
    }

    /// The files a case describes: contents and an `mtime`, which is the pair
    /// the `ETag` is over and the pair a real disk will not let a test choose.
    #[derive(Default)]
    struct Fake(BTreeMap<PathBuf, (Vec<u8>, i128)>);

    impl Fake {
        fn with(files: &[(&str, &[u8], i128)]) -> Self {
            Self(
                files
                    .iter()
                    .map(|(path, bytes, mtime)| (p(path), ((*bytes).to_vec(), *mtime)))
                    .collect(),
            )
        }
    }

    impl Existing for Fake {
        fn file(&self, path: &Path) -> Option<PathBuf> {
            self.0.contains_key(path).then(|| path.to_path_buf())
        }
    }

    impl Source for Fake {
        fn stat(&self, path: &Path) -> Option<Stat> {
            let (bytes, mtime_nanos) = self.0.get(path)?;
            Some(Stat {
                len: u64::try_from(bytes.len()).ok()?,
                mtime_nanos: *mtime_nanos,
            })
        }

        fn read(&self, path: &Path, at: u64, len: u64) -> Option<Vec<u8>> {
            let (bytes, _) = self.0.get(path)?;
            let at = usize::try_from(at).ok()?;
            let len = usize::try_from(len).ok()?;
            bytes.get(at..at.checked_add(len)?).map(<[u8]>::to_vec)
        }
    }

    /// One mount, as [`nvs_config::mount::expand`] would have produced it.
    fn mount(prefix: &str, entry: &str) -> Mounted {
        let entry = p(entry);
        Mounted {
            prefix: prefix.to_string(),
            host: None,
            root: entry
                .parent()
                .expect("an entry has a directory")
                .to_path_buf(),
            entry,
            origin: None,
            captures: Vec::new(),
        }
    }

    /// A request carrying `headers`, each `name: value`.
    fn asking(headers: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.insert(
                hyper::header::HeaderName::from_bytes(name.as_bytes()).expect("a header name"),
                HeaderValue::from_str(value).expect("a header value"),
            );
        }
        map
    }

    /// What a [`Reply`] answered, as the three things a case compares: the
    /// status, the headers that decide caching and framing, and the bytes.
    fn answered(reply: Reply) -> (StatusCode, Vec<(String, String)>, Vec<u8>) {
        let Reply::Done(response) = reply else {
            panic!("a static file is never a program");
        };
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        let status = response.status();
        (status, headers, response.body().bytes().to_vec())
    }

    /// The header a case names, or `""` where the answer did not carry it.
    fn header(answer: &(StatusCode, Vec<(String, String)>, Vec<u8>), name: &str) -> String {
        answer
            .1
            .iter()
            .find(|(header, _)| header == name)
            .map_or_else(String::new, |(_, value)| value.clone())
    }

    /// `rule:http-server/a-request-resolves-in-five-steps`'s static paragraph, asserted as **one** policy: the same
    /// file, reached through a development deployment and through a production
    /// one that turned `static` on, is answered byte for byte identically.
    ///
    /// The two tables below differ in the switch that is not step 3's —
    /// `dispatch` — because that is the only axis a deployment has left once
    /// `static` is on, and a reading that grew a second policy would have grown
    /// it there: freshness "in development", a listing "for convenience", a
    /// looser `Range`. Every assertion after the first pair is therefore made
    /// against both answers at once.
    ///
    #[test]
    fn static_files_are_one_policy_in_both_deployments() {
        let fs = Fake::with(&[
            ("/www/public/index.nvs", b"<?nvs", 11),
            ("/www/public/style.css", b"body{}", 7),
            ("/www/public/report", b"body{}", 7),
            ("/www/public/docs/index.html", b"<p>hi", 9),
        ]);
        let development = Table::new(
            vec![mount("/", "/www/public/index.nvs")],
            Dispatch::Path,
            true,
        );
        let production = Table::new(
            vec![mount("/", "/www/public/index.nvs")],
            Dispatch::Entry,
            true,
        );
        let selected = |table: &Table, path: &str| {
            table
                .resolve(None, path, &fs)
                .and_then(crate::mount::Resolved::selection)
                .expect("the root mount matches everything")
                .what
        };
        let sent = |path: &str, headers: &[(&str, &str)]| {
            let asked = asking(headers);
            let answers: Vec<_> = [&development, &production]
                .iter()
                .map(|table| match selected(table, path) {
                    What::Static(file) => answered(send(&file, &asked, &fs)),
                    other => panic!("{path} selected {other:?} rather than a static file"),
                })
                .collect();
            // The whole claim of this test, asserted on every answer it makes
            // rather than once at the end: the two deployments agree.
            assert_eq!(answers[0], answers[1], "for {path:?} {headers:?}");
            answers[0].clone()
        };

        // Both deployments select the same file for the same request, which is
        // what makes comparing their answers meaningful at all.
        assert_eq!(
            selected(&development, "/style.css"),
            What::Static(p("/www/public/style.css"))
        );
        assert_eq!(
            selected(&production, "/style.css"),
            What::Static(p("/www/public/style.css"))
        );

        // The exact file, with § 4's freshness pair and the fixed table's type.
        let whole = sent("/style.css", &[]);
        assert_eq!(whole.0, StatusCode::OK);
        assert_eq!(whole.2, b"body{}");
        assert_eq!(header(&whole, "cache-control"), "no-cache");
        assert_eq!(header(&whole, "content-type"), "text/css; charset=utf-8");
        assert_eq!(header(&whole, "accept-ranges"), "bytes");
        // The `ETag` is over `(size, mtime_nanos)`: a file with the same bytes
        // and a different name has the same tag, and it is a *strong* one.
        let tag = header(&whole, "etag");
        assert_eq!(tag, "\"6-7\"");
        assert!(!tag.starts_with("W/"), "the validator is strong: {tag}");
        // An unknown extension is `application/octet-stream` and not a guess,
        // which is the same bytes with a different name.
        let unknown = sent("/report", &[]);
        assert_eq!(header(&unknown, "content-type"), "application/octet-stream");
        assert_eq!(header(&unknown, "etag"), tag);

        // `If-None-Match` against that tag: `304`, no body, and the validator
        // restated. A tag from a previous edit is not a match.
        let fresh = sent("/style.css", &[("if-none-match", &tag)]);
        assert_eq!(fresh.0, StatusCode::NOT_MODIFIED);
        assert!(fresh.2.is_empty(), "a 304 carries no body");
        assert_eq!(header(&fresh, "etag"), tag);
        assert_eq!(
            sent("/style.css", &[("if-none-match", "\"6-6\"")]).0,
            StatusCode::OK
        );
        // A list, a weak spelling of our own tag, and `*` all match.
        for asked in ["\"6-6\", \"6-7\"", "W/\"6-7\"", "*"] {
            assert_eq!(
                sent("/style.css", &[("if-none-match", asked)]).0,
                StatusCode::NOT_MODIFIED,
                "for {asked:?}"
            );
        }

        // One `Range` is honoured, exactly — and a `206` says which bytes it is.
        let part = sent("/style.css", &[("range", "bytes=1-3")]);
        assert_eq!(part.0, StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.2, b"ody");
        assert_eq!(header(&part, "content-range"), "bytes 1-3/6");
        // A suffix, an open end, and an end past the last byte, which is the one
        // clamp: the range was satisfiable and only its bound was generous.
        assert_eq!(sent("/style.css", &[("range", "bytes=-3")]).2, b"y{}");
        assert_eq!(sent("/style.css", &[("range", "bytes=4-")]).2, b"{}");
        let clamped = sent("/style.css", &[("range", "bytes=2-99")]);
        assert_eq!(clamped.2, b"dy{}");
        assert_eq!(header(&clamped, "content-range"), "bytes 2-5/6");

        // Every range this server will not honour exactly is the same refusal,
        // and never a `200` carrying the whole file instead.
        for refused in [
            "bytes=0-0,2-3",
            "bytes=6-7",
            "bytes=3-1",
            "bytes=-0",
            "bytes=x-2",
            "items=0-1",
            "bytes=-",
        ] {
            let answer = sent("/style.css", &[("range", refused)]);
            assert_eq!(
                answer.0,
                StatusCode::RANGE_NOT_SATISFIABLE,
                "for {refused:?}"
            );
            assert_eq!(header(&answer, "content-range"), "bytes */6");
            assert!(answer.2.is_empty(), "for {refused:?}");
        }

        // Never a listing, and never a `.nvs` as source: neither deployment
        // reaches this module for either, so the policy has nothing to differ
        // about. `/` is the mount's own root, where the entry answers.
        for table in [&development, &production] {
            assert!(matches!(selected(table, "/"), What::Run(_)));
            assert!(matches!(selected(table, "/docs"), What::Run(_)));
            assert!(
                matches!(selected(table, "/index.nvs"), What::Run(file) if file == p("/www/public/index.nvs"))
            );
        }
        // § 4's sole default document, below the root and by its own spelling.
        let indexed = sent("/docs/", &[]);
        assert_eq!(indexed.2, b"<p>hi");
        assert_eq!(header(&indexed, "content-type"), "text/html; charset=utf-8");

        // A file that went away between selection and sending is not there,
        // rather than a fall-back to some other file.
        let gone = answered(send(&p("/www/public/gone.css"), &asking(&[]), &fs));
        assert_eq!(gone.0, StatusCode::NOT_FOUND);
    }
}
