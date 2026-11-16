//! A running `nvs serve` for an integration test: the boot, the wait for its
//! address, its standard error and its end.
//!
//! The crate cannot name the `nvs` binary, because only the test binaries of
//! `nvs-cli` are built beside it. So the test builds the [`Command`], and
//! [`Served::spawn`] owns everything after that: the pipes, the
//! `listening on` line, the address it names, and stopping the process and
//! removing its directory when the value is dropped.

use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// How long `nvs serve` has to compile its program and bind its port.
pub const BOOT: Duration = Duration::from_secs(60);

/// The time between two requests of one poll.
pub const POLL: Duration = Duration::from_millis(50);

/// A running `nvs serve`, stopped and its directory removed on drop.
#[derive(Debug)]
pub struct Served {
    child: Child,
    /// The address its `listening on` line named.
    pub addr: SocketAddr,
    /// The directory the test wrote the program into.
    pub dir: PathBuf,
    stderr: Arc<Mutex<String>>,
}

impl Served {
    /// Starts `command` with both output pipes read on threads of their own,
    /// so a server that writes a lot never blocks on a full pipe, and waits
    /// for its `listening on http://` line. `dir` is removed once the process
    /// has stopped.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or names no address within [`BOOT`];
    /// the message carries what it wrote to standard error.
    pub fn spawn(command: &mut Command, dir: PathBuf) -> Self {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
            addr: SocketAddr::from(([0, 0, 0, 0], 0)),
            dir,
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

    /// Everything the server has written to standard error so far.
    pub fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Runs `command` to its end and checks that it succeeded.
///
/// # Panics
///
/// When it cannot start or exits with a failure; the message names its
/// arguments and carries its standard error.
pub fn succeeds(command: &mut Command) {
    let ran = command
        .stdin(Stdio::null())
        .output()
        .expect("the `nvs` binary this test was built beside starts");
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect();
    assert!(
        ran.status.success(),
        "`nvs {}` failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&ran.stderr)
    );
}
