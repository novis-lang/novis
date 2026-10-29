//! `rule:http-server/a-request-resolves-in-five-steps`'s static file policy: the bytes a [`What::Static`] selection
//! is, and the four headers that decide whether the peer needs them.
//!
//! [`crate::mount`] answered *which* file; this module answers *with* it. The
//! two are deliberately separate — selection is where
//! `rule:http-server/a-path-is-never-derived-from-a-url` lives and it
//! touches a filesystem only to ask whether a path is a file, while sending is
//! where a response is shaped and it reads bytes — but they are one policy, and
//! `rule:http-server/static-serving-is-one-policy` is why: **static serving is the same in both
//! deployments, because a second policy is a second security model.** Nothing
//! here takes a mode, a [`Dispatch`](crate::mount::Dispatch) or a `[server]`
//! switch as an argument, so there is no development reading of freshness and
//! no production one; the switch decides *whether* step 3 runs, never what it
//! does.
//!
//! What that rule states, in the order [`serve`] and [`send`] answer it:
//!
//! - **`GET` and `HEAD` only.** Any other method at a static file is a `405`
//!   with `Allow: GET, HEAD`, which is [`serve`]'s test in front of [`send`].
//! - **The exact file, never a listing.** A directory answers nothing here —
//!   [`crate::mount::Existing::file`] refused it a step earlier — and a `.nvs`
//!   file is never served as source, which is [`crate::mount`]'s extension test
//!   read from step 3's end.
//! - **`Cache-Control: no-cache` with a strong `ETag` over `(size,
//!   mtime_nanos)`**, and `If-None-Match` against it. One validator, exact, and
//!   specifically not `Last-Modified`: a one-second granularity serves stale
//!   bytes for two edits inside the same second, which is the failure a
//!   development server hits most.
//! - **One `Range`, honoured only under an `If-Range` naming the same tag.**
//!   Anything else is a `416` carrying `Content-Range: bytes */len`, or the
//!   whole file where `If-Range` missed. That decision is not this module's:
//!   it is [`nvs_runtime::conditional::decide`], which `Core\Response::stream`
//!   calls too, because a second copy of it would be a second policy. Its
//!   module doc states each answer.
//! - **A fixed extension table**, with `application/octet-stream` for an unknown
//!   one — which `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s
//!   `nosniff` renders inert rather than leaving to a browser to guess.
//!
//! # Decision: the default document is not the mount's root
//!
//! The static policy makes `index.html` the sole default document, and [`crate::mount`]
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
//! Per `rule:programs/memory-priority`: one open file handle and one chunk of
//! [`crate::serve::FILE_CHUNK`] bytes per in-flight file response, plus what
//! `hyper` holds in its write buffer, whatever the file's size or the range's.
//! `hyper` polls the body only when that buffer has room, so a slow peer holds
//! the read back rather than letting it run ahead. Nothing is cached between
//! requests, so it is O(in-flight) and not O(files served): a second request
//! for the same asset reads it again, from a disk the OS has usually cached
//! already. The place a byte cache would go is behind [`Source`], whose
//! [`Contents::Bytes`] is the shape it would hand over.
//!
//! [`What::Static`]: crate::mount::What::Static

use std::ffi::OsStr;
use std::path::Path;
use std::time::SystemTime;

use hyper::body::Bytes;
use hyper::header::{
    ACCEPT_RANGES, ALLOW, CACHE_CONTROL, CONTENT_RANGE, CONTENT_SECURITY_POLICY, CONTENT_TYPE,
    ETAG, HeaderMap, HeaderValue, IF_NONE_MATCH, IF_RANGE, RANGE,
};
use hyper::{Method, Response, StatusCode};

use nvs_runtime::conditional::{Asked, Decision, Window, decide, unsatisfied_range};

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
    /// it. Nanoseconds rather than seconds is the static policy's own choice, and the whole
    /// reason the validator is not `Last-Modified`.
    pub mtime_nanos: i128,
}

/// A file [`Source::open`] opened: the [`Stat`] read off the open handle, and
/// where its bytes are.
///
/// One value rather than a stat and a later read, so the `ETag` and the bytes
/// are always the same file's: a file replaced between two path lookups would
/// otherwise pair the old file's tag with the new file's bytes.
#[derive(Debug)]
pub struct Opened {
    /// The size and modification time, read from the handle `contents` reads.
    pub stat: Stat,
    /// The bytes themselves.
    pub contents: Contents,
}

/// Where an opened file's bytes are.
#[derive(Debug)]
pub enum Contents {
    /// A file on disk, read one chunk at a time while the response is written
    /// ([`Answer::File`]).
    File(std::fs::File),
    /// Bytes already in memory, which is what the cases' sources hold and what
    /// a byte cache behind [`Source`] would hold.
    Bytes(Bytes),
}

/// Why [`Source::open`] has no file to give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unopened {
    /// Nothing is there, or what is there is not a regular file: a `404`.
    Missing,
    /// A regular file is there and cannot be opened: a `500`, since that is a
    /// deployment's problem and not the peer's.
    Unreadable,
}

/// The one question this module asks of a filesystem.
///
/// A trait for the reason [`crate::mount::Existing`] is one: a policy asserted
/// against a real directory is a test that needs a disk, a clock the case does
/// not control and a `mtime` it cannot set twice in the same second — which is
/// exactly the edge the `ETag` exists for. [`OnDisk`] is the production
/// implementation and the cases below script the other one.
pub trait Source {
    /// The regular file at `path`, opened, with the size and modification time
    /// of what was opened.
    ///
    /// # Errors
    ///
    /// [`Unopened::Missing`] for anything that is not a regular file, and
    /// [`Unopened::Unreadable`] for one that cannot be opened.
    fn open(&self, path: &Path) -> Result<Opened, Unopened>;
}

// The lookup and the open are recorded when `NVS_FOOTPRINT_LOG` names a log, so a check that
// serves a static file is selected again when that file changes.
impl Source for OnDisk {
    fn open(&self, path: &Path) -> Result<Opened, Unopened> {
        nvs_footprint::exists(path);
        // Asked of the path before it is opened: opening a FIFO waits for a
        // writer, and this open blocks the connection's own thread.
        if !std::fs::metadata(path).is_ok_and(|meta| meta.is_file()) {
            return Err(Unopened::Missing);
        }
        nvs_footprint::file(path);
        let file = std::fs::File::open(path).map_err(|_| Unopened::Unreadable)?;
        let meta = file.metadata().map_err(|_| Unopened::Unreadable)?;
        if !meta.is_file() {
            return Err(Unopened::Missing);
        }
        Ok(Opened {
            stat: Stat {
                len: meta.len(),
                mtime_nanos: meta.modified().ok().map_or(0, nanos),
            },
            contents: Contents::File(file),
        })
    }
}

/// The request headers [`send`] reads, copied out of a request that still has
/// them — the one home of which headers this policy is a function of.
///
/// A static selection hands [`send`] the request itself and needs none of this.
/// A **program's** file body does: `Core\Response::sendFile` declares one only
/// while the request runs, by which time the request is the isolate's, so
/// [`crate::serve`] has to decide before the handler what it may still be asked
/// for. Copying the three named here rather than the whole map is what keeps
/// that decision free — a request that sent none leaves an empty map, which
/// allocates nothing, and one that sent all three pays three header values on a
/// response that is about to read a file.
///
/// A header this policy comes to read is added here in the same edit, and a
/// stale copy of the list cannot exist elsewhere: nothing but [`send`] reads a
/// request here, and nothing but this builds what it reads.
#[must_use]
pub fn asked(headers: &HeaderMap) -> HeaderMap {
    let mut kept = HeaderMap::new();
    for name in [IF_NONE_MATCH, IF_RANGE] {
        if let Some(value) = headers.get(&name) {
            kept.insert(name, value.clone());
        }
    }
    // Every `Range` line, because a second one changes the answer.
    for value in headers.get_all(RANGE) {
        kept.append(RANGE, value.clone());
    }
    kept
}

/// Step 3's answer to a request whose path selected `file`: a `405` with
/// `Allow: GET, HEAD` for any other method, and [`send`]'s answer for those two.
///
/// A file under a static root is read-only, so a write method has nothing to
/// do there. Sending the file for a `POST` would answer a request that meant to
/// change something as if it had. A program's `Core\Response::sendFile` does not
/// come through here: a program may answer any method with a file.
#[must_use]
pub fn serve(file: &Path, method: &Method, headers: &HeaderMap, source: &dyn Source) -> Reply {
    if !matches!(*method, Method::GET | Method::HEAD) {
        let mut response = Response::new(Answer::empty());
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        response
            .headers_mut()
            .insert(ALLOW, HeaderValue::from_static("GET, HEAD"));
        return Reply::Done(response);
    }
    send(file, headers, source)
}

/// `rule:http-server/a-request-resolves-in-five-steps`'s static policy over one selected file.
///
/// `headers` are the request's, which is all of a request this reads: the
/// method was judged by [`serve`] for a mount's file and is the program's for
/// `sendFile`, `hyper` drops the body of a `HEAD` while keeping the headers
/// this built, and a path was already turned into `file` by [`crate::mount`],
/// so nothing here goes near request bytes.
///
/// A file that vanished between selection and this call answers `404` rather
/// than falling back to the mount's entry: re-entering
/// `rule:http-server/a-request-resolves-in-five-steps` at step 5 from here
/// would be a second resolution of the same request, and the file genuinely is
/// not there. A file that is there and unreadable is a `500`, since that is a
/// deployment's problem and not the peer's.
///
/// The body carries exactly the bytes that were asked for, so its length is
/// stated up front. A file on disk is read while it is sent, one chunk at a
/// time, and is never held whole ([`Answer::File`]).
#[must_use]
pub fn send(file: &Path, headers: &HeaderMap, source: &dyn Source) -> Reply {
    let opened = match source.open(file) {
        Ok(opened) => opened,
        Err(Unopened::Missing) => return Reply::status(StatusCode::NOT_FOUND),
        Err(Unopened::Unreadable) => return Reply::status(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let stat = opened.stat;
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

    let asked = Asked {
        if_none_match: headers.get(IF_NONE_MATCH).map(HeaderValue::as_bytes),
        if_range: headers.get(IF_RANGE).map(HeaderValue::as_bytes),
        range: headers.get(RANGE).map(HeaderValue::as_bytes),
        range_repeated: headers.get_all(RANGE).iter().nth(1).is_some(),
    };
    let wanted = match decide(asked, Some(&tag), stat.len) {
        Decision::NotModified => {
            *response.status_mut() = StatusCode::NOT_MODIFIED;
            return Reply::Done(response);
        }
        Decision::Unsatisfiable => {
            *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
            response
                .headers_mut()
                .insert(CONTENT_RANGE, ascii(&unsatisfied_range(stat.len)));
            return Reply::Done(response);
        }
        Decision::Send(wanted) => wanted,
    };
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(mime(file)));
    if runs_script(file) {
        response
            .headers_mut()
            .insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(SANDBOXED));
    }
    let Some(body) = body(opened.contents, wanted) else {
        return Reply::status(StatusCode::INTERNAL_SERVER_ERROR);
    };
    if wanted.partial {
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        response
            .headers_mut()
            .insert(CONTENT_RANGE, ascii(&wanted.content_range(stat.len)));
    }
    *response.body_mut() = body;
    Reply::Done(response)
}

/// The body that carries the `wanted` bytes of `contents`, or `None` where they
/// cannot be reached: a file that cannot seek to their start, or bytes in
/// memory that are fewer than the stat said.
fn body(contents: Contents, wanted: Window) -> Option<Answer> {
    match contents {
        Contents::File(file) => Answer::file(file, wanted.at, wanted.len).ok(),
        Contents::Bytes(bytes) => {
            let at = usize::try_from(wanted.at).ok()?;
            let end = at.checked_add(usize::try_from(wanted.len).ok()?)?;
            (end <= bytes.len()).then(|| Answer::new(bytes.slice(at..end)))
        }
    }
}

/// The static policy's strong validator: `(size, mtime_nanos)` and nothing else, so two
/// writes of the same length inside the same second are two different tags.
fn etag(stat: Stat) -> String {
    format!("\"{:x}-{:x}\"", stat.len, stat.mtime_nanos)
}

/// The static policy's fixed extension table, matched case-insensitively, with
/// `application/octet-stream` for everything it does not name.
///
/// Fixed is the point: a table read from configuration is a per-deployment
/// content type, which is the second policy
/// `rule:http-server/static-serving-is-one-policy` refuses. A `charset` is stated
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

/// The policy a static file a browser runs script from is sent with. `sandbox`
/// gives the document a unique origin and runs none of its script, so an SVG or
/// HTML file somebody uploaded under a static root cannot act as this site.
/// `frame-ancestors 'none'` is the response policy's own default, written
/// again because a policy set here replaces the default one
/// ([`crate::secure`] fills only what a response has not set).
const SANDBOXED: &str = "sandbox; frame-ancestors 'none'";

/// Whether `path` is a type a browser runs script from when it is opened as a
/// page: HTML, SVG and XML (which may hold XHTML), by [`mime`]'s table.
fn runs_script(path: &Path) -> bool {
    matches!(
        mime(path),
        "text/html; charset=utf-8" | "image/svg+xml" | "application/xml"
    )
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
    use crate::serve::{BodyCut, FILE_CHUNK};
    use hyper::body::Body as _;
    use nvs_config::mount::Mounted;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A path written with `/` separators, the way a rule writes one, as the host spells it.
    fn p(path: &str) -> PathBuf {
        PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    #[test]
    fn the_disk_records_each_stat_and_read() {
        // The test binary itself: a file that is certainly there, inside this package's build.
        let manifest = std::env::current_exe().expect("the test binary's path");
        let (opened, lines) = nvs_footprint::capture(|| OnDisk.open(&manifest));
        assert!(opened.is_ok_and(|opened| matches!(opened.contents, Contents::File(_))));
        let shown = nvs_footprint::shown(&manifest);
        assert_eq!(
            lines,
            [format!("exists\t{shown}"), format!("file\t{shown}")]
        );
    }

    /// The next frame `body` gives, as its bytes or the error it ended in, and
    /// `None` at its end. A file body never waits, so a poll that would is a
    /// failure of the case.
    fn next(body: &mut Answer) -> Option<Result<Vec<u8>, BodyCut>> {
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(frame) = std::pin::Pin::new(body).poll_frame(&mut cx) else {
            panic!("a file body waited");
        };
        frame.map(|frame| frame.map(|frame| frame.into_data().expect("a data frame").to_vec()))
    }

    /// `len` bytes that differ from one offset to the next, so a chunk read from
    /// the wrong place cannot compare equal.
    fn numbered(len: usize) -> Vec<u8> {
        (0..len)
            .map(|at| u8::try_from(at % 251).expect("under 251"))
            .collect()
    }

    /// The body of what `send` answered for the file at `path` under `headers`,
    /// with the status.
    fn sent_from_disk(path: &Path, headers: &[(&str, &str)]) -> (StatusCode, Answer) {
        let Reply::Done(response) = send(path, &asking(headers), &OnDisk) else {
            panic!("a static file is never a program");
        };
        (response.status(), response.into_body())
    }

    /// A file on disk goes out as one frame per [`FILE_CHUNK`] under an exact
    /// length, and a range is read from its own start: the response never
    /// holds more than one chunk of the file.
    #[test]
    fn a_file_on_disk_is_sent_one_chunk_at_a_time() {
        let dir = nvs_repo::scratch("statics-chunks");
        let path = dir.path().join("big.bin");
        let contents = numbered(2 * FILE_CHUNK + 5);
        std::fs::write(&path, &contents).expect("the scratch file was not written");

        let (status, mut body) = sent_from_disk(&path, &[]);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body.size_hint().exact(),
            Some(u64::try_from(contents.len()).expect("fits")),
            "the length is stated up front"
        );
        let mut sizes = Vec::new();
        let mut whole = Vec::new();
        while let Some(frame) = next(&mut body) {
            let frame = frame.expect("the file ended early");
            sizes.push(frame.len());
            whole.extend(frame);
        }
        assert_eq!(sizes, [FILE_CHUNK, FILE_CHUNK, 5]);
        assert!(whole == contents, "the bytes sent are not the file's");
        assert!(body.is_end_stream());

        let (status, mut part) = sent_from_disk(&path, &[("range", "bytes=-3")]);
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.size_hint().exact(), Some(3));
        assert_eq!(
            next(&mut part).map(Result::ok),
            Some(Some(contents[contents.len() - 3..].to_vec()))
        );
        assert_eq!(next(&mut part), None);
    }

    /// A file that shrinks after its length was stated ends the body in an
    /// error, so `hyper` cuts the connection and the peer never reads a short
    /// body as a whole one.
    #[test]
    fn a_file_that_shrinks_while_it_is_sent_ends_in_an_error() {
        let dir = nvs_repo::scratch("statics-shrinks");
        let path = dir.path().join("big.bin");
        std::fs::write(&path, numbered(3 * FILE_CHUNK)).expect("the scratch file was not written");

        let (status, mut body) = sent_from_disk(&path, &[]);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            next(&mut body).map(|frame| frame.map(|frame| frame.len())),
            Some(Ok(FILE_CHUNK))
        );
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_len(u64::try_from(FILE_CHUNK + 10).expect("fits")))
            .expect("the scratch file was not truncated");
        assert_eq!(next(&mut body), Some(Err(BodyCut::FileEndedEarly)));
    }

    /// `If-Range` with the current tag honours the range. An old tag, the weak
    /// spelling of the current one, a date and a value that is not a tag all
    /// send the whole file with a `200`, and so does a range that would
    /// otherwise be a `416`, because the range is ignored.
    #[test]
    fn if_range_with_the_current_tag_gets_the_range_and_any_other_gets_the_whole_file() {
        let fs = Fake::with(&[("/www/style.css", b"body{}", 7)]);
        let file = p("/www/style.css");
        let sent = |headers: &[(&str, &str)]| answered(send(&file, &asking(headers), &fs));
        let tag = header(&sent(&[]), "etag");

        let part = sent(&[("range", "bytes=1-3"), ("if-range", &tag)]);
        assert_eq!(part.0, StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.2, b"ody");

        let weak = format!("W/{tag}");
        for asked in [
            "\"6-6\"",
            weak.as_str(),
            "Tue, 13 Oct 2026 10:00:00 GMT",
            "not a tag",
        ] {
            let whole = sent(&[("range", "bytes=1-3"), ("if-range", asked)]);
            assert_eq!(whole.0, StatusCode::OK, "for {asked:?}");
            assert_eq!(whole.2, b"body{}", "for {asked:?}");
            assert_eq!(header(&whole, "content-range"), "", "for {asked:?}");
            let unsatisfiable = sent(&[("range", "bytes=9-"), ("if-range", asked)]);
            assert_eq!(unsatisfiable.0, StatusCode::OK, "for {asked:?}");
        }

        // A conditional that matched is answered first, whatever `If-Range` says.
        let fresh = sent(&[
            ("if-none-match", &tag),
            ("range", "bytes=1-3"),
            ("if-range", "\"6-6\""),
        ]);
        assert_eq!(fresh.0, StatusCode::NOT_MODIFIED);
        // `If-Range` without a `Range` changes nothing.
        assert_eq!(sent(&[("if-range", "\"6-6\"")]).0, StatusCode::OK);
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
        fn open(&self, path: &Path) -> Result<Opened, Unopened> {
            let (bytes, mtime_nanos) = self.0.get(path).ok_or(Unopened::Missing)?;
            Ok(Opened {
                stat: Stat {
                    len: u64::try_from(bytes.len()).expect("a case's file fits in a u64"),
                    mtime_nanos: *mtime_nanos,
                },
                contents: Contents::Bytes(Bytes::from(bytes.clone())),
            })
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

    /// A static file answers `GET` and `HEAD`, and every write method gets a
    /// `405` naming those two, with none of the file's bytes or headers.
    #[test]
    fn a_static_file_answers_a_write_method_with_405() {
        let fs = Fake::with(&[("/www/public/style.css", b"body{}", 7)]);
        let file = p("/www/public/style.css");
        for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
            let refused = answered(serve(&file, &method, &HeaderMap::new(), &fs));
            assert_eq!(refused.0, StatusCode::METHOD_NOT_ALLOWED, "{method}");
            assert_eq!(header(&refused, "allow"), "GET, HEAD", "{method}");
            assert_eq!(header(&refused, "etag"), "", "{method}");
            assert!(refused.2.is_empty(), "{method} was sent the file");
        }
        for method in [Method::GET, Method::HEAD] {
            let sent = answered(serve(&file, &method, &HeaderMap::new(), &fs));
            assert_eq!(sent.0, StatusCode::OK, "{method}");
            assert_eq!(sent.2, b"body{}", "{method}");
        }
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

        // The exact file, with the static policy's freshness pair and the fixed table's type.
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
        // The static policy's sole default document, below the root and by its own spelling.
        let indexed = sent("/docs/", &[]);
        assert_eq!(indexed.2, b"<p>hi");
        assert_eq!(header(&indexed, "content-type"), "text/html; charset=utf-8");

        // A file that went away between selection and sending is not there,
        // rather than a fall-back to some other file.
        let gone = answered(send(&p("/www/public/gone.css"), &asking(&[]), &fs));
        assert_eq!(gone.0, StatusCode::NOT_FOUND);
    }

    /// An SVG, HTML or XML file a browser could run script from is sent
    /// sandboxed, so a file uploaded under a static root cannot act as the
    /// site. Every other type keeps the response policy's default.
    #[test]
    fn a_file_a_browser_runs_script_from_is_sandboxed() {
        let fs = Fake::with(&[
            ("/www/public/logo.svg", b"<svg/>", 1),
            ("/www/public/page.html", b"<p>", 1),
            ("/www/public/PAGE.HTM", b"<p>", 1),
            ("/www/public/feed.xml", b"<x/>", 1),
            ("/www/public/style.css", b"a{}", 1),
            ("/www/public/photo.png", b"png", 1),
        ]);
        let policy = |file: &str, headers: &[(&str, &str)]| {
            let answer = answered(send(&p(file), &asking(headers), &fs));
            header(&answer, "content-security-policy")
        };
        for file in [
            "/www/public/logo.svg",
            "/www/public/page.html",
            "/www/public/PAGE.HTM",
            "/www/public/feed.xml",
        ] {
            assert_eq!(policy(file, &[]), SANDBOXED, "{file}");
            assert_eq!(
                policy(file, &[("Range", "bytes=0-1")]),
                SANDBOXED,
                "{file} in part"
            );
        }
        for file in ["/www/public/style.css", "/www/public/photo.png"] {
            assert_eq!(policy(file, &[]), "", "{file}");
        }
    }

    /// A `Range` the server reads one way and a cache could read another is a
    /// `416`: a sign or a space in a bound, and a second `Range` line. The copy
    /// [`asked`] keeps for a program's file holds both lines.
    #[test]
    fn a_range_written_two_ways_is_unsatisfiable() {
        let fs = Fake::with(&[("/www/public/data.bin", b"0123456789", 1)]);
        let file = p("/www/public/data.bin");
        for range in ["bytes=+1-2", "bytes= 0-1", "bytes=0 -1"] {
            let answer = answered(send(&file, &asking(&[("Range", range)]), &fs));
            assert_eq!(answer.0, StatusCode::RANGE_NOT_SATISFIABLE, "{range}");
        }
        let mut twice = HeaderMap::new();
        twice.append(RANGE, HeaderValue::from_static("bytes=0-1"));
        twice.append(RANGE, HeaderValue::from_static("bytes=5-6"));
        for headers in [twice.clone(), asked(&twice)] {
            let answer = answered(send(&file, &headers, &fs));
            assert_eq!(answer.0, StatusCode::RANGE_NOT_SATISFIABLE);
            assert_eq!(header(&answer, "content-range"), "bytes */10");
        }
        let once = answered(send(&file, &asking(&[("Range", "bytes=0-1")]), &fs));
        assert_eq!(once.0, StatusCode::PARTIAL_CONTENT);
        assert_eq!(once.2, b"01");
    }
}
