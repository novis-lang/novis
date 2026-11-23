//! `rule:http-server/a-development-server-answers-the-hosts-it-knows`, asked
//! of the socket peer: a local proxy listed in `[server] trusted_proxies`
//! forwards a browser's request with another client address, and the guard
//! still answers an unknown host `421`. The `421` has a body that names
//! `[server] development_hosts` and the host, and its `Warn` line is written
//! once per host per minute.
//!
//! Through `nvs serve` itself, because the guard runs at the door.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// A running `nvs serve app.nvs` in development, which trusts loopback as a
/// proxy and logs to `server.log`. Dropping it stops the process and removes
/// its directory.
struct Served {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    _data: nvs_repo::Scratch,
}

impl Served {
    fn start() -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("rebinding-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        std::fs::write(dir.join("app.nvs"), "<?nvs\necho \"ran\";\n")
            .expect("the program is written");
        std::fs::write(
            dir.join("nvs.toml"),
            "[mode]\ndefault = \"development\"\n\n\
             [server]\ntrusted_proxies = [\"127.0.0.1\", \"::1\"]\n\n\
             [debug]\nstore = false\n\n\
             [log]\ntarget = \"file:server.log\"\nformat = \"json\"\n",
        )
        .expect("the configuration is written");
        let data = nvs_repo::scratch_private("nvsdata");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");
        let out = child.stdout.take().expect("standard output is piped");
        let (sent, bound) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let _ = sent.send(
                        rest.split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                    );
                }
            }
        });
        let Ok(addr) = bound.recv_timeout(Duration::from_secs(60)) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`nvs serve` did not start listening");
        };
        Self {
            child,
            addr: addr.parse().expect("the server printed an address"),
            dir,
            _data: data,
        }
    }

    /// The whole answer the server writes to `raw`, which closes the connection.
    fn answer_to(&self, raw: &str) -> String {
        let mut stream = TcpStream::connect(self.addr).expect("the server answers");
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .expect("a read timeout is set");
        stream
            .write_all(raw.as_bytes())
            .expect("the request is sent");
        let mut answer = Vec::new();
        std::io::Read::read_to_end(&mut stream, &mut answer).expect("an answer arrives");
        String::from_utf8_lossy(&answer).into_owned()
    }

    /// The log's lines for a `421`, once there are `count` of them.
    fn refused_lines(&self, count: usize) -> Vec<String> {
        let log = self.dir.join("server.log");
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            let text = std::fs::read_to_string(&log).unwrap_or_default();
            let lines: Vec<String> = text
                .lines()
                .filter(|line| line.contains("\"status\":421"))
                .map(str::to_owned)
                .collect();
            if lines.len() >= count {
                return lines;
            }
            assert!(
                Instant::now() < until,
                "the log has {} of {count} lines for a 421: {text}",
                lines.len()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn an_unknown_host_from_a_trusted_local_proxy_is_a_421_whatever_it_forwards() {
    let served = Served::start();
    let request = "GET /blog/ HTTP/1.1\r\nHost: rebound.example.com\r\n\
                   X-Forwarded-For: 203.0.113.9\r\nConnection: close\r\n\r\n";
    for _ in 0..2 {
        let answer = served.answer_to(request);
        assert!(
            answer.starts_with("HTTP/1.1 421 Misdirected Request\r\n"),
            "an unknown host through a local proxy: {answer}"
        );
        let (_, body) = answer
            .split_once("\r\n\r\n")
            .expect("the answer has a head and a body");
        assert!(
            body.contains("development_hosts") && body.contains("\"rebound.example.com\""),
            "the body names the key and the host: {answer}"
        );
        assert!(
            answer
                .to_ascii_lowercase()
                .contains("content-type: text/plain"),
            "the body is text: {answer}"
        );
    }

    // A host this machine knows is answered through the same proxy.
    let known = served.answer_to(
        "GET / HTTP/1.1\r\nHost: localhost\r\n\
         X-Forwarded-For: 203.0.113.9\r\nConnection: close\r\n\r\n",
    );
    assert!(known.starts_with("HTTP/1.1 200 OK\r\n"), "{known}");

    // Two refusals within a minute write one `Warn`, and the repeat is an
    // ordinary access line.
    let lines = served.refused_lines(2);
    let warned = lines
        .iter()
        .filter(|line| line.to_ascii_lowercase().contains("\"level\":\"warn\""))
        .count();
    assert_eq!(warned, 1, "one `Warn` for one host: {lines:?}");
    assert!(
        lines
            .iter()
            .all(|line| line.contains("\"host\":\"rebound.example.com\"")),
        "every line names the host: {lines:?}"
    );
}
