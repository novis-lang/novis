//! `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`, from outside the
//! process: every `http.*` key a request may set through `Core\Config::set` changes that request's
//! own response, and `set` returns `false` for every other one
//! (`rule:config/three-changeability-classes`).
//!
//! The keys come from [`nvs_config::request::keys`], the roster `set` itself reads, so a key added
//! to the configuration is walked here with no edit to this file. A key `set` accepts must have a
//! row in [`response_probe`] saying what it changes on the response, or be one of
//! `[http.client]`'s outbound-call keys, whose readers ask `Core\Config::get`. A key with neither
//! fails the case by name.
//!
//! One `nvs serve` answers every probe: the program sets the key a query number picks, adds one
//! cookie and prints what `set` and then `Core\Config::get` returned. Each probe's check is asked
//! of the answer to a request that set nothing too, and must not hold there, so a check the file
//! already satisfies cannot pass.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// How long one answer may take.
const ANSWER: Duration = Duration::from_secs(30);

/// The file's `[http]` block: one CORS origin, and the loopback peer trusted to say the scheme is
/// `https`, so the file's answer carries HSTS and a CORS grant for the probes to take away.
const CONFIG: &str = "[mode]\ndefault = \"production\"\n\n\
                      [server]\ntrusted_proxies = [\"127.0.0.1/32\"]\n\n\
                      [http.cors]\norigins = [\"https://b.example\"]\n";

/// What every probe sends: the origin the file grants, and the client and scheme the trusted peer
/// says.
const SENT: [(&str, &str); 3] = [
    ("Origin", "https://b.example"),
    ("X-Forwarded-For", "203.0.113.9"),
    ("X-Forwarded-Proto", "https"),
];

/// One answer: the header lines and the body, as text.
#[derive(Debug)]
struct Answer {
    head: String,
    body: String,
}

impl Answer {
    /// The value of the header `name`, compared without regard to case.
    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
    }

    /// Whether the header `name` is present and contains `part`.
    fn has(&self, name: &str, part: &str) -> bool {
        self.header(name).is_some_and(|value| value.contains(part))
    }
}

/// What holds of a response once a probe's setting is in force.
type Check = fn(&Answer) -> bool;

/// The value a probe sets for `key`, and what then holds of the response, or `None` for a key no
/// row here covers.
fn response_probe(key: &str) -> Option<(&'static str, Check)> {
    let probe: (&'static str, Check) = match key {
        "http.headers.content_type_options" => ("false", |a| {
            a.header("x-content-type-options").is_none()
        }),
        "http.headers.frame_ancestors" => ("https://c.example", |a| {
            a.has("content-security-policy", "frame-ancestors https://c.example")
        }),
        "http.headers.referrer_policy" => ("no-referrer", |a| {
            a.header("referrer-policy") == Some("no-referrer")
        }),
        "http.headers.hsts" => ("false", |a| a.header("strict-transport-security").is_none()),
        "http.headers.hsts_subdomains" => ("true", |a| {
            a.has("strict-transport-security", "includeSubDomains")
        }),
        "http.headers.content_security_policy" => ("default-src https:", |a| {
            a.has("content-security-policy", "default-src https:")
        }),
        "http.headers.permissions_policy" => ("usb=(self)", |a| {
            a.has("permissions-policy", "usb=(self)")
        }),
        "http.cors.origins" => ("https://a.example", |a| {
            a.header("access-control-allow-origin").is_none()
        }),
        "http.cors.credentials" => ("true", |a| {
            a.header("access-control-allow-credentials") == Some("true")
        }),
        "http.cors.expose" => ("X-Probe", |a| {
            a.has("access-control-expose-headers", "X-Probe")
        }),
        "http.cookies.secure" => ("false", |a| {
            a.header("set-cookie").is_some_and(|c| !c.contains("Secure"))
        }),
        "http.cookies.http_only" => ("false", |a| {
            a.header("set-cookie").is_some_and(|c| !c.contains("HttpOnly"))
        }),
        "http.cookies.same_site" => ("Strict", |a| a.has("set-cookie", "SameSite=Strict")),
        "http.cookies.path" => ("/app", |a| a.has("set-cookie", "Path=/app")),
        _ => return None,
    };
    Some(probe)
}

/// The value a probe sets for an outbound-call key under `[http.client]`. Its readers ask
/// `Core\Config::get`, so the probe checks that `get` returns it.
fn outbound_value(key: &str) -> &'static str {
    match key.rsplit('.').next() {
        Some("max_redirects") => "2",
        Some("max_message") => "1M",
        _ => "2s",
    }
}

/// What one probe sets, and what is asked of its answer.
enum Expect {
    /// `set` returns `true`, and the check holds of the response.
    Response(Check),
    /// `set` returns `true`, and `get` returns the value.
    Outbound,
    /// `set` returns `false`.
    Refused,
}

/// A running `nvs serve app.nvs` in a directory of its own. Dropping it stops the process and
/// removes the directory.
struct Server {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    _data: nvs_repo::Scratch,
}

impl Server {
    /// Writes `program` and [`CONFIG`] into a fresh directory and serves it on a free loopback
    /// port.
    fn start(program: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("request-http-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        std::fs::write(dir.join("app.nvs"), program).expect("the program is written");
        std::fs::write(dir.join("nvs.toml"), CONFIG).expect("the configuration is written");
        let data = nvs_repo::scratch_private("nvsdata");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");

        let err_pipe = child.stderr.take().expect("standard error is piped");
        let (said, heard) = mpsc::channel();
        thread::spawn(move || {
            let all: Vec<String> = BufReader::new(err_pipe)
                .lines()
                .map_while(Result::ok)
                .collect();
            let _ = said.send(all.join("\n"));
        });
        let out_pipe = child.stdout.take().expect("standard output is piped");
        let (sent, bound) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out_pipe).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let _ = sent.send(rest.split_whitespace().next().unwrap_or_default().to_owned());
                }
            }
        });
        let Ok(addr) = bound.recv_timeout(BOOT) else {
            let _ = child.kill();
            let _ = child.wait();
            let said = heard.recv_timeout(ANSWER).unwrap_or_default();
            panic!("`nvs serve` did not start listening within {BOOT:?}: {said}");
        };
        Self {
            child,
            addr: addr
                .parse()
                .unwrap_or_else(|error| panic!("`{addr}` is not an address: {error}")),
            dir,
            _data: data,
        }
    }

    /// One `GET` of `path` sending [`SENT`], on a connection of its own.
    fn get(&self, path: &str) -> Answer {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(ANSWER))
            .expect("a read timeout is set");
        let extra: String = SENT
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();
        write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n{extra}\r\n")
            .expect("the request is sent");
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .unwrap_or_else(|error| panic!("the answer to `{path}` was cut off: {error}"));
        let text = String::from_utf8_lossy(&raw);
        let (head, body) = text
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("the answer to `{path}` has no header block: {text}"));
        Answer {
            head: head.to_owned(),
            body: body.to_owned(),
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

/// `text` as a single-quoted Novis string.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// The program: for `?n=<index>` it sets that probe's key, then it adds one cookie and prints
/// what `set` returned and, for an outbound key, what `get` returns, separated by `|`.
fn program(probes: &[(String, &'static str, Expect)]) -> String {
    let mut text = String::from(
        "<?nvs\nvar $n = Core\\Request::query('n') ?? '';\nvar $ok = 'none';\nvar $got = '';\n",
    );
    for (index, (key, value, expect)) in probes.iter().enumerate() {
        let (key, value) = (quoted(key), quoted(value));
        // Only an outbound key reads its value back. A secret key's `get` has another type.
        let got = if matches!(expect, Expect::Outbound) {
            format!("    $got = Core\\Config::get({key}) ?? '';\n")
        } else {
            String::new()
        };
        text.push_str(&format!(
            "if ($n == '{index}') {{\n    $ok = Core\\Config::set({key}, {value}) ? 'true' : 'false';\n{got}}}\n"
        ));
    }
    text.push_str("Core\\Response::addCookie('probe', '1');\necho $ok, '|', $got;\n");
    text
}

#[test]
fn every_http_setting_a_request_may_set_reaches_its_response() {
    let probes: Vec<(String, &'static str, Expect)> = nvs_config::request::keys()
        .iter()
        .filter(|key| key.starts_with("http."))
        .map(|key| {
            let row = nvs_config::directive::lookup(key)
                .unwrap_or_else(|| panic!("`{key}` is a key no directive row governs"));
            let (value, expect) = if let Some((value, check)) = response_probe(key) {
                (value, Expect::Response(check))
            } else if key.starts_with("http.client.") && row.class.settable_by_a_request() {
                (outbound_value(key), Expect::Outbound)
            } else {
                ("x", Expect::Refused)
            };
            (key.clone(), value, expect)
        })
        .collect();
    assert!(
        probes.len() > 20,
        "the key roster names only {} `http.*` keys",
        probes.len()
    );

    let server = Server::start(&program(&probes));
    let plain = server.get("/?n=none");
    assert!(
        plain.body.starts_with("none|"),
        "the program did not run: {plain:?}"
    );

    let mut wrong = Vec::new();
    for (index, (key, value, expect)) in probes.iter().enumerate() {
        let answer = server.get(&format!("/?n={index}"));
        let (set, got) = answer.body.split_once('|').unwrap_or((&answer.body, ""));
        match expect {
            Expect::Response(check) => {
                if set != "true" {
                    wrong.push(format!("`{key}` = `{value}`: `set` returned `{set}`"));
                } else if check(&plain) {
                    wrong.push(format!(
                        "`{key}`: the check already holds with nothing set: {plain:?}"
                    ));
                } else if !check(&answer) {
                    wrong.push(format!(
                        "`{key}` = `{value}`: `set` accepted it and the response ignores it: \
                         {answer:?}"
                    ));
                }
            }
            Expect::Outbound => {
                if set != "true" || got != *value {
                    wrong.push(format!(
                        "`{key}` = `{value}`: `set` returned `{set}` and `get` `{got}`"
                    ));
                }
            }
            Expect::Refused => {
                if set != "false" {
                    wrong.push(format!(
                        "`{key}`: `set` returned `{set}`, and no probe shows the response \
                         reading it"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
