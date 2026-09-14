//! `rule:config/one-local-control-socket`, `rule:config/a-reload-names-what-it-could-not-apply` and `rule:config/no-network-control-surface`'s operator surface: the one local endpoint, the three operations a client
//! connected to it may ask for, and what each of them answers.
//!
//! **The endpoint itself is [`nvs_config::control`] and is re-exported here**, because creating an
//! object no other account can reach is a mode on Unix and a DACL on Windows and that crate already
//! owns both spellings for `rule:config/ownership-is-the-trust-boundary`'s trust boundary. What is *here* is the half that is about
//! being a server: which requests arriving on that endpoint mean anything.
//!
//! **The roster is three operations and it is closed.** `POST /reload` re-reads the tree and
//! publishes it; `GET /config` is `rule:config/ctl-config-reports-the-live-snapshot`'s read of the
//! snapshot this process is actually serving, with each directive's origin; `GET /status` reports
//! what the process is doing right now. A target outside those is refused by name rather than
//! resolved, and the refusal names it back so an operator holding a `nvs ctl` from a newer build
//! learns that is what they have.
//!
//! **No control operation runs Novis code, ever**, which is § 3's own sentence and the reason the
//! roster is an enum rather than a route table: a control surface that could dispatch is
//! `rule:security/no-eval`'s `eval` door with a different name on it. There is no path from here into the
//! compiler, and there is deliberately nothing to add one to.
//!
//! **What the process has to supply is [`Controlled`], and everything else is here.** Re-reading the
//! tree needs the roots this server booted on and the unit cache it holds, which are `nvs-cli`'s;
//! the in-flight count is the accept loop's; the drain bit is `nvs-runtime`'s. Behind that trait,
//! [`answer`] is the whole of what a connected client can cause, and a test drives it with a
//! recording process rather than a running one.
//!
//! **[`serve`] is the loop that reaches [`answer`], and it is one thread.** It accepts, answers the
//! client whole, closes, and accepts again; `rule:concurrency/one-scheduler` is why that is a
//! thread and not a second runtime, and [`answer_connection`]'s doc is why the connection on it is
//! driven by a loop of its own rather than by the runtime's. `hyper` frames both halves of the
//! exchange, as it does on the request path.
//!
//! Cost, as `rule:programs/memory-priority` requires: one answer's bytes, built and written and
//! dropped, and for `config` the flattened key list it is rendered from, plus
//! [`crate::io::SCRATCH`] bytes of the control thread's own stack while it is reading. Operations
//! serialize, so that is one at a time for the whole process rather than one per connection.

use std::convert::Infallible;
use std::io;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, header};
use nvs_config::audit::Audit;
use nvs_config::snapshot::Snapshot;

use crate::io::Nonblocking;
use crate::serve::Answer;

pub use nvs_config::control::{Address, Endpoint, Refusal, Report, bind, boundary, reload};

/// The header every answer carries, so a client can tell what it is talking to before it reads a
/// body — `rule:config/one-local-control-socket`.
pub const VERSION_HEADER: &str = "nvs-control-version";

/// What this build answers as. The wire shape is unstable until 1.0, so the version is the whole of
/// the compatibility story: there is no negotiation and no minimum, and two builds either match or
/// do not.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether an answer carrying `reported` came from a server this build may read.
///
/// `nvs ctl` refuses when this is false, and it lives here rather than in the client so that the
/// constant above has one reader as well as one home. An answer with no version header at all is a
/// mismatch: it is not this surface, whatever it is.
#[must_use]
pub fn same_build(reported: Option<&str>) -> bool {
    reported == Some(VERSION)
}

/// What the control surface asks of the process it is controlling.
///
/// The seam is here because each answer needs something only the running process holds, and none of
/// it belongs to this crate: re-reading the tree needs the roots and the unit cache `nvs-cli` owns,
/// the count is the accept loop's, and the drain bit is the runtime's. A test supplies a recording
/// implementation, which is what makes [`answer`] assertable without a server.
pub trait Controlled {
    /// Re-read the whole configuration tree and publish it.
    ///
    /// # Errors
    ///
    /// The refusal as text, already rendered. A malformed tree is reported as a diagnostic against
    /// the files it was read from, and the `SourceMap` those spans point into belongs to the caller
    /// that read them — so what crosses this seam is the rendering and never the span.
    fn reload(&self) -> Result<Report, String>;

    /// The snapshot serving now.
    fn snapshot(&self) -> Arc<Snapshot>;

    /// The `Boot` keys the last reload reported and left unapplied, if one has happened.
    ///
    /// Publishing carries every running `Boot` value back over the incoming tree, so by the time a
    /// snapshot exists the change that was refused is nowhere in it. This is the process
    /// remembering what it reported.
    fn unapplied(&self) -> Vec<&'static str>;

    /// Requests in flight across this process right now.
    fn in_flight(&self) -> usize;

    /// Whether the drain has begun — `rule:concurrency/a-drain-closes-a-connection-cleanly`.
    fn draining(&self) -> bool;
}

/// What a request arriving on the control endpoint asked for — `rule:config/one-local-control-socket`.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// `POST /reload`: re-read the whole configuration tree and publish it, reporting [`Report`].
    Reload,
    /// `GET /config`: the live snapshot, every key with the file it was written in —
    /// `rule:config/ctl-config-reports-the-live-snapshot`.
    Config,
    /// `GET /status`: how many requests are in flight, and whether the process is draining.
    Status,
}

/// Why a request arriving on the control endpoint asked for nothing this surface has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Denied {
    /// The target names no operation. It carries the target so the answer can name it back, which
    /// is what tells an operator holding a `nvs ctl` from a newer build that they have one.
    NoSuchOperation(String),
    /// The target names an operation, and the method is not the one it is performed with. A reload
    /// changes what the process is serving, so `GET` is refused rather than treated as a synonym —
    /// exactly the reason `rule:http-server/secure-headers-with-nothing-written` gives for safe methods being safe.
    ///
    WrongMethod {
        /// What the operation is performed with, for the `Allow` header the answer carries.
        allow: &'static str,
    },
}

impl Denied {
    /// The status this refusal is sent as.
    ///
    /// This surface sends its own statuses, and that is not `rule:routing/matched-once-before-the-handler`'s rule being bent: that
    /// rule is about a *request the program serves*, where the server matches and the program
    /// decides. Nothing an operator sends here reaches a program at all — § 3 says no control
    /// operation runs Novis code — so there is nobody else who could answer.
    ///
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::NoSuchOperation(_) => 404,
            Self::WrongMethod { .. } => 405,
        }
    }

    /// The refusal as the body it is sent as, naming the target back where there is one.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::NoSuchOperation(target) => {
                format!("no control operation is named `{target}`\n")
            }
            Self::WrongMethod { allow } => {
                format!("that operation is performed with `{allow}`\n")
            }
        }
    }
}

impl Operation {
    /// The operation `method` and `target` name, or why they name none.
    ///
    /// The target is compared whole and never normalized, which is the opposite of what a router
    /// does and is deliberate: this roster is a fixed set of exact names, so there is no traversal
    /// to resolve and nothing a `..` or an escape could resolve *to*. A target that is not exactly
    /// `/reload` is refused rather than folded into it, so no spelling of the operation exists that
    /// a log line would not show verbatim.
    ///
    /// # Errors
    ///
    /// [`Denied`]: an unknown target, or the wrong method for a known one.
    pub fn of(method: &str, target: &str) -> Result<Self, Denied> {
        // A query string is not part of the operation's name, and a control operation takes no
        // parameters at all — so one that arrives is part of the target that must not match.
        match target {
            "/reload" if method == "POST" => Ok(Self::Reload),
            "/reload" => Err(Denied::WrongMethod { allow: "POST" }),
            "/config" if method == "GET" => Ok(Self::Config),
            "/config" => Err(Denied::WrongMethod { allow: "GET" }),
            "/status" if method == "GET" => Ok(Self::Status),
            "/status" => Err(Denied::WrongMethod { allow: "GET" }),
            other => Err(Denied::NoSuchOperation(other.to_string())),
        }
    }

    /// The operation performed, as the body an answer carries.
    fn perform(self, host: &dyn Controlled) -> Result<String, String> {
        match self {
            Self::Reload => host.reload().map(|report| {
                let mut body = String::new();
                for key in &report.applied {
                    body.push_str(&format!("applied: {key}\n"));
                }
                for key in &report.ignored {
                    body.push_str(&format!("ignored: {key}\n"));
                }
                body.push_str(&format!("invalidated: {}\n", report.invalidated));
                body
            }),
            // Always with the origin column, because the target is the operation's whole name and a
            // control operation takes no parameters: there is nowhere for a client to ask for less,
            // and the origin is what this read exists for — the offline `nvs config dump` is where
            // the listing without it already is.
            Self::Config => {
                Ok(Audit::of_snapshot(&host.snapshot(), &host.unapplied()).render(true))
            }
            Self::Status => Ok(format!(
                "in_flight: {}\ndraining: {}\n",
                host.in_flight(),
                host.draining(),
            )),
        }
    }
}

/// The answer to `method target` on the control endpoint.
///
/// This is the whole of what a connected client can cause. It is one function and not a router
/// because the roster is fixed: [`Operation::of`] either names one of three things or refuses, and
/// nothing between here and [`Controlled`] can be reached any other way.
///
/// A reload the process refused is `409`, not `500` and not `400`. The request was well formed and
/// this surface is working; what conflicts with it is the state of the tree on disk, which the
/// operator changed out of band and is the thing they have to act on. The body is the rendered
/// refusal, so `curl --unix-socket` shows what a `nvs ctl` would print.
#[must_use]
pub fn answer(method: &str, target: &str, host: &dyn Controlled) -> Response<Answer> {
    match Operation::of(method, target) {
        Ok(operation) => match operation.perform(host) {
            Ok(body) => sent(StatusCode::OK, body, None),
            Err(why) => sent(StatusCode::CONFLICT, why, None),
        },
        Err(denied) => {
            let status = StatusCode::from_u16(denied.status()).unwrap_or(StatusCode::BAD_REQUEST);
            let allow = match &denied {
                Denied::WrongMethod { allow } => Some(*allow),
                Denied::NoSuchOperation(_) => None,
            };
            sent(status, denied.message(), allow)
        }
    }
}

/// One answer, with the header every answer carries and nothing a browser would act on.
///
/// The body is `text/plain` for the reason the wire protocol is HTTP at all: an operator reaches
/// this with `curl --unix-socket` and reads what comes back. `nosniff` is here because a `config`
/// listing is operator-supplied text going out over a channel whose client is not always `nvs ctl`.
fn sent(status: StatusCode, body: String, allow: Option<&'static str>) -> Response<Answer> {
    let mut response = Response::new(Answer::new(body));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::HeaderName::from_static(VERSION_HEADER),
        header::HeaderValue::from_static(VERSION),
    );
    if let Some(allow) = allow {
        headers.insert(header::ALLOW, header::HeaderValue::from_static(allow));
    }
    response
}

/// The endpoint's accept loop: one client at a time, until the process is draining.
///
/// This is `rule:config/one-local-control-socket`'s "operations serialize" as a mechanism rather
/// than as a promise. There is one endpoint, one thread and one stream alive at a time — the type
/// of [`Endpoint::accept`] says so — so two reloads arriving together are two connections, the
/// second of which waits on the transport and is answered whole after the first. Nothing here
/// shares state with a request, and nothing here runs Novis code.
///
/// The drain bit is read between clients and not during one: an operation already being answered
/// finishes, which is the same clean close
/// `rule:concurrency/a-drain-closes-a-connection-cleanly` gives a request. A thread parked in the
/// accept when the drain begins ends with the process, having answered nobody — there is no client
/// on it to close cleanly.
///
/// # Errors
///
/// The OS's error from the accept, which is the endpoint itself having gone: the loop ends rather
/// than spinning on a name that will not answer again. A *connection* that fails is one client's
/// problem — a `nvs ctl` killed mid-request — and the loop takes the next one.
pub fn serve(endpoint: &Endpoint, host: &dyn Controlled) -> io::Result<()> {
    while !host.draining() {
        drop(answer_connection(endpoint.accept()?, host));
    }
    Ok(())
}

/// Answers one connected client, and returns when its connection is closed.
///
/// `hyper` frames both halves, which is § 1's reason for the wire protocol being HTTP at all and
/// the same argument `crate::serve` makes for the request path: framing is where smuggling lives,
/// and a parser of ours reachable from an operator's `curl` is a parser of ours to get right.
/// Keep-alive is **off**: a control operation is one request, `nvs ctl` makes one per invocation,
/// and a client holding a connection open would hold the whole endpoint with it.
///
/// **The drive is this loop and not [`nvs_host::block_on`]**, because a park needs something to
/// end it. On a core that is the reactor; here there is neither, so the thing that would wake this
/// thread is this thread, and the loop says so: poll, and where the connection is waiting on its
/// peer, wait [`SETTLE`] and ask again. That is a sleep and not a spin, it is one thread that has
/// nothing else to do, and it costs an operator's command at most that much latency. The
/// alternative is overlapped IO and a second readiness mechanism for one connection at a time,
/// which is a great deal of machinery to save a millisecond on a reload.
///
/// [`IDLE`] is what keeps the wait bounded, as
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` requires of every wait: a
/// connection that has moved no byte in either direction for that long is given up on, and it is
/// **idle** time rather than total, so an operation that takes a while to answer is not cut off in
/// the middle of taking it.
///
/// # Errors
///
/// Whatever `hyper` ended the connection on, which is a client that went away or sent something
/// that is not HTTP, and [`io::ErrorKind::TimedOut`] for one that stopped saying anything at all.
/// It is this client's failure and never the endpoint's.
pub fn answer_connection(
    connected: impl std::io::Read + std::io::Write + Unpin,
    host: &dyn Controlled,
) -> io::Result<()> {
    let serving = service_fn(|request: Request<Incoming>| {
        // The query string is part of the target, because the roster matches a target whole:
        // `Operation::of`'s doc is why there is no spelling of an operation it would not show.
        let target = request.uri().path_and_query().map_or_else(
            || request.uri().path().to_owned(),
            |named| named.to_string(),
        );
        std::future::ready(Ok::<_, Infallible>(answer(
            request.method().as_str(),
            &target,
            host,
        )))
    });
    let driven = Nonblocking::new(connected);
    let moved = driven.moved();
    let mut connection = pin!(
        http1::Builder::new()
            .keep_alive(false)
            .serve_connection(driven, serving)
    );
    let mut context = Context::from_waker(Waker::noop());
    let mut since = Instant::now();
    loop {
        match connection.as_mut().poll(&mut context) {
            Poll::Ready(ended) => return ended.map_err(io::Error::other),
            Poll::Pending if moved.replace(false) => since = Instant::now(),
            Poll::Pending if since.elapsed() >= IDLE => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "a control client stopped part way through its operation",
                ));
            }
            Poll::Pending => std::thread::sleep(SETTLE),
        }
    }
}

/// How long a connection that is moving no bytes is left alone before the loop above gives up on
/// it. A control client is a program that sends one request and reads one answer, so a silence this
/// long is a client that is not coming back.
const IDLE: Duration = Duration::from_secs(30);

/// How long the drive waits before asking a connection that had nothing to say whether it has
/// anything now. Short enough to be invisible to the operator whose command is waiting behind it,
/// long enough that a connected client costs a thousandth of this thread rather than all of it.
const SETTLE: Duration = Duration::from_millis(1);

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::sync::Arc;

    use nvs_config::resolve::Origin;
    use nvs_config::snapshot::{Current, Snapshot};
    use nvs_config::tree::{Config, Control, Setting};
    use nvs_config::trust::Untrusted;

    use super::{
        Address, Answer, Controlled, Denied, Operation, Refusal, Report, VERSION, VERSION_HEADER,
        answer, answer_connection, bind, boundary, reload, same_build,
    };

    /// A directory of this case's own, empty, beside the test binary under `target/`.
    ///
    /// Not `std::env::temp_dir()`: on Unix that is `/tmp`, mode 1777, and
    /// the `rule:config/ownership-is-the-trust-boundary` check § 3's socket directory is held to refuses a directory the world can
    /// write. It runs on the named directory *and* on its parent, so a scratch directory of our own
    /// is refused for the `/tmp` above it however tight its own bits are. That refusal is § 3
    /// working, so the case names a directory the rule accepts rather than asking for a rule that
    /// accepts `/tmp`. `target/` is owned by this account and writable by neither its group nor the
    /// world, which is the same bar the operator's `/run/nvs` has to clear.
    fn scratch(name: &str) -> PathBuf {
        let beside = std::env::current_exe().expect("the test binary knows its own path");
        let dir = beside
            .parent()
            .expect("a test binary sits in a directory")
            .join(format!("nvs-control-{}-{name}", std::process::id()));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
        dir
    }

    /// The endpoint one case creates, in the platform's own namespace: a socket under `dir` on
    /// Unix, a pipe name on Windows, where `dir` exists only to be the thing § 3's directory rule
    /// is about.
    fn endpoint(dir: &std::path::Path, name: &str) -> PathBuf {
        #[cfg(unix)]
        {
            dir.join(format!("{name}.sock"))
        }
        #[cfg(windows)]
        {
            let _ = dir;
            PathBuf::from(format!(
                r"\\.\pipe\nvs-control-{}-{name}",
                std::process::id()
            ))
        }
    }

    /// Whether anything at all answers at `name`, and this account is the one it answers to — the
    /// question "was the endpoint created", asked in each platform's own terms, because a pipe is
    /// not a directory entry.
    ///
    /// `ERROR_PIPE_BUSY` counts as yes, and distinguishing it is the whole reason this is a match
    /// rather than an `is_ok`. One `CreateNamedPipeW` creates **one** instance, and reading a
    /// pipe's security descriptor opens it, so a case that has just asked what the DACL says finds
    /// the only instance already taken. The answers that matter are all different numbers:
    /// `2` is a name that is not there, `5` is one this account is refused, and `231` is one that
    /// exists and let this account through.
    fn exists(name: &std::path::Path) -> bool {
        #[cfg(unix)]
        {
            name.exists()
        }
        #[cfg(windows)]
        {
            const ERROR_PIPE_BUSY: i32 = 231;

            match fs::OpenOptions::new().read(true).write(true).open(name) {
                Ok(_) => true,
                Err(err) => err.raw_os_error() == Some(ERROR_PIPE_BUSY),
            }
        }
    }

    /// A `[control]` block holding one written value.
    fn written(socket: Setting) -> Config {
        Config {
            control: Some(Control {
                socket: Some(socket),
            }),
            ..Config::default()
        }
    }

    /// A snapshot of one TOML document, typed and untyped halves agreeing — which is what
    /// `Snapshot::build` produces and what `Current::publish` compares key by key.
    fn snapshot(document: &str) -> Snapshot {
        Snapshot {
            config: toml::from_str(document).expect("the case's own TOML deserializes"),
            table: document.parse().expect("the case's own TOML parses"),
            ..Snapshot::default()
        }
    }

    /// `rule:config/one-local-control-socket`: the socket is created mode `0600`, owned by the runtime's account — which is
    /// the whole authentication story, since there is no token and no auth middleware to fall back
    /// on.
    ///
    /// The claim is one and the spelling is two, exactly as `rule:config/ownership-is-the-trust-boundary`'s boundary is: on Unix a
    /// mode is readable off the socket file, and on Windows the equivalent question is whether any
    /// of § 6's untrusted principals reaches the object at all, which
    /// `nvs_config::trust::exposure` answers from its DACL. The pipe is opened afterwards on that
    /// platform because a descriptor that locked *this* account out would satisfy the first half
    /// and be useless.
    #[test]
    fn the_control_socket_is_created_0600() {
        let dir = scratch("created");
        let name = endpoint(&dir, "created");

        let created = bind(&name, boundary).expect("this account owns the directory it names");
        assert_eq!(
            created.name(),
            name,
            "it reports the name it was created under"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(&name)
                .expect("the socket is on disk")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(
                mode, 0o600,
                "nobody but the runtime's own account may connect: mode {mode:04o}",
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(
                nvs_config::trust::exposure(&name),
                None,
                "no account outside § 6's boundary reaches the pipe",
            );
            assert!(
                exists(&name),
                "and the descriptor did not lock this account out of its own endpoint",
            );
        }

        drop(created);
        drop(fs::remove_dir_all(&dir));
    }

    /// `rule:config/one-local-control-socket`'s other half of the same sentence: the server refuses to start if the socket's
    /// directory is world-writable, because an account that can write that directory can put a
    /// socket of its own there and speak for the server.
    ///
    /// The verdict is injected rather than constructed, for a reason the platforms force: Windows
    /// keeps named pipes in a kernel namespace with no directory to put in that state at all. What
    /// is asserted portably is the wiring — the refusal comes before anything is created — and the
    /// Unix half then runs the real `boundary` against a real directory in the real state, which is
    /// what stops the injection from being the only thing under test.
    #[test]
    fn a_world_writable_socket_directory_refuses_the_socket() {
        let dir = scratch("exposed");
        let name = endpoint(&dir, "exposed");

        let why = bind(&name, |_| {
            Err(Untrusted::Breach(
                "is world-writable (mode 0777, gid 1000)".to_string(),
            ))
        })
        .expect_err("a directory anyone can write is outside the boundary");
        assert!(
            matches!(why, Refusal::Untrusted(_)) && why.message().contains("world-writable"),
            "the refusal is the boundary's and says what is wrong: {why:?}",
        );
        assert!(
            !exists(&name),
            "and it was refused before anything was created — an endpoint left behind by a \
             refused boot is one an operator would find and trust",
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&dir, fs::Permissions::from_mode(0o777))
                .expect("a world-writable mode");
            let why = bind(&name, boundary).expect_err("the real check reads the real directory");
            assert!(
                why.message().contains("world-writable"),
                "the real boundary refuses the same state the injected verdict describes: {}",
                why.message(),
            );
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).expect("an owned mode");
        }

        drop(fs::remove_dir_all(&dir));
    }

    /// `rule:config/one-local-control-socket`: the surface has three operations and nothing else.
    /// Nothing is dispatched, nothing is compiled, and no Novis code runs — so a target this roster
    /// does not hold is refused rather than resolved, and the refusal names it back.
    ///
    /// The method each is performed with is part of its name. A reload changes what the process is
    /// serving, so `GET /reload` is refused rather than treated as a synonym, and the two reads are
    /// refused the other way round for the same reason read as a rule.
    #[test]
    fn the_socket_answers_three_operations_and_nothing_else() {
        assert_eq!(Operation::of("POST", "/reload"), Ok(Operation::Reload));
        assert_eq!(Operation::of("GET", "/config"), Ok(Operation::Config));
        assert_eq!(Operation::of("GET", "/status"), Ok(Operation::Status));

        for target in [
            "/shutdown",   // the operation an operator would most expect a control socket to have
            "/eval",       // `rule:security/no-eval`'s door, under the name it would arrive as
            "/",           // the root, which names nothing
            "/reload/",    // not a spelling of the operation: the target is compared whole
            "/reload?now", // a control operation takes no parameters
            "/RELOAD",
            "//reload",
            "/reload/../config",
            "/config/origin", // the origin column is not a second target; `/config` carries it
        ] {
            assert_eq!(
                Operation::of("POST", target),
                Err(Denied::NoSuchOperation(target.to_string())),
                "`POST {target}` names no operation this surface has",
            );
        }

        for method in ["GET", "HEAD", "PUT", "DELETE", "PATCH", "OPTIONS"] {
            let refused = Operation::of(method, "/reload")
                .expect_err("a reload changes what the process serves");
            assert_eq!(refused, Denied::WrongMethod { allow: "POST" });
            assert_eq!(
                refused.status(),
                405,
                "and the answer says which method does"
            );
        }
        for target in ["/config", "/status"] {
            assert_eq!(
                Operation::of("POST", target),
                Err(Denied::WrongMethod { allow: "GET" }),
                "`{target}` reads and changes nothing, so it is not performed with `POST`",
            );
        }
        assert_eq!(
            Operation::of("POST", "/shutdown").map_err(|why| why.status()),
            Err(404),
        );
    }

    /// The process a control answer is about, recorded rather than running: everything
    /// [`Controlled`] asks for, set by the case that is asking about it.
    struct Process {
        current: Current,
        unapplied: Vec<&'static str>,
        in_flight: usize,
        draining: bool,
        refuse: Option<String>,
    }

    impl Process {
        /// A process serving `document`, read from `written_in`, with nothing else going on.
        fn serving(document: &str, written_in: &str) -> Self {
            Self {
                current: Current::new(Arc::new(traced(document, written_in))),
                unapplied: Vec::new(),
                in_flight: 0,
                draining: false,
                refuse: None,
            }
        }
    }

    impl Controlled for Process {
        fn reload(&self) -> Result<Report, String> {
            match &self.refuse {
                Some(why) => Err(why.clone()),
                None => Ok(Report {
                    applied: vec!["limits.memory".to_string()],
                    ignored: vec!["control.socket"],
                    invalidated: 7,
                }),
            }
        }

        fn snapshot(&self) -> Arc<Snapshot> {
            self.current.load()
        }

        fn unapplied(&self) -> Vec<&'static str> {
            self.unapplied.clone()
        }

        fn in_flight(&self) -> usize {
            self.in_flight
        }

        fn draining(&self) -> bool {
            self.draining
        }
    }

    /// A snapshot of `document` with every leaf recorded as written in `written_in`, which is what
    /// makes the origin column have something to print. One file for the whole tree is enough here:
    /// what the case is about is that the column comes from the *snapshot*, and a second file only
    /// widens the same assertion.
    fn traced(document: &str, written_in: &str) -> Snapshot {
        let mut sources = nvs_diagnostics::SourceMap::new();
        let source = sources.add(written_in, document);
        let table: toml::Table = document.parse().expect("the case's own TOML parses");
        let origin = Origin {
            path: PathBuf::from(written_in),
            source,
        };
        let origins = nvs_config::audit::leaves(&table)
            .into_iter()
            .map(|(key, _)| (key, origin.clone()))
            .collect();
        Snapshot {
            config: toml::from_str(document).expect("the case's own TOML deserializes"),
            table,
            origins,
            ..Snapshot::default()
        }
    }

    /// The bytes an answer carries, which a control answer always has whole.
    fn body(response: hyper::Response<Answer>) -> String {
        match response.into_body() {
            Answer::Whole(Some(bytes)) => {
                String::from_utf8(bytes.to_vec()).expect("a control answer is text")
            }
            other => panic!("a control answer is one frame of bytes, not {other:?}"),
        }
    }

    /// `rule:config/ctl-config-reports-the-live-snapshot`: `config` reports what the running process
    /// actually holds, with the file each directive was written in — not what the files on disk say
    /// now.
    ///
    /// The difference is the whole point of the operation, so the case publishes a second tree over
    /// the first and asks again: the answer moves because it is read out of the snapshot, and both
    /// the value and its origin move with it. A `Boot` key the last reload could not apply is named
    /// on the row that still holds the running value, which is § *the live snapshot*'s other half.
    #[test]
    fn a_config_request_is_answered_with_the_live_snapshot_and_each_origin() {
        let mut process = Process::serving(
            "[control]\nsocket = \"/run/nvs/control.sock\"\n[limits]\nmemory = \"128M\"\n",
            "/etc/nvs/nvs.toml",
        );

        let first = body(answer("GET", "/config", &process));
        let row = first
            .lines()
            .find(|line| line.starts_with("limits.memory"))
            .expect("a key in force is a row in the listing");
        assert!(
            row.contains("= \"128M\"") && row.trim_end().ends_with("/etc/nvs/nvs.toml"),
            "every directive is reported with the file it was written in: {row}",
        );

        process.current = Current::new(Arc::new(traced(
            "[control]\nsocket = \"/run/nvs/control.sock\"\n[limits]\nmemory = \"256M\"\n",
            "/etc/nvs/conf.d/limits.toml",
        )));
        process.unapplied = vec!["control.socket"];

        let second = body(answer("GET", "/config", &process));
        assert!(
            second.contains("\"256M\"") && second.contains("/etc/nvs/conf.d/limits.toml"),
            "the answer is the snapshot now serving, and the origin moved with it:\n{second}",
        );
        assert!(
            !second.contains("\"128M\""),
            "the tree that stopped serving is gone from it:\n{second}",
        );
        let unapplied = second
            .lines()
            .find(|line| line.starts_with("control.socket"))
            .expect("the key is in the listing because it is in force");
        assert!(
            unapplied.ends_with("(changed on disk; needs a restart)"),
            "a `Boot` key the last reload left unapplied is named on its own row: {unapplied}",
        );
    }

    /// `status` reports what the process is doing right now: how many requests are in flight, and
    /// whether `rule:concurrency/a-drain-closes-a-connection-cleanly`'s drain has begun.
    ///
    /// Both are read at the moment of the answer and neither is remembered, which is what makes this
    /// the operation an operator polls during a stop: a `draining: true` with a falling count is a
    /// drain finishing, and the same two numbers are what a health check reads.
    #[test]
    fn a_status_request_reports_the_in_flight_count_and_whether_the_process_is_draining() {
        let mut process = Process::serving("[limits]\nmemory = \"128M\"\n", "/etc/nvs/nvs.toml");
        process.in_flight = 3;

        assert_eq!(
            body(answer("GET", "/status", &process)),
            "in_flight: 3\ndraining: false\n",
        );

        process.draining = true;
        process.in_flight = 1;
        assert_eq!(
            body(answer("GET", "/status", &process)),
            "in_flight: 1\ndraining: true\n",
            "a stop in progress says so, and says how much of it is left",
        );
    }

    /// `rule:config/one-local-control-socket`: operations serialize, so two reloads arriving
    /// together are answered one after the other rather than interleaving two snapshots.
    ///
    /// Two clients are on the endpoint at once and the loop is one thread, which is the mechanism
    /// that makes that true: the endpoint hands out one stream at a time, so the second client is
    /// answered only once the first one's connection is closed. What the case can assert about
    /// *timing* is the same thing twice, so what it asserts instead is the consequence — both
    /// answers arrive whole, each one a complete HTTP message with the version header and the
    /// report's own lines, neither of them the other's bytes.
    #[test]
    fn two_reloads_on_one_endpoint_are_answered_one_after_the_other() {
        let dir = scratch("two-reloads");
        let name = endpoint(&dir, "two-reloads");
        let listening = bind(&name, boundary).unwrap_or_else(|why| panic!("{}", why.message()));
        let process = Process::serving("[limits]\nmemory = \"128M\"\n", "/etc/nvs/nvs.toml");

        let clients: Vec<_> = (0..2)
            .map(|_| {
                let addressed = name.clone();
                std::thread::spawn(move || {
                    let mut connected = nvs_config::control::connect(&addressed)
                        .expect("a client reaches the endpoint the case bound");
                    connected
                        .write_all(
                            b"POST /reload HTTP/1.1\r\nHost: localhost\r\ncontent-length: 0\r\n\r\n",
                        )
                        .expect("the client's request");
                    connected.flush().expect("the client's request, delivered");
                    // Read to the report's last line rather than to the close, because the close is
                    // the *client's* to take: the answer is on the wire long before the server's
                    // connection is finished, and what finishes it is this client hanging up. A
                    // client that waited for an end of stream would be waiting for the server that
                    // is waiting for it.
                    let mut answered = String::new();
                    while !answered.contains("invalidated: 7") {
                        let mut arrived = [0_u8; 256];
                        let read = connected.read(&mut arrived).expect("the answer's next bytes");
                        assert!(read > 0, "the answer ended early: {answered:?}");
                        answered.push_str(&String::from_utf8_lossy(&arrived[..read]));
                    }
                    answered
                })
            })
            .collect();

        for _ in 0..2 {
            answer_connection(
                listening.accept().expect("a client is on the endpoint"),
                &process,
            )
            .expect("a client that asked for a reload is answered");
        }

        for client in clients {
            let answered = client.join().expect("the client thread");
            assert!(
                answered.starts_with("HTTP/1.1 200 OK\r\n"),
                "each client is answered whole: {answered:?}",
            );
            assert!(
                answered.contains(&format!("{VERSION_HEADER}: {VERSION}\r\n")),
                "every answer carries the version, this one over the wire: {answered:?}",
            );
            assert!(
                answered
                    .contains("applied: limits.memory\nignored: control.socket\ninvalidated: 7"),
                "the report is the body, whole and in one piece: {answered:?}",
            );
        }

        drop(listening);
        drop(fs::remove_dir_all(&dir));
    }

    /// `rule:config/one-local-control-socket`: the wire shape is unstable until 1.0, so **every**
    /// answer carries the server version and `nvs ctl` refuses a mismatch.
    ///
    /// Every means every: a refusal carries it too, because a `404` from a server of another build
    /// is exactly the answer an operator would otherwise read as "that operation does not exist"
    /// rather than as "you are talking to the wrong process". A reload the tree refused carries it
    /// for the same reason.
    #[test]
    fn every_control_answer_carries_the_server_version() {
        let mut process = Process::serving("[limits]\nmemory = \"128M\"\n", "/etc/nvs/nvs.toml");

        for (method, target) in [
            ("POST", "/reload"),
            ("GET", "/config"),
            ("GET", "/status"),
            ("GET", "/reload"),   // 405
            ("GET", "/shutdown"), // 404
        ] {
            let response = answer(method, target, &process);
            let reported = response
                .headers()
                .get(VERSION_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            assert!(
                same_build(reported.as_deref()),
                "`{method} {target}` answered {} without this build's version: {reported:?}",
                response.status(),
            );
        }

        process.refuse = Some("error: the tree does not parse\n".to_string());
        let refused = answer("POST", "/reload", &process);
        assert_eq!(
            refused.status(),
            409,
            "a tree the reload refused conflicts with the request; it is not this surface failing",
        );
        assert!(same_build(
            refused
                .headers()
                .get(VERSION_HEADER)
                .and_then(|value| value.to_str().ok())
        ));
        assert_eq!(body(refused), "error: the tree does not parse\n");

        assert!(
            !same_build(None) && !same_build(Some("0.0.0")),
            "an answer from another build, or from something that is not this surface at all, \
             is refused rather than read",
        );
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    /// `rule:config/no-network-control-surface`: there is no network-reachable control surface, in either direction of
    /// configuration.
    ///
    /// "Either direction" is what makes this a *boot* refusal rather than an absence: nothing in
    /// this tree binds a TCP socket for control, and a directive that asks for one is refused where
    /// it is written instead of being read as the relative path a host and a port happen to spell.
    /// A file named `127.0.0.1:9000` is a legal name on Unix, and creating one is the reading that
    /// leaves an operator believing they bound a port.
    #[test]
    fn there_is_no_network_reachable_control_surface() {
        for written_value in [
            "127.0.0.1:9000",
            "0.0.0.0:9000",
            "[::1]:9000",
            ":9000",
            "localhost:9000",
            "example.com:9000",
            "tcp://127.0.0.1:9000",
            "http://localhost/reload",
            "unix://run/nvs/control.sock",
        ] {
            let refused = Address::of(&written(Setting::Text(written_value.to_string())))
                .expect_err("`{written_value}` would be reached over a network");
            assert!(
                refused.message.contains("names no local endpoint"),
                "`socket = \"{written_value}\"` is refused at boot: {}",
                refused.message,
            );
        }
        assert!(
            Address::of(&written(Setting::Integer(9000))).is_err(),
            "a bare number is a port, and there is no listener for one to be bound on",
        );
        assert!(
            Address::of(&written(Setting::Bool(true))).is_err(),
            "`true` names no endpoint; only `false` means anything here",
        );

        // And the local spellings the ADR itself writes are accepted, so the refusal above is the
        // network shape and not a directive nothing can satisfy.
        let accepted = |value: &str| {
            Address::of(&written(Setting::Text(value.to_string())))
                .expect("§ 3's own example is a local endpoint")
        };
        assert_eq!(
            accepted("/run/nvs/control.sock"),
            Address::Local(PathBuf::from("/run/nvs/control.sock")),
        );
        assert_eq!(
            accepted(r"\\.\pipe\nvs-control"),
            Address::Local(PathBuf::from(r"\\.\pipe\nvs-control")),
        );
        assert_eq!(
            Address::of(&written(Setting::Bool(false))).expect("`false` disables the surface"),
            Address::Disabled,
        );
        assert_eq!(
            Address::of(&Config::default()).expect("no `[control]` block is not a refusal"),
            Address::Disabled,
            "a tree that asked for no control surface does not get one",
        );
    }

    /// `rule:config/a-reload-names-what-it-could-not-apply`: a reload names the `Boot` keys whose values changed and therefore did not take
    /// effect, individually, beside what it did apply.
    ///
    /// Silently ignoring a changed `Boot` key is how a deployment ends up believing it applied a
    /// change it did not, which is § 5's own sentence. The key used here is `control.socket`
    /// itself — a `Boot` row by `rule:config/reloadability-is-its-own-field` — so the case also pins that moving the control
    /// endpoint needs a restart rather than taking effect underneath the connection asking for it.
    #[test]
    fn a_changed_boot_key_is_reported_and_does_not_take_effect() {
        let current = Current::new(Arc::new(snapshot(
            "[control]\nsocket = \"/run/nvs/control.sock\"\n[limits]\nmemory = \"128M\"\n",
        )));

        let report = reload(
            &current,
            snapshot("[control]\nsocket = \"/run/nvs/other.sock\"\n[limits]\nmemory = \"256M\"\n"),
            12,
        )
        .expect("the incoming tree is well formed");

        assert_eq!(
            report.ignored,
            vec!["control.socket"],
            "the changed `Boot` key is named individually, not counted",
        );
        assert_eq!(
            report.applied,
            vec!["limits.memory".to_string()],
            "and the `Reload` key beside it did take effect",
        );
        assert_eq!(
            current
                .load()
                .config
                .control
                .as_ref()
                .and_then(|it| it.socket.clone()),
            Some(Setting::Text("/run/nvs/control.sock".to_string())),
            "the running value is still the one the process booted on",
        );
        assert_eq!(
            report.invalidated, 0,
            "and no compiled unit was invalidated: § 4 rekeys every unit or none, on `env_hash`, \
             which a `[limits]` change is no part of",
        );
    }
}
