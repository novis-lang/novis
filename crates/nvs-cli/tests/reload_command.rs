//! `rule:config/a-reload-command-checks-now-and-waits`, from outside the process: the built
//! `nvs serve` and the reload endpoint it opens in its data folder's `run/`.
//!
//! Each case starts the server in a fresh directory with a private data folder, finds its endpoint
//! by the server's pid, and stops it the way a service manager does where the case needs a clean
//! end. Nothing waits on time: each wait polls until [`BOUND`] runs out, and the failure message
//! carries what the server wrote.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nvs_test::serve::{BOOT, POLL};

/// How long one change has to arrive before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// Every case's `nvs.toml`: a quick program check that compiles a change at once, and a drain
/// short enough for a stop.
const CONFIG: &str = "[opcache]\nvalidate = \"hash\"\nrevalidate_freq = \"100ms\"\nsettle = \"0s\"\n\n\
                      [server]\ndrain_timeout = \"5s\"\n";

/// The program every case starts with.
const PLAIN: &str = "<?nvs\necho \"ok\";\n";

/// A running `nvs serve app.nvs`, started as the leader of its own process group so a case can
/// stop it the way a service manager does. Dropping it kills the process and removes the
/// directory.
struct Served {
    child: Child,
    dir: PathBuf,
    data: nvs_repo::Scratch,
    addr: SocketAddr,
    stderr: Arc<Mutex<String>>,
}

impl Served {
    /// Starts the server over [`PLAIN`] in a fresh directory named for `case`. `before` runs on
    /// the data folder first.
    fn start(case: &str, before: impl FnOnce(&Path)) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("reload-command-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        write_file(&dir.join("app.nvs"), PLAIN);
        write_file(&dir.join("nvs.toml"), CONFIG);
        let data = nvs_repo::scratch_private("nvsdata");
        before(&data);

        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        command
            .arg("--data")
            .arg(&*data)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Its own process group, so the Ctrl-Break `stop` sends reaches the server and not this
        // test.
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(
            &mut command,
            windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP,
        );
        let mut child = command
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");
        let stderr = Arc::new(Mutex::new(String::new()));
        let err_pipe = child.stderr.take().expect("standard error is piped");
        let collected = Arc::clone(&stderr);
        thread::spawn(move || {
            for line in BufReader::new(err_pipe).lines().map_while(Result::ok) {
                let mut all = collected.lock().expect("no reader panicked");
                all.push_str(&line);
                all.push('\n');
            }
        });
        let out_pipe = child.stdout.take().expect("standard output is piped");
        let (sent, bound) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out_pipe).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let addr = rest.split_whitespace().next().unwrap_or_default();
                    let _ = sent.send(addr.to_owned());
                }
            }
        });
        let mut served = Self {
            child,
            dir,
            data,
            addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            stderr,
        };
        let Ok(addr) = bound.recv_timeout(BOOT) else {
            panic!(
                "`nvs serve` did not start listening within {BOOT:?}: {}",
                served.said()
            );
        };
        served.addr = addr
            .parse()
            .unwrap_or_else(|error| panic!("`{addr}` is not an address: {error}"));
        served
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// The data folder's `run/`.
    fn run(&self) -> PathBuf {
        self.data.join("run")
    }

    /// The endpoints in `run/` whose name carries this server's pid.
    fn endpoints(&self) -> Vec<PathBuf> {
        let prefix = format!("serve-{}-", self.child.id());
        std::fs::read_dir(self.run())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// This server's one endpoint.
    ///
    /// # Panics
    ///
    /// When `run/` has none or more than one with its pid.
    fn endpoint(&self) -> PathBuf {
        let found = self.endpoints();
        assert_eq!(
            found.len(),
            1,
            "the server opened {} endpoints in `{}`: {found:?}\n{}",
            found.len(),
            self.run().display(),
            self.said()
        );
        found.into_iter().next().expect("one endpoint")
    }

    /// The body of `GET /`.
    fn get(&self) -> String {
        let mut stream = TcpStream::connect(self.addr).expect("the server takes a connection");
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(
            stream,
            "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .expect("the request is sent");
        let mut answer = String::new();
        let _ = stream.read_to_string(&mut answer);
        answer
            .split_once("\r\n\r\n")
            .map_or(answer.clone(), |(_, body)| body.to_owned())
    }

    /// Stops the server the way a service manager does, and waits for it to exit.
    ///
    /// # Panics
    ///
    /// When it does not exit within [`BOUND`] or exits with a failure.
    fn stop(&mut self) {
        stop(self.child.id());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("the server's status is read") {
                break status;
            }
            assert!(
                started.elapsed() < BOUND,
                "`nvs serve` did not exit after the stop: {}",
                self.said()
            );
            thread::sleep(POLL);
        };
        assert!(
            status.success(),
            "`nvs serve` exited with {status}: {}",
            self.said()
        );
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Polls `done` until it is true.
///
/// # Panics
///
/// When [`BOUND`] runs out first. The message names `what` and carries `said()`.
fn wait(what: &str, mut done: impl FnMut() -> bool, said: impl Fn() -> String) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < BOUND,
            "{what} did not happen within {BOUND:?}; the server wrote: {}",
            said()
        );
        thread::sleep(POLL);
    }
}

/// Writes `text` to `path` in one rename, so the server never reads half a file.
fn write_file(path: &Path, text: &str) {
    let name = path.file_name().expect("the path names a file");
    let temp = path.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::write(&temp, text)
        .unwrap_or_else(|error| panic!("`{}` could not be written: {error}", temp.display()));
    std::fs::rename(&temp, path)
        .unwrap_or_else(|error| panic!("`{}` could not be replaced: {error}", path.display()));
}

/// Asks the server `pid` to stop, the way a service manager does.
#[cfg(unix)]
fn stop(pid: u32) {
    let sent = nvs_repo::spawn("kill", &[])
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("`kill` runs");
    assert!(sent.success(), "`kill -TERM {pid}` was refused");
}

/// Asks the server `pid` to stop, the way Ctrl-Break in its console does. The server was started
/// as the leader of its own process group, so `pid` is that group's id.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "`GenerateConsoleCtrlEvent` is the platform's only way to send a console's stop"
)]
fn stop(pid: u32) {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};
    // SAFETY: the call takes two integers and reads no memory of ours.
    let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) };
    assert_ne!(
        sent,
        0,
        "the Ctrl-Break was not sent: {}",
        std::io::Error::last_os_error()
    );
}

/// The endpoint is in a folder no other account can write, and the serving account reaches it.
/// On Unix the folder is `0700` and the endpoint is a socket. On Windows the endpoint is a file
/// naming a local pipe, whose own DACL `nvs_host::endpoint`'s tests read back.
#[test]
fn the_endpoint_is_private_to_the_serving_account() {
    let served = Served::start("private", |_| {});
    let endpoint = served.endpoint();
    nvs_config::trust::check(&served.run()).unwrap_or_else(|why| {
        panic!(
            "another account can write `{}`: {}",
            served.run().display(),
            why.message()
        )
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, PermissionsExt};
        let mode = std::fs::metadata(served.run())
            .expect("`run/` is read")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "`run/` has mode {mode:o}");
        let kind = std::fs::symlink_metadata(&endpoint)
            .expect("the endpoint is read")
            .file_type();
        assert!(kind.is_socket(), "the endpoint is not a socket");
    }
    #[cfg(windows)]
    {
        let named = std::fs::read_to_string(&endpoint).expect("the endpoint's file is read");
        assert!(
            named.starts_with(r"\\.\pipe\nvs-reload-"),
            "the endpoint names {named:?}, which is not a local pipe"
        );
    }
    nvs_host::endpoint::connect(&endpoint).unwrap_or_else(|error| {
        panic!(
            "the serving account cannot reach `{}`: {error}\n{}",
            endpoint.display(),
            served.said()
        )
    });
}

/// A stop removes the endpoint once the drain is over.
#[test]
fn a_stopped_server_removes_its_endpoint() {
    let mut served = Served::start("stopped", |_| {});
    let endpoint = served.endpoint();
    served.stop();
    assert!(
        !endpoint.exists(),
        "`{}` is still there after the stop: {}",
        endpoint.display(),
        served.said()
    );
}

/// An endpoint nobody listens at is what a killed server leaves. A starting server removes it and
/// opens its own beside it.
#[test]
fn a_stale_endpoint_is_skipped_and_removed() {
    let stale = std::cell::RefCell::new(PathBuf::new());
    let served = Served::start("stale", |data| {
        let run = data.join("run");
        nvs_config::data::create_private_root(&run).expect("`run/` is created");
        let path = run.join("serve-1-1");
        // On Windows an endpoint is a file naming a pipe, and this pipe does not exist. On Unix
        // it is a socket, and a plain file is one nobody listens at.
        std::fs::write(&path, format!(r"\\.\pipe\nvs-reload-{}", "0".repeat(32)))
            .expect("the stale endpoint is written");
        *stale.borrow_mut() = path;
    });
    let stale = stale.into_inner();
    let endpoint = served.endpoint();
    assert!(
        !stale.exists(),
        "the stale `{}` is still there beside `{}`: {}",
        stale.display(),
        endpoint.display(),
        served.said()
    );
    let reached = nvs_host::endpoint::reach_all(&served.run()).expect("`run/` is listed");
    let paths: Vec<_> = reached.iter().map(|(path, _)| path.clone()).collect();
    assert_eq!(paths, vec![endpoint], "the live endpoints in `run/`");
}

/// A request cannot read the endpoint: the data folder is outside every grant, so the read throws
/// before anything is opened.
#[test]
fn a_request_cannot_reach_the_endpoint() {
    let served = Served::start("request", |_| {});
    let endpoint = served.endpoint();
    let target = endpoint.display().to_string().replace('\\', "/");
    assert!(!target.contains('\''), "the path {target} has a quote");
    write_file(
        &served.dir.join("app.nvs"),
        &format!(
            "<?nvs\ntry {{\n    echo Core\\IO::read('{target}');\n}} catch (RuntimeError $error) {{\n    \
             echo 'denied: ', $error->message;\n}}\n"
        ),
    );
    let last = std::cell::RefCell::new(String::new());
    wait(
        "the edit that reads the endpoint",
        || {
            *last.borrow_mut() = served.get();
            last.borrow().starts_with("denied: ")
        },
        || format!("{}\nthe last body: {}", served.said(), last.borrow()),
    );
    let body = last.into_inner();
    assert!(
        body.contains("needs the capability `fs.read`"),
        "the read was not refused by the grant: {body}"
    );
}
