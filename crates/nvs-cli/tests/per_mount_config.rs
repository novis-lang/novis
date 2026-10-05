//! `rule:config/every-matching-app-block-applies-least-specific-first` under
//! `nvs serve`, from outside the process: one server over a mount table of
//! several entries, each with an `[[app]]` block of its own, and what a program
//! under each mount reads back.
//!
//! Every case goes through [`Server`], `live_config.rs`'s harness without the
//! control endpoint. It starts the built `nvs serve` over a fresh directory
//! whose `nvs.toml` writes `[server] root = "."`, one `[[server.mount]]` per
//! entry and the case's own blocks, and names no file unless the case does.
//! Each mount is `/<name>`, answered by `<name>/index.nvs`.
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

/// How long one poll has to see the answer it waits for.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two requests of one poll.
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
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Writes `files` — each a path relative to the program's directory and its
    /// text — into a fresh directory named for `case`, writes `nvs.toml` as
    /// [`configured`] builds it from `mounts` and `blocks`, and starts
    /// `nvs serve` there on a free loopback port, naming `named` when it is
    /// given.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(
        case: &str,
        mounts: &[&str],
        blocks: &str,
        files: &[(&str, &str)],
        named: Option<&str>,
    ) -> Self {
        let dir = directory(case);
        Self::start_in(dir, mounts, blocks, files, named)
    }

    /// [`Server::start`] in `dir`, which [`directory`] made.
    fn start_in(
        dir: PathBuf,
        mounts: &[&str],
        blocks: &str,
        files: &[(&str, &str)],
        named: Option<&str>,
    ) -> Self {
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }
        write_file(&dir.join("nvs.toml"), &configured(mounts, blocks));

        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        command.arg("serve");
        if let Some(file) = named {
            command.arg(file);
        }
        let mut child = command
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
            stderr,
        }
    }

    /// One `GET` of `path`, on a connection of its own.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn get(&self, path: &str) -> Answer {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n")
            .expect("the request is sent");
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
                let said = self.stderr.lock().expect("no reader panicked").clone();
                panic!(
                    "{what} did not happen within {BOUND:?}; the last answer was {answer:?}, \
                     and the server wrote: {said}"
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

/// A fresh, empty directory for `case`, named for the case and this process so
/// no two cases share one.
fn directory(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("per-mount-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    dir
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

/// `path` with forward slashes, which Novis strings and TOML both take as
/// they are on every platform.
fn slashed(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// The entry file of the mount `name`.
fn entry(name: &str) -> String {
    format!("{name}/index.nvs")
}

/// A program that builds a string of 32 MiB and prints its length.
const GROWING: &str = r"<?nvs
string $text = 'x';
uint $round = 0;
while ($round < 25) {
    $text = $text . $text;
    $round = $round + 1;
}
echo Core\Str::length($text);
";

/// `[app.limits] memory` is read off the snapshot of the file that runs. The
/// mount whose block allows 16M stops before its string reaches 32 MiB, and the
/// mount whose block allows 256M builds it. Each reads its own value back.
#[test]
fn two_mounts_run_under_their_own_app_limits() {
    let (one, two) = (entry("one"), entry("two"));
    let reading = "<?nvs\necho Core\\Config::get(\"limits.memory\") ?? \"unset\";\n";
    let server = Server::start(
        "limits",
        &["one", "two", "onereads", "tworeads"],
        "[limits.hard]\nmemory = \"512M\"\n\n\
         [[app]]\nroot = \"one\"\n\n[app.limits]\nmemory = \"16M\"\n\n\
         [[app]]\nroot = \"onereads\"\n\n[app.limits]\nmemory = \"16M\"\n\n\
         [[app]]\nroot = \"two\"\n\n[app.limits]\nmemory = \"256M\"\n\n\
         [[app]]\nroot = \"tworeads\"\n\n[app.limits]\nmemory = \"256M\"\n",
        &[
            (&one, GROWING),
            (&two, GROWING),
            ("onereads/index.nvs", reading),
            ("tworeads/index.nvs", reading),
        ],
        None,
    );

    assert_eq!(server.body("/two"), "33554432", "the 256M mount");
    let small = server.get("/one");
    assert_ne!(
        small.status, 200,
        "the 16M mount built a 32 MiB string: {small:?}"
    );
    assert!(
        !small.body.contains("33554432"),
        "the 16M mount printed the length of a 32 MiB string: {small:?}"
    );
    assert_eq!(server.body("/onereads"), "16M");
    assert_eq!(server.body("/tworeads"), "256M");
}

/// `[app.capabilities]` is read off the snapshot of the file that runs. The
/// block for `one` grants reading `data`, and `two` has a block that grants
/// nothing, so the same read succeeds under `one` and throws under `two`.
#[test]
fn a_mount_cannot_use_a_capability_only_another_applications_block_grants() {
    let dir = directory("capability");
    let data = slashed(&dir.join("data"));
    let reading = format!(
        "<?nvs\ntry {{\n    echo Core\\IO::read('{data}/shared.txt');\n}} catch (Throwable $denied) {{\n    echo \"denied\";\n}}\n"
    );
    let server = Server::start_in(
        dir,
        &["one", "two"],
        &format!(
            "[[app]]\nroot = \"one\"\n\n[app.capabilities.fs]\nread = [\"{data}\"]\n\n\
             [[app]]\nroot = \"two\"\nmode = \"development\"\n"
        ),
        &[
            (&entry("one"), &reading),
            (&entry("two"), &reading),
            ("data/shared.txt", "granted"),
        ],
        None,
    );

    assert_eq!(
        server.body("/one"),
        "granted",
        "the mount whose block grants the read"
    );
    assert_eq!(
        server.body("/two"),
        "denied",
        "the mount whose block grants nothing read the other application's file"
    );
}

/// A program that prints the mode it runs in.
const MODE: &str = "<?nvs\necho Core\\Env::mode() == Core\\Env\\Mode::Production ? \"production\" : \"development\";\n";

/// `mode` is read off the snapshot of the file that runs: the block for `one`
/// writes `production` and the block for `two` writes `development`, and each
/// mount reads its own.
#[test]
fn each_mount_reads_its_own_app_mode() {
    let server = Server::start(
        "mode",
        &["one", "two"],
        "[[app]]\nroot = \"one\"\nmode = \"production\"\n\n\
         [[app]]\nroot = \"two\"\nmode = \"development\"\n",
        &[(&entry("one"), MODE), (&entry("two"), MODE)],
        None,
    );

    assert_eq!(server.body("/one"), "production");
    assert_eq!(server.body("/two"), "development");
}

/// A program that declares one route and prints the absolute link to it.
const LINKING: &str = r#"<?nvs
class Docs {
    #[Core\Route(path: "/here", method: Core\Http\Method::Get, name: "Docs::here")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function here(): string { return "here"; }
}

echo Core\Router::urlAbsolute("Docs::here", []);
"#;

/// The two `[[app]] origin` blocks, for `one` and `two`.
const ORIGINS: &str = "[[app]]\nroot = \"one\"\norigin = \"https://one.example.test\"\n\n\
                       [[app]]\nroot = \"two\"\norigin = \"https://two.example.test\"\n";

/// `[[app]] origin` is folded into each row from its own entry's snapshot, so
/// each mount links from its own origin and never from the other's.
#[test]
fn each_mount_takes_its_own_app_origin() {
    let server = Server::start(
        "origin",
        &["one", "two"],
        ORIGINS,
        &[(&entry("one"), LINKING), (&entry("two"), LINKING)],
        None,
    );

    let one = server.body("/one");
    assert!(one.starts_with("https://one.example.test/"), "{one}");
    let two = server.body("/two");
    assert!(two.starts_with("https://two.example.test/"), "{two}");
}

/// A program that prints the mode it runs in and its `limits.memory`.
const READING: &str = "<?nvs\necho Core\\Env::mode() == Core\\Env\\Mode::Production ? \"production\" : \"development\", \" \", Core\\Config::get(\"limits.memory\") ?? \"unset\";\n";

/// A mount whose entry no `[[app]]` block matches runs under the global
/// configuration: the global mode and `[limits] memory`, and nothing the
/// other mount's block writes.
#[test]
fn a_mount_no_block_matches_runs_under_the_global_configuration() {
    let server = Server::start(
        "global",
        &["one", "three"],
        "[limits]\nmemory = \"100M\"\n\n\
         [[app]]\nroot = \"one\"\nmode = \"production\"\n\n[app.limits]\nmemory = \"16M\"\n",
        &[(&entry("one"), READING), (&entry("three"), READING)],
        None,
    );

    assert_eq!(server.body("/one"), "production 16M");
    assert_eq!(server.body("/three"), "development 100M");
}

/// A program that prints the mode it runs in and the absolute link to its
/// one route.
const LINKING_AND_MODE: &str = r#"<?nvs
class Docs {
    #[Core\Route(path: "/here", method: Core\Http\Method::Get, name: "Docs::here")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function here(): string { return "here"; }
}

echo Core\Env::mode() == Core\Env\Mode::Production ? "production" : "development", " ";
echo Core\Router::urlAbsolute("Docs::here", []);
"#;

/// `nvs serve one/index.nvs` over a mount table only checks that the file is
/// mounted. The block for `one` does not reach `two`: `two` keeps its own
/// origin and the global mode, and `one` keeps its own.
#[test]
fn a_named_file_does_not_change_another_mounts_configuration() {
    let server = Server::start(
        "named",
        &["one", "two"],
        "[[app]]\nroot = \"one\"\nmode = \"production\"\norigin = \"https://one.example.test\"\n\n\
         [[app]]\nroot = \"two\"\norigin = \"https://two.example.test\"\n",
        &[
            (&entry("one"), LINKING_AND_MODE),
            (&entry("two"), LINKING_AND_MODE),
        ],
        Some("one/index.nvs"),
    );

    let one = server.body("/one");
    assert!(
        one.starts_with("production https://one.example.test/"),
        "the named mount: {one}"
    );
    let two = server.body("/two");
    assert!(
        two.starts_with("development https://two.example.test/"),
        "the other mount took the named file's blocks: {two}"
    );
}

/// A program that writes its mount's name into the process tier when the query
/// says `put=yes`, and then prints what the tier has under the same key.
fn caching(name: &str) -> String {
    format!(
        "<?nvs\nvar $cache = Core\\Cache::process();\n\
         if (Core\\Request::query(\"put\") == \"yes\") {{\n    $cache->put(\"shared-key\", \"{name}\");\n}}\n\
         echo ($cache->get(\"shared-key\") ?? \"absent\") as string;\n"
    )
}

/// The process tier's key carries the `[[app]]` the request runs under, so an
/// entry one mount writes is read back by that mount and not by the other,
/// even under the same key.
#[test]
fn two_mounts_do_not_share_process_tier_cache_entries() {
    let server = Server::start(
        "cache",
        &["one", "two"],
        "[[app]]\nroot = \"one\"\nmode = \"development\"\n\n\
         [[app]]\nroot = \"two\"\nmode = \"development\"\n",
        &[
            (&entry("one"), &caching("one")),
            (&entry("two"), &caching("two")),
        ],
        None,
    );

    assert_eq!(server.body("/one?put=yes"), "one");
    assert_eq!(server.body("/one"), "one", "the mount that wrote the entry");
    assert_eq!(
        server.body("/two"),
        "absent",
        "the other mount read the first mount's entry"
    );
}

/// A program that prints the mode it runs in, its `limits.memory` and the
/// absolute link to its one route.
const READING_ALL: &str = r#"<?nvs
class Docs {
    #[Core\Route(path: "/here", method: Core\Http\Method::Get, name: "Docs::here")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function here(): string { return "here"; }
}

echo Core\Env::mode() == Core\Env\Mode::Production ? "production" : "development", " ";
echo Core\Config::get("limits.memory") ?? "unset", " ";
echo Core\Router::urlAbsolute("Docs::here", []);
"#;

/// The `[[app]]` block for `two`, which neither case below changes.
const TWO: &str = "[[app]]\nroot = \"two\"\norigin = \"https://two.example.test\"\n\n\
                   [app.limits]\nmemory = \"100M\"\n";

/// A saved `nvs.toml` is a reload, and the reload publishes every mounted
/// entry's snapshot again. The edit changes `one`'s block, and `one` reads its
/// new limit and origin. `two` keeps its own block's, and never the host's.
#[test]
fn a_reload_that_changes_one_app_block_reaches_only_its_mount() {
    let one_block = |memory: &str, origin: &str| {
        format!(
            "[[app]]\nroot = \"one\"\nmode = \"production\"\norigin = \"{origin}\"\n\n\
             [app.limits]\nmemory = \"{memory}\"\n\n{TWO}"
        )
    };
    let server = Server::start(
        "reload",
        &["one", "two"],
        &one_block("16M", "https://one.example.test"),
        &[(&entry("one"), READING_ALL), (&entry("two"), READING_ALL)],
        None,
    );
    let before = server.body("/one");
    assert!(
        before.starts_with("production 16M https://one.example.test/"),
        "{before}"
    );

    write_file(
        &server.dir.join("nvs.toml"),
        &configured(
            &["one", "two"],
            &one_block("32M", "https://renamed.example.test"),
        ),
    );
    // The limit is in force from the publish, and the origin from the next
    // pass of the background expansion, which folds it into the rows.
    server.awaits("/one", "the reload of `one`'s block", |answer| {
        answer.status == 200
            && answer
                .body
                .starts_with("production 32M https://renamed.example.test/")
    });
    let two = server.body("/two");
    assert!(
        two.starts_with("development 100M https://two.example.test/"),
        "`two` lost its own block in the reload: {two}"
    );
}

/// A row the background expansion adds after boot runs under the blocks that
/// match its own entry: its mode, its limit and its origin. The directory the
/// block names is there at boot, and its entry file is written later.
#[test]
fn a_mount_the_rescan_adds_runs_under_its_own_app_blocks() {
    let server = Server::start(
        "rescan",
        &[],
        "[[server.mount]]\nscan = \"*/index.nvs\"\nprefix = \"/{1}\"\n\n\
         [[app]]\nroot = \"one\"\norigin = \"https://one.example.test\"\n\n\
         [[app]]\nroot = \"two\"\nmode = \"production\"\norigin = \"https://two.example.test\"\n\n\
         [app.limits]\nmemory = \"48M\"\n",
        &[(&entry("one"), READING_ALL), ("two/notes.txt", "later")],
        None,
    );
    let one = server.body("/one");
    assert!(
        one.starts_with("development unset https://one.example.test/"),
        "{one}"
    );

    write_file(&server.dir.join(entry("two")), READING_ALL);
    let two = server.body("/two");
    assert!(
        two.starts_with("production 48M https://two.example.test/"),
        "the row the rescan added did not run under its own block: {two}"
    );
}
