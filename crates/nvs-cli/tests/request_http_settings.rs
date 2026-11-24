//! `rule:config/set-changes-only-listed-keys`, from outside the process: every key a request may
//! set through `Core\Config::set` changes what that request does, and `set` returns `false` for
//! every `http.*` key `nvs_config::settable::SETTABLE` does not list.
//!
//! [`every_http_setting_a_request_may_set_reaches_its_response`] walks the `http.*` keys of
//! [`nvs_config::request::keys`], the roster `set` itself reads, so a key added to the
//! configuration is walked with no edit to this file. A key [`response_probe`] has a row for must
//! change the response as the row says, an `[http.client]` key `set` accepts must read back through
//! `Core\Config::get`, and `set` must return `false` for every other key.
//!
//! [`every_settable_key_changes_what_a_request_does`] walks [`nvs_config::settable::SETTABLE`]
//! and runs each row's [`Probe`] twice, once with the set and once without. The probe's check must
//! hold of the first run and must not hold of the second, so a check the file already satisfies
//! cannot pass, and a row with no probe fails the case by name. A check reads the answer, the
//! server's standard error since the request was sent, or the debug spool under the data folder.
//! Outbound calls go to [`peer`], a server in this process that answers slowly, redirects, drips
//! its body, speaks WebSocket, or is the proxy that calls to `localhost` go through.
//!
//! Each test starts one `nvs serve`. A probe that waits for a log line or a spool record waits on
//! that event. Every timing probe sets a bound of 100 to 300 ms against work or a peer that needs
//! a second or more, so a loaded runner moves neither side of the check.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use nvs_test::serve::{POLL, Served};

/// How long one answer, or one event a probe waits for, may take.
const ANSWER: Duration = Duration::from_secs(30);

/// How long the peer keeps a slow answer back: over six times every bound a probe sets.
const SLOW: Duration = Duration::from_millis(2000);

/// The file: production with a development ceiling, so a request may flip its mode up; the debug
/// spool on, so a request has a recording to turn off; loopback reachable, for [`peer`]; the
/// loopback peer trusted to say the scheme is `https`; and one CORS origin, so the file's answer
/// carries HSTS and a CORS grant for the probes to take away. The second test adds
/// [`proxied`]'s block.
const CONFIG: &str = "[mode]\ndefault = \"production\"\nceiling = \"development\"\n\n\
                      [debug]\nstore = true\n\n\
                      [capabilities.net]\nconnect = [\"127.0.0.1\", \"localhost\"]\n\
                      internal = [\"127.0.0.1\", \"::1\"]\n\n\
                      [server]\ntrusted_proxies = [\"127.0.0.1/32\"]\n\n\
                      [http.cors]\norigins = [\"https://b.example\"]\n";

/// The `[http.client.proxy]` block that sends every outbound call to `localhost` through `peer`,
/// and every call to `127.0.0.1` directly.
fn proxied(peer: SocketAddr) -> String {
    format!(
        "{CONFIG}\n[http.client.proxy]\nurl = \"http://{peer}\"\nresolve = \"local\"\n\
         bypass = [\"127.0.0.1\"]\n"
    )
}

/// What every request sends: the origin the file grants, and the client and scheme the trusted
/// peer says. A request with a body leaves the origin out, as a page's own form does.
const SENT: [(&str, &str); 3] = [
    ("Origin", "https://b.example"),
    ("X-Forwarded-For", "203.0.113.9"),
    ("X-Forwarded-Proto", "https"),
];

/// One answer: the status line and header lines, and the body, as text.
#[derive(Debug)]
struct Answer {
    head: String,
    body: String,
}

impl Answer {
    /// The status code of the first line.
    fn status(&self) -> u16 {
        self.head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0)
    }

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
        "http.headers.content_type_options" => {
            ("false", |a| a.header("x-content-type-options").is_none())
        }
        "http.headers.frame_ancestors" => ("https://c.example", |a| {
            a.has(
                "content-security-policy",
                "frame-ancestors https://c.example",
            )
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
        "http.headers.permissions_policy" => {
            ("usb=(self)", |a| a.has("permissions-policy", "usb=(self)"))
        }
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
            a.header("set-cookie")
                .is_some_and(|c| !c.contains("Secure"))
        }),
        "http.cookies.http_only" => ("false", |a| {
            a.header("set-cookie")
                .is_some_and(|c| !c.contains("HttpOnly"))
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
    served: Served,
    data: nvs_repo::Scratch,
}

impl std::ops::Deref for Server {
    type Target = Served;

    fn deref(&self) -> &Served {
        &self.served
    }
}

/// What a request sends after its header block.
#[derive(Clone, Copy)]
enum Body {
    /// Nothing: a `GET`.
    None,
    /// A `POST` of this many bytes of `text/plain`.
    Text(usize),
    /// A `POST` of a multipart form with one file part of each of these sizes.
    Files(&'static [usize]),
}

/// The boundary of every multipart body this file sends.
const BOUNDARY: &str = "probe-boundary";

impl Body {
    /// The method, the content type and the bytes.
    fn encoded(self) -> (&'static str, Option<&'static str>, Vec<u8>) {
        match self {
            Body::None => ("GET", None, Vec::new()),
            Body::Text(size) => ("POST", Some("text/plain"), vec![b'x'; size]),
            Body::Files(sizes) => {
                let mut bytes = Vec::new();
                for (index, size) in sizes.iter().enumerate() {
                    bytes.extend_from_slice(
                        format!(
                            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"f{index}\"; \
                             filename=\"f{index}.txt\"\r\nContent-Type: text/plain\r\n\r\n"
                        )
                        .as_bytes(),
                    );
                    bytes.extend(std::iter::repeat_n(b'x', *size));
                    bytes.extend_from_slice(b"\r\n");
                }
                bytes.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
                (
                    "POST",
                    Some("multipart/form-data; boundary=probe-boundary"),
                    bytes,
                )
            }
        }
    }
}

impl Server {
    /// Writes `program`, `files` and `config` into a fresh directory named after `name` and
    /// serves it on a free loopback port.
    fn start(name: &str, config: &str, program: &str, files: &[(&str, &str)]) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("request-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        std::fs::write(dir.join("app.nvs"), program).expect("the program is written");
        std::fs::write(dir.join("nvs.toml"), config).expect("the configuration is written");
        for (file, text) in files {
            std::fs::write(dir.join(file), text).expect("a script beside the program is written");
        }
        let data = nvs_repo::scratch_private("nvsdata");
        let served = Served::spawn(
            Command::new(env!("CARGO_BIN_EXE_nvs"))
                .arg("--data")
                .arg(&*data)
                .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
                .current_dir(&dir),
            dir,
        );
        Self { served, data }
    }

    /// One `GET` of `path` sending [`SENT`], on a connection of its own.
    fn get(&self, path: &str) -> Answer {
        self.send(path, Body::None)
    }

    /// One request for `path` sending [`SENT`] and `body`, on a connection of its own.
    fn send(&self, path: &str, body: Body) -> Answer {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(ANSWER))
            .expect("a read timeout is set");
        let (method, kind, bytes) = body.encoded();
        // A body is sent from this origin, as a form of its own page is.
        let mut extra: String = SENT
            .iter()
            .filter(|(name, _)| kind.is_none() || *name != "Origin")
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();
        if let Some(kind) = kind {
            extra.push_str(&format!(
                "Content-Type: {kind}\r\nContent-Length: {}\r\n",
                bytes.len()
            ));
        }
        write!(
            stream,
            "{method} {path} HTTP/1.0\r\nHost: localhost\r\n{extra}\r\n"
        )
        .expect("the request is sent");
        stream.write_all(&bytes).expect("the body is sent");
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

    /// Every file under the debug spool, as text.
    fn spool(&self) -> String {
        let mut text = String::new();
        let mut dirs = vec![self.data.join("debug")];
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    text.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
        }
        text
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
        "<?nvs\nvar $n = Core\\Request::queryParameter('n') ?? '';\nvar $ok = 'none';\nvar $got = '';\n\
         var $key = '';\n",
    );
    for (index, (key, value, expect)) in probes.iter().enumerate() {
        let (key, value) = (quoted(key), quoted(value));
        // Only an outbound key reads its value back. A secret key's `get` has another type.
        let got = if matches!(expect, Expect::Outbound) {
            format!("    $got = Core\\Config::get({key}) ?? '';\n")
        } else {
            String::new()
        };
        // A written name no row lists is E0667, so a refused key goes through a variable and
        // reaches the run-time `false` this test pins.
        let (bind, name) = if matches!(expect, Expect::Refused) {
            (format!("    $key = {key};\n"), "$key".to_owned())
        } else {
            (String::new(), key)
        };
        text.push_str(&format!(
            "if ($n == '{index}') {{\n{bind}    $ok = Core\\Config::set({name}, {value}) ? 'true' : 'false';\n{got}}}\n"
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
            let (value, expect) = if let Some((value, check)) = response_probe(key) {
                (value, Expect::Response(check))
            } else if key.starts_with("http.client.") && nvs_config::settable::how(key).is_some() {
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

    let server = Server::start("http", CONFIG, &program(&probes), &[]);
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

/// What one run of a probe left behind.
#[derive(Debug)]
struct Outcome {
    /// The run's own tag, `tag-<index>-<1 or 0>`, which its program writes where it writes.
    tag: String,
    answer: Answer,
    /// What the server wrote to standard error from the moment the request was sent.
    log: String,
    /// Every file under the debug spool, read once the run settled.
    spool: String,
}

impl std::fmt::Display for Outcome {
    /// The run as a failure message shows it: the head, the body and the log, each cut to its
    /// last 1500 characters.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let last = |text: &str| -> String {
            let skip = text.chars().count().saturating_sub(1500);
            text.chars().skip(skip).collect()
        };
        write!(
            f,
            "{}\n--- head\n{}\n--- body\n{}\n--- log\n{}",
            self.tag,
            self.answer.head,
            last(&self.answer.body),
            last(&self.log)
        )
    }
}

/// What must hold of a run once the probe's setting is in force.
enum Judge {
    /// A check of the answer alone.
    Answer(Check),
    /// A check of everything the run left behind.
    Outcome(fn(&Outcome) -> bool),
}

/// What a run waits for once it is answered, before it is judged. `{tag}` is the run's tag.
#[derive(Clone, Copy)]
enum Settle {
    /// Nothing: the answer is the whole outcome.
    Answered,
    /// Any of these texts in what the server wrote since the request was sent.
    Log(&'static [&'static str]),
    /// This text in the debug spool.
    Spool(&'static str),
}

/// One row's proof that setting it changes what the request does.
struct Probe {
    value: &'static str,
    /// Novis that runs before the set, in both runs.
    before: &'static str,
    /// Novis that runs after the set, in both runs. `$tag` is the run's tag, and `{peer}` the
    /// address of [`peer`].
    work: &'static str,
    body: Body,
    settle: Settle,
    judge: Judge,
}

impl Probe {
    /// A probe that sets `value`, runs `work`, and judges the outcome with `judge`.
    fn new(value: &'static str, work: &'static str, judge: fn(&Outcome) -> bool) -> Self {
        Self {
            value,
            before: "",
            work,
            body: Body::None,
            settle: Settle::Answered,
            judge: Judge::Outcome(judge),
        }
    }

    /// The same probe, with `before` run ahead of the set.
    fn before(mut self, before: &'static str) -> Self {
        self.before = before;
        self
    }

    /// The same probe, sending `body`.
    fn sending(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    /// The same probe, waiting for `settle` before it is judged.
    fn settled(mut self, settle: Settle) -> Self {
        self.settle = settle;
        self
    }
}

/// The work of a probe whose limit stops the request: the answer is a `500` with no body, where
/// without the limit the work ends and prints `work-done`.
fn stopped(outcome: &Outcome) -> bool {
    outcome.answer.status() == 500 && !outcome.answer.body.contains("work-done")
}

/// Whether the body has `text` in it.
fn printed(outcome: &Outcome, text: &str) -> bool {
    outcome.answer.body.contains(text)
}

/// A multipart reader's work: read every file part, then print `parts-read`.
const READ_FILES: &str = "    try {\n\
        foreach (Core\\Request::files() as Core\\Request\\Part $part) {\n\
            tainted bytes $content = $part->readAll();\n\
        }\n\
        echo 'parts-read';\n\
    } catch (Throwable $refused) {\n\
        echo 'parts-refused';\n\
    }\n";

/// The probe for the `SETTABLE` row `key`, or `None` for a row no probe covers yet.
fn probe(key: &str) -> Option<Probe> {
    if let Some((value, check)) = response_probe(key) {
        return Some(Probe {
            value,
            before: "",
            work: "    Core\\Response::addCookie('probe', '1');\n",
            body: Body::None,
            settle: Settle::Answered,
            judge: Judge::Answer(check),
        });
    }
    let probe = match key {
        // The mode the program reads is the one it set.
        "mode.default" => Probe::new(
            "development",
            "    echo match (Core\\Env::mode()) {\n\
                    Core\\Env::Mode::Production => 'mode=production',\n\
                    Core\\Env::Mode::Development => 'mode=development',\n\
                };\n",
            |o| printed(o, "mode=development"),
        ),
        // 32 MiB held is over 8M and under the file's 128M.
        "limits.memory" => Probe::new(
            "8M",
            "    string $big = Core\\Str::repeat('x', 33554432);\n\
                 echo Core\\Str::length($big) > 0 ? 'work-done' : 'empty';\n",
            stopped,
        ),
        // A loop that spins for over a second of CPU.
        "limits.cpu_time" => Probe::new(
            "100ms",
            "    var $start = Core\\Time::monotonic();\n\
                 int $spins = 0;\n\
                 while (Core\\Time::monotonic()->minus($start)->toMilliseconds() < 1500) {\n\
                     $spins = $spins + 1;\n\
                 }\n\
                 echo 'work-done';\n",
            stopped,
        ),
        // A sleep of a second.
        "limits.wall_time" => Probe::new(
            "100ms",
            "    Core\\Time::sleep(1s);\n    echo 'work-done';\n",
            stopped,
        ),
        // A thousand bytes written, a line at a time, past a limit of a hundred.
        "limits.max_output" => Probe::new(
            "100",
            "    int $lines = 0;\n\
                 while ($lines < 20) {\n\
                     echo Core\\Str::repeat('x', 49), \"\\n\";\n\
                     $lines = $lines + 1;\n\
                 }\n\
                 echo 'work-done';\n",
            stopped,
        ),
        // A body of 4 KiB read under a limit of 1 KiB.
        "limits.request_body" => Probe::new(
            "1K",
            "    try {\n\
                     var $read = Core\\Request::body();\n\
                     echo Core\\Str::length($read) > 0 ? 'body-read' : 'body-empty';\n\
                 } catch (Throwable $refused) {\n\
                     echo 'body-refused';\n\
                 }\n",
            |o| !printed(o, "body-read"),
        )
        .sending(Body::Text(4096)),
        // An upload of 4 KiB under a total of 1 KiB.
        "limits.upload_total" => Probe::new("1K", READ_FILES, |o| !printed(o, "parts-read"))
            .sending(Body::Files(&[4096])),
        // A form of three parts under a limit of one.
        "limits.max_multipart_parts" => Probe::new("1", READ_FILES, |o| !printed(o, "parts-read"))
            .sending(Body::Files(&[8, 8, 8])),
        // A second child script while the first still runs.
        "limits.max_tasks" => Probe::new(
            "1",
            "    var $first = spawn script \"child.nvs\" with(output: \"capture\");\n\
                 var $second = spawn script \"child.nvs\" with(output: \"capture\");\n\
                 echo (await $first)->output, (await $second)->output, 'work-done';\n",
            stopped,
        ),
        // A backreference over a subject of two hundred characters.
        "limits.max_regex_steps" => Probe::new(
            "10",
            "    try {\n\
                     bool $found = Core\\Regex::matches(Core\\Str::repeat('x', 200) . 'aa', '(a)\\\\1');\n\
                     echo $found ? 'regex-matched' : 'regex-missed';\n\
                 } catch (RuntimeError $stopped) {\n\
                     echo 'regex-stopped';\n\
                 }\n",
            |o| printed(o, "regex-stopped"),
        ),
        // The run's error record is written as text, not JSON.
        "log.format" => Probe::new(
            "text",
            "    Core\\Log::write(Core\\Log::Level::Error, $tag);\n",
            |o| {
                o.log
                    .lines()
                    .any(|line| line.contains(&o.tag) && line.starts_with("[error]"))
            },
        )
        .settled(Settle::Log(&["{tag}"])),
        // A debug record is written ahead of the run's error record.
        "log.level" => Probe::new(
            "debug",
            "    Core\\Log::write(Core\\Log::Level::Debug, 'debug-' . $tag);\n\
                 Core\\Log::write(Core\\Log::Level::Error, $tag);\n",
            |o| o.log.contains(&format!("debug-{}", o.tag)),
        )
        .settled(Settle::Log(&["{tag}"])),
        // A call of a deprecated method throws.
        "errors.deprecated" => Probe::new(
            "throw",
            "    try {\n\
                     echo (new Api())->old();\n\
                 } catch (Core\\DeprecatedError $thrown) {\n\
                     echo 'deprecated-threw';\n\
                 }\n",
            |o| printed(o, "deprecated-threw"),
        ),
        // Work after the response that sleeps a second is stopped at the deadline.
        "deferred.deadline" => Probe::new(
            "100ms",
            "    string $done = 'deferred-done-' . $tag;\n\
                 Core\\Task::runAfterResponse(fn(): void => {\n\
                     Core\\Time::sleep(1s);\n\
                     Core\\Log::write(Core\\Log::Level::Error, $done);\n\
                 });\n\
                 echo 'work-done';\n",
            |o| o.log.contains("its deadline expired") && !o.log.contains("deferred-done-"),
        )
        .settled(Settle::Log(&[
            "deferred-done-{tag}",
            "its deadline expired",
        ])),
        // A dump in development appends no block to the page.
        "debug.inline" => Probe::new(
            "false",
            "    Core\\Debug::dump($tag);\n    echo 'work-done';\n",
            |o| printed(o, "work-done") && !printed(o, &o.tag),
        )
        .before("    Core\\Config::set('mode.default', 'development');\n"),
        // A dump writes no spool record; the one after `restore` does.
        "debug.store" => Probe::new(
            "false",
            "    Core\\Debug::dump($tag);\n\
                 Core\\Config::restore('debug.store');\n\
                 Core\\Debug::dump('after-' . $tag);\n",
            |o| !o.spool.contains(&format!("\"{}\"", o.tag)),
        )
        .settled(Settle::Spool("after-{tag}")),
        // A proxy that opens its tunnel after two seconds is given up on.
        "http.client.connect_timeout" => Probe::new(
            "100ms",
            "    try {\n\
                     Core\\Http\\Response $reply = Core\\Http\\Client::get('http://localhost:{port}/hop/0');\n\
                     echo 'status=', $reply->status();\n\
                 } catch (Throwable $failed) {\n\
                     echo 'call-failed';\n\
                 }\n",
            |o| printed(o, "call-failed"),
        ),
        // A call to a peer that answers after two seconds times out.
        "http.client.deadline" => Probe::new(
            "200ms",
            "    try {\n\
                     Core\\Http\\Response $reply = Core\\Http\\Client::get('http://{peer}/slow');\n\
                     echo 'status=', $reply->status();\n\
                 } catch (Throwable $failed) {\n\
                     echo 'call-failed';\n\
                 }\n",
            |o| printed(o, "call-failed"),
        ),
        // Two redirects are followed to the page that answers `200`.
        "http.client.max_redirects" => Probe::new(
            "3",
            "    Core\\Http\\Response $reply = Core\\Http\\Client::get('http://{peer}/hop/2');\n\
                 echo 'status=', $reply->status();\n",
            |o| printed(o, "status=200"),
        ),
        // A body that pauses for two seconds midway stops at the pause.
        "http.client.idle" => Probe::new("200ms", STREAM_PAUSE, |o| printed(o, "stream-failed")),
        // A body that drips for two seconds stops partway.
        "http.client.max_duration" => {
            Probe::new("300ms", STREAM_DRIP, |o| printed(o, "stream-failed"))
        }
        // A message of 64 KiB is over a cap of 1 KiB.
        "http.client.socket.max_message" => Probe::new(
            "1K",
            "    try {\n\
                     Core\\Http\\Socket $socket = Core\\Http\\Client::openSocket('ws://{peer}/ws-big');\n\
                     var $message = $socket->receive();\n\
                     echo $message == null ? 'socket-empty' : 'socket-got';\n\
                 } catch (Throwable $failed) {\n\
                     echo 'socket-refused';\n\
                 }\n",
            |o| printed(o, "socket-refused"),
        ),
        // `close` waits for the peer's close frame no longer than the send wait.
        "http.client.socket.send_timeout" => Probe::new(
            "100ms",
            "    Core\\Http\\Socket $socket = Core\\Http\\Client::openSocket('ws://{peer}/ws-late');\n\
                 var $start = Core\\Time::monotonic();\n\
                 $socket->close();\n\
                 echo Core\\Time::monotonic()->minus($start)->toMilliseconds() < 1000 ? 'close-quick' : 'close-waited';\n",
            |o| printed(o, "close-quick"),
        ),
        _ => return None,
    };
    Some(probe)
}

/// Streams the peer's pausing body, and prints `stream-done` once all of it arrived.
const STREAM_PAUSE: &str = "    try {\n\
        Core\\Http\\Stream $feed = Core\\Http\\Client::stream(Core\\Http\\Method::Get, 'http://{peer}/pause');\n\
        foreach ($feed->chunks() as tainted bytes $piece) {\n\
            echo '';\n\
        }\n\
        echo 'stream-done';\n\
    } catch (Throwable $failed) {\n\
        echo 'stream-failed';\n\
    }\n";

/// [`STREAM_PAUSE`] over the peer's dripping body.
const STREAM_DRIP: &str = "    try {\n\
        Core\\Http\\Stream $feed = Core\\Http\\Client::stream(Core\\Http\\Method::Get, 'http://{peer}/drip');\n\
        foreach ($feed->chunks() as tainted bytes $piece) {\n\
            echo '';\n\
        }\n\
        echo 'stream-done';\n\
    } catch (Throwable $failed) {\n\
        echo 'stream-failed';\n\
    }\n";

/// The script `limits.max_tasks` spawns twice: it runs for a second.
const CHILD: &str = "<?nvs\nCore\\Time::sleep(1s);\necho 'child ';\n";

/// The program of the second test: one function per probe, called for `?n=<index>`, which sets
/// the probe's key when `?set=1` and prints what `set` returned and `|` before its work.
fn behaviour_program(probes: &[(&'static str, Probe)], peer: SocketAddr) -> String {
    let mut text = String::from(
        "<?nvs\nclass Api {\n    #[Core\\Deprecated(since: \"2.0\")]\n    \
         public function old(): string { return 'deprecated-ran'; }\n}\nclass Probes {\n",
    );
    for (index, (key, probe)) in probes.iter().enumerate() {
        let (key, value) = (quoted(key), quoted(probe.value));
        let work = probe
            .work
            .replace("{peer}", &peer.to_string())
            .replace("{port}", &peer.port().to_string());
        text.push_str(&format!(
            "public static function probe{index}(bool $on): void {{\n    \
             string $tag = $on ? 'tag-{index}-1' : 'tag-{index}-0';\n{before}    \
             if ($on) {{\n        bool $accepted = Core\\Config::set({key}, {value});\n        \
             echo $accepted ? 'true' : 'false', '|';\n        if (!$accepted) {{\n            \
             return;\n        }}\n    }}\n{work}}}\n",
            before = probe.before,
        ));
    }
    text.push_str(
        "}\nvar $n = Core\\Request::queryParameter('n') ?? '';\nbool $on = Core\\Request::queryParameter('set') == '1';\n",
    );
    for index in 0..probes.len() {
        text.push_str(&format!(
            "if ($n == '{index}') {{\n    Probes::probe{index}($on);\n}}\n"
        ));
    }
    text
}

/// Waits until `ready` holds, and says whether it did within [`ANSWER`].
fn waited(mut ready: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + ANSWER;
    loop {
        if ready() {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::park_timeout(POLL);
    }
}

/// One run of probe `index`: the request, then what its [`Settle`] waits for.
fn run(server: &Server, index: usize, probe: &Probe, on: bool) -> Result<Outcome, String> {
    let tag = format!("tag-{index}-{}", u8::from(on));
    let from = server.said().len();
    let answer = server.send(&format!("/?n={index}&set={}", u8::from(on)), probe.body);
    let since = |server: &Server| server.said().get(from..).unwrap_or_default().to_owned();
    let settled = match probe.settle {
        Settle::Answered => true,
        Settle::Log(marks) => {
            let marks: Vec<String> = marks.iter().map(|m| m.replace("{tag}", &tag)).collect();
            waited(|| {
                let log = since(server);
                marks.iter().any(|mark| log.contains(mark.as_str()))
            })
        }
        Settle::Spool(mark) => {
            let mark = mark.replace("{tag}", &tag);
            waited(|| server.spool().contains(&mark))
        }
    };
    let outcome = Outcome {
        tag,
        answer,
        log: since(server),
        spool: server.spool(),
    };
    if settled {
        Ok(outcome)
    } else {
        Err(format!(
            "the run did not settle within {ANSWER:?}: {outcome}"
        ))
    }
}

#[test]
fn every_settable_key_changes_what_a_request_does() {
    let rows = nvs_config::settable::SETTABLE;
    assert!(rows.len() > 30, "`SETTABLE` lists only {} keys", rows.len());
    let mut wrong = Vec::new();
    let mut probes = Vec::new();
    for row in rows {
        match probe(row.key) {
            Some(probe) => probes.push((row.key, probe)),
            None => wrong.push(format!(
                "`{}`: no probe shows that setting it changes what a request does",
                row.key
            )),
        }
    }

    let peer = peer();
    let server = Server::start(
        "behaviour",
        &proxied(peer),
        &behaviour_program(&probes, peer),
        &[("child.nvs", CHILD)],
    );
    for (index, (key, probe)) in probes.iter().enumerate() {
        let with = match run(&server, index, probe, true) {
            Ok(outcome) => outcome,
            Err(why) => {
                wrong.push(format!("`{key}` set: {why}"));
                continue;
            }
        };
        let without = match run(&server, index, probe, false) {
            Ok(outcome) => outcome,
            Err(why) => {
                wrong.push(format!("`{key}` not set: {why}"));
                continue;
            }
        };
        let holds = |outcome: &Outcome| match probe.judge {
            Judge::Answer(check) => check(&outcome.answer),
            Judge::Outcome(check) => check(outcome),
        };
        let body = &with.answer.body;
        let value = probe.value;
        if !(body.is_empty() || body.starts_with("true|")) {
            wrong.push(format!(
                "`{key}` = `{value}`: `set` did not return `true`: {with}"
            ));
        } else if holds(&without) {
            wrong.push(format!(
                "`{key}`: the check already holds with nothing set: {without}"
            ));
        } else if !holds(&with) {
            wrong.push(format!(
                "`{key}` = `{value}`: `set` accepted it and nothing the request does changed: \
                 {with}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Starts the peer the outbound probes call, and returns its address. Its threads end with the
/// test process.
///
/// - `/slow` answers after [`SLOW`].
/// - `/hop/<n>` redirects to `/hop/<n - 1>`, and `/hop/0` answers `200`.
/// - `/pause` sends half its body, waits [`SLOW`], and sends the rest.
/// - `/drip` sends its body one byte every 50 ms for [`SLOW`].
/// - `/ws-big` accepts a WebSocket and sends one message of 64 KiB.
/// - `/ws-late` accepts a WebSocket and reads nothing for [`SLOW`].
/// - `CONNECT`, as the proxy of [`proxied`], opens its tunnel after [`SLOW`] and answers the
///   request sent through it as above.
fn peer() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the peer binds a loopback port");
    let addr = listener.local_addr().expect("the peer has an address");
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || answer_as_peer(stream));
        }
    });
    addr
}

/// The path of the request line waiting on `stream`, read without taking it off the socket.
fn peeked_path(stream: &TcpStream) -> Option<String> {
    let mut buffer = [0_u8; 2048];
    loop {
        let read = stream.peek(&mut buffer).ok()?;
        let text = String::from_utf8_lossy(&buffer[..read]);
        if let Some((line, _)) = text.split_once("\r\n") {
            return line.split_whitespace().nth(1).map(str::to_owned);
        }
        if read == 0 || read == buffer.len() {
            return None;
        }
        std::thread::park_timeout(Duration::from_millis(5));
    }
}

/// Answers one connection to [`peer`].
fn answer_as_peer(mut stream: TcpStream) {
    let Some(path) = peeked_path(&stream) else {
        return;
    };
    if path.starts_with("/ws-") {
        let Ok(mut socket) = tungstenite::accept(stream) else {
            return;
        };
        if path == "/ws-big" {
            let _ = socket.send(tungstenite::Message::text("x".repeat(65536)));
        } else {
            std::thread::sleep(SLOW);
        }
        while socket.read().is_ok() {}
        return;
    }
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    // A `CONNECT`: the tunnel opens late, and the request sent through it is answered here.
    if !path.starts_with('/') {
        std::thread::sleep(SLOW);
        if stream
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .is_ok()
        {
            answer_as_peer(stream);
        }
        return;
    }
    let ok = |body: &str| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    };
    let _ = match path.as_str() {
        "/slow" => {
            std::thread::sleep(SLOW);
            stream.write_all(ok("slow").as_bytes())
        }
        "/pause" => stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nhalf")
            .and_then(|()| stream.flush())
            .and_then(|()| {
                std::thread::sleep(SLOW);
                stream.write_all(b"done")
            }),
        "/drip" => {
            let drops = SLOW.as_millis() / 50;
            let mut sent = stream.write_all(
                format!("HTTP/1.1 200 OK\r\nContent-Length: {drops}\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            );
            for _ in 0..drops {
                if sent.is_err() {
                    break;
                }
                sent = stream.write_all(b"x").and_then(|()| stream.flush());
                std::thread::sleep(Duration::from_millis(50));
            }
            sent
        }
        hop => match hop
            .strip_prefix("/hop/")
            .and_then(|n| n.parse::<u32>().ok())
        {
            Some(0) => stream.write_all(ok("end").as_bytes()),
            Some(n) => stream.write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: /hop/{}\r\nContent-Length: 0\r\n\
                     Connection: close\r\n\r\n",
                    n - 1
                )
                .as_bytes(),
            ),
            None => stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
        },
    };
}
