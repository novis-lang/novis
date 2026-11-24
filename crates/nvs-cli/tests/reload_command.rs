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

/// The `nvs.toml` of a case where only a reload can be the cause of a change: the program check
/// runs once an hour and waits a minute for quiet, and a reload skips both.
const HOURLY: &str = "[opcache]\nvalidate = \"hash\"\nrevalidate_freq = \"1h\"\nsettle = \"1m\"\n\n\
                      [server]\ndrain_timeout = \"5s\"\n";

/// The program every case starts with.
const PLAIN: &str = "<?nvs\necho \"ok\";\n";

/// A private data folder for a server, named so its endpoint fits a socket path on every runner.
/// The endpoint is `run/serve-<pid>-<ms>`, bound under a `.new` name first. A macOS socket path
/// is at most 103 bytes, so the folder's own name is one letter.
fn data_folder() -> nvs_repo::Scratch {
    nvs_repo::scratch_private_for_socket("d", "run/serve-99999-1700000000000.new")
}

/// What an event case adds to its `nvs.toml`: scripts under `jobs/` that may write there, and an
/// `on = "reload"` entry that runs [`RECORDING`].
const ON_RELOAD: &str = "\n[capabilities.script]\nspawn = [\"jobs/\"]\n\n\
                         [capabilities.fs]\nwrite = [\"jobs/\"]\n\n\
                         [[schedule]]\nname = \"redeploy\"\non = \"reload\"\n\
                         script = \"jobs/redeploy.nvs\"\nscope = \"host\"\n";

/// The event script: it writes the map its run was given, as JSON, to `jobs/reloaded.json`.
const RECORDING: &str =
    "<?nvs\nCore\\IO::write('reloaded.json', Core\\Json::encode(Core\\Script::arguments()));\n";

/// A running `nvs serve app.nvs`, started as the leader of its own process group so a case can
/// stop it the way a service manager does. Dropping it kills the process and removes the
/// directory.
struct Served {
    child: Child,
    dir: PathBuf,
    data: Arc<nvs_repo::Scratch>,
    addr: SocketAddr,
    stderr: Arc<Mutex<String>>,
}

impl Served {
    /// Starts the server over [`PLAIN`] and [`CONFIG`] in a fresh directory named for `case`.
    /// `before` runs on the data folder first.
    fn start(case: &str, before: impl FnOnce(&Path)) -> Self {
        Self::start_with(case, CONFIG, before)
    }

    /// Starts the server over [`PLAIN`] and `config` in a fresh directory named for `case`.
    /// `before` runs on the data folder first.
    fn start_with(case: &str, config: &str, before: impl FnOnce(&Path)) -> Self {
        let data = data_folder();
        before(&data);
        Self::launch(case, config, &[], Arc::new(data))
    }

    /// Starts the server over [`PLAIN`], `config` with [`ON_RELOAD`] after it, and
    /// `jobs/redeploy.nvs`, in a fresh directory named for `case`.
    fn start_recording(case: &str, config: &str) -> Self {
        let data = data_folder();
        Self::launch(
            case,
            &format!("{config}{ON_RELOAD}"),
            &[("jobs/redeploy.nvs", RECORDING)],
            Arc::new(data),
        )
    }

    /// Starts a second server over [`PLAIN`] and `config`, in a fresh directory named for `case`,
    /// that shares `self`'s data folder.
    fn beside(&self, case: &str, config: &str) -> Self {
        Self::launch(case, config, &[], Arc::clone(&self.data))
    }

    /// Starts the server over [`PLAIN`], `config` and `files` in a fresh directory named for
    /// `case`, with `data` as its data folder.
    fn launch(
        case: &str,
        config: &str,
        files: &[(&str, &str)],
        data: Arc<nvs_repo::Scratch>,
    ) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("reload-command-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("jobs")).expect("the case's directory is created");
        write_file(&dir.join("app.nvs"), PLAIN);
        write_file(&dir.join("nvs.toml"), config);
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }

        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        command
            .arg("--data")
            .arg(data.path())
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

    /// The lines the server has written so far that contain `message`.
    fn records(&self, message: &str) -> Vec<String> {
        self.said()
            .lines()
            .filter(|line| line.contains(message))
            .map(str::to_owned)
            .collect()
    }

    /// Waits until the server has written one line containing `message`, and returns it.
    ///
    /// # Panics
    ///
    /// When none is written within [`BOUND`].
    fn record(&self, message: &str) -> String {
        wait(
            &format!("a `{message}` record"),
            || !self.records(message).is_empty(),
            || self.said(),
        );
        self.records(message).remove(0)
    }

    /// The map the `on = "reload"` entry's last run was given, once `wanted` accepts it.
    ///
    /// # Panics
    ///
    /// When no run writes one `wanted` accepts within [`BOUND`]. The message names `what`.
    fn reloaded(
        &self,
        what: &str,
        wanted: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        let path = self.dir.join("jobs").join("reloaded.json");
        let mut last = None;
        wait(
            what,
            || {
                last = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| serde_json::from_str(&text).ok());
                last.as_ref().is_some_and(&wanted)
            },
            || self.said(),
        );
        last.expect("the wait ended on a map")
    }

    /// Runs `nvs reload` over this server's data folder and checks it exits with `code`.
    ///
    /// # Panics
    ///
    /// When it exits with another code. The message carries what it and the server printed.
    fn reload_exits(&self, code: i32) {
        let ran = nvs_reload(self.data.path(), false);
        assert_eq!(
            ran.code,
            Some(code),
            "`nvs reload` printed {}{}\nthe server wrote: {}",
            ran.stdout,
            ran.stderr,
            self.said()
        );
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

/// Sends one reload to `endpoint` and returns its JSON reply.
///
/// # Panics
///
/// When the endpoint cannot be reached or the reply is not one JSON line. The message carries
/// `said()`.
fn reload(endpoint: &Path, said: impl Fn() -> String) -> serde_json::Value {
    let mut connection = nvs_host::endpoint::connect(endpoint).unwrap_or_else(|error| {
        panic!(
            "`{}` is not reached: {error}\n{}",
            endpoint.display(),
            said()
        )
    });
    connection
        .write_all(b"reload 1\n")
        .unwrap_or_else(|error| panic!("the reload is not sent: {error}\n{}", said()));
    let mut line = String::new();
    BufReader::new(connection)
        .read_line(&mut line)
        .unwrap_or_else(|error| panic!("the reply is not read: {error}\n{}", said()));
    serde_json::from_str(&line)
        .unwrap_or_else(|error| panic!("the reply {line:?} is not JSON: {error}\n{}", said()))
}

/// What one run of `nvs reload` over the data folder `data` did.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs `nvs --data <data> reload`, with `--json` where `json` is set, and waits for it.
fn nvs_reload(data: &Path, json: bool) -> Ran {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command.arg("--data").arg(data).arg("reload");
    if json {
        command.arg("--json");
    }
    let output = command
        .stdin(Stdio::null())
        .output()
        .expect("the `nvs` binary this test was built beside starts");
    Ran {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
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
// covers: tools:server/nvs-reload
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

/// A grant that covers the whole data folder still does not reach its `run/`: the read throws
/// before anything is opened. On Windows the path is read in capitals, which names the same file.
// covers: tools:server/nvs-reload
#[test]
fn a_grant_over_the_data_folder_does_not_reach_the_endpoint() {
    let data = data_folder();
    let root = data.path().display().to_string().replace('\\', "/");
    assert!(!root.contains('"'), "the path {root} has a quote");
    let config = format!("{CONFIG}\n[capabilities.fs]\nread = [\"{root}\"]\n");
    let served = Served::launch("granted", &config, &[], Arc::new(data));
    let mut target = served.endpoint().display().to_string().replace('\\', "/");
    if cfg!(windows) {
        target = target.to_uppercase();
    }
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
        body.contains("inside the `run/` folder"),
        "the read was not refused for the data folder: {body}"
    );
}

/// A reload compiles an edit at once and replies once it is live. The program check runs once an
/// hour, so nothing else can be the cause.
// covers: tools:server/nvs-reload
#[test]
fn a_reload_makes_an_edit_live_before_the_timer() {
    let served = Served::start_with("edit", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    let reply = reload(&served.endpoint(), || served.said());
    assert_eq!(reply["config"], "unchanged", "the reply: {reply}");
    assert_eq!(reply["programs"], 1, "the reply: {reply}");
    assert_eq!(reply["failed"], 0, "the reply: {reply}");
    assert_eq!(
        served.get(),
        "new",
        "the request after the reload: {}",
        served.said()
    );
}

/// A reload reads a saved `nvs.toml` at once and replies that it applied it.
#[test]
fn a_reload_applies_a_saved_configuration_at_once() {
    let served = Served::start_with("config", HOURLY, |_| {});
    write_file(
        &served.dir.join("nvs.toml"),
        &HOURLY.replace("drain_timeout = \"5s\"", "drain_timeout = \"6s\""),
    );
    let reply = reload(&served.endpoint(), || served.said());
    assert_eq!(
        reply["config"],
        "applied",
        "the reply: {reply}\n{}",
        served.said()
    );
    assert_eq!(reply["failed"], 0, "the reply: {reply}");
    let again = reload(&served.endpoint(), || served.said());
    assert_eq!(again["config"], "unchanged", "the second reply: {again}");
}

/// A broken edit is one refusal in the reply, with the file and line of its first error.
#[test]
fn a_reload_reports_a_broken_edit_with_its_line() {
    let served = Served::start_with("broken", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(
        &served.dir.join("app.nvs"),
        "<?nvs\necho \"ok\";\necho (;\n",
    );
    let reply = reload(&served.endpoint(), || served.said());
    assert_eq!(reply["failed"], 1, "the reply: {reply}\n{}", served.said());
    assert_eq!(reply["programs"], 0, "the reply: {reply}");
    let refusal = &reply["refusals"][0];
    assert_eq!(refusal["kind"], "program", "the refusal: {refusal}");
    let file = refusal["file"]
        .as_str()
        .unwrap_or_default()
        .replace('\\', "/");
    assert!(
        file.ends_with("/app.nvs"),
        "the refusal names {file}: {refusal}"
    );
    assert_eq!(refusal["line"], 3, "the refusal: {refusal}");
    assert!(
        refusal["message"]
            .as_str()
            .is_some_and(|text| !text.is_empty()),
        "the refusal has no message: {refusal}"
    );
}

/// Reloads sent at once are each answered by a check that began after they arrived, so every one
/// sees the edit, and it is compiled once. That reloads arriving during a check share the next one
/// is pinned by `reload_endpoint`'s own test, which holds a check open.
#[test]
fn two_reloads_at_once_share_one_check() {
    const RELOADS: usize = 4;
    let served = Served::start_with("burst", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    let endpoint = served.endpoint();
    let gate = Arc::new(std::sync::Barrier::new(RELOADS));
    let sent: Vec<_> = (0..RELOADS)
        .map(|_| {
            let endpoint = endpoint.clone();
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                gate.wait();
                reload(&endpoint, String::new)
            })
        })
        .collect();
    let replies: Vec<serde_json::Value> = sent
        .into_iter()
        .map(|reloading| reloading.join().expect("no reload panicked"))
        .collect();
    let swapped: u64 = replies
        .iter()
        .map(|reply| {
            assert_eq!(reply["failed"], 0, "a reply: {reply}");
            let programs = reply["programs"].as_u64().unwrap_or(u64::MAX);
            assert!(programs <= 1, "a reply: {reply}");
            programs
        })
        .sum();
    assert!(
        swapped >= 1,
        "no reply saw the edit: {replies:?}\n{}",
        served.said()
    );
    assert_eq!(
        served.get(),
        "new",
        "the request after the reloads: {}",
        served.said()
    );
}

/// `nvs reload` exits `0` once an edit is live, and the next request runs it.
// covers: tools:server/nvs-reload
#[test]
fn nvs_reload_exits_zero_when_everything_is_live() {
    let served = Served::start_with("cli-live", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    let ran = nvs_reload(served.data.path(), false);
    assert_eq!(
        ran.code,
        Some(0),
        "`nvs reload` printed {}{}\nthe server wrote: {}",
        ran.stdout,
        ran.stderr,
        served.said()
    );
    let pid = served.child.id();
    assert!(
        ran.stdout.contains(&format!(
            "server {pid}: configuration unchanged, 1 program reloaded"
        )),
        "`nvs reload` printed {}",
        ran.stdout
    );
    assert_eq!(served.get(), "new", "the request after the reload");
}

/// `nvs reload` exits `1` when a file is refused, and prints each refusal with its file.
// covers: tools:server/nvs-reload
#[test]
fn nvs_reload_exits_one_and_prints_every_refusal() {
    let served = Served::start_with("cli-refused", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(
        &served.dir.join("app.nvs"),
        "<?nvs\necho \"ok\";\necho (;\n",
    );
    write_file(
        &served.dir.join("nvs.toml"),
        &HOURLY.replace("[server]\n", "[server]\nno_such_key = 1\n"),
    );
    let ran = nvs_reload(served.data.path(), false);
    assert_eq!(
        ran.code,
        Some(1),
        "`nvs reload` printed {}{}\nthe server wrote: {}",
        ran.stdout,
        ran.stderr,
        served.said()
    );
    let errors: Vec<&str> = ran
        .stdout
        .lines()
        .filter(|line| line.starts_with("  error: "))
        .collect();
    assert_eq!(errors.len(), 2, "`nvs reload` printed {}", ran.stdout);
    let printed = ran.stdout.replace('\\', "/");
    assert!(
        printed.contains("/app.nvs:3: "),
        "`nvs reload` printed {}",
        ran.stdout
    );
    assert!(
        printed.contains("nvs.toml"),
        "`nvs reload` printed {}",
        ran.stdout
    );
}

/// `nvs reload` exits `2` when no server uses the data folder, and says so.
// covers: tools:server/nvs-reload
#[test]
fn nvs_reload_exits_two_with_no_server() {
    let data = nvs_repo::scratch_private("nvsdata-none");
    let ran = nvs_reload(data.path(), false);
    assert_eq!(
        ran.code,
        Some(2),
        "`nvs reload` printed {}{}",
        ran.stdout,
        ran.stderr
    );
    assert!(
        ran.stderr
            .contains("no `nvs serve` process uses the data folder"),
        "`nvs reload` wrote {}",
        ran.stderr
    );
}

/// Two servers that share one data folder are both reloaded, and each is reported.
// covers: tools:server/nvs-reload
#[test]
fn nvs_reload_reaches_every_server_of_a_data_folder() {
    let first = Served::start_with("cli-first", HOURLY, |_| {});
    let second = first.beside("cli-second", HOURLY);
    for served in [&first, &second] {
        assert_eq!(served.get(), "ok", "the first request: {}", served.said());
        write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    }
    let ran = nvs_reload(first.data.path(), false);
    assert_eq!(
        ran.code,
        Some(0),
        "`nvs reload` printed {}{}\nthe servers wrote: {}\n{}",
        ran.stdout,
        ran.stderr,
        first.said(),
        second.said()
    );
    for served in [&first, &second] {
        let pid = served.child.id();
        assert!(
            ran.stdout.contains(&format!(
                "server {pid}: configuration unchanged, 1 program reloaded"
            )),
            "`nvs reload` printed {}",
            ran.stdout
        );
        assert_eq!(served.get(), "new", "the request after the reload");
    }
}

/// `nvs reload --json` prints one JSON document with one entry per server.
// covers: tools:server/nvs-reload
#[test]
fn nvs_reload_json_is_one_document() {
    let served = Served::start_with("cli-json", HOURLY, |_| {});
    let ran = nvs_reload(served.data.path(), true);
    assert_eq!(
        ran.code,
        Some(0),
        "`nvs reload --json` printed {}{}",
        ran.stdout,
        ran.stderr
    );
    let document: serde_json::Value = serde_json::from_str(&ran.stdout).unwrap_or_else(|error| {
        panic!(
            "`nvs reload --json` printed {:?}, which is not one document: {error}",
            ran.stdout
        )
    });
    assert_eq!(document["schema"], 1, "the document: {document}");
    let servers = document["servers"].as_array().expect("`servers` is a list");
    assert_eq!(servers.len(), 1, "the document: {document}");
    let server = &servers[0];
    assert_eq!(server["pid"], served.child.id(), "the server: {server}");
    assert_eq!(server["state"], "replied", "the server: {server}");
    assert_eq!(server["config"], "unchanged", "the server: {server}");
    assert_eq!(server["failed"], 0, "the server: {server}");
    assert!(server["refusals"].is_array(), "the server: {server}");
}

/// A program that `nvs reload` swaps is one `program reloaded` record, with its entry file. A
/// second reload with nothing changed writes none.
// covers: tools:server/nvs-reload
#[test]
fn a_swapped_program_is_one_log_record() {
    let served = Served::start_with("log-swap", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    served.reload_exits(0);
    let record = served.record("program reloaded");
    assert!(record.contains("app.nvs"), "the record: {record}");
    served.reload_exits(0);
    served.reload_exits(0);
    assert_eq!(
        served.records("program reloaded").len(),
        1,
        "the server wrote: {}",
        served.said()
    );
}

/// A program that does not compile is one `program reload failed` record, with its entry file and
/// the line of its first error. Later reloads still report it, but write no second record.
// covers: tools:server/nvs-reload
#[test]
fn a_failed_compile_is_one_log_record() {
    let served = Served::start_with("log-failed", HOURLY, |_| {});
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(
        &served.dir.join("app.nvs"),
        "<?nvs\necho \"ok\";\necho (;\n",
    );
    served.reload_exits(1);
    let record = served.record("program reload failed");
    assert!(record.contains("app.nvs"), "the record: {record}");
    assert!(record.contains('3'), "the record has no line 3: {record}");
    served.reload_exits(1);
    served.reload_exits(1);
    assert_eq!(
        served.records("program reload failed").len(),
        1,
        "the server wrote: {}",
        served.said()
    );
}

/// `nvs reload` runs the `on = "reload"` entry once, and its map counts the programs swapped and
/// the programs that do not compile.
// covers: tools:server/nvs-reload
#[test]
fn a_code_swap_fires_the_reload_event_with_its_counts() {
    let served = Served::start_recording("event-swap", HOURLY);
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    served.reload_exits(0);
    let map = served.reloaded("the run for the swap", |map| map["programs"] == 1);
    assert_eq!(map["on"], "reload", "{map}");
    assert_eq!(map["outcome"], "applied", "{map}");
    assert_eq!(map["failed"], 0, "{map}");
    assert_eq!(map["restart"], serde_json::json!([]), "{map}");

    write_file(
        &served.dir.join("app.nvs"),
        "<?nvs\necho \"new\";\necho (;\n",
    );
    served.reload_exits(1);
    let map = served.reloaded("the run for the broken edit", |map| map["failed"] == 1);
    assert_eq!(map["outcome"], "refused", "{map}");
    assert_eq!(map["programs"], 0, "{map}");
}

/// A swap the timed check finds, with no `nvs reload`, runs the `on = "reload"` entry too.
#[test]
fn a_timer_swap_fires_the_reload_event_too() {
    let served = Served::start_recording("event-timer", CONFIG);
    assert_eq!(served.get(), "ok", "the first request: {}", served.said());
    write_file(&served.dir.join("app.nvs"), "<?nvs\necho \"new\";\n");
    let map = served.reloaded("the run for the timed swap", |map| map["programs"] == 1);
    assert_eq!(map["on"], "reload", "{map}");
    assert_eq!(map["outcome"], "applied", "{map}");
    assert_eq!(map["failed"], 0, "{map}");
    served.record("program reloaded");
}

/// A request longer than the line is closed without a reply, and the endpoint still answers the
/// next one.
#[test]
fn a_request_longer_than_the_line_is_closed() {
    let served = Served::start_with("long", HOURLY, |_| {});
    let endpoint = served.endpoint();
    let mut connection = nvs_host::endpoint::connect(&endpoint).expect("the endpoint is reached");
    let _ = connection.write_all(&[b'x'; 100]);
    let mut answer = Vec::new();
    let _ = connection.read_to_end(&mut answer);
    assert!(
        answer.is_empty(),
        "the long request was answered: {:?}",
        String::from_utf8_lossy(&answer)
    );
    let reply = reload(&endpoint, || served.said());
    assert_eq!(reply["config"], "unchanged", "the reply after it: {reply}");
}
