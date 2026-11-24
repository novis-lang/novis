//! `rule:security/csrf-is-on-by-default` and
//! `rule:core-classes/a-session-opens-on-its-first-use` under `nvs serve`, from
//! outside the process: a page that renders `Core\Csrf::token()` for a visitor
//! with no cookie sends the identifier the token is bound to and stores no
//! session row, and the door still accepts that token on the visitor's next
//! request.
//!
//! Every case starts the built `nvs serve` over a fresh directory with one
//! program, [`PROGRAM`], and a private data folder whose `session.sqlite` the
//! case counts. The directory lives under `CARGO_TARGET_TMPDIR` and is removed
//! when its [`Server`] is dropped.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use nvs_test::serve::{POLL, Served};

/// How long one request, or one poll, has to see its answer.
const BOUND: Duration = Duration::from_secs(30);

/// The `[http] csrf_key` every case serves under: 32 octets, each `7`, as
/// URL-safe base64 with no padding.
const KEY: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";

/// The session cookie's name, `[session] cookie`'s default.
const COOKIE: &str = "nvsid";

/// How many cookieless requests the flood sends.
const FLOOD: usize = 1_000;

/// One routed write, `POST /form`, which the door checks for a token, and a
/// page whose `do` query parameter picks what it does with the session:
/// `token` prints a token, `token-then-write` and `write-then-token` print a
/// token and write a value in that order, `write` only writes, and `read`
/// prints the value or `absent`.
const PROGRAM: &str = r#"<?nvs
class Form {
    #[Core\Route(path: "/form", method: Core\Http\Method::Post)]
    #[Core\Access(allow: Core\Audience::Public)]
    public function send(): string { return "sent"; }
}

tainted string $do = Core\Request::queryParameter("do") ?? "token";
if ($do == "token") {
    echo Core\Csrf::token();
} elseif ($do == "token-then-write") {
    string $token = Core\Csrf::token();
    Core\Session::setLongLived("seen", "yes");
    echo $token;
} elseif ($do == "write-then-token") {
    Core\Session::setLongLived("seen", "yes");
    echo Core\Csrf::token();
} elseif ($do == "write") {
    Core\Session::setLongLived("seen", "yes");
    echo "written";
} elseif ($do == "read") {
    echo (Core\Session::get("seen") ?? "absent") as string;
}
"#;

/// One answer: the status code, the header block and the body, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    /// The session identifier the answer's `Set-Cookie` headers send, or
    /// `None` without one.
    ///
    /// # Panics
    ///
    /// When two of its headers send two different identifiers.
    fn session(&self) -> Option<String> {
        let mut sent: Option<String> = None;
        for line in self.head.lines() {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            if !name.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let pair = value.trim().split(';').next().unwrap_or_default();
            let Some(id) = pair.strip_prefix(&format!("{COOKIE}=")) else {
                continue;
            };
            if let Some(earlier) = &sent {
                assert_eq!(earlier, id, "one answer sent two identifiers: {self:?}");
            }
            sent = Some(id.to_owned());
        }
        sent
    }
}

/// A running `nvs serve` over [`PROGRAM`] in its own directory, with a
/// private data folder.
///
/// Dropping it stops the process and removes both.
struct Server {
    served: Served,
    data: nvs_repo::Scratch,
}

impl Server {
    /// Writes [`PROGRAM`] and an `nvs.toml` that names [`KEY`] into a fresh
    /// directory named for `case`, and starts `nvs serve` there on a free
    /// loopback port, waiting until the program answers.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or the program does not answer `200`
    /// within [`BOUND`].
    fn start(case: &str) -> Self {
        let dir = directory(case);
        write_file(&dir.join("app.nvs"), PROGRAM);
        write_file(
            &dir.join("nvs.toml"),
            &format!("[mode]\ndefault = \"development\"\n\n[http]\ncsrf_key = \"{KEY}\"\n"),
        );
        let data = nvs_repo::scratch_private("nvsdata");
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_nvs"));
        command
            .arg("--data")
            .arg(&*data)
            .arg("serve")
            .arg("app.nvs")
            .current_dir(&dir);
        let served = Served::spawn(command.args(["--listen", "127.0.0.1:0"]), dir);
        let server = Self { served, data };
        let started = Instant::now();
        loop {
            let answer = server.send("GET", "/?do=read", &[]);
            if answer.status == 200 {
                return server;
            }
            assert!(
                started.elapsed() <= BOUND,
                "the program did not answer `200` within {BOUND:?}; the last answer was \
                 {answer:?}, and the server wrote: {}",
                server.served.said()
            );
            thread::sleep(POLL);
        }
    }

    /// One request of `method` to `path` with `headers`, on a connection of
    /// its own, with an empty body.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn send(&self, method: &str, path: &str, headers: &[(&str, &str)]) -> Answer {
        let addr = self.served.addr;
        let mut stream = TcpStream::connect(addr)
            .unwrap_or_else(|error| panic!("{addr} does not answer: {error}"));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        let mut request =
            format!("{method} {path} HTTP/1.0\r\nHost: localhost\r\nContent-Length: 0\r\n");
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
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
            head: head.to_owned(),
            body: body.to_owned(),
        }
    }

    /// A `GET` of `/?do=<what>`, sending the session `id` when it is given,
    /// which must answer `200`.
    fn page(&self, what: &str, id: Option<&str>) -> Answer {
        let cookie = id.map(|id| format!("{COOKIE}={id}"));
        let headers: Vec<(&str, &str)> = cookie
            .iter()
            .map(|pair| ("Cookie", pair.as_str()))
            .collect();
        let answer = self.send("GET", &format!("/?do={what}"), &headers);
        assert_eq!(
            answer.status,
            200,
            "`{what}` did not answer `200`: {answer:?}; the server wrote: {}",
            self.served.said()
        );
        answer
    }

    /// The status of a `POST /form` carrying `token`, sending the session
    /// `id` when it is given.
    fn post(&self, token: &str, id: Option<&str>) -> u16 {
        let cookie = id.map(|id| format!("{COOKIE}={id}"));
        let mut headers = vec![("X-CSRF-Token", token)];
        if let Some(pair) = &cookie {
            headers.push(("Cookie", pair.as_str()));
        }
        self.send("POST", "/form", &headers).status
    }

    /// Whether the session store has a row under `id`, and how many rows it
    /// has in all.
    fn rows(&self, id: &str) -> (bool, i64) {
        let path = self.data.join("session.sqlite");
        if !path.exists() {
            return (false, 0);
        }
        let db = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap_or_else(|error| panic!("`{}` does not open: {error}", path.display()));
        let all: i64 = db
            .query_row("SELECT COUNT(*) FROM nvs_session_heads", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);
        let held: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM nvs_session_heads WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        (held > 0, all)
    }

    /// The `(status, body)` of a `read` sent with the session `id`, polled
    /// until it is `wanted`, so a write-back that ends after its response is
    /// waited for.
    fn reads(&self, id: &str, wanted: &str) {
        let started = Instant::now();
        loop {
            let answer = self.page("read", Some(id));
            if answer.body == wanted {
                return;
            }
            assert!(
                started.elapsed() <= BOUND,
                "a read with `{id}` did not print `{wanted}` within {BOUND:?}: {answer:?}"
            );
            thread::sleep(POLL);
        }
    }
}

/// A fresh, empty directory for `case`, named for the case and this process so
/// no two cases share one.
fn directory(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("csrf-cookieless-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    dir
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

/// [`FLOOD`] requests with no cookie each get a token and the cookie it is
/// bound to, and leave no row in the store. A later write proves the store is
/// in use, so the count is read from the store every request reaches. A token
/// from the flood passes the door with its cookie, and is refused without it.
// covers: Core\Csrf::token
#[test]
fn a_flood_of_cookieless_token_pages_stores_no_session() {
    let server = Server::start("flood");
    let mut first = None;
    for _ in 0..FLOOD {
        let answer = server.page("token", None);
        let id = answer
            .session()
            .unwrap_or_else(|| panic!("a token for a new visitor sends its cookie: {answer:?}"));
        assert!(
            !answer.body.is_empty(),
            "the page printed no token: {answer:?}"
        );
        first.get_or_insert((id, answer.body));
    }

    let written = server.page("write", None);
    let marker = written
        .session()
        .unwrap_or_else(|| panic!("a first write sends its cookie: {written:?}"));
    server.reads(&marker, "yes");
    let (held, all) = server.rows(&marker);
    assert!(held, "the write stored no row under `{marker}`");
    assert_eq!(
        all, 1,
        "{FLOOD} cookieless token pages and one write left {all} session rows"
    );

    let (id, token) = first.expect("the flood sent at least one request");
    assert_eq!(
        server.post(&token, Some(&id)),
        200,
        "the door refused the token with its cookie"
    );
    assert_eq!(
        server.post(&token, None),
        403,
        "a token with no session passed"
    );
    assert_eq!(
        server.post(&token, Some(&marker)),
        403,
        "a token passed with another session's cookie"
    );
}

/// A token and a session write in one request name one identifier, in both
/// orders: the answer sends one identifier, the store's row is under it, and
/// the token passes the door with it.
// covers: Core\Csrf::token
#[test]
fn a_token_and_a_write_in_one_request_name_the_same_session() {
    let server = Server::start("same");
    for what in ["token-then-write", "write-then-token"] {
        let answer = server.page(what, None);
        let id = answer
            .session()
            .unwrap_or_else(|| panic!("`{what}` sends its cookie: {answer:?}"));
        server.reads(&id, "yes");
        assert!(
            server.rows(&id).0,
            "`{what}` stored no row under its cookie's `{id}`"
        );
        assert_eq!(
            server.post(&answer.body, Some(&id)),
            200,
            "`{what}`'s token was refused"
        );
    }
}

/// A visitor who comes back with the cookie a token page sent, and has no
/// row, gets a token bound to that cookie and no new cookie. Every token they
/// were shown passes with the one cookie, so a form in another tab still
/// works, and the store still has no row.
// covers: Core\Csrf::token
#[test]
fn a_returning_visitor_with_no_row_keeps_the_cookie_a_token_page_sent() {
    let server = Server::start("returning");
    let first = server.page("token", None);
    let id = first
        .session()
        .expect("a token for a new visitor sends its cookie");
    let second = server.page("token", Some(&id));
    assert_eq!(
        second.session(),
        None,
        "a second token page sent a new cookie: {second:?}"
    );
    assert_ne!(first.body, second.body, "every token has its own nonce");
    for token in [&first.body, &second.body] {
        assert_eq!(
            server.post(token, Some(&id)),
            200,
            "a token was refused with its cookie"
        );
    }
    assert_eq!(server.rows(&id), (false, 0), "a token page stored a row");
}

/// An identifier the store never issued opens a fresh session, whether a
/// token page drew it or a client made it up. The first write draws a new
/// identifier, and the presented one stays without a row. A token rendered
/// before that write is refused with the new identifier, as a form from an
/// expired session is.
// covers: Core\Csrf::token
#[test]
fn an_identifier_the_store_never_issued_opens_a_fresh_session() {
    let server = Server::start("forged");
    let shown = server.page("token", None);
    let drawn = shown
        .session()
        .expect("a token for a new visitor sends its cookie");

    for presented in [drawn.as_str(), "AAAAAAAAAAAAAAAAAAAAAA"] {
        let written = server.page("write", Some(presented));
        let fresh = written.session().unwrap_or_else(|| {
            panic!("a write over `{presented}` sends a new cookie: {written:?}")
        });
        assert_ne!(
            fresh, presented,
            "the write kept an identifier the store never issued"
        );
        server.reads(&fresh, "yes");
        assert_eq!(server.page("read", Some(presented)).body, "absent");
        assert!(!server.rows(presented).0, "`{presented}` has a row");
        if presented == drawn {
            assert_eq!(
                server.post(&shown.body, Some(&fresh)),
                403,
                "a token for the drawn identifier passed with the new one"
            );
        }
    }

    // A token rendered before the write in one request is bound to the
    // presented value, and the write draws a new identifier, so the door
    // refuses that token with the new cookie.
    let both = server.page("token-then-write", Some(&drawn));
    let fresh = both.session().expect("the write sends a new cookie");
    assert_ne!(
        fresh, drawn,
        "the write kept an identifier the store never issued"
    );
    server.reads(&fresh, "yes");
    assert_eq!(server.post(&both.body, Some(&fresh)), 403);
}
