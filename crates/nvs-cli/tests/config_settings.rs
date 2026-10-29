//! `rule:config/settings-and-secrets-are-a-programs-own-values` under
//! `nvs serve`, from outside the process: what a program under each mount reads
//! back with `Core\Config::setting` and `Core\Config::secret`, across a reload,
//! and what `Core\Config::get` and `all` never return.
//!
//! Every case goes through [`Server`], the harness `per_mount_config.rs` uses.
//! It starts the built `nvs serve` over a fresh directory whose `nvs.toml`
//! writes `[server] root = "."`, one `[[server.mount]]` per entry and the case's
//! own blocks. Each mount is `/<name>`, answered by `<name>/index.nvs`.
//!
//! The program lives under `CARGO_TARGET_TMPDIR`, inside the build directory,
//! and is removed when its [`Server`] is dropped.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long one poll has to see the answer or the file it waits for.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two looks of one poll.
const POLL: Duration = Duration::from_millis(50);

/// One answer: the status code and the body, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    body: String,
}

/// A running `nvs serve` over a mount table in its own directory.
///
/// Dropping it stops the process and removes the directory.
struct Server {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    /// The private data folder `nvs serve` is given.
    _data: nvs_repo::Scratch,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Writes `files` — each a path relative to the program's directory and its
    /// text — into a fresh directory named for `case`, writes `nvs.toml` as
    /// [`configured`] builds it from `mounts` and `blocks`, and starts
    /// `nvs serve` there on a free loopback port.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str, mounts: &[&str], blocks: &str, files: &[(&str, &str)]) -> Self {
        let dir = directory(case);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }
        write_file(&dir.join("nvs.toml"), &configured(mounts, blocks));

        let data = nvs_repo::scratch_private("nvsdata");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .arg("serve")
            .args(["--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let addr = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    let _ = sent.send(addr);
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
            dir,
            _data: data,
            stderr,
        }
    }

    /// What the server wrote to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// One `GET` of `path`, on a connection of its own.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn get(&self, path: &str) -> Answer {
        get(self.addr, path)
    }

    /// Polls `path` until `wanted` holds of its answer, and returns that answer.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first. The message names `what` was awaited, the
    /// last answer and what the server wrote to standard error.
    fn awaits(&self, path: &str, what: &str, wanted: impl Fn(&Answer) -> bool) -> Answer {
        let started = Instant::now();
        loop {
            let answer = self.get(path);
            if wanted(&answer) {
                return answer;
            }
            if started.elapsed() > BOUND {
                panic!(
                    "{what} did not happen within {BOUND:?}; the last answer was {answer:?}, \
                     and the server wrote: {}",
                    self.said()
                );
            }
            thread::sleep(POLL);
        }
    }

    /// Polls `path` until it answers `200`, and returns its body.
    fn body(&self, path: &str) -> String {
        self.awaits(path, &format!("a `200` from `{path}`"), |answer| {
            answer.status == 200
        })
        .body
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The directory [`Server::start`] runs `case` in, named for the case and this
/// process so no two cases share one.
fn directory(case: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("config-settings-{case}-{}", std::process::id()))
}

/// One `GET` of `path` at `addr`, on a connection of its own.
fn get(addr: SocketAddr, path: &str) -> Answer {
    let mut stream =
        TcpStream::connect(addr).unwrap_or_else(|error| panic!("{addr} does not answer: {error}"));
    stream
        .set_read_timeout(Some(BOUND))
        .expect("a read timeout is set");
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("the request is sent");
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .unwrap_or_else(|error| panic!("the answer to `{path}` was cut off: {error}"));
    let text = String::from_utf8_lossy(&raw);
    let (head, body) = text
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("the answer to `{path}` has no header block: {text}"));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("the answer to `{path}` has no status: {head}"));
    Answer {
        status,
        body: body.to_owned(),
    }
}

/// `nvs.toml`: `[mode] default = "development"`, the background check every
/// 100ms, `[server] root = "."`, one `[[server.mount]]` at `/<name>` for
/// `<name>/index.nvs` per name in `mounts`, and then `blocks`.
fn configured(mounts: &[&str], blocks: &str) -> String {
    let mut text = String::from(
        "[mode]\ndefault = \"development\"\n\n[opcache]\nrevalidate_freq = \"100ms\"\n\n\
         [server]\nroot = \".\"\n",
    );
    for name in mounts {
        text.push_str(&format!(
            "\n[[server.mount]]\nprefix = \"/{name}\"\nentry = \"{name}/index.nvs\"\n"
        ));
    }
    text.push('\n');
    text.push_str(blocks);
    text
}

/// Writes `text` to `path`, creating any directory it needs.
fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the file's directory is created");
    }
    std::fs::write(path, text).unwrap_or_else(|error| {
        panic!("`{}` could not be written: {error}", path.display());
    });
}

/// The entry file of the mount `name`.
fn entry(name: &str) -> String {
    format!("{name}/index.nvs")
}

/// A program that prints its `page_size` and `title`, and which secret it reads.
const READING: &str = r#"<?nvs
echo Core\Config::setting("page_size") as int, " ";
echo Core\Config::setting("title") as string, " ";
string $key = "none";
if (Core\Config::secret("api_key") == "shop-key") {
    $key = "shop";
} else if (Core\Config::secret("api_key") == "global-key") {
    $key = "global";
}
echo $key;
"#;

/// The global blocks, and one `[[app]]` block for `shop` that replaces
/// `page_size` and `api_key`. `blog` has no block of its own.
const SHOP_AND_BLOG: &str = "[settings]\npage_size = 10\ntitle = \"Store\"\n\n\
                             [secrets]\napi_key = \"global-key\"\n\n\
                             [[app]]\nroot = \"shop\"\n\n\
                             [app.settings]\npage_size = 50\n\n\
                             [app.secrets]\napi_key = \"shop-key\"\n";

/// Each mount reads the snapshot of its own application. `shop` reads its
/// block's values over the global ones, and `blog` reads the global ones and
/// never `shop`'s.
#[test]
fn an_app_reads_its_own_settings_and_not_another_apps() {
    let server = Server::start(
        "apps",
        &["shop", "blog"],
        SHOP_AND_BLOG,
        &[(&entry("shop"), READING), (&entry("blog"), READING)],
    );
    assert_eq!(server.body("/shop"), "50 Store shop");
    assert_eq!(
        server.body("/blog"),
        "10 Store global",
        "the mount with no block read another application's values"
    );
}

/// A program that reads `title`, and when the query says `hold=yes` writes
/// `started.txt` in the folder the `marks` setting names and waits for
/// `go.txt` there before it reads `title` again.
const HOLDING: &str = r#"<?nvs
string $before = Core\Config::setting("title") as string;
if (Core\Request::query("hold") == "yes") {
    string $marks = Core\Config::setting("marks") as string;
    Core\IO::write($marks . "/started.txt", "started");
    while (!Core\IO::exists($marks . "/go.txt")) {
        Core\Time::sleep(20ms);
    }
}
echo $before, " ", Core\Config::setting("title") as string;
"#;

/// The blocks of the reload case, with `title` set to `title`. `marks` is the
/// absolute path of the folder the held request signals through.
fn titled(title: &str) -> String {
    let marks = directory("reload")
        .join("marks")
        .display()
        .to_string()
        .replace('\\', "/");
    format!(
        "[capabilities.fs]\nread = [\"{marks}\"]\nwrite = [\"{marks}\"]\n\n\
         [settings]\ntitle = \"{title}\"\nmarks = \"{marks}\"\n"
    )
}

/// A saved `nvs.toml` is a reload. The next request reads the new value, and a
/// request that started before the reload reads the old value to its end.
#[test]
fn a_reload_reaches_the_next_request_and_not_a_running_one() {
    let server = Server::start(
        "reload",
        &["app"],
        &titled("Old"),
        &[(&entry("app"), HOLDING), ("marks/.keep", "")],
    );
    assert_eq!(server.body("/app"), "Old Old");

    let started = server.dir.join("marks").join("started.txt");
    let addr = server.addr;
    let held = thread::spawn(move || get(addr, "/app?hold=yes"));
    let waited = Instant::now();
    while !started.exists() {
        assert!(
            waited.elapsed() < BOUND,
            "the held request did not start within {BOUND:?}: {}",
            server.said()
        );
        thread::sleep(POLL);
    }

    write_file(
        &server.dir.join("nvs.toml"),
        &configured(&["app"], &titled("New")),
    );
    server.awaits("/app", "the reload of `title`", |answer| {
        answer.status == 200 && answer.body == "New New"
    });

    write_file(&server.dir.join("marks").join("go.txt"), "go");
    let answer = held.join().expect("the held request's thread panicked");
    assert_eq!(
        (answer.status, answer.body.as_str()),
        (200, "Old Old"),
        "the request in flight across the reload read the new value: {answer:?}"
    );
}

/// A program that prints what `get` returns for the four blocks' keys, and then
/// every `key=value` line of `all`.
const DIRECTIVES: &str = r#"<?nvs
echo "settings.title=", Core\Config::get("settings.title") ?? "null", "\n";
echo "secrets.api_key=", Core\Config::get("secrets.api_key") ?? "null", "\n";
echo "settings=", Core\Config::get("settings") ?? "null", "\n";
foreach (Core\Config::all() as string $name => string $value) {
    echo $name, "=", $value, "\n";
}
"#;

/// `get` returns `null` for every key of the four blocks, and `all` lists none
/// of them, so no value of `[settings]` or `[secrets]` reaches a program as a
/// directive. The secret's value appears nowhere in the answer.
#[test]
fn config_get_and_all_never_return_a_setting_or_a_secret() {
    let server = Server::start(
        "directives",
        &["shop"],
        SHOP_AND_BLOG,
        &[(&entry("shop"), DIRECTIVES)],
    );
    let body = server.body("/shop");
    let mut lines = body.lines();
    assert_eq!(lines.next(), Some("settings.title=null"), "{body}");
    assert_eq!(lines.next(), Some("secrets.api_key=null"), "{body}");
    assert_eq!(lines.next(), Some("settings=null"), "{body}");
    let listed: Vec<&str> = lines
        .filter(|line| {
            ["settings", "secrets", "app."]
                .iter()
                .any(|block| line.starts_with(block))
        })
        .collect();
    assert!(listed.is_empty(), "`all` listed {listed:?}");
    assert!(
        body.lines().any(|line| line.starts_with("log.level=")),
        "`all` listed no directive at all: {body}"
    );
    for value in ["shop-key", "global-key", "Store"] {
        assert!(
            !body.contains(value),
            "`{value}` reached the answer: {body}"
        );
    }
}
