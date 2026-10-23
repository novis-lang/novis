//! `rule:config/the-resolved-root-is-announced-and-stored`, from outside the
//! process: a running `nvs serve` whose root is reached through a link reads
//! the file that link names now, with no restart.
//!
//! Two deployments have this shape. A Kubernetes ConfigMap volume reaches every
//! key through `..data`, and the kubelet updates it by swapping `..data` to a
//! new timestamped folder and deleting the old one. A link deploy swaps a
//! `current` link between two release folders and keeps the old one. Each case
//! writes `[http.headers] referrer_policy` into the linked folder, swaps the
//! link, and polls until a response carries the new value.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long a swapped link has to reach a request before the case fails. The
/// configuration check runs every two seconds and waits one more pass.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two requests of one poll.
const POLL: Duration = Duration::from_millis(100);

/// A program that answers every request with `ok`.
const PLAIN: &str = "<?nvs\necho \"ok\";\n";

/// A running `nvs --config <root> serve app.nvs`, killed when dropped.
struct Server {
    child: Child,
    addr: SocketAddr,
    stderr: Arc<Mutex<String>>,
    /// The private data folder, kept until the server is gone.
    _data: nvs_repo::Scratch,
}

impl Server {
    /// Starts `nvs serve` over `program`'s `app.nvs`, with `root` as its one
    /// `--config`, on a free loopback port.
    ///
    /// # Panics
    ///
    /// When the process cannot start or does not listen within [`BOOT`].
    fn start(program: &Path, root: &Path) -> Self {
        let data = nvs_repo::scratch_private("follows-links-data");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .arg("--config")
            .arg(root)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(program)
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
        let Ok(addr) = bound.recv_timeout(BOOT) else {
            let _ = child.kill();
            let _ = child.wait();
            let said = stderr.lock().expect("no reader panicked").clone();
            panic!("`nvs serve` did not start listening within {BOOT:?}: {said}");
        };
        let addr = addr
            .parse()
            .unwrap_or_else(|error| panic!("`{addr}` is not an address: {error}"));
        Self {
            child,
            addr,
            stderr,
            _data: data,
        }
    }

    /// The `Referrer-Policy` of one `GET /`, or `None` where the response has
    /// none or there is no response.
    fn policy(&self) -> Option<String> {
        let mut stream = TcpStream::connect(self.addr).ok()?;
        stream.set_read_timeout(Some(BOUND)).ok()?;
        write!(stream, "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n").ok()?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).ok()?;
        let text = String::from_utf8_lossy(&raw);
        let (head, _) = text.split_once("\r\n\r\n")?;
        head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case("referrer-policy")
                .then(|| value.trim().to_owned())
        })
    }

    /// Polls until a response carries `wanted` as its `Referrer-Policy`.
    ///
    /// # Panics
    ///
    /// When that takes longer than [`BOUND`]. The message has the last value
    /// and what the server wrote to standard error.
    fn awaits(&self, wanted: &str) {
        let started = Instant::now();
        loop {
            let seen = self.policy();
            if seen.as_deref() == Some(wanted) {
                return;
            }
            assert!(
                started.elapsed() <= BOUND,
                "the policy was {seen:?}, not `{wanted}`, after {BOUND:?}; the server wrote: {}",
                self.stderr.lock().expect("no reader panicked")
            );
            thread::sleep(POLL);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Writes `text` to `path`, creating any folder it needs.
fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the file's folder is created");
    }
    std::fs::write(path, text)
        .unwrap_or_else(|error| panic!("`{}` could not be written: {error}", path.display()));
}

/// An `nvs.toml` that includes `headers.toml` beside it.
const ROOT: &str = "[mode]\ndefault = \"production\"\n\n[[include]]\npath = \"headers.toml\"\n";

/// A `headers.toml` that sets `referrer_policy` to `policy`.
fn headers(policy: &str) -> String {
    format!("[http.headers]\nreferrer_policy = \"{policy}\"\n")
}

/// The program folder every case serves from, beside its configuration.
fn program(base: &Path) -> PathBuf {
    let program = base.join("program");
    write_file(&program.join("app.nvs"), PLAIN);
    program
}

/// The kubelet's update of a ConfigMap volume, step by step: every key is a link
/// through `..data` into a timestamped folder. An update writes a new folder,
/// points a new link at it, renames that link over `..data`, and deletes the old
/// folder. The root and its include both reach the new folder, and the server
/// publishes the new value.
#[cfg(unix)]
#[test]
fn a_kubelet_configmap_update_is_read_by_the_next_check() {
    use std::os::unix::fs::symlink;
    let base = nvs_repo::scratch_private("follows-links-kubelet");
    let program = program(&base);
    let volume = base.join("etc-nvs");
    let first = volume.join("..2026_10_23_08_00_00.000000001");
    write_file(&first.join("nvs.toml"), ROOT);
    write_file(&first.join("headers.toml"), &headers("no-referrer"));
    symlink(first.file_name().expect("a name"), volume.join("..data")).expect("`..data` is linked");
    for key in ["nvs.toml", "headers.toml"] {
        symlink(Path::new("..data").join(key), volume.join(key)).expect("the key is linked");
    }

    let server = Server::start(&program, &volume.join("nvs.toml"));
    server.awaits("no-referrer");

    let second = volume.join("..2026_10_23_08_01_00.000000002");
    write_file(&second.join("nvs.toml"), ROOT);
    write_file(&second.join("headers.toml"), &headers("same-origin"));
    symlink(second.file_name().expect("a name"), volume.join("..data_tmp"))
        .expect("`..data_tmp` is linked");
    std::fs::rename(volume.join("..data_tmp"), volume.join("..data"))
        .expect("`..data_tmp` replaces `..data`");
    std::fs::remove_dir_all(&first).expect("the old folder is deleted");

    server.awaits("same-origin");
}

/// A link to the folder `target` at `at`. On Windows a junction, which needs no
/// right, is made where a symbolic link cannot be.
fn link_dir(target: &Path, at: &Path) {
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(target, at).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(target, at).is_ok()
        || nvs_repo::spawn("cmd", &[])
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(at)
            .arg(target)
            .output()
            .is_ok_and(|out| out.status.success());
    assert!(linked, "`{}` could not be linked", at.display());
}

/// A link deploy: `current` points at one release folder, and the next deploy
/// points it at another. The old release stays on disk, so only the root's own
/// stamp moves. The server reads the new release's value.
#[test]
fn a_swapped_current_link_is_read_by_the_next_check() {
    let base = nvs_repo::scratch_private("follows-links-current");
    let program = program(&base);
    let releases = base.join("releases");
    let one = releases.join("1");
    let two = releases.join("2");
    for (release, policy) in [(&one, "no-referrer"), (&two, "same-origin")] {
        write_file(&release.join("nvs.toml"), ROOT);
        write_file(&release.join("headers.toml"), &headers(policy));
    }
    let current = base.join("current");
    link_dir(&one, &current);

    let server = Server::start(&program, &current.join("nvs.toml"));
    server.awaits("no-referrer");

    // A directory link is removed as a directory on Windows and as a file on Unix.
    #[cfg(windows)]
    std::fs::remove_dir(&current).expect("`current` is unlinked");
    #[cfg(not(windows))]
    std::fs::remove_file(&current).expect("`current` is unlinked");
    link_dir(&two, &current);

    server.awaits("same-origin");
}

/// A `current` link in a folder its group can write. The reload that follows a
/// swap there is refused with `E0607`, and the old value keeps serving. A boot
/// through the same link is refused too.
#[cfg(unix)]
#[test]
fn a_link_in_a_group_writable_folder_is_refused_at_reload_and_at_boot() {
    use std::os::unix::fs::PermissionsExt;

    let base = nvs_repo::scratch_private("follows-links-shared");
    let program = program(&base);
    let one = base.join("releases").join("1");
    let two = base.join("releases").join("2");
    for (release, policy) in [(&one, "no-referrer"), (&two, "same-origin")] {
        write_file(&release.join("nvs.toml"), ROOT);
        write_file(&release.join("headers.toml"), &headers(policy));
    }
    let shared = base.join("shared");
    std::fs::create_dir(&shared).expect("the folder the link sits in");
    let current = shared.join("current");
    link_dir(&one, &current);
    let root = current.join("nvs.toml");

    let server = Server::start(&program, &root);
    server.awaits("no-referrer");
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o775))
        .expect("a group-writable mode");
    std::fs::remove_file(&current).expect("`current` is unlinked");
    link_dir(&two, &current);
    let started = Instant::now();
    while !server.stderr.lock().expect("no reader panicked").contains("E0607") {
        assert!(
            started.elapsed() <= BOUND,
            "no reload refusal after {BOUND:?}; the server wrote: {}",
            server.stderr.lock().expect("no reader panicked")
        );
        thread::sleep(POLL);
    }
    assert_eq!(server.policy().as_deref(), Some("no-referrer"));
    drop(server);

    let data = nvs_repo::scratch_private("follows-links-shared-data");
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--config")
        .arg(&root)
        .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
        .current_dir(&program)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside starts");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child can be waited on") {
            break status;
        }
        if started.elapsed() > BOOT {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`nvs serve` still runs after {BOOT:?} through a shared link");
        }
        thread::sleep(POLL);
    };
    let mut said = String::new();
    child
        .stderr
        .take()
        .expect("standard error is piped")
        .read_to_string(&mut said)
        .expect("standard error is text");
    assert!(
        !status.success() && said.contains("E0607"),
        "the boot is refused by the link's folder ({status}): {said}"
    );
}
