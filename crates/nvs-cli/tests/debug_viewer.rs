//! `nvs debug` serves the viewer to its own page and to nothing else
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! Each case starts one service on a data folder of its own and a port the OS picks, then speaks
//! HTTP/1.1 to it by hand, so every header the attack sends is the one written here. The service
//! prints its link on standard error once it listens, and a case waits for that line.
//!
//! The attack: a page that rebinds its own name to the service sends a `Host` the service does not
//! answer to; a page on another origin sends a changing request; a service bound off loopback is
//! asked in plain HTTP and without its token.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};

/// How long a case waits for something the service does.
const PATIENCE: Duration = Duration::from_secs(60);

/// What the service prints before its link.
const LINK: &str = "the debug viewer is at ";

/// A running `nvs debug` on its own data folder.
struct Service {
    child: Child,
    data: nvs_repo::Scratch,
    /// The link it printed.
    link: String,
    /// The address it listens on.
    addr: SocketAddr,
}

impl Service {
    /// Starts `nvs debug --listen <listen>` and waits for its link.
    fn start(listen: &str) -> Self {
        let data = nvs_repo::scratch_private("nvsdata");
        let mut child = spawn(&data, listen);
        let Some(link) = link_of(&mut child) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`nvs debug --listen {listen}` printed no link");
        };
        let port = link
            .rsplit_once(':')
            .and_then(|(_, rest)| rest.split('/').next()?.parse::<u16>().ok())
            .expect("the link names a port");
        Self {
            child,
            data,
            link,
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    /// `request` over plain HTTP, as written, and the whole response.
    fn plain(&self, request: &str) -> Answer {
        let mut stream = TcpStream::connect(self.addr).expect("the service accepts");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("a timeout is set");
        stream
            .write_all(request.as_bytes())
            .expect("the request is sent");
        // The end of the request is the end of what is sent. A TLS listener reads plain text as
        // the head of a long record, and without this it would wait for the rest.
        stream
            .shutdown(std::net::Shutdown::Write)
            .expect("the sending half closes");
        Answer::read(&mut stream)
    }

    /// `request` over TLS, trusting the certificate the service generated, and the whole response.
    fn tls(&self, request: &str) -> Answer {
        let cert = CertificateDer::from_pem_file(self.data.join("debug").join("viewer.crt"))
            .expect("the service wrote its certificate");
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert).expect("the certificate is a root");
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("the provider has versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
        let name = ServerName::try_from("localhost").expect("a valid name");
        let conn = rustls::ClientConnection::new(Arc::new(config), name).expect("a client");
        let stream = TcpStream::connect(self.addr).expect("the service accepts");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("a timeout is set");
        let mut tls = rustls::StreamOwned::new(conn, stream);
        tls.write_all(request.as_bytes())
            .expect("the request is sent");
        Answer::read(&mut tls)
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn(data: &std::path::Path, listen: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(data)
        .arg("--data")
        .arg(data)
        .arg("--no-init")
        .args(["debug", "--listen", listen])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside starts")
}

/// The link `child` prints on standard error, or `None` when it ends first.
fn link_of(child: &mut Child) -> Option<String> {
    let err = child.stderr.take().expect("standard error is piped");
    let (sent, link) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(err).lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix(LINK) {
                let _ = sent.send(rest.to_owned());
            }
        }
    });
    link.recv_timeout(PATIENCE).ok()
}

/// A response: its status, its head as text, and its body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    /// Reads a `Connection: close` response to its end.
    fn read(stream: &mut impl Read) -> Self {
        let mut bytes = Vec::new();
        // A connection the peer resets still has what it sent before.
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .strip_prefix("HTTP/1.1 ")
            .and_then(|rest| rest.get(..3)?.parse().ok())
            .unwrap_or(0);
        Self {
            status,
            head: head.to_ascii_lowercase(),
            body: body.to_owned(),
        }
    }

    fn has(&self, header: &str) -> bool {
        self.head.contains(&header.to_ascii_lowercase())
    }
}

fn get(path: &str, host: &str, extra: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n{extra}Connection: close\r\n\r\n")
}

fn post(host: &str, extra: &str) -> String {
    format!(
        "POST /api/replay HTTP/1.1\r\nHost: {host}\r\n{extra}Content-Length: 2\r\n\
         Connection: close\r\n\r\n{{}}"
    )
}

#[test]
fn every_view_path_is_the_page_with_the_rules_headers() {
    let service = Service::start("127.0.0.1:0");
    assert!(
        service.link.starts_with("http://127.0.0.1:"),
        "{}",
        service.link
    );
    let host = service.addr.to_string();
    for path in [
        "/",
        "/requests/4bf92f3577b34da6a3ce929d0e0e4736/00f067aa0ba902b7",
    ] {
        let page = service.plain(&get(path, &host, ""));
        assert_eq!(page.status, 200, "{path}");
        assert!(page.body.contains("<div id=\"app\"></div>"), "{path}");
        assert!(page.has("content-security-policy: default-src 'none'; script-src 'self'"));
        assert!(page.has("x-content-type-options: nosniff"));
        assert!(page.has("referrer-policy: no-referrer"));
    }
    let script = service.plain(&get(
        "/viewer.js",
        &format!("localhost:{}", service.addr.port()),
        "",
    ));
    assert_eq!(script.status, 200);
    assert!(script.has("content-type: text/javascript"));
}

#[test]
fn a_rebinding_host_is_refused() {
    let service = Service::start("127.0.0.1:0");
    let port = service.addr.port();
    for host in [
        format!("attacker.example:{port}"),
        format!("127.0.0.1.attacker.example:{port}"),
        format!("127.0.0.1:{}", port.wrapping_add(1)),
        "localhost".to_owned(),
    ] {
        let answer = service.plain(&get("/api/events", &host, ""));
        assert_eq!(answer.status, 421, "{host}");
        assert!(answer.has("content-security-policy"), "{host}");
    }
}

#[test]
fn a_changing_request_needs_the_own_origin_and_a_json_body() {
    let service = Service::start("127.0.0.1:0");
    let host = service.addr.to_string();
    let own = format!("Origin: http://{host}\r\n");
    let json = "Content-Type: application/json\r\n";
    let forged = service.plain(&post(
        &host,
        &format!("Origin: http://attacker.example\r\n{json}"),
    ));
    assert_eq!(forged.status, 403);
    let absent = service.plain(&post(&host, json));
    assert_eq!(absent.status, 403);
    let form = service.plain(&post(
        &host,
        &format!("{own}Content-Type: application/x-www-form-urlencoded\r\n"),
    ));
    assert_eq!(form.status, 415);
    let allowed = service.plain(&post(&host, &format!("{own}{json}")));
    assert_eq!(
        allowed.status, 409,
        "every check passed, and a service started by hand has no replay target"
    );
    assert!(allowed.body.contains("started by hand"), "{}", allowed.body);
    let read = service.plain(&get("/", &host, "Origin: http://attacker.example\r\n"));
    assert_eq!(
        read.status, 403,
        "a read from another origin is refused too"
    );
}

#[test]
fn the_watermark_is_pushed_after_a_batch_is_committed() {
    let service = Service::start("127.0.0.1:0");
    let mut stream = TcpStream::connect(service.addr).expect("the service accepts");
    stream
        .set_read_timeout(Some(PATIENCE))
        .expect("a timeout is set");
    let request = get("/api/events", &service.addr.to_string(), "");
    stream
        .write_all(request.as_bytes())
        .expect("the request is sent");
    let mut events = BufReader::new(stream);
    let mut next = || {
        let mut line = String::new();
        loop {
            line.clear();
            assert!(
                events.read_line(&mut line).expect("the stream is open") > 0,
                "the stream ended"
            );
            if let Some(batch) = line.trim_end().strip_prefix("data: ") {
                return batch.parse::<i64>().expect("the event is a number");
            }
        }
    };
    assert_eq!(next(), 0, "a new index has committed nothing");
    let spool = service.data.join("debug").join("spool");
    let sample = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "debug",
        "one-of-each.spool",
    ]
    .iter()
    .collect::<std::path::PathBuf>();
    // Copied beside the spool and then renamed into it, so the service never sees half a file.
    let staged = service.data.join("debug").join("staged.spool");
    std::fs::copy(sample, &staged).expect("the debug folder is writable");
    std::fs::rename(&staged, spool.join("1-1-0.0.spool")).expect("the spool is writable");
    assert_eq!(next(), 1, "the batch that read the file is pushed");
}

#[test]
fn a_taken_port_stops_the_service_with_the_keys_that_move_it() {
    let first = Service::start("127.0.0.1:0");
    let data = nvs_repo::scratch_private("nvsdata");
    let out = spawn(&data, &first.addr.to_string())
        .wait_with_output()
        .expect("the second service ends");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("`[debug] listen`") && err.contains("`--listen`"),
        "{err}"
    );
}

#[test]
fn off_loopback_it_serves_tls_and_admits_only_the_token() {
    let service = Service::start("0.0.0.0:0");
    let (_, token) = service
        .link
        .split_once("?token=")
        .expect("the link carries the token");
    assert!(
        service.link.starts_with("https://localhost:"),
        "{}",
        service.link
    );
    let host = format!("localhost:{}", service.addr.port());

    let clear = service.plain(&get("/", &host, ""));
    assert_eq!(
        clear.status, 0,
        "plain HTTP gets no HTTP answer: {}",
        clear.head
    );

    let without = service.tls(&get("/", &host, ""));
    assert_eq!(without.status, 401);
    assert!(!without.body.contains("<div id=\"app\">"));
    let wrong = service.tls(&get("/?token=0123456789abcdef", &host, ""));
    assert_eq!(wrong.status, 401);
    let forged = service.tls(&get(
        "/",
        &host,
        "Cookie: __Host-nvs-debug=0123456789abcdef\r\n",
    ));
    assert_eq!(forged.status, 401);

    let admitted = service.tls(&get(&format!("/requests?token={token}"), &host, ""));
    assert_eq!(admitted.status, 303);
    assert!(admitted.has("location: /requests"));
    assert!(admitted.has(&format!(
        "set-cookie: __Host-nvs-debug={token}; Path=/; Secure; HttpOnly; SameSite=Strict"
    )));
    let page = service.tls(&get(
        "/requests",
        &host,
        &format!("Cookie: __Host-nvs-debug={token}\r\n"),
    ));
    assert_eq!(page.status, 200);
    assert!(page.body.contains("<div id=\"app\"></div>"));
}
