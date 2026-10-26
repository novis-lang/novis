//! The debug spool: the files a run with `[debug] store` on writes its debug records to, and the one
//! line type `nvs debug` reads them back with
//! (`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`).
//!
//! # One type for both sides
//!
//! [`Line`] is what the writer serializes and what the ingester deserializes, so the two cannot come
//! to disagree about a field. A field added later is optional on read (`#[serde(default)]`), so an
//! older spool still ingests, and an unknown field is ignored, so a newer one does too. The rule's
//! § *What a line is* states the fields per kind, and
//! `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool` is one line of each, which this module's
//! test writes and the ingester's tests read.
//!
//! # One writer per thread, shaped like the log file target
//!
//! A [`Writer`] is one thread's, so it takes no lock: a core's thread under `nvs serve`, and the one
//! thread of `nvs run` and `nvs test`. It opens `<writer>.<n>.open` on its first record and never
//! before, appends one line per record through a buffer, and renames the file to
//! `<writer>.<n>.spool` at [`SEAL_BYTES`] or [`SEAL_AFTER`] after its first record. A sealed file is
//! never opened for writing again. A line is at most [`LINE_MAX`]: a longer record is written with
//! its tree or its bodies elided ([`Record::elide`]), and one still too long is counted as dropped.
//!
//! **The writer arms no timer.** [`Writer::deadline`] is when its open file is due, every append
//! seals a file past it first, and the caller that owns the thread's loop calls [`seal_due`] when
//! that time comes. That caller names a wake with [`on_open`], which runs each time the thread's
//! writer opens a file, so the caller parks with no deadline while the writer is idle. An idle writer
//! has no open file and no deadline. A file nobody sealed is read by the ingester once it is a minute
//! old ([`UNSEALED_AFTER`]).
//!
//! **Its share of `[debug] spool_max` is kept before each write.** The cap is split evenly between
//! the process's writers ([`set_writers`]), and a writer past its share deletes its own oldest sealed
//! file and then writes a `dropped` line with what it deleted. It counts only its own files, so the
//! cap needs no directory scan. A file the ingester already deleted is not counted as dropped.
//!
//! # The gate
//!
//! [`record`] writes nothing unless `[debug] store` is in force for the request
//! (`rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`), and with it off it costs one
//! read of the resolved configuration. The spool directory is prepared and checked once per process
//! ([`nvs_config::data::Folder::prepare_spool`]); a directory that fails the check gets a warning
//! once and no record.
//!
//! **What it spends:** per writer with an open file, one write buffer and one descriptor; per writer,
//! one entry per sealed file it has not evicted. On disk, at most `spool_max` per process. Nothing is
//! queued. Per recorded response body in flight, a [`Recorder`] holds at most [`BODY_CHUNK`]
//! bytes. Per recorded request body in flight, it holds the bytes the program read, up to
//! `[limits] request_body`, so a `secret` field the decoder found is replaced before any byte of
//! the body is written ([`Recorder::redact`]).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::Ctx;

/// The size at which a writer seals its open file.
pub const SEAL_BYTES: u64 = 4 << 20;
/// How long after its first record a writer's open file is sealed. It is the delay between a dump
/// and the viewer showing it.
pub const SEAL_AFTER: Duration = Duration::from_millis(250);
/// The longest line a writer writes, its newline included.
pub const LINE_MAX: usize = 4 << 20;
/// How long an unsealed file goes untouched before the ingester reads it as sealed: past any live
/// writer's [`SEAL_AFTER`].
pub const UNSEALED_AFTER: Duration = Duration::from_secs(60);
/// The extension of a file a writer is still writing.
pub const OPEN: &str = "open";
/// The extension of a sealed file, the only one the ingester reads before [`UNSEALED_AFTER`].
pub const SEALED: &str = "spool";
/// What a recorded credential is replaced with
/// (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`).
pub const REDACTED: &str = "[redacted]";
/// The headers whose values are replaced with [`REDACTED`] before a record is written, in the
/// lower case a header name is compared in.
pub const CREDENTIALS: [&str; 4] = [
    "authorization",
    "cookie",
    "set-cookie",
    "proxy-authorization",
];
/// The longest recorded body written inside its `request` line. A longer one is written as `body`
/// lines.
pub const BODY_INLINE: usize = 64 << 10;
/// The most bytes of a body one `body` line carries, and the most a response's [`Recorder`] holds.
pub const BODY_CHUNK: usize = 1 << 20;
/// Room kept for the `dropped` line an eviction writes, so it never needs a second eviction.
const DROPPED_ROOM: u64 = 256;

/// One line of a spool file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Line {
    /// The writer's id: the process id, the process's start in Unix milliseconds, and the thread's
    /// index, joined by `-`. With [`Line::seq`] it is the ingester's dedupe key.
    pub writer: String,
    /// This line's number among its writer's lines, from `0`.
    pub seq: u64,
    /// The request's trace id as 32 hex digits. Every line has one but `dropped`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
    /// When the line was written, in Unix microseconds.
    pub at_us: u64,
    /// The line's `kind` and the fields of that kind.
    #[serde(flatten)]
    pub record: Record,
}

impl Line {
    /// Reads one line of a spool file, without its newline.
    ///
    /// # Errors
    ///
    /// When `text` is not JSON, or not a line of a kind this build knows.
    pub fn parse(text: &str) -> serde_json::Result<Self> {
        serde_json::from_str(text)
    }
}

/// What a line records, tagged by its `kind`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Record {
    /// A request's outcome, and in `development` its input and its response. Boxed, because it is
    /// many times the size of every other kind.
    Request(Box<Request>),
    /// A trace event (`rule:observability/trace-events-carry-a-kind`).
    Event(Event),
    /// A `Core\Debug::dump` record's bounded tree (`rule:errors/record-transformations`).
    Dump(Dump),
    /// A log record the request wrote.
    Log(Log),
    /// A part of a recorded body too long for its `request` line ([`BODY_INLINE`]).
    Body(BodyPart),
    /// What this writer's share of the cap deleted since its last `dropped` line.
    Dropped(Dropped),
    /// A profile the request started with `Core\Debug::startProfile`.
    Profile(Profile),
    /// The lines a request ran, on a development server whose `[debug] mode` names `coverage`.
    Coverage(Coverage),
}

impl Record {
    /// Removes what can make a record long: a dump's tree, a log record's fields, and a request's
    /// recorded bodies. Each says so, with `elided` or `cut`.
    pub fn elide(&mut self) {
        match self {
            Self::Request(request) => {
                if let Some(input) = &mut request.input {
                    input.body.elide();
                }
                if let Some(response) = &mut request.response {
                    response.body.elide();
                }
            }
            Self::Dump(dump) => {
                dump.tree = serde_json::Value::Null;
                dump.elided = true;
            }
            Self::Log(log) => {
                log.fields = serde_json::Map::new();
                log.elided = true;
            }
            Self::Coverage(coverage) => {
                coverage.files = std::collections::BTreeMap::new();
                coverage.elided = true;
            }
            // A `body` line is at most [`BODY_CHUNK`] of bytes, so it is never too long.
            Self::Event(_) | Self::Body(_) | Self::Dropped(_) | Self::Profile(_) => {}
        }
    }
}

/// `kind: "request"`: what the index's `requests` row is made of, and its `inputs` row in
/// `development`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// The span id the server drew for the request, as 16 hex digits: the third part of its
    /// `traceresponse`. With the line's trace id it names the request, because an inbound
    /// `traceparent` can give two requests one trace id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    /// When the request started, in Unix microseconds.
    pub start_us: u64,
    /// How long it ran, in microseconds.
    pub duration_us: u64,
    /// The request method.
    pub method: String,
    /// The route pattern that matched, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    /// The path, without the query.
    pub path: String,
    /// The response status.
    pub status: u16,
    /// The exception the request did not catch, when there was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception: Option<Exception>,
    /// The request's query, headers and body, recorded in `development` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<Input>,
    /// The response's headers and body, recorded in `development` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,
}

/// An exception a request did not catch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exception {
    /// Its class.
    pub class: String,
    /// Its message.
    pub message: String,
    /// The file that threw it.
    pub file: String,
    /// The line that threw it.
    pub line: u32,
}

/// A request's input as the program saw it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    /// The query string, without the `?`.
    pub query: String,
    /// Each header as a name and a value, in the order they arrived, credentials replaced with
    /// [`REDACTED`].
    pub headers: Vec<(String, String)>,
    /// The body bytes the program read.
    pub body: Body,
}

/// A request's response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// Each header as a name and a value, `Set-Cookie` replaced with [`REDACTED`].
    pub headers: Vec<(String, String)>,
    /// The body the program wrote, up to `[limits] request_body`.
    pub body: Body,
}

/// A recorded body: its bytes as text when they are UTF-8, and as base64 when they are not.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Body {
    /// Whether the program read the body. A request body the program never read is `false`, and a
    /// replay sends none.
    pub read: bool,
    /// The length the request declared, when it declared one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<u64>,
    /// The bytes, when they are UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The bytes as standard base64, when they are not UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    /// Whether the bytes stop short of the body: at the cap, or by [`Record::elide`].
    #[serde(default)]
    pub cut: bool,
    /// How many `body` lines carry the bytes, for a body longer than [`BODY_INLINE`]. Such a body
    /// has no [`Body::text`] and no [`Body::base64`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunks: Option<u64>,
    /// Whether the bytes were left out, because a `secret` field in them could not be found to be
    /// replaced (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub withheld: bool,
}

impl Body {
    /// A body that was read, holding `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8], cut: bool) -> Self {
        let (text, base64) = encoded(bytes);
        Self {
            read: true,
            declared: None,
            text,
            base64,
            cut,
            chunks: None,
            withheld: false,
        }
    }

    /// The recorded bytes, decoded; `None` for a body never read and for base64 that does not
    /// decode.
    #[must_use]
    pub fn bytes(&self) -> Option<Vec<u8>> {
        match (&self.text, &self.base64) {
            (Some(text), _) => Some(text.as_bytes().to_vec()),
            (None, Some(base64)) => base64::engine::general_purpose::STANDARD
                .decode(base64)
                .ok(),
            (None, None) => self.read.then(Vec::new),
        }
    }

    fn elide(&mut self) {
        let had = self.text.is_some() || self.base64.is_some();
        self.text = None;
        self.base64 = None;
        self.cut |= had;
    }
}

/// `bytes` as a line writes them: as text when they are UTF-8, and as base64 when they are not.
fn encoded(bytes: &[u8]) -> (Option<String>, Option<String>) {
    match std::str::from_utf8(bytes) {
        Ok(text) => (Some(text.to_owned()), None),
        Err(_) => (
            None,
            Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        ),
    }
}

/// Which body a [`BodyPart`] is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyOf {
    /// The request's, as the program read it.
    Request,
    /// The response's, as the program wrote it.
    Response,
}

/// `kind: "body"`: the bytes of a recorded body from [`BodyPart::offset`] on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyPart {
    /// Which body the bytes are of.
    pub of: BodyOf,
    /// Where the bytes start in the body.
    pub offset: u64,
    /// The bytes, when they are UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The bytes as standard base64, when they are not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
}

/// One body as it is recorded, cut at `[limits] request_body`
/// (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`).
///
/// A response's bytes are held up to [`BODY_CHUNK`], and each full chunk is written as a `body`
/// line. A request's bytes are held until [`Recorder::finish`], because `jsonAs` and `postAs`
/// find a `secret` field only once the program has read the whole body. Each range
/// [`Recorder::redact`] names is replaced there, before any byte is written.
///
/// **What it spends:** per recorded response body in flight, at most [`BODY_CHUNK`] bytes. Per
/// recorded request body in flight, at most `[limits] request_body` bytes, and only in
/// `development`. A buffering reader holds the same bytes on the request already.
#[derive(Debug)]
pub struct Recorder {
    of: BodyOf,
    store: Store,
    trace: crate::TraceContext,
    held: Vec<u8>,
    /// The bytes already written as `body` lines.
    written: u64,
    chunks: u64,
    read: bool,
    cut: bool,
    /// The byte ranges of a request body to replace, each with its replacement.
    redactions: Vec<(std::ops::Range<usize>, &'static [u8])>,
    /// Whether the bytes are left out of the record ([`Recorder::withhold`]).
    withheld: bool,
}

impl Recorder {
    /// A recorder of the `of` body of the request in `trace`, under `store`.
    #[must_use]
    pub fn new(of: BodyOf, store: Store, trace: crate::TraceContext) -> Self {
        Self {
            of,
            store,
            trace,
            held: Vec::new(),
            written: 0,
            chunks: 0,
            read: false,
            cut: false,
            redactions: Vec::new(),
            withheld: false,
        }
    }

    /// Records `bytes`, the next ones of the body, up to the cap. An empty `bytes` records that
    /// the body was read.
    pub fn push(&mut self, bytes: &[u8]) {
        self.read = true;
        let taken = self.written + u64::try_from(self.held.len()).unwrap_or(u64::MAX);
        let room = usize::try_from(self.store.body_max.saturating_sub(taken)).unwrap_or(usize::MAX);
        if bytes.len() > room {
            self.cut = true;
        }
        let mut rest = &bytes[..bytes.len().min(room)];
        if self.of == BodyOf::Request {
            self.held.extend_from_slice(rest);
            return;
        }
        while !rest.is_empty() {
            let take = rest.len().min(BODY_CHUNK - self.held.len());
            self.held.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.held.len() == BODY_CHUNK {
                self.flush();
            }
        }
    }

    /// Replaces each of `spans`, byte ranges of the request body, with `with` when the body is
    /// written. A range past the bytes recorded is cut to them, so a value the cap cut in two is
    /// replaced as well. Ranges that overlap are replaced once.
    pub fn redact(&mut self, spans: &[std::ops::Range<usize>], with: &'static [u8]) {
        self.redactions
            .extend(spans.iter().map(|span| (span.clone(), with)));
    }

    /// Leaves the bytes out of the record. The body is still recorded as read.
    pub const fn withhold(&mut self) {
        self.withheld = true;
    }

    /// The body as its `request` line records it, with `declared` as the length the request
    /// declared. A body longer than [`BODY_INLINE`] writes its last `body` lines here.
    #[must_use]
    pub fn finish(mut self, declared: Option<u64>) -> Body {
        if !self.read {
            return Body {
                declared,
                ..Body::default()
            };
        }
        if self.withheld {
            return Body {
                read: true,
                declared,
                cut: self.cut,
                withheld: true,
                ..Body::default()
            };
        }
        self.replace_redactions();
        if self.chunks == 0 && self.held.len() <= BODY_INLINE {
            return Body {
                declared,
                ..Body::of(&self.held, self.cut)
            };
        }
        self.flush();
        Body {
            read: true,
            declared,
            text: None,
            base64: None,
            cut: self.cut,
            chunks: Some(self.chunks),
            withheld: false,
        }
    }

    /// Writes the held bytes as `body` lines of at most [`BODY_CHUNK`] bytes each.
    fn flush(&mut self) {
        let held = std::mem::take(&mut self.held);
        for bytes in held.chunks(BODY_CHUNK) {
            let (text, base64) = encoded(bytes);
            let part = BodyPart {
                of: self.of,
                offset: self.written,
                text,
                base64,
            };
            record_under(self.store, &self.trace, Record::Body(part));
            self.written += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            self.chunks += 1;
        }
    }

    /// Replaces each range [`Recorder::redact`] named in the held bytes, in order.
    fn replace_redactions(&mut self) {
        if self.redactions.is_empty() {
            return;
        }
        let mut spans = std::mem::take(&mut self.redactions);
        spans.sort_by_key(|(span, _)| span.start);
        let held = std::mem::take(&mut self.held);
        let mut out = Vec::with_capacity(held.len());
        let mut at = 0;
        for (span, with) in spans {
            if span.start >= held.len() {
                break;
            }
            let end = span.end.min(held.len());
            if span.start < at {
                // It overlaps a range already replaced, so its bytes are part of that one.
                at = at.max(end);
                continue;
            }
            out.extend_from_slice(&held[at..span.start]);
            out.extend_from_slice(with);
            at = end.max(span.start);
        }
        out.extend_from_slice(&held[at..]);
        self.held = out;
    }
}

/// `kind: "event"`: one trace event, the index's `events` row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Which of `rule:observability/trace-events-carry-a-kind`'s kinds it is.
    pub event: EventKind,
    /// What it is of, as the routine that filed it rendered it.
    pub label: String,
    /// When it started, in Unix microseconds.
    pub start_us: u64,
    /// When it ended, in Unix microseconds.
    pub end_us: u64,
    /// A `call` exit's checked-return status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    /// A `query` event's own fields, when its driver passed them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<Query>,
}

impl Event {
    /// `event` as a line records it: its clock readings as Unix microseconds.
    #[must_use]
    pub fn of(event: &crate::TraceEvent) -> Self {
        let micros = |reading| crate::unix_nanos(reading) / 1_000;
        Self {
            event: event.kind.into(),
            label: event.callee.clone(),
            start_us: micros(event.start),
            end_us: micros(event.end),
            status: event.status,
            query: None,
        }
    }
}

/// [`crate::TraceKind`], spelled as the rule spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A call site's entry or exit.
    Call,
    /// A cycle-collector pause.
    Gc,
    /// A spawned child.
    Spawn,
    /// One database statement.
    Query,
    /// One outbound HTTP call.
    Http,
}

impl From<crate::TraceKind> for EventKind {
    fn from(kind: crate::TraceKind) -> Self {
        match kind {
            crate::TraceKind::Call => Self::Call,
            crate::TraceKind::Gc => Self::Gc,
            crate::TraceKind::Spawn => Self::Spawn,
            crate::TraceKind::Query => Self::Query,
            crate::TraceKind::Http => Self::Http,
        }
    }
}

/// A `query` event's fields: never a bound parameter value, because a trace is a `secret` sink.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    /// The driver, such as `sqlite`.
    pub driver: String,
    /// The SQL text, truncated as the span truncates it.
    pub sql: String,
    /// The rows returned or affected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u64>,
    /// The call site id the compiler passed, when it passed one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<u32>,
}

/// `kind: "dump"`: one `Core\Debug::dump` record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dump {
    /// The file that called `dump`.
    pub file: String,
    /// The line that called it.
    pub line: u32,
    /// The bounded tree, as the dump's JSON rendering. `null` when [`Dump::elided`].
    pub tree: serde_json::Value,
    /// Whether the tree was removed to keep the line under [`LINE_MAX`].
    #[serde(default)]
    pub elided: bool,
}

/// `kind: "log"`: one log record the request wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Log {
    /// The level, such as `info`.
    pub level: String,
    /// The message.
    pub message: String,
    /// The record's own fields.
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
    /// Whether the fields were removed to keep the line under [`LINE_MAX`].
    #[serde(default)]
    pub elided: bool,
}

/// `kind: "profile"`: the file a request's `Core\Debug::startProfile` writes. The line is written
/// at the start, so the file is complete once the request, or its `stopProfile`, has ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// The file's absolute path, as the program's working directory resolved it.
    pub path: String,
}

/// `kind: "coverage"`: each line one request ran, by file, with its count — what
/// `Core\Debug::coverage` gives inside the request. Written once, when the request ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// The span id the server drew for the request, as its `request` line has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    /// File path to line to count, for the lines that ran. A line no statement ran on is absent.
    pub files: std::collections::BTreeMap<String, std::collections::BTreeMap<u32, u64>>,
    /// Whether `files` was removed because the line was too long.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub elided: bool,
}

/// `kind: "dropped"`: what a writer's share of the cap deleted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dropped {
    /// The records lost: the lines of the files deleted, and each record too long to write.
    pub records: u64,
    /// The files deleted.
    pub files: u64,
}

/// A clock reading for [`Writer::append`]: the wall clock a line is stamped with, and the monotonic
/// clock its file's seal is timed on.
#[derive(Clone, Copy, Debug)]
pub struct Now {
    /// Unix microseconds.
    pub unix_us: u64,
    /// The same moment on the monotonic clock.
    pub instant: Instant,
}

impl Now {
    /// The current time.
    #[must_use]
    pub fn read() -> Self {
        Self {
            unix_us: unix_micros(SystemTime::now()),
            instant: Instant::now(),
        }
    }
}

fn unix_micros(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH).map_or(0, |since| {
        u64::try_from(since.as_micros()).unwrap_or(u64::MAX)
    })
}

/// One thread's spool files. See the module doc.
#[derive(Debug)]
pub struct Writer {
    dir: PathBuf,
    id: String,
    /// This writer's share of `[debug] spool_max`.
    share: u64,
    /// The number the next file opened takes.
    next_file: u64,
    /// The number the next line takes.
    seq: u64,
    open: Option<OpenFile>,
    /// This writer's sealed files not yet evicted, oldest first.
    sealed: VecDeque<Written>,
    /// The bytes of [`Writer::sealed`].
    held: u64,
    /// What the cap deleted since the last `dropped` line.
    lost: Dropped,
}

#[derive(Debug)]
struct OpenFile {
    path: PathBuf,
    out: BufWriter<File>,
    bytes: u64,
    lines: u64,
    opened: Instant,
}

#[derive(Debug)]
struct Written {
    path: PathBuf,
    bytes: u64,
    lines: u64,
}

impl Writer {
    /// A writer named `id` that writes into `dir`, a directory that exists and passed the ownership
    /// check, and keeps to `share` bytes. It opens nothing until its first record.
    #[must_use]
    pub fn new(dir: PathBuf, id: String, share: u64) -> Self {
        Self {
            dir,
            id,
            share,
            next_file: 0,
            seq: 0,
            open: None,
            sealed: VecDeque::new(),
            held: 0,
            lost: Dropped::default(),
        }
    }

    /// The writer's id, the `<writer>` of its file names.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Sets this writer's share of the cap, which a reload of `[debug] spool_max` changes. The next
    /// append keeps to it.
    pub fn set_share(&mut self, share: u64) {
        self.share = share;
    }

    /// When the open file is due to be sealed; `None` while no file is open.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.open.as_ref().map(|open| open.opened + SEAL_AFTER)
    }

    /// Writes one line for `record`, stamped with `trace` and `now`. A file past its deadline is
    /// sealed first, the share is kept before the write, and a file that reaches [`SEAL_BYTES`] is
    /// sealed after it.
    ///
    /// # Errors
    ///
    /// The error of the create, the write or the rename that failed. A failed rename leaves the
    /// file as `.open`, and the ingester reads it after [`UNSEALED_AFTER`].
    pub fn append(
        &mut self,
        trace: Option<&str>,
        mut record: Record,
        now: Now,
    ) -> std::io::Result<()> {
        self.seal_if_due(now.instant)?;
        let mut length = self.render(trace, &record, now)?.len();
        if length > LINE_MAX {
            record.elide();
            length = self.render(trace, &record, now)?.len();
        }
        if length > LINE_MAX {
            self.lost.records += 1;
            return Ok(());
        }
        let length = u64::try_from(length).unwrap_or(u64::MAX);
        self.make_room(length.saturating_add(DROPPED_ROOM))?;
        if self.used().saturating_add(length) > self.share {
            self.lost.records += 1;
            return Ok(());
        }
        if self.lost != Dropped::default() {
            let lost = std::mem::take(&mut self.lost);
            let text = self.render(None, &Record::Dropped(lost), now)?;
            self.write(&text, now.instant)?;
        }
        let text = self.render(trace, &record, now)?;
        self.write(&text, now.instant)
    }

    /// Seals the open file when `now` is past its [`deadline`](Self::deadline).
    ///
    /// # Errors
    ///
    /// The flush's or the rename's error.
    pub fn seal_if_due(&mut self, now: Instant) -> std::io::Result<()> {
        if self.deadline().is_some_and(|due| due <= now) {
            self.seal()?;
        }
        Ok(())
    }

    /// Seals the open file now, if there is one.
    ///
    /// # Errors
    ///
    /// The flush's or the rename's error. The file is counted against the share either way.
    pub fn seal(&mut self) -> std::io::Result<()> {
        let Some(open) = self.open.take() else {
            return Ok(());
        };
        let OpenFile {
            path,
            out,
            bytes,
            lines,
            ..
        } = open;
        // Closed before the rename, which Windows refuses for a file this process still has open.
        let flushed = out.into_inner().map(drop).map_err(|err| err.into_error());
        let sealed = path.with_extension(SEALED);
        let renamed = flushed.and_then(|()| std::fs::rename(&path, &sealed));
        self.held += bytes;
        self.sealed.push_back(Written {
            path: if renamed.is_ok() { sealed } else { path },
            bytes,
            lines,
        });
        renamed
    }

    fn used(&self) -> u64 {
        self.held + self.open.as_ref().map_or(0, |open| open.bytes)
    }

    /// Deletes this writer's oldest files until `need` more bytes fit in its share.
    fn make_room(&mut self, need: u64) -> std::io::Result<()> {
        while self.used().saturating_add(need) > self.share {
            if let Some(oldest) = self.sealed.pop_front() {
                self.held -= oldest.bytes;
                match std::fs::remove_file(&oldest.path) {
                    Ok(()) => {
                        self.lost.files += 1;
                        self.lost.records += oldest.lines;
                    }
                    // The ingester read it and deleted it, so nothing was lost.
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    // Counted as gone, so a file that cannot be deleted never stops the writer.
                    Err(_) => {}
                }
            } else if self.open.as_ref().is_some_and(|open| open.bytes > 0) {
                self.seal()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn render(&self, trace: Option<&str>, record: &Record, now: Now) -> std::io::Result<String> {
        #[derive(Serialize)]
        struct Borrowed<'a> {
            writer: &'a str,
            seq: u64,
            #[serde(skip_serializing_if = "Option::is_none")]
            trace: Option<&'a str>,
            at_us: u64,
            #[serde(flatten)]
            record: &'a Record,
        }
        let mut text = serde_json::to_string(&Borrowed {
            writer: &self.id,
            seq: self.seq,
            trace,
            at_us: now.unix_us,
            record,
        })?;
        text.push('\n');
        Ok(text)
    }

    fn write(&mut self, text: &str, now: Instant) -> std::io::Result<()> {
        if self.open.is_none() {
            self.open = Some(self.create(now)?);
        }
        let open = self.open.as_mut().expect("opened above");
        open.out.write_all(text.as_bytes())?;
        open.bytes += u64::try_from(text.len()).unwrap_or(u64::MAX);
        open.lines += 1;
        self.seq += 1;
        if open.bytes >= SEAL_BYTES {
            self.seal()?;
        }
        Ok(())
    }

    fn create(&mut self, now: Instant) -> std::io::Result<OpenFile> {
        let path = self
            .dir
            .join(format!("{}.{}.{OPEN}", self.id, self.next_file));
        self.next_file += 1;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(&path)?;
        Ok(OpenFile {
            path,
            out: BufWriter::new(file),
            bytes: 0,
            lines: 0,
            opened: now,
        })
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        let _ = self.seal();
    }
}

thread_local! {
    /// This thread's writer, made on its first record.
    static WRITER: RefCell<Option<Writer>> = const { RefCell::new(None) };
    /// What runs when this thread's writer opens a file: the wake of the task that seals it.
    static ON_OPEN: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// How many writers share `[debug] spool_max`: one per core under `nvs serve`, set before the cores
/// start, and one otherwise.
static WRITERS: AtomicU64 = AtomicU64::new(1);
/// The index the next thread's writer takes.
static NEXT_THREAD: AtomicU32 = AtomicU32::new(0);
/// Whether a write failure has been reported, so a lasting one is printed once.
static REPORTED: AtomicBool = AtomicBool::new(false);

/// Sets how many writers split `[debug] spool_max` evenly.
pub fn set_writers(count: usize) {
    WRITERS.store(u64::try_from(count.max(1)).unwrap_or(1), Ordering::Relaxed);
}

/// Names what runs each time this thread's writer opens a file, and returns the one it replaces. The
/// task that calls [`seal_due`] on this thread names its own wake here, so it can park with no
/// deadline while no file is open.
pub fn on_open(wake: Option<Box<dyn Fn()>>) -> Option<Box<dyn Fn()>> {
    ON_OPEN.replace(wake)
}

/// Makes `writer` this thread's writer, for a caller that names its own directory, and answers the
/// one it replaces.
pub fn install(writer: Writer) -> Option<Writer> {
    WRITER.with_borrow_mut(|slot| slot.replace(writer))
}

/// `[debug] store` as one request has it, read where its configuration is and carried to a caller
/// that holds no request context, such as the server's door on a
/// [`Completion`](crate::host::Completion).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Store {
    /// `[debug] spool_max`, the cap the process's writers split.
    pub spool_max: u64,
    /// Whether the request ran in `development`, which is when its input and its response are
    /// recorded (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`).
    pub development: bool,
    /// `[limits] request_body`, where a recorded body is cut.
    pub body_max: u64,
}

impl Store {
    /// The gate `config` is under, or `None` when `[debug] store` is off for it. Off costs the one
    /// read [`nvs_config::Request::debug_store`] makes.
    #[must_use]
    pub fn of(config: &nvs_config::Request) -> Option<Self> {
        if !config.debug_store() {
            return None;
        }
        let body_max = match config.limit("request_body", nvs_config::Unit::Bytes) {
            nvs_config::Reading::Read(nvs_config::Quantity::Bytes(bytes)) if bytes > 0 => bytes,
            _ => nvs_config::defaults::LIMITS_REQUEST_BODY,
        };
        Some(Self {
            spool_max: config.debug_spool_max(),
            development: config.mode() == nvs_config::mode::DEVELOPMENT,
            body_max,
        })
    }
}

/// One header as a record writes it: its value replaced with [`REDACTED`] when its name is one of
/// the [`CREDENTIALS`], and read as UTF-8 with each invalid byte replaced.
#[must_use]
pub fn header(name: &str, value: &[u8]) -> (String, String) {
    let value = if CREDENTIALS
        .iter()
        .any(|credential| name.eq_ignore_ascii_case(credential))
    {
        REDACTED.to_owned()
    } else {
        String::from_utf8_lossy(value).into_owned()
    };
    (name.to_owned(), value)
}

/// Writes `record` for the request `ctx` runs, when `[debug] store` is in force for it. A failure
/// is printed once per process and never reaches the request.
pub fn record(ctx: &Ctx, record: Record) {
    if let Some(store) = ctx.config().and_then(Store::of) {
        record_under(store, ctx.trace_context(), record);
    }
}

/// Writes `record` for the request in `trace`, under the gate [`Store::of`] read for it, for a
/// caller that holds no request context. A failure is printed once per process.
pub fn record_under(store: Store, trace: &crate::TraceContext, record: Record) {
    let share = store.spool_max / WRITERS.load(Ordering::Relaxed).max(1);
    let trace = trace.trace_id_hex();
    with_writer(|writer| {
        writer.set_share(share);
        writer.append(Some(&trace), record, Now::read())
    });
}

/// Seals this thread's open file if it is due, and answers when the next one is due. The caller
/// that owns the thread's loop calls it at the time the last answer named.
pub fn seal_due() -> Option<Instant> {
    let now = Instant::now();
    WRITER.with_borrow_mut(|slot| {
        let writer = slot.as_mut()?;
        if let Err(err) = writer.seal_if_due(now) {
            report(&err);
        }
        writer.deadline()
    })
}

/// Seals this thread's open file now: where a run ends, since a thread's local values are not
/// dropped at process exit.
pub fn seal_now() {
    WRITER.with_borrow_mut(|slot| {
        if let Some(writer) = slot.as_mut()
            && let Err(err) = writer.seal()
        {
            report(&err);
        }
    });
}

fn with_writer(write: impl FnOnce(&mut Writer) -> std::io::Result<()>) {
    let opened = WRITER.with_borrow_mut(|slot| {
        if slot.is_none() {
            let Some(dir) = spool_dir() else {
                return false;
            };
            *slot = Some(Writer::new(dir.to_path_buf(), writer_id(), u64::MAX));
        }
        let Some(writer) = slot.as_mut() else {
            return false;
        };
        let was_open = writer.deadline();
        if let Err(err) = write(writer) {
            report(&err);
        }
        let now_open = writer.deadline();
        now_open.is_some() && now_open != was_open
    });
    // Run once the writer is no longer borrowed, so the wake may call [`seal_due`] at once.
    if opened {
        ON_OPEN.with_borrow(|wake| {
            if let Some(wake) = wake {
                wake();
            }
        });
    }
}

/// The process's spool directory, prepared and checked on first use; `None` when the data folder is
/// unusable, after one warning.
fn spool_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let folder = nvs_config::data::current()?;
        folder
            .prepare_spool()
            .inspect_err(|unusable| {
                let _ = writeln!(std::io::stderr(), "{}", unusable.warning());
            })
            .ok()
    })
    .as_deref()
}

/// `<pid>-<start>-<thread>`, which a restarted process never repeats.
fn writer_id() -> String {
    static STARTED: OnceLock<u64> = OnceLock::new();
    let started = *STARTED.get_or_init(|| unix_micros(SystemTime::now()) / 1_000);
    let thread = NEXT_THREAD.fetch_add(1, Ordering::Relaxed);
    format!("{}-{started}-{thread}", std::process::id())
}

fn report(err: &std::io::Error) {
    if !REPORTED.swap(true, Ordering::Relaxed) {
        // A write to `stderr` that fails is ignored, as the floor's own is.
        let _ = writeln!(
            std::io::stderr(),
            "warning: a debug record could not be written to the spool: {err}"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{
        BODY_CHUNK, BODY_INLINE, Body, BodyOf, BodyPart, Dropped, Dump, Event, EventKind,
        Exception, Input, Line, Log, Now, Query, Record, Recorder, Request, Response, SEAL_AFTER,
        Store, Writer,
    };

    const TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";

    /// A clock reading `ms` milliseconds after `base`, on both clocks.
    fn at(base: Instant, ms: u64) -> Now {
        Now {
            unix_us: 1_700_000_000_000_000 + ms * 1_000,
            instant: base + Duration::from_millis(ms),
        }
    }

    fn log(message: &str) -> Record {
        Record::Log(Log {
            level: "info".to_owned(),
            message: message.to_owned(),
            fields: serde_json::Map::new(),
            elided: false,
        })
    }

    /// Every file in `dir`, by name.
    fn files(dir: &Path) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        found.sort();
        found
    }

    fn lines(path: &Path) -> Vec<Line> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn name(path: &Path) -> String {
        path.file_name().unwrap().to_string_lossy().into_owned()
    }

    #[test]
    fn two_writers_never_share_a_file() {
        let dir = nvs_repo::scratch("spool-two-writers");
        let base = Instant::now();
        let mut first = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX);
        let mut second = Writer::new(dir.to_path_buf(), "1-2-1".to_owned(), u64::MAX);
        for ms in 0..3 {
            first
                .append(Some(TRACE), log("first"), at(base, ms))
                .unwrap();
            second
                .append(Some(TRACE), log("second"), at(base, ms))
                .unwrap();
        }
        first.seal().unwrap();
        second.seal().unwrap();

        let found = files(&dir);
        assert_eq!(
            found.iter().map(|path| name(path)).collect::<Vec<_>>(),
            ["1-2-0.0.spool", "1-2-1.0.spool"]
        );
        for path in &found {
            let writer = name(path).split('.').next().unwrap().to_owned();
            let read = lines(path);
            assert_eq!(read.len(), 3, "{}", path.display());
            assert!(read.iter().all(|line| line.writer == writer));
            assert_eq!(
                read.iter().map(|line| line.seq).collect::<Vec<_>>(),
                [0, 1, 2]
            );
        }
    }

    #[test]
    fn a_sealed_file_is_never_written_again() {
        let dir = nvs_repo::scratch("spool-sealed");
        let base = Instant::now();
        let mut writer = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX);
        assert_eq!(
            writer.deadline(),
            None,
            "a writer with no record has no file"
        );
        assert!(files(&dir).is_empty());

        writer.append(Some(TRACE), log("one"), at(base, 0)).unwrap();
        assert_eq!(writer.deadline(), Some(base + SEAL_AFTER));
        assert_eq!(name(&files(&dir)[0]), "1-2-0.0.open");
        let late = u64::try_from(SEAL_AFTER.as_millis()).unwrap();
        writer.seal_if_due(base + SEAL_AFTER).unwrap();
        let sealed = dir.join("1-2-0.0.spool");
        let before = std::fs::read(&sealed).unwrap();

        writer
            .append(Some(TRACE), log("two"), at(base, late + 1))
            .unwrap();
        writer.seal().unwrap();
        assert_eq!(
            std::fs::read(&sealed).unwrap(),
            before,
            "the sealed file is unchanged"
        );
        let second = lines(&dir.join("1-2-0.1.spool"));
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].seq, 1);
    }

    #[test]
    fn the_cap_evicts_oldest_first_and_counts_what_it_dropped() {
        let dir = nvs_repo::scratch("spool-cap");
        let base = Instant::now();
        let probe = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX);
        let one = u64::try_from(
            probe
                .render(Some(TRACE), &log("0"), at(base, 0))
                .unwrap()
                .len(),
        )
        .unwrap();
        drop(probe);
        // Room for three files of one line each, and the `dropped` line's reserve.
        let mut writer = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), 3 * one + 256);
        let step = u64::try_from(SEAL_AFTER.as_millis()).unwrap();
        for index in 0..5 {
            // Each record lands past the last file's deadline, so each file holds one line.
            writer
                .append(Some(TRACE), log(&index.to_string()), at(base, index * step))
                .unwrap();
        }
        writer.seal().unwrap();

        let names: Vec<String> = files(&dir).iter().map(|path| name(path)).collect();
        assert!(
            !names.contains(&"1-2-0.0.spool".to_owned()),
            "the oldest file is evicted first: {names:?}"
        );
        let all: Vec<Line> = files(&dir).iter().flat_map(|path| lines(path)).collect();
        let dropped: Dropped = all
            .iter()
            .filter_map(|line| match line.record {
                Record::Dropped(dropped) => Some(dropped),
                _ => None,
            })
            .fold(Dropped::default(), |sum, each| Dropped {
                records: sum.records + each.records,
                files: sum.files + each.files,
            });
        let kept = all
            .iter()
            .filter(|line| matches!(line.record, Record::Log(_)))
            .count();
        assert_eq!(
            u64::try_from(kept).unwrap() + dropped.records,
            5,
            "every record is either kept or counted as dropped: {all:?}"
        );
        assert_eq!(
            dropped.files, dropped.records,
            "one record per evicted file"
        );
        assert!(dropped.files >= 1);
        assert!(all.iter().all(|line| match line.record {
            Record::Dropped(_) => line.trace.is_none(),
            _ => line.trace.as_deref() == Some(TRACE),
        }));
    }

    #[test]
    fn a_file_the_ingester_deleted_is_not_counted_as_dropped() {
        let dir = nvs_repo::scratch("spool-ingested");
        let base = Instant::now();
        let probe = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX);
        let one = u64::try_from(
            probe
                .render(Some(TRACE), &log("0"), at(base, 0))
                .unwrap()
                .len(),
        )
        .unwrap();
        drop(probe);
        let mut writer = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), one + 256);
        writer.append(Some(TRACE), log("0"), at(base, 0)).unwrap();
        writer.seal().unwrap();
        std::fs::remove_file(dir.join("1-2-0.0.spool")).unwrap();
        writer.append(Some(TRACE), log("1"), at(base, 1)).unwrap();
        writer.seal().unwrap();

        let all = lines(&dir.join("1-2-0.1.spool"));
        assert_eq!(all.len(), 1, "no `dropped` line: {all:?}");
    }

    #[test]
    fn a_record_too_long_for_a_line_is_written_elided() {
        let dir = nvs_repo::scratch("spool-elided");
        let mut writer = Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX);
        let tree = serde_json::Value::String("x".repeat(super::LINE_MAX));
        writer
            .append(
                Some(TRACE),
                Record::Dump(Dump {
                    file: "app.nvs".to_owned(),
                    line: 3,
                    tree,
                    elided: false,
                }),
                Now::read(),
            )
            .unwrap();
        writer.seal().unwrap();

        let all = lines(&files(&dir)[0]);
        let Record::Dump(dump) = &all[0].record else {
            panic!("a dump: {all:?}");
        };
        assert!(dump.elided);
        assert_eq!(dump.tree, serde_json::Value::Null);
    }

    fn snapshot_of(written: &str) -> Arc<nvs_config::Snapshot> {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
    }

    #[test]
    fn with_the_gate_off_no_file_is_created() {
        let dir = nvs_repo::scratch("spool-gate");
        super::install(Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX));

        let mut ctx = crate::Ctx::buffered();
        super::record(&ctx, log("no configuration"));
        ctx.set_config(snapshot_of(""));
        super::record(&ctx, log("store unset"));
        ctx.set_config(snapshot_of("[debug]\nstore = false\n"));
        super::record(&ctx, log("store off"));
        super::seal_now();
        assert!(files(&dir).is_empty(), "{:?}", files(&dir));

        ctx.set_config(snapshot_of("[debug]\nstore = true\n"));
        super::record(&ctx, log("store on"));
        super::seal_now();
        assert_eq!(files(&dir).len(), 1);
        drop(super::WRITER.with_borrow_mut(Option::take));
    }

    /// The sealing task parks with no deadline, so the wake it names runs each time a file opens,
    /// and only then.
    #[test]
    fn the_wake_runs_when_a_file_opens_and_not_on_each_record() {
        let dir = nvs_repo::scratch("spool-wake");
        super::install(Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX));
        let woken = std::rc::Rc::new(std::cell::Cell::new(0));
        let previous = super::on_open(Some(Box::new({
            let woken = std::rc::Rc::clone(&woken);
            move || woken.set(woken.get() + 1)
        })));

        let mut ctx = crate::Ctx::buffered();
        ctx.set_config(snapshot_of("[debug]\nstore = true\n"));
        super::record(&ctx, log("first"));
        super::record(&ctx, log("second"));
        assert_eq!(woken.get(), 1);
        assert!(super::seal_due().is_some(), "a file is open");

        super::seal_now();
        assert_eq!(super::seal_due(), None);
        super::record(&ctx, log("third"));
        assert_eq!(woken.get(), 2);

        super::seal_now();
        super::on_open(previous);
        drop(super::WRITER.with_borrow_mut(Option::take));
    }

    /// With the gate on a request files its span events whatever the head draw said, and with it
    /// off only the draw decides.
    #[test]
    fn with_the_gate_on_a_request_records_its_spans_unsampled() {
        let mut ctx = crate::Ctx::buffered();
        assert!(
            !ctx.trace_context().sampled(),
            "a fresh context is not sampled"
        );
        ctx.set_config(snapshot_of("[debug]\nstore = false\n"));
        assert!(!ctx.records_spans());
        ctx.set_config(snapshot_of("[debug]\nstore = true\n"));
        assert!(ctx.records_spans());
    }

    /// One line of each kind, as `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool` holds them.
    fn one_of_each() -> Vec<Record> {
        let request = Record::Request(Box::new(Request {
            span: Some("00f067aa0ba902b7".to_owned()),
            start_us: 1_700_000_000_000_000,
            duration_us: 1_250,
            method: "POST".to_owned(),
            route: Some("/orders/{id}".to_owned()),
            path: "/orders/7".to_owned(),
            status: 500,
            exception: Some(Exception {
                class: "Shop\\OutOfStock".to_owned(),
                message: "item 7 is out of stock".to_owned(),
                file: "src/Shop/Orders.nvs".to_owned(),
                line: 42,
            }),
            input: Some(Input {
                query: "page=2".to_owned(),
                headers: vec![
                    ("Content-Type".to_owned(), "application/json".to_owned()),
                    ("Authorization".to_owned(), super::REDACTED.to_owned()),
                ],
                body: Body {
                    declared: Some(34),
                    ..Body::of(br#"{"item":7,"password":"[redacted]"}"#, false)
                },
            }),
            response: Some(Response {
                headers: vec![("Content-Type".to_owned(), "text/plain".to_owned())],
                body: Body {
                    read: true,
                    chunks: Some(1),
                    ..Body::default()
                },
            }),
        }));
        let body = Record::Body(BodyPart {
            of: BodyOf::Response,
            offset: 0,
            text: None,
            base64: Some("/wAB".to_owned()),
        });
        let event = Record::Event(Event {
            event: EventKind::Query,
            label: "sqlite main SELECT * FROM items WHERE id = ?".to_owned(),
            start_us: 1_700_000_000_000_100,
            end_us: 1_700_000_000_000_400,
            status: None,
            query: Some(Query {
                driver: "sqlite".to_owned(),
                sql: "SELECT * FROM items WHERE id = ?".to_owned(),
                rows: Some(1),
                site: Some(12),
            }),
        });
        let dump = Record::Dump(Dump {
            file: "src/Shop/Orders.nvs".to_owned(),
            line: 40,
            tree: serde_json::json!({"class": "Shop\\Item", "fields": {"id": 7, "stock": 0}}),
            elided: false,
        });
        let mut fields = serde_json::Map::new();
        fields.insert("item".to_owned(), serde_json::json!(7));
        let log = Record::Log(Log {
            level: "warning".to_owned(),
            message: "stock is empty".to_owned(),
            fields,
            elided: false,
        });
        let dropped = Record::Dropped(Dropped {
            records: 3,
            files: 1,
        });
        vec![body, request, event, dump, log, dropped]
    }

    /// The sample spool the ingester's tests read is exactly what this writer writes, and every line
    /// of it reads back as the record it was written from.
    #[test]
    fn the_sample_spool_is_what_the_writer_writes() {
        let dir = nvs_repo::scratch("spool-sample");
        let base = Instant::now();
        let mut writer = Writer::new(
            dir.to_path_buf(),
            "4242-1700000000000-0".to_owned(),
            u64::MAX,
        );
        for (index, record) in one_of_each().into_iter().enumerate() {
            let trace = (!matches!(record, Record::Dropped(_))).then_some(TRACE);
            writer
                .append(trace, record, at(base, u64::try_from(index).unwrap()))
                .unwrap();
        }
        writer.seal().unwrap();
        let written = std::fs::read_to_string(dir.join("4242-1700000000000-0.0.spool")).unwrap();

        let sample = nvs_repo::path("crates/nvs-cli/tests/fixtures/debug/one-of-each.spool");
        let committed = std::fs::read_to_string(&sample).unwrap_or_default();
        if committed != written {
            std::fs::create_dir_all(sample.parent().unwrap()).unwrap();
            std::fs::write(&sample, &written).unwrap();
            panic!(
                "{} was not what the writer writes, and is rewritten; review it and commit it",
                sample.display()
            );
        }
        let read: Vec<Record> = written
            .lines()
            .map(|line| serde_json::from_str::<Line>(line).unwrap().record)
            .collect();
        assert_eq!(read, one_of_each());
    }

    /// A request body's redacted ranges are replaced once each when it is written, a range past
    /// the cap is cut to it, and a withheld body is recorded as read with no bytes.
    #[test]
    fn a_request_recorder_replaces_each_redacted_range_or_withholds_the_bytes() {
        let store = |body_max| Store {
            spool_max: u64::MAX,
            development: true,
            body_max,
        };
        let trace = crate::TraceContext::started();
        let sent = br#"{"a":"s1","b":"s2"}"#;

        let mut whole = Recorder::new(BodyOf::Request, store(64), trace);
        whole.push(sent);
        whole.redact(&[5..9, 6..7], b"\"[redacted]\"");
        whole.redact(&[14..18, 5..9, 6..8], b"\"[redacted]\"");
        assert_eq!(
            whole.finish(None),
            Body::of(br#"{"a":"[redacted]","b":"[redacted]"}"#, false)
        );

        let mut cut = Recorder::new(BodyOf::Request, store(16), trace);
        cut.push(sent);
        cut.redact(&[14..18, 15..16], b"\"[redacted]\"");
        assert_eq!(
            cut.finish(None),
            Body::of(br#"{"a":"s1","b":"[redacted]""#, true),
            "a value the cap cut in two is replaced"
        );

        let mut withheld = Recorder::new(BodyOf::Request, store(64), trace);
        withheld.push(sent);
        withheld.withhold();
        assert_eq!(
            withheld.finish(Some(19)),
            Body {
                read: true,
                declared: Some(19),
                withheld: true,
                ..Body::default()
            }
        );
    }

    /// A body up to [`BODY_INLINE`] stays in its `request` line. A longer one is written as `body`
    /// lines of at most [`BODY_CHUNK`], and the whole stops at the cap and says so.
    #[test]
    fn a_long_body_is_written_as_body_lines_and_cut_at_the_cap() {
        let dir = nvs_repo::scratch("spool-body-lines");
        super::install(Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX));
        let store = Store {
            spool_max: u64::MAX,
            development: true,
            body_max: u64::try_from(BODY_CHUNK + 10).unwrap(),
        };
        let trace = crate::TraceContext::started();

        let mut short = Recorder::new(BodyOf::Request, store, trace);
        short.push(&[b'a'; BODY_INLINE]);
        assert_eq!(
            short.finish(None),
            Body::of(&[b'a'; BODY_INLINE], false),
            "a body of `BODY_INLINE` bytes is written inline"
        );

        let mut long = Recorder::new(BodyOf::Response, store, trace);
        long.push(&vec![b'b'; BODY_CHUNK - 5]);
        long.push(&[b'c'; 20]);
        let body = long.finish(None);
        assert_eq!(body.chunks, Some(2));
        assert!(body.cut, "the body passed the cap");
        assert_eq!((body.text, body.base64), (None, None));
        super::seal_now();

        let parts: Vec<BodyPart> = files(&dir)
            .iter()
            .flat_map(|path| lines(path))
            .filter_map(|line| match line.record {
                Record::Body(part) => Some(part),
                _ => None,
            })
            .collect();
        assert_eq!(
            parts.iter().map(|part| part.offset).collect::<Vec<_>>(),
            [0, u64::try_from(BODY_CHUNK).unwrap()]
        );
        let mut joined = String::new();
        for part in &parts {
            assert_eq!(part.of, BodyOf::Response);
            joined.push_str(part.text.as_deref().unwrap());
        }
        assert_eq!(joined.len(), BODY_CHUNK + 10, "the bytes stop at the cap");
        assert!(joined.ends_with(&"c".repeat(15)));
        drop(super::WRITER.with_borrow_mut(Option::take));
    }

    #[test]
    fn a_body_reads_back_as_its_bytes() {
        assert_eq!(Body::of(b"text", false).bytes(), Some(b"text".to_vec()));
        assert_eq!(Body::of(&[0xff, 0], false).bytes(), Some(vec![0xff, 0]));
        assert_eq!(Body::default().bytes(), None, "a body never read");
    }

    /// A context under `written`, carrying the trace [`TRACE`] names, with a writer of this
    /// thread's own on a fresh scratch directory. The test keeps the directory until it ends.
    fn storing(name: &str, written: &str) -> (crate::Ctx, nvs_repo::Scratch) {
        let dir = nvs_repo::scratch(name);
        super::install(Writer::new(dir.to_path_buf(), "1-2-0".to_owned(), u64::MAX));
        let mut ctx = crate::Ctx::buffered();
        ctx.set_config(snapshot_of(written));
        ctx.set_trace_context(crate::trace_context::TraceContext::continuing(
            Some(&format!("00-{TRACE}-00f067aa0ba902b7-01")),
            1.0,
        ));
        (ctx, dir)
    }

    /// Every line this thread's writer wrote to `dir`, once its file is sealed. The writer is
    /// taken out of the thread, so a later case on it starts with none.
    fn sealed_lines(dir: &Path) -> Vec<Line> {
        super::seal_now();
        drop(super::WRITER.with_borrow_mut(Option::take));
        files(dir).iter().flat_map(|path| lines(path)).collect()
    }

    /// `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`: a log record past
    /// `[log] level` is one `log` line with its level, its message and its fields, under the trace
    /// id of the request that wrote it. A record below the level writes no line.
    #[test]
    fn a_written_log_record_is_one_log_line_under_the_requests_trace() {
        let (mut ctx, dir) = storing(
            "spool-log-line",
            "[debug]\nstore = true\n\n[log]\nlevel = \"Info\"\n",
        );
        let mut quiet = nvs_render::Record::at(nvs_render::Level::Debug);
        quiet.envelope.message = Some(nvs_render::Rendered::new("below the level"));
        ctx.write_log_record(&quiet, crate::LogChannel::Output)
            .expect("a buffer never fails");
        let mut record = nvs_render::Record::at(nvs_render::Level::Warn);
        record.envelope.message = Some(nvs_render::Rendered::new("an order was paid twice"));
        record.envelope.fields = vec![(
            "order".to_owned(),
            nvs_render::Node::Scalar(nvs_render::Scalar::Int(7)),
        )];
        ctx.write_log_record(&record, crate::LogChannel::Output)
            .expect("a buffer never fails");

        let all = sealed_lines(&dir);
        let [line] = all.as_slice() else {
            panic!("one record past the level, one line: {all:?}");
        };
        assert_eq!(line.trace.as_deref(), Some(TRACE));
        let Record::Log(log) = &line.record else {
            panic!("not a log line: {line:?}");
        };
        assert_eq!(
            (log.level.as_str(), log.message.as_str()),
            ("warn", "an order was paid twice")
        );
        assert_eq!(log.fields.get("order"), Some(&serde_json::json!(7)));
        assert!(!log.elided);
    }

    /// `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`: a filed `query`
    /// event is one `event` line whose `query` has the driver, the SQL and the rows the span
    /// recorded, under the request's trace id.
    #[test]
    fn a_filed_query_event_is_an_event_line_with_its_driver_sql_and_rows() {
        let (mut ctx, dir) = storing("spool-query-event", "[debug]\nstore = true\n");
        let query = Query {
            driver: "postgres".to_owned(),
            sql: "select id from orders where shop = $1".to_owned(),
            rows: Some(3),
            site: None,
        };
        ctx.record_query(
            "query sql=select id from orders where shop = $1",
            query.clone(),
            Duration::from_millis(2),
        );

        let all = sealed_lines(&dir);
        let [line] = all.as_slice() else {
            panic!("one filed event, one line: {all:?}");
        };
        assert_eq!(line.trace.as_deref(), Some(TRACE));
        let Record::Event(event) = &line.record else {
            panic!("not an event line: {line:?}");
        };
        assert_eq!(event.event, EventKind::Query);
        assert_eq!(event.query.as_ref(), Some(&query));
        assert!(event.start_us <= event.end_us);
        assert_eq!(ctx.trace().len(), 1, "the event is in the trace as well");
    }
}
