//! `rule:config/the-ownership-check-runs-where-it-can-be-answered` from outside the process:
//! `nvs serve` reads every configuration file through the ownership check, at boot and at each
//! reload, in development mode as in production.
//!
//! Each case works in a directory from `nvs_repo::scratch_private`, which only this account can
//! reach, so a tree the case leaves alone passes the check on every machine. A case then opens
//! one file to other accounts: `chmod g+w` on Unix, and a write grant for `Authenticated Users` on
//! Windows, given by SID because the account names are localized.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long `nvs serve` has to compile its program and bind its port, or to refuse.
const BOOT: Duration = Duration::from_secs(60);

/// How long a reload has to notice a saved file: the check runs every two seconds, and a stamp
/// that moved must hold for one more check.
const RELOAD: Duration = Duration::from_secs(30);

/// The time between two looks at a running server.
const POLL: Duration = Duration::from_millis(100);

/// The program every case serves.
const PROGRAM: &str = "<?nvs\necho \"served\";\n";

/// A tree in a private directory: `app.nvs`, `nvs.toml` with `config`, and `files`.
fn tree(case: &str, config: &str, files: &[(&str, &str)]) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch_private(&format!("serve-ownership-{case}"));
    std::fs::write(dir.join("app.nvs"), PROGRAM).expect("the program is written");
    std::fs::write(dir.join("nvs.toml"), config).expect("the configuration is written");
    for (name, text) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the file's folder is created");
        }
        std::fs::write(path, text).expect("the case's file is written");
    }
    dir
}

/// `nvs serve app.nvs` in `dir` on a free loopback port, with its standard error collected and
/// the address its `listening on` line names sent once.
struct Started {
    child: Child,
    stderr: Arc<Mutex<String>>,
    bound: mpsc::Receiver<String>,
    _data: nvs_repo::Scratch,
}

fn start(dir: &Path) -> Started {
    let data = nvs_repo::scratch_private("serve-ownership-data");
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
        .current_dir(dir)
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
    Started {
        child,
        stderr,
        bound,
        _data: data,
    }
}

impl Started {
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// The exit status of a server that must refuse to start, and what it wrote.
    fn refused(mut self) -> (ExitStatus, String) {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("the server's status is read") {
                // The reader thread may still hold the last lines.
                thread::sleep(POLL);
                return (status, self.said());
            }
            if self.bound.try_recv().is_ok() || started.elapsed() > BOOT {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!(
                    "`nvs serve` started on a tree it must refuse: {}",
                    self.said()
                );
            }
            thread::sleep(POLL);
        }
    }

    /// The address of a server that must start.
    fn listening(&mut self) -> String {
        if let Ok(addr) = self.bound.recv_timeout(BOOT) {
            return addr;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        panic!(
            "`nvs serve` did not start listening within {BOOT:?}: {}",
            self.said()
        );
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One `GET /`, and the status and body of the answer.
fn get(addr: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|error| panic!("{addr}: {error}"));
    stream
        .set_read_timeout(Some(RELOAD))
        .expect("a read timeout is set");
    write!(stream, "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("the request is sent");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .expect("the answer is read");
    let status = answer
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("not an HTTP answer: {answer}"));
    let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
    (status, body.to_owned())
}

/// The path as the check names it.
fn named(path: &Path) -> String {
    nvs_config::trust::canonical(path)
        .expect("the path is there")
        .display()
        .to_string()
}

/// Opens `path` to writes by other accounts: the group's write bit on Unix, a modify grant for
/// `Authenticated Users` on Windows.
fn open_to_others(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .expect("the path is there")
            .permissions()
            .mode();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode | 0o020))
            .expect("the group may write");
    }
    #[cfg(windows)]
    {
        let status = Command::new("icacls")
            .arg(path)
            .args(["/grant", "*S-1-5-11:(M)"])
            .stdout(Stdio::null())
            .status()
            .expect("icacls runs");
        assert!(status.success(), "icacls granted the write");
    }
}

/// A private tree serves: the check passes what an ordinary installation looks like, in
/// development mode too.
#[test]
fn a_tree_only_this_account_can_write_serves() {
    let dir = tree("private", "[mode]\ndefault = \"development\"\n", &[]);
    let mut server = start(&dir);
    let addr = server.listening();
    assert_eq!(get(&addr), (200, "served".to_owned()));
}

/// A root `nvs.toml` another account can write stops the boot with `E0607`, naming the file.
/// Development mode is checked too: the mode is written in the same file.
#[test]
fn a_root_another_account_can_write_stops_the_boot() {
    let dir = tree("root", "[mode]\ndefault = \"development\"\n", &[]);
    let root = dir.join("nvs.toml");
    open_to_others(&root);
    let (status, said) = start(&dir).refused();
    assert!(!status.success(), "the boot fails: {said}");
    assert!(
        said.contains("E0607") && said.contains(&named(&root)),
        "the refusal is E0607 and names {}: {said}",
        root.display()
    );
}

/// An include another account can write stops the boot the same way, and names the include.
#[test]
fn an_include_another_account_can_write_stops_the_boot() {
    let dir = tree(
        "include",
        "[[include]]\npath = \"conf.d/limits.toml\"\n",
        &[("conf.d/limits.toml", "[limits]\nmemory = \"128M\"\n")],
    );
    let include = dir.join("conf.d").join("limits.toml");
    open_to_others(&include);
    let (status, said) = start(&dir).refused();
    assert!(!status.success(), "the boot fails: {said}");
    assert!(
        said.contains("E0607") && said.contains(&named(&include)),
        "the refusal is E0607 and names {}: {said}",
        include.display()
    );
}

/// A reload that finds a file another account can write is refused: the server keeps the
/// snapshot it is serving, and the log names the file.
#[test]
fn a_reload_that_finds_a_writable_file_keeps_serving_and_names_it() {
    let dir = tree("reload", "[limits]\nmemory = \"128M\"\n", &[]);
    let root: PathBuf = dir.join("nvs.toml");
    let mut server = start(&dir);
    let addr = server.listening();
    assert_eq!(get(&addr).0, 200);

    open_to_others(&root);
    std::fs::write(&root, "[limits]\nmemory = \"256M\"\n").expect("the file is saved");
    let path = named(&root);
    let started = Instant::now();
    while !(server.said().contains("E0607") && server.said().contains(&path)) {
        assert!(
            started.elapsed() < RELOAD,
            "no reload refusal named {path} within {RELOAD:?}: {}",
            server.said()
        );
        thread::sleep(POLL);
    }
    assert_eq!(
        get(&addr),
        (200, "served".to_owned()),
        "the running snapshot still serves"
    );
}
