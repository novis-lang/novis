//! `rule:http-server/a-canonical-mount-breaks-a-tie` and
//! `rule:routing/link-carries-the-mount-prefix` under `nvs serve`, from outside
//! the process: two modules, `Blog` and `Shop`, each link to a route of the
//! other through `autoload links`, and the built server picks the mount each
//! link goes through.
//!
//! The table has three blocks. A `scan` mounts every module at
//! `/base/{1:lower}` with an origin, a second block mounts `Shop`'s entry again
//! at `/shop.second`, and a third mounts it on the host `shop.example.com`.
//! Both modules share one entry file byte for byte, and their routes live
//! under `src/`, where the other module reads them for links only.
//!
//! The harness is `per_mount_config.rs`'s, with the request's `Host` header
//! chosen per request. The program lives under `CARGO_TARGET_TMPDIR`, inside
//! the build directory, and is removed when its [`Server`] is dropped.

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

/// The configuration: the three blocks the module docs name.
const CONFIG: &str = r#"[mode]
default = "development"

[server]
root = "."

[[server.mount]]
scan = "*/public/index.nvs"
prefix = "/base/{1:lower}"
origin = "https://www.example.com"

[[server.mount]]
prefix = "/shop.second"
entry = "Shop/public/index.nvs"

[[server.mount]]
host = "shop.example.com"
entry = "Shop/public/index.nvs"
origin = "https://shop.example.com"
"#;

/// The entry both modules share: its own root, every module's root for links
/// only, and a gate that lets every request through and prints what failed.
const ENTRY: &str = r"<?nvs
autoload 'Framework\{..}' from '../src';
autoload links discover 'Framework' from '../../*/src';

class Open implements Core\Router\Gate
{
    public function allows(Core\Router\Match $route): bool
    {
        return true;
    }

    public function refused(Core\Router\Refusal $reason, ?Core\Router\Match $route): void
    {
        echo 'refused';
    }

    public function failed(?Core\Router\Match $route, ?Throwable $error): void
    {
        echo 'failed: ', $error?->message ?? 'no error';
    }
}

Core\Router::dispatch(new Open());
";

/// `Blog`'s one route. It prints its own link and its link to `Shop`'s route.
const POST: &str = r"<?nvs
namespace Framework\Blog\View;

use Framework\Shop\View\Index;

class Post
{
    #[Core\Route(path: '/post/{id}', method: Core\Http\Method::Get)]
    #[Core\Access(allow: Core\Audience::Public)]
    public static function show(int $id): void
    {
        echo Core\Router::url(Post::show, ['id' => $id]), ' ', Core\Router::url(Index::show, ['id' => 3]);
    }
}
";

/// `Shop`'s one route. It prints its own link and its link to `Blog`'s route.
const INDEX: &str = r"<?nvs
namespace Framework\Shop\View;

use Framework\Blog\View\Post;

class Index
{
    #[Core\Route(path: '/item/{id}', method: Core\Http\Method::Get)]
    #[Core\Access(allow: Core\Audience::Public)]
    public static function show(int $id): void
    {
        echo Core\Router::url(Index::show, ['id' => $id]), ' ', Core\Router::url(Post::show, ['id' => 5]);
    }
}
";

/// A running `nvs serve` over the two modules in their own directory.
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
    /// Writes the two modules and [`CONFIG`] into a fresh directory named for
    /// `case`, and starts `nvs serve` there on a free loopback port.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("links-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in [
            ("nvs.toml", CONFIG),
            ("Blog/public/index.nvs", ENTRY),
            ("Shop/public/index.nvs", ENTRY),
            ("Blog/src/View/Post.nvs", POST),
            ("Shop/src/View/Index.nvs", INDEX),
        ] {
            write_file(&dir.join(path), text);
        }

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

    /// One `GET` of `path` with `host` as its `Host` header, on a connection
    /// of its own. Returns the status and the body.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn get(&self, host: &str, path: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(stream, "GET {path} HTTP/1.0\r\nHost: {host}\r\n\r\n").expect("the request is sent");
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
        (status, body.to_owned())
    }

    /// Polls `path` on `host` until it answers `200`, and returns its body.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first. The message names the last answer and
    /// what the server wrote to standard error.
    fn body(&self, host: &str, path: &str) -> String {
        let started = Instant::now();
        loop {
            let (status, body) = self.get(host, path);
            if status == 200 {
                return body;
            }
            if started.elapsed() > BOUND {
                let said = self.stderr.lock().expect("no reader panicked").clone();
                panic!(
                    "`{path}` on `{host}` did not answer `200` within {BOUND:?}; the last \
                     answer was {status} {body:?}, and the server wrote: {said}"
                );
            }
            thread::sleep(POLL);
        }
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

/// The first step: both modules are rows of one `scan`, so each links to the
/// other through the shared `/base` prefix. Neither takes `Shop`'s second
/// mount or its host mount.
#[test]
fn two_scanned_modules_link_through_the_shared_prefix() {
    let server = Server::start("scan");
    assert_eq!(
        server.body("localhost", "/base/blog/post/5"),
        "/base/blog/post/5 /base/shop/item/3",
        "`Blog` did not link to `Shop` inside the scan"
    );
    assert_eq!(
        server.body("localhost", "/base/shop/item/3"),
        "/base/shop/item/3 /base/blog/post/5",
        "`Shop` did not link to `Blog` inside the scan"
    );
}

/// A second mount of `Shop`'s entry links to its own routes under its own
/// prefix. Its link to `Blog` takes the one row on the same host.
#[test]
fn a_link_inside_a_second_mount_of_one_entry_stays_in_that_mount() {
    let server = Server::start("second");
    assert_eq!(
        server.body("localhost", "/shop.second/item/7"),
        "/shop.second/item/7 /base/blog/post/5",
        "the second mount's own link left it, or its link to `Blog` missed"
    );
}

/// The third step: under its host mount, `Shop` has no `Blog` row in its
/// block or on its host, so its link to `Blog` is absolute with the origin of
/// the row that serves `Blog`. Its own link stays a path.
#[test]
fn a_link_to_a_module_on_another_host_carries_its_origin() {
    let server = Server::start("host");
    assert_eq!(
        server.body("shop.example.com", "/item/3"),
        "/item/3 https://www.example.com/base/blog/post/5",
        "the link to `Blog` on another host did not carry its origin"
    );
}
