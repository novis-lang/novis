//! A guest call: one instance per extension per request, run as a wasmtime async call on the
//! request's own core, its CPU time and linear memory charged to the request.
//!
//! [`Host`] is the process's half: one engine with epoch interruption, the pooling allocator and
//! only the wasm features in [`PROPOSALS`], the linker every instance is made with, and the ticker thread that advances the epoch every
//! [`TICK`]. A [`Loader`](crate::load::Loader) made over [`Host::engine`] compiles each component
//! once, and every core instantiates that same compiled component.
//!
//! [`Request`] is one request's half (`rule:packaging/a-fresh-instance-per-request`). It holds no
//! instance until the request's first call to an extension, then one per extension it called, and
//! drops them all with itself. A component is not re-entered: while one task's call is running,
//! a second task of the same request calling the same extension waits, parked, until the first
//! call returns. A call that fails drops its instance, so the request's next call to that
//! extension instantiates a fresh one.
//!
//! [`Request::call`] is a future the request's coroutine polls
//! (`rule:packaging/a-guest-call-yields-on-its-core`). wasmtime runs the guest on its own fiber, and
//! the future is pending in two cases: a host import returned a pending future, or an epoch tick
//! reached the guest. At each tick the guest asks [`Budget::cpu_spent`]; a spent budget traps the
//! call as [`Limit::Cpu`], and otherwise the call yields once so the core's other tasks run. The
//! waker is whatever polls the future, which on a server is the core's `RemoteWake`.
//!
//! **Memory** (`rule:packaging/a-guest-runs-under-the-requests-budget`). Every growth of a guest's
//! linear memory, its initial pages included, is charged to the request through
//! [`Budget::charge`], and given back when the instance is dropped. A growth fails the call as
//! [`Limit::Memory`] when the request cannot afford it, or when it takes one memory past the
//! smaller of the entry's `memory` and the manifest's `memory`. So the store's limit is the least
//! of the three. Reaching any of them is the same limit, because the store has one: the request
//! ends, whichever of the three was smallest.
//!
//! **What crosses is the request's memory too.** wasmtime lifts a value from the guest into
//! [`Val`]s, one per element of a list, and bounds each crossing by the store's hostcall fuel.
//! The fuel is [`Budget::left`] when the call starts and at every tick, so a crossing allocates
//! no more on the host than the request can still afford, and running out is [`Limit::Memory`].
//! A call's lowered arguments are measured once they are made and charged to the request until
//! the guest returns, and its lifted results are charged while they are converted; a call that
//! cannot afford either is [`Limit::Memory`], and both are given back after. A request with no
//! memory limit has no bound here either.
//!
//! **Failures** (`rule:packaging/a-guest-crash-throws`). Every export the loader accepts returns
//! `result<T, error>`, and [`Request::call`] returns the `ok` side's value or a [`Failure`]:
//! [`Failure::Error`] for an `err`, which keeps the instance, and [`Failure::Trap`] or
//! [`Failure::Limit`], which drop it. [`Failure::outcome`] is what the program sees of each: a
//! [`Outcome::Throw`] of the class the world maps an `err` to, or of `ExtensionError` for a trap,
//! or an [`Outcome::Fatal`] for a limit. The class and the limit are named in the spellings
//! `nvs_runtime::ThrownClass::name` and `nvs_runtime::Limit::name` use, which
//! `crates/nvs-ext/tests/failure.rs` pins, so the host that wires a call into a request raises the
//! throw with `Ctx::set_pending_as`, or runs `Ctx::run_limit_handler` and records the `FATAL`,
//! without this crate linking the runtime.
//!
//! **Novis values.** [`Request::call_values`] takes a call's Novis values, converts each by the
//! type the manifest declares for its parameter (`crate::convert`), lends a `mixed` one as a handle
//! and a third-party export's `bytes` as the `bytes` resource (`crate::handle`), and converts the
//! result back. An argument that is not a value of its type is
//! [`Error::Invalid`]: it throws `LogicError` and keeps the instance. A result that does not convert
//! is the extension's fault, a [`Failure::Trap`]. When the call returns, the borrows it was lent
//! are dropped and every handle it made stops being valid, whatever the outcome. It parses the
//! method's types on every call. A built-in component is called typed instead, through the
//! bindings `crate::typed` has for its world: its values convert to the generated types in the
//! instance's store, its handles lent, passed and kept as here, and the same charges are made at
//! those types' host size.
//!
//! **A resource lives until the request ends** (`rule:packaging/a-value-crosses-as-its-wit-type`).
//! One a call returns is kept in the store of the instance that made it, under a number the
//! request gives out once, and the program holds [`Value::Resource`]. Passed back, it crosses as a
//! `borrow`, so the request still owns it. [`Request::end`] drops every kept resource in its own
//! instance, which runs the guest's destructor, before the instances go. An instance dropped
//! earlier, by a trap or a limit, takes its resources with it and runs no destructor, and so does a
//! request dropped without [`Request::end`]; a number kept by an instance that is gone is refused
//! as [`Error::Invalid`], because no later instance gives it out again.
//!
//! **The budget is a trait.** [`Budget`] is what the store needs of its request — its CPU
//! deadline, its memory accounting, its log, its clocks, its random generator, its extension
//! settings, its caller's authority, which narrows each instance's preopens, and its outbound
//! HTTP — so this crate does not link the runtime. [`Budget::send`] returns an [`Outbound`] the
//! guest waits on, and the host fills its [`Answer`] where it holds the request's context, so the
//! guest's call is pending while the response is on its way. The host that wires a call into a request implements it over the
//! request's own; [`Meter`] is a standalone budget for a test and for a run with no request around
//! it, keeps no log, and reads the process's clocks and generator.
//!
//! What it spends: one instance per extension a request calls, its linear memory charged to that
//! request and freed with it, so O(in-flight). The pooling allocator reserves address space for
//! [`Host::new`]'s `slots` instances, not committed memory, and one ticker thread per process.

use std::ffi::OsString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Poll, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{fmt, future};

use rand::Rng;
use wasmtime::component::{ComponentExportIndex, Instance, Linker, ResourceAny, Val};
use wasmtime::{
    AsContextMut, Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig,
    ResourceLimiter, Store, Trap, UpdateDeadline, WasmFeatures,
};

use nvs_config::resolve::Disk;

use crate::convert::{self, Crossing, Value};
use crate::grants::{self, Caller};
use crate::handle::{self, Handles};
use crate::load::Extension;
use crate::manifest::{Method, Settings};
use crate::{typed, wasi, world};

/// How often the ticker advances the engine's epoch, which is how often a running guest checks
/// its request's CPU deadline and yields to the other tasks on its core.
pub const TICK: Duration = Duration::from_millis(1);

/// What a guest's store needs of the request it runs for.
pub trait Budget: Send + Sync {
    /// Whether the request has used all of its CPU time.
    fn cpu_spent(&self) -> bool;

    /// Charges `bytes` of guest linear memory to the request, or gives them back when negative.
    /// Returns `false`, and charges nothing, when the request cannot afford them.
    fn charge(&self, bytes: i64) -> bool;

    /// The bytes the request may still charge, or `None` for a request with no memory limit. The
    /// default is `None`.
    fn left(&self) -> Option<u64> {
        None
    }

    /// Writes `message` to the request's log at `level`, with `channel` as its channel.
    fn log(&self, level: Level, channel: &str, message: &str);

    /// The request's wall clock: the time since the Unix epoch, which a fixed clock fixes.
    fn wall_clock(&self) -> Duration;

    /// The request's monotonic clock, in nanoseconds since an origin the request chooses.
    fn monotonic_clock(&self) -> u64;

    /// Fills `out` from the request's random generator, which a declared seed fixes.
    fn random(&self, out: &mut [u8]);

    /// The value `key` has in the `[ext.<block>]` settings block of the request's configuration
    /// snapshot, or `None` when the block does not set it.
    fn setting(&self, block: &str, key: &str) -> Option<serde_json::Value>;

    /// The calling code's own authority: the `[capabilities]` of the request's snapshot and the
    /// `grants:` list of the isolate that runs it. It narrows what each instance may reach
    /// (`rule:security/extension-grants-are-an-intersection`). The default holds nothing.
    fn caller(&self) -> Caller<'_> {
        Caller::default()
    }

    /// Sends one outbound HTTP request for the guest, and returns the response it waits on.
    ///
    /// The host sends it through `Core\Http\Client`'s transport, under the caller's own
    /// `net.connect` grant, the address policy, TLS trust and the proxy. The guest's own grant
    /// is checked before this is asked (`rule:security/extension-grants-are-an-intersection`).
    /// The default sends nothing and returns [`Unsent::Refused`].
    fn send(&self, _request: Outgoing) -> Outbound {
        Outbound::ready(Err(Unsent::Refused(
            "this request has no outbound HTTP".to_owned(),
        )))
    }

    /// Reads at most `length` bytes at `offset` of the source the host lent this call as `id`
    /// ([`crate::convert::Value::Lent`]), fewer where the source ends first, and returns the
    /// read the guest waits on. The host serves it where it holds the source, between two polls
    /// of the call. The default lends nothing, so every read fails.
    fn read(&self, _id: u64, _offset: u64, _length: u64) -> Lending {
        Lending::ready(Err("this request lends no source".to_owned()))
    }

    /// Opens the file at `path` for the guest, as `how` says.
    ///
    /// `path` is resolved and absolute, and `wasi:filesystem` has checked it against the
    /// instance's preopens. The host opens it beneath a handle
    /// (`rule:security/writes-open-beneath-a-handle`), so a folder of it that became a link after
    /// the check fails the open. The default opens it by its whole path.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a level of `path` is a link.
    fn open_file(&self, path: &Path, how: FileOpen) -> io::Result<File> {
        OpenOptions::new()
            .read(how.read)
            .write(how.write)
            .create(how.create && !how.exclusive)
            .create_new(how.exclusive)
            .truncate(how.truncate)
            .open(path)
    }

    /// Creates the one folder `path` names for the guest, as [`Budget::open_file`] opens a file.
    /// The default creates it by its whole path.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a level of `path` is a link.
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    /// Moves the name `from` to the name `to` for the guest, as [`Budget::open_file`] opens a
    /// file. The default renames by the whole paths.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a folder above either name is a link.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    /// Removes the file `path` names for the guest, as [`Budget::open_file`] opens a file. A link
    /// at the name is the entry removed. The default removes it by its whole path.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a folder above the name is a link.
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    /// Removes the empty folder `path` names for the guest, as [`Budget::remove_file`] removes a
    /// file. The default removes it by its whole path.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a folder above the name is a link.
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }

    /// Reads the metadata at `path` for the guest, as [`Budget::open_file`] opens a file. With
    /// `follow` unset, a link at the last name is read as the link; with it set, the host does not
    /// follow a link there and fails with `NotFound`. The default reads by the whole path,
    /// following a link only when `follow` is set.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a folder of `path` is a link.
    fn metadata(&self, path: &Path, follow: bool) -> io::Result<Metadata> {
        if follow {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        }
    }

    /// Lists the folder at `path` for the guest, as [`Budget::open_file`] opens a file. Each
    /// entry's kind is read without following a link. The default lists it by its whole path.
    ///
    /// # Errors
    ///
    /// The operating system's error, or the host's when a level of `path` is a link.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<Listed>> {
        fs::read_dir(path)?
            .map(|entry| {
                let entry = entry?;
                let kind = entry.file_type().map_or(ListedKind::Other, |kind| {
                    if kind.is_symlink() {
                        ListedKind::Link
                    } else if kind.is_dir() {
                        ListedKind::Dir
                    } else if kind.is_file() {
                        ListedKind::File
                    } else {
                        ListedKind::Other
                    }
                });
                Ok(Listed {
                    name: entry.file_name(),
                    kind,
                })
            })
            .collect()
    }
}

/// One name in a folder [`Budget::read_dir`] listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The name, without the folder's path.
    pub name: OsString,
    /// What the name is, read without following a link.
    pub kind: ListedKind,
}

/// What a [`Listed`] name is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListedKind {
    /// A folder.
    Dir,
    /// A regular file.
    File,
    /// A symbolic link, or on Windows a junction.
    Link,
    /// Anything else.
    Other,
}

/// How [`Budget::open_file`] opens a file, flag by flag, as `std::fs::OpenOptions` names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per open option, the shape `std::fs::OpenOptions` has"
)]
pub struct FileOpen {
    /// The file may be read.
    pub read: bool,
    /// The file may be written.
    pub write: bool,
    /// A missing file is created.
    pub create: bool,
    /// The file is created, and a name already there fails with `AlreadyExists`.
    pub exclusive: bool,
    /// What was there is emptied. It needs `write`.
    pub truncate: bool,
}

/// One outbound HTTP request a guest sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
    /// The method, upper-case: `GET`, `POST`.
    pub method: String,
    /// The absolute `http` or `https` URL.
    pub url: String,
    /// The guest's headers, in the order it wrote them.
    pub headers: Vec<(String, String)>,
    /// The body, or `None` for a request that carries none.
    pub body: Option<Vec<u8>>,
    /// The hosts the extension may connect to: the intersection
    /// `rule:security/extension-grants-are-an-intersection` names. The URL's host is one of
    /// them, and so must be the host of every redirect the request follows.
    pub connect: Vec<String>,
}

/// The response to an [`Outgoing`] request, whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    /// The status code.
    pub status: u16,
    /// Every header, names lower-case, in arrival order.
    pub headers: Vec<(String, String)>,
    /// The body, with its transfer and content codings undone.
    pub body: Vec<u8>,
}

/// Why an [`Outgoing`] request has no response, with the message `Core\Http\Client` gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsent {
    /// The deadline passed: a `TimeoutError`.
    Timeout(String),
    /// The connection failed: an `IOError`.
    Connection(String),
    /// Anything else: a grant, the address policy, a TLS check, or a response too large.
    Refused(String),
}

/// What [`Budget::send`] returns: a future the guest's call waits on until the [`Answer`] made
/// with it is given the response.
#[derive(Debug)]
pub struct Outbound(Arc<Mutex<Awaited>>);

/// The half of an [`Outbound`] that the host gives the response to. Dropped without one, it
/// gives the guest [`Unsent::Refused`], so a guest never waits on a request nobody sends.
#[derive(Debug)]
pub struct Answer(Option<Arc<Mutex<Awaited>>>);

/// The response an [`Outbound`] waits on, and the waker of the call that waits.
#[derive(Debug, Default)]
struct Awaited {
    result: Option<Result<Incoming, Unsent>>,
    waker: Option<Waker>,
}

impl Outbound {
    /// A pending response and the [`Answer`] that fills it.
    #[must_use]
    pub fn pair() -> (Self, Answer) {
        let slot = Arc::new(Mutex::new(Awaited::default()));
        (Self(Arc::clone(&slot)), Answer(Some(slot)))
    }

    /// A response that is already there.
    #[must_use]
    pub fn ready(result: Result<Incoming, Unsent>) -> Self {
        Self(Arc::new(Mutex::new(Awaited {
            result: Some(result),
            waker: None,
        })))
    }
}

impl Future for Outbound {
    type Output = Result<Incoming, Unsent>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Answer {
    /// Gives the waiting call its response, and wakes it.
    pub fn fill(mut self, result: Result<Incoming, Unsent>) {
        self.filled(result);
    }

    fn filled(&mut self, result: Result<Incoming, Unsent>) {
        let Some(slot) = self.0.take() else {
            return;
        };
        let waker = {
            let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
            slot.result = Some(result);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        self.filled(Err(Unsent::Refused(
            "the request ended before this response arrived".to_owned(),
        )));
    }
}

/// What [`Budget::read`] returns: a future the guest's `range` waits on until the [`Lender`] made
/// with it is given the bytes.
#[derive(Debug)]
pub struct Lending(Arc<Mutex<Lent>>);

/// The half of a [`Lending`] that the host gives the bytes to. Dropped without them, it gives the
/// guest an error, so a guest never waits on a read nobody serves.
#[derive(Debug)]
pub struct Lender(Option<Arc<Mutex<Lent>>>);

/// The bytes a [`Lending`] waits on, and the waker of the call that waits.
#[derive(Debug, Default)]
struct Lent {
    result: Option<Result<Vec<u8>, String>>,
    waker: Option<Waker>,
}

impl Lending {
    /// A pending read and the [`Lender`] that fills it.
    #[must_use]
    pub fn pair() -> (Self, Lender) {
        let slot = Arc::new(Mutex::new(Lent::default()));
        (Self(Arc::clone(&slot)), Lender(Some(slot)))
    }

    /// A read that is already done.
    #[must_use]
    pub fn ready(result: Result<Vec<u8>, String>) -> Self {
        Self(Arc::new(Mutex::new(Lent {
            result: Some(result),
            waker: None,
        })))
    }
}

impl Future for Lending {
    type Output = Result<Vec<u8>, String>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Lender {
    /// Gives the waiting call its bytes, or the reason there are none, and wakes it.
    pub fn fill(mut self, result: Result<Vec<u8>, String>) {
        self.filled(result);
    }

    fn filled(&mut self, result: Result<Vec<u8>, String>) {
        let Some(slot) = self.0.take() else {
            return;
        };
        let waker = {
            let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
            slot.result = Some(result);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for Lender {
    fn drop(&mut self) {
        self.filled(Err("the call ended before this read was served".to_owned()));
    }
}

/// The levels of a request's log, as `nvs:ext/log` and `Core\Log` name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `debug`.
    Debug,
    /// `info`.
    Info,
    /// `warn`.
    Warn,
    /// `error`.
    Error,
    /// `critical`.
    Critical,
}

/// A budget of its own: a CPU deadline and an optional memory limit, with what is charged counted
/// here. Its clocks are the process's, its monotonic clock counting from the meter's creation,
/// and its random bytes come from `rand::rng()`, the thread's ChaCha12 generator `Core\Random`
/// draws from. It has no configuration, so every setting reads as its default.
#[derive(Debug)]
pub struct Meter {
    started: Instant,
    deadline: Instant,
    memory: Option<u64>,
    charged: AtomicI64,
}

impl Meter {
    /// A budget of `cpu` from now and at most `memory` bytes, or no memory limit for `None`.
    #[must_use]
    pub fn new(cpu: Duration, memory: Option<u64>) -> Self {
        let started = Instant::now();
        Self {
            started,
            deadline: started + cpu,
            memory,
            charged: AtomicI64::new(0),
        }
    }

    /// The bytes charged now.
    #[must_use]
    pub fn charged(&self) -> u64 {
        u64::try_from(self.charged.load(Ordering::Acquire)).unwrap_or(0)
    }
}

impl Budget for Meter {
    fn cpu_spent(&self) -> bool {
        Instant::now() >= self.deadline
    }

    fn charge(&self, bytes: i64) -> bool {
        self.charged
            .try_update(Ordering::AcqRel, Ordering::Acquire, |now| {
                let next = now.saturating_add(bytes);
                let over = bytes > 0
                    && self
                        .memory
                        .is_some_and(|limit| u64::try_from(next).unwrap_or(0) > limit);
                (!over).then_some(next)
            })
            .is_ok()
    }

    fn left(&self) -> Option<u64> {
        self.memory
            .map(|limit| limit.saturating_sub(self.charged()))
    }

    /// A meter has no request log, so it keeps nothing a guest writes.
    fn log(&self, _level: Level, _channel: &str, _message: &str) {}

    fn wall_clock(&self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    }

    fn monotonic_clock(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn random(&self, out: &mut [u8]) {
        rand::rng().fill_bytes(out);
    }

    fn setting(&self, _block: &str, _key: &str) -> Option<serde_json::Value> {
        None
    }
}

/// The request limit a call reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// The request's CPU time.
    Cpu,
    /// The request's memory, or the entry's or the manifest's ceiling where that is smaller.
    Memory,
}

impl Limit {
    /// The `[limits]` directive this limit is, as `nvs_runtime::Limit::name` spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu_time",
            Self::Memory => "memory",
        }
    }
}

/// An export's `err`: `nvs:ext/types`'s `error` variant, with its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `invalid(m)`: the caller passed a value the extension does not accept.
    Invalid(String),
    /// `parse(m)`: an input the extension reads is malformed.
    Parse(String),
    /// `runtime(m)`: the extension failed for a reason of its own.
    Runtime(String),
}

/// A guest that trapped, or did not instantiate, in a call to one export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crash {
    /// The extension's class.
    pub extension: String,
    /// The export the call was for.
    pub export: String,
    /// What the guest did.
    pub reason: String,
}

/// Why a call did not return a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The guest reached a request limit. The request ends.
    Limit(Limit),
    /// The export returned `err`. The instance is kept.
    Error(Error),
    /// The guest trapped, or did not instantiate. The instance is dropped.
    Trap(Crash),
    /// The manifest declares no such method, or the component does not export it.
    NoMethod(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(Limit::Cpu) => {
                f.write_str("the extension used all of the request's CPU time")
            }
            Self::Limit(Limit::Memory) => {
                f.write_str("the extension used all of the memory it may use")
            }
            Self::Error(
                Error::Invalid(message) | Error::Parse(message) | Error::Runtime(message),
            ) => f.write_str(message),
            Self::Trap(crash) => write!(
                f,
                "the extension `{}` crashed in `{}`: {}",
                crash.extension, crash.export, crash.reason
            ),
            Self::NoMethod(name) => write!(f, "the extension has no method `{name}`"),
        }
    }
}

impl std::error::Error for Failure {}

/// The class a failure throws, as `nvs_hir::errors::TREE` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `LogicError`, for `err(invalid(m))`.
    Logic,
    /// `ParseError`, for `err(parse(m))`.
    Parse,
    /// `RuntimeError`, for `err(runtime(m))`.
    Runtime,
    /// `ExtensionError`, for a trap.
    Extension,
}

impl Class {
    /// The class's name, as `nvs_runtime::ThrownClass::name` spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Logic => "LogicError",
            Self::Parse => "ParseError",
            Self::Runtime => "RuntimeError",
            Self::Extension => "ExtensionError",
        }
    }
}

/// What the program sees of a failed call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A throw of the class, with the message. A `catch` sees it, and the request goes on.
    Throw(Class, String),
    /// A resource-limit `FATAL`, with the message. It reaches `Core\Fatal::onLimit` with the
    /// limit's name, no `catch` sees it, and the request ends.
    Fatal(Limit, String),
}

impl Failure {
    /// What the program sees of this failure.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        let class = match self {
            Self::Limit(limit) => return Outcome::Fatal(*limit, self.to_string()),
            Self::Error(Error::Invalid(_)) => Class::Logic,
            Self::Error(Error::Parse(_)) => Class::Parse,
            Self::Error(Error::Runtime(_)) => Class::Runtime,
            // The loader refuses a manifest that does not match the exports, so a missing method
            // is a component the loader did not check, and that is the extension's fault.
            Self::Trap(_) | Self::NoMethod(_) => Class::Extension,
        };
        Outcome::Throw(class, self.to_string())
    }
}

/// What one instance's store carries: its request's budget, what it charged, and the settings
/// block its manifest declares.
pub struct Guest {
    budget: Arc<dyn Budget>,
    settings: Option<Settings>,
    ceiling: Option<u64>,
    charged: u64,
    reached: Option<Limit>,
    live: Arc<AtomicUsize>,
    pub(crate) handles: Handles,
    kept: Vec<Kept>,
    pub(crate) wasi: wasi::Context,
}

/// A resource the request keeps: its number, its type's short name, and the resource.
struct Kept {
    id: u64,
    name: String,
    resource: ResourceAny,
}

impl Guest {
    /// The request's budget.
    pub(crate) const fn budget(&self) -> &Arc<dyn Budget> {
        &self.budget
    }

    /// The settings block the extension's manifest declares, if any.
    pub(crate) const fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }
}

impl fmt::Debug for Guest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Guest")
            .field("ceiling", &self.ceiling)
            .field("charged", &self.charged)
            .finish_non_exhaustive()
    }
}

impl Drop for Guest {
    fn drop(&mut self) {
        self.budget
            .charge(-i64::try_from(self.charged).unwrap_or(i64::MAX));
        self.live.fetch_sub(1, Ordering::AcqRel);
    }
}

impl ResourceLimiter for Guest {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if maximum.is_some_and(|maximum| desired > maximum) {
            return Ok(false);
        }
        let grow = u64::try_from(desired.saturating_sub(current)).unwrap_or(u64::MAX);
        let past_ceiling = self
            .ceiling
            .is_some_and(|ceiling| u64::try_from(desired).unwrap_or(u64::MAX) > ceiling);
        if past_ceiling || !self.budget.charge(i64::try_from(grow).unwrap_or(i64::MAX)) {
            self.reached = Some(Limit::Memory);
            return Err(wasmtime::format_err!(
                "growing a memory to {desired} bytes is past the limit"
            ));
        }
        self.charged = self.charged.saturating_add(grow);
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(maximum.is_none_or(|maximum| desired <= maximum))
    }
}

/// The process's half: the engine, the linker every instance is made with, and the epoch ticker.
/// Cloning it shares all three.
#[derive(Clone)]
pub struct Host {
    shared: Arc<Shared>,
}

struct Shared {
    engine: Engine,
    linker: Linker<Guest>,
    live: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    ticker: Option<JoinHandle<()>>,
}

impl Drop for Shared {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(ticker) = self.ticker.take() {
            // A ticker that panicked has nothing left to stop.
            let _ = ticker.join();
        }
    }
}

impl fmt::Debug for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Host")
            .field("live_instances", &self.live_instances())
            .finish_non_exhaustive()
    }
}

/// The core instances one guest may hold: its own module, and the shim and fixup modules
/// `wit-component` adds beside it to lower an import through the guest's memory, which every
/// toolchain's libc does.
const CORE_INSTANCES: u32 = 3;

/// The tables one guest may hold: its own module's function table and the shim's.
const TABLES: u32 = 2;

/// The wasm features a guest may use, and the engine turns every other one off: WebAssembly 2.0
/// without `externref`, and the component model. That is what `wasm32-wasip2` with `+simd128`
/// emits for the built-in components and for the `nvs ext new` template, so a component built
/// either way loads. GC, exceptions, threads, tail calls, memory64, multi-memory, relaxed SIMD,
/// wide arithmetic and the component model's async are off, so a wasmtime bump that turns a new
/// proposal on by default does not widen the sandbox. A proposal joins this set only when a
/// built-in component or the template needs it.
pub const PROPOSALS: WasmFeatures = WasmFeatures::WASM2
    .difference(WasmFeatures::GC_TYPES)
    .union(WasmFeatures::COMPONENT_MODEL);

impl Host {
    /// A host with room for `slots` instances at once, its linker filled by `imports`, and its
    /// ticker started. Each slot reserves the pool's address space for one memory,
    /// [`CORE_INSTANCES`] core instances and [`TABLES`] tables, so that a componentized guest
    /// fits in one slot.
    ///
    /// # Errors
    ///
    /// When the engine cannot be configured, or `imports` fails.
    pub fn new(
        slots: u32,
        imports: impl FnOnce(&mut Linker<Guest>) -> wasmtime::Result<()>,
    ) -> wasmtime::Result<Self> {
        let mut pool = PoolingAllocationConfig::default();
        pool.total_component_instances(slots);
        pool.total_core_instances(slots.saturating_mul(CORE_INSTANCES));
        pool.total_memories(slots);
        pool.total_tables(slots.saturating_mul(TABLES));
        pool.total_stacks(slots);
        let mut config = Config::new();
        config.wasm_features(PROPOSALS.complement(), false);
        config.epoch_interruption(true);
        // One call per store at a time, on wasmtime's own fiber: a component is not re-entered.
        config.concurrency_support(false);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        handle::link(&mut linker)?;
        world::link(&mut linker)?;
        wasi::link(&mut linker)?;
        imports(&mut linker)?;
        let stop = Arc::new(AtomicBool::new(false));
        let ticker = {
            let engine = engine.clone();
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("nvs-ext-epoch".to_owned())
                .spawn(move || {
                    while !stop.load(Ordering::Acquire) {
                        std::thread::sleep(TICK);
                        engine.increment_epoch();
                    }
                })?
        };
        Ok(Self {
            shared: Arc::new(Shared {
                engine,
                linker,
                live: Arc::default(),
                stop,
                ticker: Some(ticker),
            }),
        })
    }

    /// The engine a [`Loader`](crate::load::Loader) compiles this host's extensions into.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.shared.engine
    }

    /// How many instances exist now, across every request.
    #[must_use]
    pub fn live_instances(&self) -> usize {
        self.shared.live.load(Ordering::Acquire)
    }

    /// The instances of one request, charged to `budget`.
    #[must_use]
    pub fn request(&self, budget: Arc<dyn Budget>) -> Request {
        Request {
            host: self.clone(),
            budget,
            slots: Mutex::default(),
            ids: AtomicU64::new(0),
            crossings: AtomicU64::new(0),
        }
    }
}

/// One request's instances, one per extension it called, dropped with it.
pub struct Request {
    host: Host,
    budget: Arc<dyn Budget>,
    slots: Mutex<Vec<Slot>>,
    ids: AtomicU64,
    /// How many times this request called into a guest (`rule:packaging/the-boundary-is-the-cost`).
    crossings: AtomicU64,
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("instances", &self.instances())
            .finish_non_exhaustive()
    }
}

/// One extension's place in a request: its instance, or the task running a call on it.
struct Slot {
    class: String,
    state: State,
    waiters: Vec<Waker>,
}

enum State {
    Idle(Box<Live>),
    Busy,
}

/// An instance and the store it lives in.
struct Live {
    store: Store<Guest>,
    instance: Instance,
    interface: ComponentExportIndex,
}

impl Request {
    /// How many instances this request holds now, a call's own included.
    #[must_use]
    pub fn instances(&self) -> usize {
        self.lock().len()
    }

    /// How many host-to-guest calls this request has made, each export call counted once.
    #[must_use]
    pub fn crossings(&self) -> u64 {
        self.crossings.load(Ordering::Relaxed)
    }

    /// Calls `method` of `extension` with `args`, and returns its results: the `ok` side's value
    /// where the export returns a `result`, and every result as it is where it does not.
    ///
    /// # Errors
    ///
    /// A [`Failure`]: a limit the guest reached, the export's `err`, a trap, or a method the
    /// extension does not have.
    pub async fn call(
        &self,
        extension: &Extension,
        method: &str,
        args: &[Val],
    ) -> Result<Vec<Val>, Failure> {
        let declared = declared(extension, method)?;
        self.run(
            extension,
            declared,
            |_, _| Ok(Sent::Dynamic(args.to_vec())),
            |_, results, at| match results {
                Got::Dynamic(results) => Ok(results),
                Got::Typed(_) => Err(at.crash("a dynamic call returned a typed result")),
            },
        )
        .await
    }

    /// Calls `method` of `extension` with the Novis values `args`, converted by the types its
    /// manifest declares, and returns its result as a Novis value: `None` for a `void` method.
    ///
    /// # Errors
    ///
    /// A [`Failure`], as [`Request::call`] returns one, and [`Error::Invalid`] where `args` are not
    /// values of the parameters' types.
    pub async fn call_values(
        &self,
        extension: &Extension,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Failure> {
        let declared = declared(extension, method)?;
        let manifest = &extension.manifest;
        let invalid = |err: String| Failure::Error(Error::Invalid(err));
        if args.len() != declared.params.len() {
            return Err(invalid(format!(
                "`{}` takes {} arguments, and the call passed {}",
                declared.name,
                declared.params.len(),
                args.len()
            )));
        }
        let world = manifest
            .builtin
            .as_ref()
            .and_then(|_| typed::World::of(&manifest.interface))
            .filter(|world| world.has(&declared.name));
        if let Some(world) = world {
            return self.call_typed(extension, declared, world, args).await;
        }
        let lends_bytes = manifest.builtin.is_none();
        let lower = |store: &mut Store<Guest>, _: At<'_>| {
            let mut call = Call {
                store,
                ids: &self.ids,
                lends_bytes,
            };
            declared
                .params
                .iter()
                .zip(args)
                .map(|(param, value)| {
                    let ty = manifest.novis_type(&param.ty).map_err(invalid)?;
                    convert::to_wit_with(&ty, value, &mut call).map_err(invalid)
                })
                .collect::<Result<_, _>>()
                .map(Sent::Dynamic)
        };
        let lift = |store: &mut Store<Guest>, results: Got, at: At<'_>| {
            let Got::Dynamic(results) = results else {
                return Err(at.crash("a dynamic call returned a typed result"));
            };
            if declared.returns == "void" {
                return Ok(None);
            }
            let ty = manifest
                .novis_type(&declared.returns)
                .map_err(|err| at.crash(err))?;
            match <[Val; 1]>::try_from(results) {
                Ok([val]) => {
                    let mut call = Call {
                        store,
                        ids: &self.ids,
                        lends_bytes,
                    };
                    convert::from_wit_with(&ty, val, &mut call)
                        .map(Some)
                        .map_err(|err| at.crash(err))
                }
                Err(results) => {
                    Err(at.crash(format!("it returned {} values, not one", results.len())))
                }
            }
        };
        self.run(extension, declared, lower, lift).await
    }

    /// Calls `declared` of a built-in `extension` typed, through `world`'s bindings: converts
    /// `args` to its generated types and its result back by the type the manifest declares.
    async fn call_typed(
        &self,
        extension: &Extension,
        declared: &Method,
        world: typed::World,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Failure> {
        let manifest = &extension.manifest;
        let invalid = |err: String| Failure::Error(Error::Invalid(err));
        let params = declared
            .params
            .iter()
            .map(|param| manifest.novis_type(&param.ty))
            .collect::<Result<Vec<_>, _>>()
            .map_err(invalid)?;
        let lower = |store: &mut Store<Guest>, _: At<'_>| {
            let mut call = Call {
                store,
                ids: &self.ids,
                lends_bytes: false,
            };
            world
                .lower(&declared.name, &params, args, &mut call)
                .map(|args| Sent::Typed(Box::new(args)))
                .map_err(invalid)
        };
        let lift = |store: &mut Store<Guest>, results: Got, at: At<'_>| {
            let Got::Typed(out) = results else {
                return Err(at.crash("a typed call returned dynamic results"));
            };
            if declared.returns == "void" {
                return Ok(None);
            }
            let ty = manifest
                .novis_type(&declared.returns)
                .map_err(|err| at.crash(err))?;
            let mut call = Call {
                store,
                ids: &self.ids,
                lends_bytes: false,
            };
            out.into_value(&ty, &mut call)
                .map(Some)
                .map_err(|err| at.crash(err))
        };
        self.run(extension, declared, lower, lift).await
    }

    /// Runs one call of `declared` on this request's instance of `extension`: `args` makes the
    /// arguments in its store, and `results` reads what the export returned.
    async fn run<R>(
        &self,
        extension: &Extension,
        declared: &Method,
        args: impl FnOnce(&mut Store<Guest>, At<'_>) -> Result<Sent, Failure>,
        results: impl FnOnce(&mut Store<Guest>, Got, At<'_>) -> Result<R, Failure>,
    ) -> Result<R, Failure> {
        let export = declared.export_name();
        let at = At {
            extension: &extension.manifest.class,
            export: &export,
        };
        let mut held = self.acquire(&extension.manifest.class).await;
        if held.live.is_none() {
            held.live = Some(Box::new(self.instantiate(extension, at).await?));
        }
        let Some(live) = held.live.as_mut() else {
            unreachable!("the instance was made above");
        };
        let outcome = match args(&mut live.store, at) {
            Ok(args) => {
                let sent = args.heap();
                if self.budget.charge(sent) {
                    self.crossings.fetch_add(1, Ordering::Relaxed);
                    let called = live.call(at, &declared.name, args).await;
                    self.budget.charge(-sent);
                    match called {
                        Ok(out) => {
                            let held = out.heap();
                            if self.budget.charge(held) {
                                let read = results(&mut live.store, out, at);
                                self.budget.charge(-held);
                                read
                            } else {
                                Err(Failure::Limit(Limit::Memory))
                            }
                        }
                        Err(failure) => Err(failure),
                    }
                } else {
                    Err(Failure::Limit(Limit::Memory))
                }
            }
            Err(failure) => Err(failure),
        };
        let outcome = match (live.end_call().await, outcome) {
            (Err(err), Ok(_)) => Err(at.crash(err)),
            (_, outcome) => outcome,
        };
        if outcome
            .as_ref()
            .is_err_and(|failure| !matches!(failure, Failure::Error(_)))
        {
            held.live = None;
        }
        outcome
    }

    /// Ends the request: drops every resource its calls returned, in the instance that made it,
    /// so each one's destructor runs in the guest, and then drops the instances.
    ///
    /// # Errors
    ///
    /// The first failure a destructor met. An instance whose destructor failed drops no more of
    /// its resources, and every other instance still drops all of its own.
    pub async fn end(self) -> Result<(), Failure> {
        let slots = std::mem::take(&mut *self.lock());
        let mut ended = Ok(());
        for slot in slots {
            let State::Idle(mut live) = slot.state else {
                continue;
            };
            let kept = std::mem::take(&mut live.store.data_mut().kept);
            for kept in kept {
                if let Err(err) = kept.resource.resource_drop_async(&mut live.store).await {
                    let export = format!("[resource-drop]{}", crate::kebab(&kept.name));
                    let at = At {
                        extension: &slot.class,
                        export: &export,
                    };
                    if ended.is_ok() {
                        ended = Err(failure(&live.store, &err, at));
                    }
                    break;
                }
            }
        }
        ended
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Slot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The extension `class`'s slot, marked busy, with its instance if it has one. Pending while
    /// another task's call holds it.
    async fn acquire<'r>(&'r self, class: &'r str) -> Held<'r> {
        let live = future::poll_fn(|cx| {
            let mut slots = self.lock();
            let Some(slot) = slots.iter_mut().find(|slot| slot.class == class) else {
                slots.push(Slot {
                    class: class.to_owned(),
                    state: State::Busy,
                    waiters: Vec::new(),
                });
                return Poll::Ready(None);
            };
            match std::mem::replace(&mut slot.state, State::Busy) {
                State::Idle(live) => Poll::Ready(Some(live)),
                State::Busy => {
                    slot.waiters.push(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await;
        Held {
            request: self,
            class,
            live,
        }
    }

    /// A fresh instance of `extension`, in a store charged to this request.
    async fn instantiate(&self, extension: &Extension, at: At<'_>) -> Result<Live, Failure> {
        let ceiling = match (extension.memory, extension.manifest.memory) {
            (Some(entry), Some(manifest)) => Some(entry.min(manifest)),
            (entry, manifest) => entry.or(manifest),
        };
        let reach = grants::effective(
            &extension.grants,
            &extension.manifest.requests,
            self.budget.caller(),
            &Disk,
        );
        self.host.shared.live.fetch_add(1, Ordering::AcqRel);
        let mut store = Store::new(
            &self.host.shared.engine,
            Guest {
                budget: Arc::clone(&self.budget),
                settings: extension.manifest.settings.clone(),
                ceiling,
                charged: 0,
                reached: None,
                live: Arc::clone(&self.host.shared.live),
                handles: Handles::default(),
                kept: Vec::new(),
                wasi: wasi::Context::new(
                    Arc::clone(&self.budget),
                    &extension.manifest.class,
                    &reach,
                ),
            },
        );
        store.limiter(|guest| guest);
        store.set_hostcall_fuel(fuel(&*self.budget));
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(|mut cx| {
            let guest = cx.data_mut();
            if guest.budget.cpu_spent() {
                guest.reached = Some(Limit::Cpu);
                return Ok(UpdateDeadline::Interrupt);
            }
            let fuel = fuel(&*guest.budget);
            cx.set_hostcall_fuel(fuel);
            Ok(UpdateDeadline::Yield(1))
        });
        let instance = match self
            .host
            .shared
            .linker
            .instantiate_async(&mut store, &extension.component)
            .await
        {
            Ok(instance) => instance,
            Err(err) => return Err(failure(&store, &err, at)),
        };
        let interface = extension
            .component
            .component_type()
            .exports(&self.host.shared.engine)
            .map(|(name, _)| name)
            .find(|name| crate::load::export_names(name, &extension.manifest.interface))
            .and_then(|name| instance.get_export_index(&mut store, None, name))
            .ok_or_else(|| Failure::NoMethod(extension.manifest.interface.clone()))?;
        Ok(Live {
            store,
            instance,
            interface,
        })
    }
}

/// The method `method` of `extension`'s manifest, matched as a Novis method name is.
fn declared<'e>(extension: &'e Extension, method: &str) -> Result<&'e Method, Failure> {
    extension
        .manifest
        .methods
        .iter()
        .find(|declared| declared.name.eq_ignore_ascii_case(method))
        .ok_or_else(|| Failure::NoMethod(method.to_owned()))
}

impl Live {
    /// Drops the borrows the call's `mixed` and `bytes` arguments were lent as, ends every handle
    /// the call made, and gives back what its `append`s charged.
    async fn end_call(&mut self) -> Result<(), String> {
        let lent = self.store.data_mut().handles.take_lent();
        let mut dropped = Ok(());
        for borrow in lent {
            if let Err(err) = borrow.resource_drop_async(&mut self.store).await {
                dropped = Err(format!("a lent handle did not drop: {err}"));
            }
        }
        let grown = self.store.data_mut().handles.end_call();
        if grown > 0 {
            self.store
                .data()
                .budget
                .charge(-i64::try_from(grown).unwrap_or(i64::MAX));
        }
        dropped
    }

    /// Calls the export `at` names with `args`, which it drops once they have crossed.
    async fn call(&mut self, at: At<'_>, method: &str, args: Sent) -> Result<Got, Failure> {
        let func = self
            .instance
            .get_export_index(&mut self.store, Some(&self.interface), at.export)
            .and_then(|index| self.instance.get_func(&mut self.store, index))
            .ok_or_else(|| Failure::NoMethod(method.to_owned()))?;
        let fuel = fuel(&*self.store.data().budget);
        self.store.set_hostcall_fuel(fuel);
        let args = match args {
            Sent::Dynamic(args) => args,
            Sent::Typed(args) => {
                return match (*args).call(&mut self.store, func).await {
                    Ok(Ok(out)) => Ok(Got::Typed(out)),
                    Ok(Err(err)) => Err(Failure::Error(err)),
                    Err(err) => Err(failure(&self.store, &err, at)),
                };
            }
        };
        let mut results = vec![Val::Bool(false); func.ty(&self.store).results().len()];
        func.call_async(&mut self.store, &args, &mut results)
            .await
            .map_err(|err| failure(&self.store, &err, at))?;
        drop(args);
        Ok(Got::Dynamic(Self::dynamic_results(results, at)?))
    }

    /// The `ok` side's value of a dynamic call's `results`, or the [`Failure`] its `err` is.
    fn dynamic_results(results: Vec<Val>, at: At<'_>) -> Result<Vec<Val>, Failure> {
        match <[Val; 1]>::try_from(results) {
            Ok([Val::Result(Ok(value))]) => Ok(value.map(|value| vec![*value]).unwrap_or_default()),
            Ok([Val::Result(Err(err))]) => Err(match err.as_deref() {
                Some(Val::Variant(case, Some(message))) => match (case.as_str(), &**message) {
                    ("invalid", Val::String(m)) => Failure::Error(Error::Invalid(m.clone())),
                    ("parse", Val::String(m)) => Failure::Error(Error::Parse(m.clone())),
                    ("runtime", Val::String(m)) => Failure::Error(Error::Runtime(m.clone())),
                    _ => at.crash("its error is not the world's `error` variant"),
                },
                _ => at.crash("its error is not the world's `error` variant"),
            }),
            Ok(result) => Ok(result.into()),
            Err(results) => Ok(results),
        }
    }
}

/// One call's arguments: the dynamic `Val`s of any export, or the typed arguments of a built-in
/// world's (`crate::typed`). The typed arguments are boxed, because the widest world's are a few
/// hundred bytes and a dynamic call's are a `Vec`.
enum Sent {
    Dynamic(Vec<Val>),
    Typed(Box<typed::Args>),
}

impl Sent {
    /// The host bytes the arguments hold on the heap, which the call charges to its request.
    fn heap(&self) -> i64 {
        match self {
            Self::Dynamic(vals) => on_heap(vals),
            Self::Typed(args) => i64::try_from(args.heap()).unwrap_or(i64::MAX),
        }
    }
}

/// What one call returned, in the form its [`Sent`] crossed in.
enum Got {
    Dynamic(Vec<Val>),
    Typed(typed::Out),
}

impl Got {
    /// The host bytes the results hold on the heap, which the call charges while they convert.
    fn heap(&self) -> i64 {
        match self {
            Self::Dynamic(vals) => on_heap(vals),
            Self::Typed(out) => i64::try_from(out.heap()).unwrap_or(i64::MAX),
        }
    }
}

/// One call's [`Crossing`]: the store of the instance it runs on, the request's resource numbers,
/// and whether a `bytes` crosses as the `bytes` resource, which is a third-party export's way.
struct Call<'s> {
    store: &'s mut Store<Guest>,
    ids: &'s AtomicU64,
    lends_bytes: bool,
}

impl Crossing for Call<'_> {
    fn lend(&mut self, value: Value) -> Result<Val, String> {
        handle::lend(self.store.as_context_mut(), value)
    }

    fn lend_bytes(&mut self, value: Value) -> Result<Val, String> {
        if self.lends_bytes {
            handle::lend_bytes(self.store.as_context_mut(), value)
        } else {
            convert::bytes_to_list(value)
        }
    }

    fn take_bytes(&mut self, val: Val) -> Result<Vec<u8>, String> {
        match val {
            Val::Resource(resource) if self.lends_bytes && resource.owned() => {
                handle::take_bytes(self.store.as_context_mut(), resource)
            }
            val if !self.lends_bytes => convert::list_to_bytes(val),
            other => Err(format!(
                "the extension returned {other:?} where a `bytes` is an `own<bytes>`"
            )),
        }
    }

    fn pass(&mut self, name: &str, id: u64) -> Result<Val, String> {
        self.store
            .data()
            .kept
            .iter()
            .find(|kept| kept.id == id && kept.name == name)
            .map(|kept| Val::Resource(kept.resource))
            .ok_or_else(|| {
                format!("the request keeps no `{name}` numbered {id} in this extension's instance")
            })
    }

    fn keep(&mut self, name: &str, resource: ResourceAny) -> Result<Value, String> {
        let id = self.ids.fetch_add(1, Ordering::Relaxed);
        self.store.data_mut().kept.push(Kept {
            id,
            name: name.to_owned(),
            resource,
        });
        Ok(Value::Resource {
            name: name.to_owned(),
            id,
        })
    }
}

/// The extension and the export a call is for, which a trap names.
#[derive(Clone, Copy)]
struct At<'a> {
    extension: &'a str,
    export: &'a str,
}

impl At<'_> {
    fn crash(self, reason: impl Into<String>) -> Failure {
        Failure::Trap(Crash {
            extension: self.extension.to_owned(),
            export: self.export.to_owned(),
            reason: reason.into(),
        })
    }
}

/// The failure `err` is, in a store that may have reached a limit.
fn failure(store: &Store<Guest>, err: &wasmtime::Error, at: At<'_>) -> Failure {
    if let Some(limit) = store.data().reached {
        return Failure::Limit(limit);
    }
    if err
        .chain()
        .any(|cause| cause.is::<Unaffordable>() || cause.to_string().ends_with(FUEL_EXHAUSTED))
    {
        return Failure::Limit(Limit::Memory);
    }
    match err.downcast_ref::<Trap>() {
        Some(Trap::Interrupt) => Failure::Limit(Limit::Cpu),
        // The trap or the host function's error alone: the error's context is a backtrace of
        // offsets into the guest.
        Some(trap) => at.crash(trap.to_string()),
        None => at.crash(err.root_cause().to_string()),
    }
}

/// A host import's refusal to allocate a size the guest chose, which [`failure`] reads as
/// [`Limit::Memory`]. [`afford`] makes it.
#[derive(Debug)]
struct Unaffordable {
    what: &'static str,
    bytes: u64,
}

impl std::fmt::Display for Unaffordable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} of {} bytes is past the request's memory",
            self.what, self.bytes
        )
    }
}

impl std::error::Error for Unaffordable {}

/// Refuses a host allocation of `bytes`, a size the guest chose, before it is made, when the
/// request has less than that left (`rule:packaging/a-guest-runs-under-the-requests-budget`).
/// The call then ends at the memory limit. A request with no memory limit refuses nothing here.
pub(crate) fn afford(budget: &dyn Budget, what: &'static str, bytes: u64) -> wasmtime::Result<()> {
    if budget.left().is_some_and(|left| bytes > left) {
        return Err(wasmtime::Error::new(Unaffordable { what, bytes }));
    }
    Ok(())
}

/// Charges `bytes` of host memory, a size the guest chose, to the request before it is allocated,
/// or ends the call at the memory limit as [`afford`] does when the request cannot afford them.
/// Whoever charged them gives them back.
pub(crate) fn grow(budget: &dyn Budget, what: &'static str, bytes: u64) -> wasmtime::Result<()> {
    if !budget.charge(i64::try_from(bytes).unwrap_or(i64::MAX)) {
        return Err(wasmtime::Error::new(Unaffordable { what, bytes }));
    }
    Ok(())
}

/// The end of the message wasmtime fails a crossing with when it has used up the store's hostcall
/// fuel. Its error type is private, so the message is the only thing to match.
const FUEL_EXHAUSTED: &str = "fuel allocated for hostcalls has been exhausted";

/// The hostcall fuel a crossing starts with: the bytes `budget` has left, or no bound for a
/// request with no memory limit. wasmtime counts a lifted value at its host size, so what one
/// crossing allocates on the host is at most what the request can still afford.
fn fuel(budget: &dyn Budget) -> usize {
    budget.left().map_or(usize::MAX, |left| {
        usize::try_from(left).unwrap_or(usize::MAX)
    })
}

/// The host bytes `vals` hold on the heap, which a call charges to its request while it holds them.
fn on_heap(vals: &[Val]) -> i64 {
    i64::try_from(vals.iter().map(lifted).sum::<u64>()).unwrap_or(i64::MAX)
}

/// The host bytes `val` puts on the heap: one [`Val`] for each value inside it, and the text of
/// each string and name. A scalar puts nothing there.
fn lifted(val: &Val) -> u64 {
    let text = |text: &str| text.len() as u64;
    let inside = |val: &Val| size_of::<Val>() as u64 + lifted(val);
    match val {
        Val::String(s) | Val::Enum(s) => text(s),
        Val::List(items) | Val::Tuple(items) => items.iter().map(inside).sum(),
        Val::Map(pairs) => pairs.iter().map(|(k, v)| inside(k) + inside(v)).sum(),
        Val::Record(fields) => fields.iter().map(|(k, v)| text(k) + inside(v)).sum(),
        Val::Variant(case, payload) => text(case) + payload.as_deref().map_or(0, inside),
        Val::Option(payload) | Val::Result(Ok(payload) | Err(payload)) => {
            payload.as_deref().map_or(0, inside)
        }
        Val::Flags(names) => names.iter().map(|name| text(name)).sum(),
        _ => 0,
    }
}

/// A slot one task's call holds. Dropping it puts the instance back, or removes the slot when the
/// call left no instance, and wakes the tasks waiting for it.
struct Held<'r> {
    request: &'r Request,
    class: &'r str,
    live: Option<Box<Live>>,
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        let live = self.live.take();
        let waiters = {
            let mut slots = self.request.lock();
            let Some(index) = slots.iter().position(|slot| slot.class == self.class) else {
                return;
            };
            match live {
                Some(live) => {
                    slots[index].state = State::Idle(live);
                    std::mem::take(&mut slots[index].waiters)
                }
                None => slots.remove(index).waiters,
            }
        };
        for waker in waiters {
            waker.wake();
        }
    }
}
