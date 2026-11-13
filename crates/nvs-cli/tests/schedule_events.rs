//! `rule:config/an-entry-runs-on-a-server-event`, from outside the process: the
//! built `nvs serve` with a `[[schedule]]` entry for each event, and the file
//! each entry's script writes when it runs.
//!
//! Every event script writes `Core\Script::args()` as JSON to a file beside
//! itself, so a case waits for that file and reads the map the run was given.
//! Nothing waits on time: each wait polls for a file or a log line until
//! [`BOUND`] runs out, and the failure message carries what the server wrote.
//!
//! The program lives under `CARGO_TARGET_TMPDIR`, inside the build directory,
//! and is removed when its [`Served`] is dropped.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long an event has to run its script before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two looks of one poll.
const POLL: Duration = Duration::from_millis(50);

/// What every case's `nvs.toml` starts with: production mode, a quick program
/// check, scripts under `jobs/` that may write there, and a drain long enough
/// for a stop run.
const BASE: &str = "[mode]\ndefault = \"production\"\n\n\
                    [opcache]\nrevalidate_freq = \"100ms\"\n\n\
                    [server]\ndrain_timeout = \"20s\"\n\n\
                    [capabilities.script]\nspawn = [\"jobs/\"]\n\n\
                    [capabilities.fs]\nwrite = [\"jobs/\"]\n\n";

/// The program every case serves.
const PLAIN: &str = "<?nvs\necho \"ok\";\n";

/// A scheduled script that throws, so its run is a failure.
const THROWING: &str = "<?nvs\nthrow new RuntimeError(\"ticked\");\n";

/// A script that writes the map it was given, as JSON, to `jobs/<file>`.
fn recording(file: &str) -> String {
    format!("<?nvs\nCore\\IO::write('{file}', Core\\Json::encode(Core\\Script::args()));\n")
}

/// A `[[schedule]]` entry named `name` that runs `jobs/<name>.nvs` on `event`.
fn on(name: &str, event: &str) -> String {
    format!(
        "[[schedule]]\nname = \"{name}\"\non = \"{event}\"\nscript = \"jobs/{name}.nvs\"\n\
         scope = \"host\"\n\n"
    )
}

/// A running `nvs serve app.nvs` over a program in its own directory, started
/// as the leader of its own process group so a case can stop it the way a
/// service manager does.
///
/// Dropping it kills the process and removes the directory.
struct Served {
    child: Child,
    dir: PathBuf,
    /// The private data folder the server is given.
    _data: nvs_repo::Scratch,
    stderr: Arc<Mutex<String>>,
}

impl Served {
    /// Writes `files` and `nvs.toml` (`BASE` and then `config`) into a fresh
    /// directory named for `case`, and starts `nvs serve app.nvs` there.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`].
    fn start(case: &str, config: &str, files: &[(&str, &str)]) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("schedule-events-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("jobs")).expect("the case's directory is created");
        write_file(&dir.join("app.nvs"), PLAIN);
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }
        write_file(&dir.join("nvs.toml"), &format!("{BASE}{config}"));
        let data = nvs_repo::scratch_private("nvsdata");

        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        command
            .arg("--data")
            .arg(&*data)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Its own process group, so the Ctrl-Break `stop` sends reaches the
        // server and not this test.
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(
            &mut command,
            windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP,
        );
        let mut child = command
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");

        // Both pipes are read to their end on a thread of their own, so a
        // server that writes a lot never blocks on a full pipe.
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
                if line.starts_with("listening on http://") {
                    let _ = sent.send(());
                }
            }
        });
        let served = Self {
            child,
            dir,
            _data: data,
            stderr,
        };
        assert!(
            bound.recv_timeout(BOOT).is_ok(),
            "`nvs serve` did not start listening within {BOOT:?}: {}",
            served.said()
        );
        served
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// Rewrites `nvs.toml` as [`Served::start`] wrote it, with `config` in
    /// place of what followed `BASE`, and waits until the server logs a reload
    /// record past what it had said before.
    ///
    /// # Panics
    ///
    /// When no record is logged within [`BOUND`].
    fn save(&self, config: &str) {
        let from = self.said().len();
        write_file(&self.dir.join("nvs.toml"), &format!("{BASE}{config}"));
        wait(
            &format!("a reload record after saving {config}"),
            BOUND,
            || {
                let said = self.said();
                said[from..].contains("configuration reload")
            },
            || self.said(),
        );
    }

    /// The map the script that writes `jobs/<file>` was given, once the file
    /// holds JSON that `wanted` accepts.
    ///
    /// # Panics
    ///
    /// When it does not within `bound`. The message names `what`.
    fn recorded(
        &self,
        file: &str,
        what: &str,
        bound: Duration,
        wanted: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        let path = self.dir.join("jobs").join(file);
        let mut last = None;
        wait(
            what,
            bound,
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

    /// Stops the server the way a service manager does, and waits for it to
    /// exit.
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
/// When `bound` runs out first. The message names `what` and carries `said()`.
fn wait(what: &str, bound: Duration, mut done: impl FnMut() -> bool, said: impl Fn() -> String) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < bound,
            "{what} did not happen within {bound:?}; the server wrote: {}",
            said()
        );
        thread::sleep(POLL);
    }
}

/// Writes `text` to `path` in one rename, creating any directory it needs, so
/// the server never reads half a file.
fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the file's directory is created");
    }
    let name = path.file_name().expect("the path names a file");
    let temp = path.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::write(&temp, text).unwrap_or_else(|error| {
        panic!("`{}` could not be written: {error}", temp.display());
    });
    std::fs::rename(&temp, path).unwrap_or_else(|error| {
        panic!("`{}` could not be replaced: {error}", path.display());
    });
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

/// Asks the server `pid` to stop, the way Ctrl-Break in its console does. The
/// server was started as the leader of its own process group, so `pid` is that
/// group's id.
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

/// The start entry's file appears while the server serves, and the stop
/// entry's file exists once the stopped process has exited.
#[test]
fn a_start_and_a_stop_entry_run_under_nvs_serve() {
    let started = recording("started.json");
    let stopped = recording("stopped.json");
    let mut served = Served::start(
        "start-stop",
        &format!("{}{}", on("warm", "start"), on("bye", "stop")),
        &[("jobs/warm.nvs", &started), ("jobs/bye.nvs", &stopped)],
    );
    let map = served.recorded("started.json", "the start entry's run", BOUND, |_| true);
    assert_eq!(map["name"], "warm", "{map}");
    assert_eq!(map["on"], "start", "{map}");
    assert!(
        !served.dir.join("jobs").join("stopped.json").exists(),
        "the stop entry ran before the stop: {}",
        served.said()
    );

    served.stop();
    let text = std::fs::read_to_string(served.dir.join("jobs").join("stopped.json"))
        .unwrap_or_else(|_| panic!("the stop entry did not run: {}", served.said()));
    let map: serde_json::Value = serde_json::from_str(&text).expect("the stop run wrote JSON");
    assert_eq!(map["name"], "bye", "{map}");
    assert_eq!(map["on"], "stop", "{map}");
}

/// A saved valid edit runs the reload entry with `outcome = applied`, and a
/// saved broken edit runs it again with `outcome = refused`.
#[test]
fn a_reload_entry_runs_after_a_saved_configuration() {
    let reloaded = recording("reloaded.json");
    let entry = on("redeploy", "reload");
    let served = Served::start("reload", &entry, &[("jobs/redeploy.nvs", &reloaded)]);

    served.save(&format!(
        "{entry}[http.headers]\nreferrer_policy = \"same-origin\"\n"
    ));
    let map = served.recorded(
        "reloaded.json",
        "the run for the applied reload",
        BOUND,
        |map| map["outcome"] == "applied",
    );
    assert_eq!(map["name"], "redeploy", "{map}");
    assert_eq!(map["on"], "reload", "{map}");
    assert!(map["restart"].is_array(), "{map}");

    served.save(&format!("{entry}[not_a_block]\nkey = 1\n"));
    let map = served.recorded(
        "reloaded.json",
        "the run for the refused reload",
        BOUND,
        |map| map["outcome"] == "refused",
    );
    assert_eq!(map["on"], "reload", "{map}");
    assert!(
        !map.to_string().contains("not_a_block"),
        "the run was given the refusal's text: {map}"
    );
}

/// A `* * * * *` entry that throws runs the failed entry, and the failed
/// script receives the thrower's name. The cron entry runs at the next whole
/// minute, so this case waits up to a minute more than the others.
#[test]
fn a_failed_entry_runs_after_a_scheduled_script_throws() {
    let alerted = recording("alerted.json");
    let config = format!(
        "[[schedule]]\nname = \"minutely\"\ncron = \"* * * * *\"\nscript = \"jobs/tick.nvs\"\n\
         scope = \"host\"\n\n{}",
        on("alert", "failed")
    );
    let served = Served::start(
        "failed",
        &config,
        &[("jobs/tick.nvs", THROWING), ("jobs/alert.nvs", &alerted)],
    );
    let map = served.recorded(
        "alerted.json",
        "the failed entry's run",
        BOUND + Duration::from_secs(60),
        |_| true,
    );
    assert_eq!(map["name"], "alert", "{map}");
    assert_eq!(map["on"], "failed", "{map}");
    assert_eq!(map["schedule"], serde_json::json!(["minutely"]), "{map}");
    assert_eq!(map["dead_letter"], 0, "{map}");
}
