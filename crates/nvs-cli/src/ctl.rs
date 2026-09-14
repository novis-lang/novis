//! `nvs ctl` — `rule:config/one-local-control-socket`'s client: the operator's end of the one
//! local endpoint a running server answers on, and the three operations it may ask for.
//!
//! A namespace of its own for the reason `nvs service` is one: every other subcommand in this
//! binary acts on files with no server involved, and these three exist only where a long-running
//! process does. `reload` re-reads the whole configuration tree and publishes it, `config` prints
//! the snapshot the process is actually holding with each directive's origin
//! (`rule:config/ctl-config-reports-the-live-snapshot`), and `status` reports what it is doing
//! right now. The roster is closed and it is [`nvs_server::control`]'s, which is also where the
//! method and target each operation is performed with are decided; nothing here may add a fourth.
//!
//! ## The request is `hyper`'s framing, in both directions
//!
//! The wire protocol is HTTP so that `curl --unix-socket` debugs it, and the framing on this half
//! is the same crate that frames the answer — the argument the workspace manifest makes over the
//! `hyper` line, which is that a parser of ours reachable from an operator's shell is a parser of
//! ours to get right. `hyper`'s client is a connection and a dispatcher that have to be polled
//! beside each other, and this thread has no reactor to park on, so [`exchange`] is the drive:
//! poll the exchange, poll the connection, settle where neither moved. That is
//! `nvs_server::control::answer_connection`'s loop from the other end, and it is a loop for the
//! same stated reason — a park needs something to end it.
//!
//! The transport it drives answers [`io::ErrorKind::WouldBlock`] rather than waiting, and that is
//! load-bearing rather than tidy: `hyper`'s client dispatcher reads the socket **before** it has
//! written anything, so a read that waited there would be waiting for a server that is waiting for
//! the request, and the whole exchange sits there until the server's own idle timeout ends it.
//! `nvs_config::control`'s `Client` is the half that answers `WouldBlock`, and its doc owns the
//! platform spellings.
//!
//! ## What it refuses
//!
//! An answer that does not carry this build's version is refused unread — the wire shape is
//! unstable until 1.0, so there is no negotiation and no minimum, and a `nvs` from one build
//! reading a server from another is the one failure mode the version header exists to make loud.
//! `nvs_server::control::same_build` is the comparison, so the constant has one reader as well as
//! one home.
//!
//! Cost, as `rule:programs/memory-priority` requires: one connection and one answer's bytes, held
//! for the length of a command that exits immediately after printing them.

use std::cell::Cell;
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::{Pin, pin};
use std::process::ExitCode;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use hyper::body::{Body, Bytes, Frame, SizeHint};
use hyper::client::conn::http1;
use hyper::header::{self, HeaderValue};
use hyper::{Request, StatusCode};
use nvs_config::control::Address;
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_server::control::{VERSION, VERSION_HEADER, same_build};
use nvs_server::io::Nonblocking;

use crate::config::{LocalFiles, named_roots, working_directory};
use crate::render_diagnostics;

/// `nvs ctl reload`: re-read the tree and publish it, printing what was applied and what the
/// running process could not take — `rule:config/a-reload-names-what-it-could-not-apply`.
pub(crate) fn reload(config: &[PathBuf], socket: Option<&Path>) -> ExitCode {
    run(config, socket, "POST", "/reload")
}

/// `nvs ctl config`: the live snapshot with each directive's origin.
pub(crate) fn config(config: &[PathBuf], socket: Option<&Path>) -> ExitCode {
    run(config, socket, "GET", "/config")
}

/// `nvs ctl status`: how many requests are in flight, and whether the process is draining.
pub(crate) fn status(config: &[PathBuf], socket: Option<&Path>) -> ExitCode {
    run(config, socket, "GET", "/status")
}

/// One operation, end to end: find the endpoint, ask, check what answered, print it.
///
/// The answer goes to standard output only when the server performed the operation. A refusal is
/// the server's own text and goes to standard error with the status that carried it, so a
/// `nvs ctl config > snapshot.toml` in a shell script writes a snapshot or writes nothing.
fn run(config: &[PathBuf], socket: Option<&Path>, method: &str, target: &str) -> ExitCode {
    let Some(address) = address(config, socket) else {
        return ExitCode::FAILURE;
    };
    let connected = match nvs_config::control::connect(&address) {
        Ok(connected) => connected,
        Err(failed) => {
            eprintln!(
                "error: no control endpoint answered at `{}`: {failed}\nA server is reachable \
                 here only while it is running, and only by the account it runs as.",
                address.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let answered = match exchange(connected, method, target) {
        Ok(answered) => answered,
        Err(failed) => {
            eprintln!(
                "error: the control operation `{method} {target}` did not complete: {failed}"
            );
            return ExitCode::FAILURE;
        }
    };
    match judged(answered, &address, method, target) {
        Ok(body) => {
            print!("{body}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprint!("{why}");
            ExitCode::FAILURE
        }
    }
}

/// What the operator gets: the answer's body where the server performed the operation, or the text
/// of the refusal to print instead of it.
///
/// Apart from [`run`] because the decision is the whole of what a client does with an answer and
/// the printing is not: a case can assert which of the two an answer earns without a process whose
/// streams it would have to read.
///
/// # Errors
///
/// The refusal, rendered: an answer from another build, which is refused before its body is looked
/// at, and any status but `200`, whose body is the server's own account of why.
fn judged(
    answered: Answered,
    address: &Path,
    method: &str,
    target: &str,
) -> Result<String, String> {
    if !same_build(answered.version.as_deref()) {
        return Err(format!(
            "error: the server on `{}` answers as `{}` and this is `{VERSION}`. The control \
             surface's wire shape is unstable until 1.0, so there is no negotiation and no \
             minimum: run the `nvs` that is serving.\n",
            address.display(),
            answered
                .version
                .as_deref()
                .unwrap_or("something that is not this surface"),
        ));
    }
    if answered.status == StatusCode::OK {
        Ok(answered.body)
    } else {
        Err(format!(
            "error: the server refused `{method} {target}` with {}:\n{}",
            answered.status.as_u16(),
            answered.body,
        ))
    }
}

/// Where the endpoint is: what `--socket` named, else what the tree's `[control] socket` says.
///
/// `--socket` is how one of several servers on a host is addressed, and it is read without a tree
/// at all — an operator who knows the path should not need the configuration that produced it, and
/// a server whose tree has moved is exactly when they need to reach it.
///
/// The tree is read through [`LocalFiles`], which does not apply
/// `rule:config/ownership-is-the-trust-boundary`'s check, and that is the same split
/// `crate::config`'s module doc argues rather than a hole in it: what is being read here is a
/// *name to dial*, and what authenticates the endpoint is its own owner and mode. A tree an
/// attacker could rewrite could point this at a socket of theirs, where they would learn that
/// somebody asked for a reload and could answer anything they liked — which is why the answer is
/// refused unless it carries this build's version, and why nothing on this path carries a secret.
fn address(config: &[PathBuf], socket: Option<&Path>) -> Option<PathBuf> {
    if let Some(named) = socket {
        return Some(named.to_path_buf());
    }
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, &[]);
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(&named, &cwd, &files);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return None;
        }
    };
    match Address::of(&resolved.config) {
        Ok(Address::Local(path)) => Some(path),
        Ok(Address::Disabled) => {
            eprintln!(
                "error: this configuration tree asks for no control surface, so there is nothing \
                 to reach.\nWrite `socket = \"/run/nvs/control.sock\"` under `[control]` in the \
                 tree the server boots on, or name a running server's endpoint with `--socket`."
            );
            None
        }
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            None
        }
    }
}

/// What came back: the status, the build that sent it, and the body whole.
struct Answered {
    /// The status the operation was answered with. Anything but `200` is a refusal whose body is
    /// the reason.
    status: StatusCode,
    /// The `nvs-control-version` header, absent where the thing on the endpoint did not send one.
    version: Option<String>,
    /// The answer's body, which this surface always sends as text.
    body: String,
}

/// The request body a control operation carries, which is none.
///
/// `hyper`'s client is generic over what it sends and every operation here is a method and a
/// target with nothing behind them. `http-body-util`'s `Empty` is a whole crate in the lock file
/// for the three lines below, and `rule:packaging/a-c-dependency-answers-two-questions`'s standing
/// preference is the dependency not taken.
struct NoBody;

impl Body for NoBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        Poll::Ready(None)
    }

    fn is_end_stream(&self) -> bool {
        true
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(0)
    }
}

/// One request over `connected`, and the answer read whole.
///
/// The connection and the exchange are two futures that only make progress beside each other —
/// the dispatcher writes and reads the socket, the exchange is what is waiting on the response —
/// so they are polled in one loop rather than spawned. There is no reactor on this thread and
/// nothing to wake it but itself, which is [`SETTLE`]: where a poll moved no byte in either
/// direction, the loop sleeps that long and asks again. It is a sleep and not a spin, it is the
/// whole of what this process is doing, and it costs an operator's command at most that much
/// latency.
///
/// # Errors
///
/// Whatever `hyper` ended the exchange on — a server that went away mid-answer, or bytes that are
/// not HTTP — and [`io::ErrorKind::UnexpectedEof`] for a connection that closed with the answer
/// unfinished. A wait for a server that is answering nothing at all is not bounded here: the
/// endpoint serializes operations, so a client behind a long reload is waiting on purpose, and the
/// operator's own interrupt is what ends it.
fn exchange(
    connected: impl io::Read + io::Write + Unpin,
    method: &str,
    target: &str,
) -> io::Result<Answered> {
    let driven = Nonblocking::new(connected);
    let moved = driven.moved();
    let mut context = Context::from_waker(Waker::noop());

    // h1 has no preface, so this settles the first time it is asked and touches no byte. It is
    // driven by the same loop as everything below rather than unwrapped, because "resolves at
    // once" is `hyper`'s business and not a thing this module gets to assume about it.
    let mut shaking = pin!(http1::handshake::<_, NoBody>(driven));
    let (mut sender, connection) = loop {
        if let Poll::Ready(shaken) = shaking.as_mut().poll(&mut context) {
            break shaken.map_err(io::Error::other)?;
        }
        settled(&moved);
    };
    let mut connection = pin!(connection);

    let request = Request::builder()
        .method(method)
        .uri(target)
        // A local endpoint has no authority for `hyper` to derive this from, and HTTP/1.1 requires
        // it. `localhost` is what `curl --unix-socket http://localhost/status` sends, so the
        // operator's debugging tool and this client put the same bytes on the wire.
        .header(header::HOST, HeaderValue::from_static("localhost"))
        .body(NoBody)
        .map_err(io::Error::other)?;

    // The connection ends of its own accord once the answer is on the wire — keep-alive is off on
    // both halves, so one connection is one operation — and it ends *before* the answer has been
    // read out of it. So a finished connection is not an error here and not a reason to stop: it
    // is the last byte having arrived, and what is left is draining what it delivered. It is an
    // error only where the thing still waiting on it never arrives, which is the `ended` turn
    // below, and polling it again after it has finished would panic.
    let mut ended = false;
    let mut asking = pin!(sender.send_request(request));
    let answering = loop {
        if let Poll::Ready(answered) = asking.as_mut().poll(&mut context) {
            break answered.map_err(io::Error::other)?;
        }
        if ended {
            return Err(cut_short());
        }
        ended = drove(connection.as_mut().poll(&mut context))?;
        settled(&moved);
    };

    let (head, body) = answering.into_parts();
    let mut body = pin!(body);
    let mut bytes = Vec::new();
    loop {
        match body.as_mut().poll_frame(&mut context) {
            Poll::Ready(None) => break,
            Poll::Ready(Some(Ok(frame))) => {
                if let Ok(data) = frame.into_data() {
                    bytes.extend_from_slice(&data);
                }
            }
            Poll::Ready(Some(Err(failed))) => return Err(io::Error::other(failed)),
            Poll::Pending if ended => return Err(cut_short()),
            Poll::Pending => {
                ended = drove(connection.as_mut().poll(&mut context))?;
                settled(&moved);
            }
        }
    }

    Ok(Answered {
        status: head.status,
        version: head
            .headers
            .get(VERSION_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        // Not lossily: this surface sends text it built itself, so bytes that are not UTF-8 are
        // not an answer of it to repair into one.
        body: String::from_utf8(bytes).map_err(io::Error::other)?,
    })
}

/// Waits [`SETTLE`] where nothing moved, and returns at once where something did.
///
/// The flag is [`Nonblocking::moved`]'s, set by any poll that carried a byte, and it is what tells
/// a connection that is working from one that is waiting on its peer — from outside the two polls
/// look identical.
fn settled(moved: &Cell<bool>) {
    if !moved.replace(false) {
        std::thread::sleep(SETTLE);
    }
}

/// One poll of the connection: whether it has finished, or the failure it finished on.
///
/// # Errors
///
/// Whatever `hyper` ended the connection on — a server that went away mid-answer, or bytes that
/// are not HTTP.
fn drove(polled: Poll<hyper::Result<()>>) -> io::Result<bool> {
    match polled {
        Poll::Pending => Ok(false),
        Poll::Ready(Ok(())) => Ok(true),
        Poll::Ready(Err(failed)) => Err(io::Error::other(failed)),
    }
}

/// A connection that ended with something still waiting on it.
fn cut_short() -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the control connection closed with the answer unfinished",
    )
}

/// How long the drive waits before asking a connection that had nothing to say whether it has
/// anything now. Short enough to be invisible to the operator whose command is waiting behind it,
/// long enough that waiting costs a thousandth of this thread rather than all of it.
const SETTLE: Duration = Duration::from_millis(1);

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{self, Read, Write};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use hyper::StatusCode;
    use nvs_config::control::bind;
    use nvs_config::snapshot::{Current, Snapshot};
    use nvs_diagnostics::SourceMap;
    use nvs_server::control::{
        Controlled, Report, VERSION, VERSION_HEADER, answer_connection, same_build,
    };
    use nvs_server::{Admission, Ceiling, Draining};

    use super::{Answered, SETTLE, exchange, judged};
    use crate::control::Process;
    use crate::script::Compiler;

    /// The process a control answer is about, recorded rather than running: what [`Controlled`]
    /// asks for, set by the case that is asking about it.
    ///
    /// The cases here are about the **client**, so this is the least server that makes a real
    /// answer: what is under test is that a request this module framed reaches
    /// [`answer_connection`] and that the answer comes back whole, not what the answer says.
    struct Answering {
        in_flight: usize,
        refuse: Option<String>,
    }

    impl Controlled for Answering {
        fn reload(&self) -> Result<Report, String> {
            match &self.refuse {
                Some(why) => Err(why.clone()),
                None => Ok(Report {
                    applied: vec!["limits.memory".to_string()],
                    ignored: vec!["server.listen"],
                    invalidated: 3,
                }),
            }
        }

        fn snapshot(&self) -> Arc<Snapshot> {
            Arc::new(Snapshot::default())
        }

        fn unapplied(&self) -> Vec<&'static str> {
            Vec::new()
        }

        fn in_flight(&self) -> usize {
            self.in_flight
        }

        fn draining(&self) -> bool {
            false
        }
    }

    /// A directory of this case's own, empty, beside the test binary under `target/`.
    ///
    /// Not `std::env::temp_dir()`, for the reason `nvs_server::control`'s own cases give: the
    /// endpoint's directory is held to `rule:config/ownership-is-the-trust-boundary`, which runs
    /// on the parent too, and a world-writable `/tmp` above a tight directory of ours fails it.
    /// These cases skip that guard — it is the server's half and has its own coverage — but they
    /// keep the directory, so nothing here leaves a socket in a shared place.
    fn scratch(name: &str) -> PathBuf {
        let beside = std::env::current_exe().expect("the test binary knows its own path");
        let dir = beside
            .parent()
            .expect("a test binary sits in a directory")
            .join(format!("nvs-ctl-{}-{name}", std::process::id()));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
        dir
    }

    /// The endpoint one case creates, in the platform's own namespace: a socket under a scratch
    /// directory on Unix, a pipe name on Windows, where the directory exists on both so that a
    /// case cleans up after itself either way.
    fn endpoint(name: &str) -> PathBuf {
        let dir = scratch(name);
        #[cfg(unix)]
        {
            dir.join("control.sock")
        }
        #[cfg(windows)]
        {
            let _ = dir;
            PathBuf::from(format!(r"\\.\pipe\nvs-ctl-{}-{name}", std::process::id()))
        }
    }

    /// `method target` asked of an endpoint that `serving` answers one client on, and what came
    /// back.
    ///
    /// The guard is `Ok` rather than `boundary` because what these cases are about is the exchange
    /// and not the trust boundary, which is `nvs-config`'s own to assert and does assert.
    fn asked<S>(case: &str, method: &str, target: &str, serving: S) -> Answered
    where
        S: FnOnce(nvs_config::control::platform::Stream<'_>) + Send + 'static,
    {
        let name = endpoint(case);
        let bound = bind(&name, |_| Ok(())).expect("an endpoint under a directory of our own");
        let answering = std::thread::spawn(move || {
            serving(bound.accept().expect("a client arrives on the endpoint"));
        });
        let connected = nvs_config::control::connect(&name).expect("the endpoint is there");
        let answered = exchange(connected, method, target).expect("the exchange completes");
        answering.join().expect("the answering thread ends cleanly");
        answered
    }

    /// The server half, answering one client for real.
    ///
    /// Generic over the process because both halves of this module's coverage go through it: the
    /// recording one above, for the cases that are about the client's framing, and the real
    /// [`Process`] below, for the two that are about what a running server answers.
    fn served<H: Controlled + Send>(
        process: H,
    ) -> impl FnOnce(nvs_config::control::platform::Stream<'_>) + Send {
        move |connected| {
            answer_connection(connected, &process).expect("one client, answered whole");
        }
    }

    /// A hand-written answer, for the shapes the real server half will not produce.
    ///
    /// Written after the request head has been read off the transport rather than before: a
    /// stream dropped with bytes still unread is a reset on Unix, which would take the answer
    /// with it.
    fn sends(answer: String) -> impl FnOnce(nvs_config::control::platform::Stream<'_>) + Send {
        move |mut connected| {
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                match connected.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => head.push(byte[0]),
                    Err(waiting) if waiting.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(SETTLE);
                    }
                    Err(failed) => panic!("the case's own client sends a request: {failed}"),
                }
            }
            connected
                .write_all(answer.as_bytes())
                .expect("the case's own answer is written");
            connected.flush().expect("and reaches the client");
        }
    }

    /// One `200` answer, carrying `version` where the thing answering claims to be a build at all.
    fn answer_of(version: Option<&str>, body: &str) -> String {
        let mut head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\n\
             connection: close\r\n",
            body.len(),
        );
        if let Some(version) = version {
            head.push_str(&format!("{VERSION_HEADER}: {version}\r\n"));
        }
        head.push_str("\r\n");
        head + body
    }

    #[test]
    fn a_status_operation_is_framed_asked_and_read_back_over_the_endpoint() {
        let answered = asked(
            "status",
            "GET",
            "/status",
            served(Answering {
                in_flight: 2,
                refuse: None,
            }),
        );
        assert_eq!(answered.status, StatusCode::OK);
        assert_eq!(
            answered.version.as_deref(),
            Some(VERSION),
            "every answer carries the build that sent it, and this one came from ours",
        );
        assert_eq!(
            answered.body, "in_flight: 2\ndraining: false\n",
            "the body is read whole: the head, the framing and the drive all agree",
        );
        assert_eq!(
            judged(answered, Path::new("here"), "GET", "/status").as_deref(),
            Ok("in_flight: 2\ndraining: false\n"),
        );
    }

    #[test]
    fn a_reload_the_process_refused_is_the_servers_own_text_and_not_output() {
        let answered = asked(
            "reload",
            "POST",
            "/reload",
            served(Answering {
                in_flight: 0,
                refuse: Some("nvs.toml line 4: `listen` is not a port\n".to_string()),
            }),
        );
        assert_eq!(
            answered.status,
            StatusCode::CONFLICT,
            "the request was well formed and the surface worked; the tree on disk is what conflicts",
        );
        let refused = judged(answered, Path::new("here"), "POST", "/reload")
            .expect_err("a refused reload is not something to print as a result");
        assert!(
            refused.contains("`listen` is not a port"),
            "the operator is given the server's own account of it: {refused}",
        );
    }

    #[test]
    fn an_operation_this_surface_does_not_have_is_named_back_in_the_refusal() {
        let answered = asked(
            "unknown",
            "POST",
            "/shutdown",
            served(Answering {
                in_flight: 0,
                refuse: None,
            }),
        );
        assert_eq!(answered.status, StatusCode::NOT_FOUND);
        let refused = judged(answered, Path::new("here"), "POST", "/shutdown")
            .expect_err("an operation that is not on the roster is refused");
        assert!(
            refused.contains("/shutdown"),
            "which is how an operator holding a newer `nvs ctl` learns that is what they have: \
             {refused}",
        );
    }

    #[test]
    fn ctl_refuses_an_answer_from_a_server_of_another_version() {
        let another = asked(
            "another-build",
            "GET",
            "/status",
            sends(answer_of(
                Some("0.0.0-another-build"),
                "in_flight: 0\ndraining: false\n",
            )),
        );
        assert!(!same_build(another.version.as_deref()));
        let refused = judged(another, Path::new("here"), "GET", "/status")
            .expect_err("the wire shape is unstable until 1.0, so a client reads only its own");
        assert!(
            refused.contains("0.0.0-another-build") && refused.contains(VERSION),
            "and the refusal names both builds so the operator knows which `nvs` to run: {refused}",
        );

        let nameless = asked(
            "no-version",
            "GET",
            "/status",
            sends(answer_of(None, "in_flight: 0\ndraining: false\n")),
        );
        assert!(
            judged(nameless, Path::new("here"), "GET", "/status").is_err(),
            "an answer with no version header at all is a mismatch too: it is not this surface, \
             whatever it is",
        );
    }

    /// A server of this case's own: the tree `written` resolved out of a file under `dir` exactly
    /// as a boot resolves it, held where a reload publishes, and the [`Process`] serving it.
    ///
    /// The drain is [`Draining::detached`] because this server's stopping is not this process's,
    /// and the compiler is one of its own: the count a reload reports is the fleet's unit cache,
    /// and this fleet has compiled nothing.
    fn a_server_over(dir: &Path, written: &str) -> (PathBuf, Arc<Current>, Process) {
        let root = dir.join("nvs.toml");
        fs::write(&root, written).expect("a tree of this case's own");
        let entry = dir.join("app.nvs");
        fs::write(&entry, "fn main(): void {}\n").expect("an entry file for the tree to be about");
        let mut sources = SourceMap::new();
        let snapshot = crate::config::boot_snapshot(
            std::slice::from_ref(&root),
            &entry,
            &mut sources,
            crate::config::Init::Never,
        )
        .expect("the tree this case wrote resolves");
        let current = Arc::new(Current::new(snapshot));
        let capacity = nvs_config::server::capacity_for(&current.load().config, &BTreeMap::new())
            .expect("a tree that named no ceiling has this machine's");
        let process = Process::new(
            Arc::clone(&current),
            vec![root.clone()],
            entry,
            Arc::new(Compiler::default()),
            Arc::new(Admission::new(&Ceiling::of(&capacity))),
            Draining::detached(),
        );
        (root, current, process)
    }

    /// `rule:config/a-reload-names-what-it-could-not-apply`, end to end over the endpoint: the
    /// reloadable key is applied and the process is serving it, and the `Boot` block the edit also
    /// changed is named back rather than taken.
    #[test]
    fn ctl_reload_publishes_the_edited_tree_and_prints_what_it_could_not_apply() {
        // A directory of its own and not the endpoint's: `scratch` empties what it hands back, so
        // a tree written under the name `asked` will use is a tree deleted before it is re-read.
        let dir = scratch("reload-live-tree");
        let (root, current, process) = a_server_over(
            &dir,
            "[limits]\nmemory = \"64M\"\n\n[server]\nlisten = [\"127.0.0.1:8080\"]\n",
        );
        fs::write(
            &root,
            "[limits]\nmemory = \"128M\"\n\n[server]\nlisten = [\"127.0.0.1:9090\"]\n",
        )
        .expect("the operator edits the tree the server booted on");

        let answered = asked("reload-live", "POST", "/reload", served(process));
        let body = judged(answered, &root, "POST", "/reload").expect("the reload is performed");

        assert!(
            body.contains("applied: limits.memory"),
            "the reloadable key the edit changed is named as applied: {body}",
        );
        assert!(
            body.contains("ignored: server"),
            "and the `Boot` block it also changed is named as not applied — a row naming a block \
             governs every key beneath it, so `[server]` is one change: {body}",
        );
        let serving = current.load();
        assert_eq!(
            serving.table["limits"]["memory"].as_str(),
            Some("128M"),
            "the published snapshot is the edited one, so the next request reads it",
        );
        assert_eq!(
            serving.table["server"]["listen"][0].as_str(),
            Some("127.0.0.1:8080"),
            "and the address already bound is still what the tree says, which is what makes \
             `does not take effect` true rather than aspirational",
        );
    }

    /// `rule:config/ctl-config-reports-the-live-snapshot`: the listing is what the process is
    /// holding, with the file each directive was written in beside it.
    ///
    /// The tree on disk is edited and **not** reloaded, so a listing taken from the files would
    /// answer `128M` and the live one answers `64M`. That difference is the whole of what this
    /// operation exists for — the offline `nvs config dump --origin` is the other half.
    #[test]
    fn ctl_config_prints_the_live_snapshot_with_each_directives_origin() {
        let dir = scratch("config-live-tree");
        let (root, _current, process) = a_server_over(&dir, "[limits]\nmemory = \"64M\"\n");
        fs::write(&root, "[limits]\nmemory = \"128M\"\n")
            .expect("the file changes under a process that has not reloaded");

        let answered = asked("config-live", "GET", "/config", served(process));
        let body = judged(answered, &root, "GET", "/config").expect("the snapshot is listed");

        let row = body
            .lines()
            .find(|line| line.starts_with("limits.memory "))
            .unwrap_or_else(|| panic!("the listing names every key in force: {body}"));
        assert!(
            row.contains("\"64M\""),
            "the value is the one the process is serving and not the one on disk: {row}",
        );
        assert!(
            row.contains(&root.display().to_string()),
            "and the origin column names the file it was written in: {row}",
        );
    }
}
