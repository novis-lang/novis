//! `rule:config/an-edit-reaches-the-next-request-without-a-restart`, from outside
//! the process: the built `nvs serve` over a program in a directory of its own,
//! an edit to that directory, and the answer a request gets afterwards.
//!
//! Every case goes through [`Server`]. It starts the binary this test was built
//! beside on a port the platform chose, reads the port back from the
//! `listening on` line, and answers each request over a fresh HTTP/1.0
//! connection, so the body ends where the connection does and no framing is
//! parsed. An edit is observed by polling: [`Server::awaits`] asks until the
//! answer is the one wanted or [`BOUND`] runs out, and the failure message says
//! what the last answer was. The bound is far above what the rule allows an
//! idle server — `revalidate_freq` plus `settle` — so a pass is never a race
//! against a slow machine, and a fail means the edit never arrived.
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

/// How long an edit has to reach a request before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two requests of one poll.
const POLL: Duration = Duration::from_millis(50);

/// `[mode] default` written out, so a case about production does not depend on
/// what a configuration with nothing in it starts in.
const PRODUCTION: &str = "[mode]\ndefault = \"production\"\n";

/// One answer: the status code and the body, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    body: String,
}

/// A running `nvs serve` over a program in its own directory.
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
    /// text — into a fresh directory named for `case`, and starts `nvs serve
    /// app.nvs` there, on a free loopback port.
    ///
    /// The server runs with that directory as its working directory, so an
    /// `nvs.toml` among `files` is the configuration it reads.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str, files: &[(&str, &str)]) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("live-edit-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }

        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
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

    /// Polls `path` until it answers `200` with exactly `body`.
    fn awaits_body(&self, path: &str, body: &str) -> Answer {
        self.awaits(path, &format!("`{path}` answering `{body}`"), |answer| {
            answer.status == 200 && answer.body == body
        })
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// Waits until the server has written `text` to standard error. Standard
    /// error is one ordered stream, so every line written before `text` has
    /// arrived too.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first.
    fn awaits_said(&self, text: &str) {
        let started = Instant::now();
        while !self.said().contains(text) {
            assert!(
                started.elapsed() <= BOUND,
                "the server did not write `{text}` within {BOUND:?}; it wrote: {}",
                self.said()
            );
            thread::sleep(POLL);
        }
    }

    /// Writes `text` to `path`, relative to the program's directory, creating
    /// any directory it needs.
    fn write(&self, path: &str, text: &str) {
        write_file(&self.dir.join(path), text);
    }

    /// Deletes `path`, relative to the program's directory.
    fn remove(&self, path: &str) {
        let path = self.dir.join(path);
        std::fs::remove_file(&path).unwrap_or_else(|error| {
            panic!("`{}` could not be removed: {error}", path.display());
        });
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
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

/// A program that prints `word` and nothing else.
fn printing(word: &str) -> String {
    format!("<?nvs\necho \"{word}\";\n")
}

/// The entry file is the one file today's resolve-time check has always seen,
/// so this case holds the harness to the half of the rule that is on disk.
#[test]
fn an_edit_to_the_entry_file_reaches_the_next_request_in_production() {
    let server = Server::start(
        "entry",
        &[("nvs.toml", PRODUCTION), ("app.nvs", &printing("before"))],
    );
    server.awaits_body("/", "before");

    server.write("app.nvs", &printing("after"));
    server.awaits_body("/", "after");
}

/// A broken entry file fails every request that reaches it, and the fix that
/// follows is served without a restart.
#[test]
fn a_broken_edit_fails_requests_and_its_fix_recovers_them() {
    let server = Server::start(
        "broken",
        &[("nvs.toml", PRODUCTION), ("app.nvs", &printing("good"))],
    );
    server.awaits_body("/", "good");

    server.write("app.nvs", "<?nvs\necho \"broken\"\n");
    let failed = server.awaits("/", "the broken edit failing `/`", |answer| {
        answer.status != 200
    });
    assert!(
        !failed.body.contains("good"),
        "the last good version is never served in place of a broken one: {failed:?}"
    );

    server.write("app.nvs", &printing("fixed"));
    server.awaits_body("/", "fixed");
}

/// An entry file that requires `lib.nvs` and prints nothing itself.
const REQUIRING: &str = "<?nvs\nrequire './lib.nvs';\n";

/// The entry file never changes here, so only a check of the file it requires
/// can bring the edit to a request.
#[test]
fn an_edit_to_a_required_file_reaches_the_next_request() {
    let server = Server::start(
        "required",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", REQUIRING),
            ("lib.nvs", &printing("before")),
        ],
    );
    server.awaits_body("/", "before");

    server.write("lib.nvs", &printing("after"));
    server.awaits_body("/", "after");
}

/// A class reached through `autoload` is a file of the program like any other,
/// though nothing names its path.
#[test]
fn an_edit_to_an_autoloaded_class_reaches_the_next_request() {
    let server = Server::start(
        "autoloaded",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", &saying("Word", &["./src"])),
            ("src/Word.nvs", &class("Word", "before")),
        ],
    );
    server.awaits_body("/", "before");

    server.write("src/Word.nvs", &class("Word", "after"));
    server.awaits_body("/", "after");
}

/// A class that no file declares yet fails the program, and writing its file
/// under the `autoload` root is found without a restart. The entry file does not
/// change after the class is named, so only the path the resolution looked for
/// and missed can bring the new file to a request.
#[test]
fn a_new_class_file_under_an_autoload_root_is_found_without_a_restart() {
    let server = Server::start(
        "new-class",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", &saying("Word", &["./src"])),
            ("src/Word.nvs", &class("Word", "first")),
        ],
    );
    server.awaits_body("/", "first");

    server.write("app.nvs", &saying("Other", &["./src"]));
    server.awaits("/", "naming a class with no file failing `/`", |answer| {
        answer.status != 200
    });

    server.write("src/Other.nvs", &class("Other", "found"));
    server.awaits_body("/", "found");
}

/// A class file written under the first `autoload` root takes over from the one
/// under the second root. No file the program read has changed, so only the
/// path the resolution probed and missed can bring it to a request.
#[test]
fn a_file_that_shadows_an_autoload_probe_miss_takes_over_without_a_restart() {
    let server = Server::start(
        "shadow",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", &saying("Word", &["./src", "./vendor"])),
            ("vendor/Word.nvs", &class("Word", "vendor")),
            ("src/.keep", ""),
        ],
    );
    server.awaits_body("/", "vendor");

    server.write("src/Word.nvs", &class("Word", "src"));
    server.awaits_body("/", "src");
}

/// A module written into a directory under the root that already holds one
/// joins what `implementing` returns. Nothing names the new class and no file
/// the program read changes, so only the listing of that directory can bring
/// it to a request.
#[test]
fn a_new_module_under_a_discovery_directory_joins_implementing_without_a_restart() {
    let server = Server::start(
        "discovery",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", IMPLEMENTING),
            ("src/Said.nvs", SAID),
            ("src/One.nvs", &module("", "One", "one")),
            ("src/Plugins/Two.nvs", &module("\\Plugins", "Two", "two")),
        ],
    );
    server.awaits_body("/", "one;two;");

    server.write(
        "src/Plugins/Three.nvs",
        &module("\\Plugins", "Three", "three"),
    );
    server.awaits_body("/", "one;three;two;");
}

/// An entry file that maps `App` to `./src` and prints what `say` returns for
/// every class implementing `App\Said`, in the order `implementing` gives them.
const IMPLEMENTING: &str = "<?nvs\nautoload 'App' from './src';\nforeach (Core\\Program::implementing<App\\Said>() as App\\Said $it) {\n    echo $it->say(), \";\";\n}\n";

/// The file that declares `App\Said`.
const SAID: &str = "<?nvs\nnamespace App;\ninterface Said { public function say(): string; }\n";

/// The file that declares `App<namespace>\<name>`, implementing `App\Said` with
/// a `say` that returns `word`.
fn module(namespace: &str, name: &str, word: &str) -> String {
    format!(
        "<?nvs\nnamespace App{namespace};\nclass {name} implements App\\Said {{ public function say(): string {{ return '{word}'; }} }}\n"
    )
}

/// An entry file that maps `App` to `roots` and prints what `App\<name>::say`
/// returns.
fn saying(name: &str, roots: &[&str]) -> String {
    let roots = roots
        .iter()
        .map(|root| format!("'{root}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("<?nvs\nautoload 'App' from {roots};\nvar $it = new App\\{name}();\necho $it->say();\n")
}

/// The file that declares `App\<name>`, whose `say` returns `word`.
fn class(name: &str, word: &str) -> String {
    format!(
        "<?nvs\nnamespace App;\nclass {name} {{ public function say(): string {{ return '{word}'; }} }}\n"
    )
}

/// A required file that is deleted fails every request that reaches it, and
/// putting it back is served without a restart.
#[test]
fn a_deleted_required_file_fails_the_requests_that_reach_it() {
    let server = Server::start(
        "deleted",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", REQUIRING),
            ("lib.nvs", &printing("present")),
        ],
    );
    server.awaits_body("/", "present");

    server.remove("lib.nvs");
    let failed = server.awaits("/", "the deletion failing `/`", |answer| {
        answer.status != 200
    });
    assert!(
        !failed.body.contains("present"),
        "the program compiled before the deletion is never served in its place: {failed:?}"
    );

    server.write("lib.nvs", &printing("restored"));
    server.awaits_body("/", "restored");
}

/// A program that prints `word`, and whose every compile writes one warning to
/// standard error: its `/** … */` comment documents nothing. The warning
/// quotes the comment, so `marker` counts the compiles of this version.
fn warned(marker: &str, word: &str) -> String {
    format!(
        "<?nvs\nclass Word {{\n    /** {marker} */\n    public static function say(): string {{ return '{word}'; }}\n}}\necho Word::say();\n"
    )
}

/// Undoing the last edit is answered from the unit compiled before it, so the
/// first version's warning is not written a second time.
#[test]
fn a_reverted_edit_is_answered_from_the_unit_already_compiled() {
    let first = warned("Compiled first.", "first");
    let server = Server::start("reverted", &[("nvs.toml", PRODUCTION), ("app.nvs", &first)]);
    server.awaits_body("/", "first");

    server.write("app.nvs", &warned("Compiled second.", "second"));
    server.awaits_body("/", "second");
    server.awaits_said("Compiled second.");
    let compiled = server.said().matches("Compiled first.").count();
    assert!(
        compiled > 0,
        "the first version compiled without its warning"
    );

    server.write("app.nvs", &first);
    server.awaits_body("/", "first");

    // A third version whose warning comes after anything the revert wrote.
    server.write("app.nvs", &warned("Compiled third.", "third"));
    server.awaits_body("/", "third");
    server.awaits_said("Compiled third.");
    assert_eq!(
        server.said().matches("Compiled first.").count(),
        compiled,
        "the reverted version was compiled again: {}",
        server.said()
    );
}
