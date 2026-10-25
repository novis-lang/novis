//! Every program directly under `examples/` runs with `nvs run` and ends with the status it is
//! written to end with, so a broken example fails `bun nv verify` and not only CI's nightly
//! `valgrind` job (`rule:testing/the-deep-lane`), which is the one other place they run.
//!
//! Each runs as `bun nv valgrind` (`tools/nv/cmd/valgrind.ts`) runs it: under the repository's
//! `nvs.toml`, with this build in `NVS_BIN` for an example that starts `nvs` itself. An example with
//! a `.nvsr` beside it answers the request that file describes, and one with a `.toml` beside it
//! runs under that file instead of the repository's. The directories under `examples/` hold what an example reads or
//! spawns, and are not programs of their own.
//!
//! An example that talks to a server `tests/db/compose.yaml` starts runs only when that server
//! answers, and the run prints each one it left out. `examples/http.nvs`'s origin is served here
//! when nothing else holds its port, as `tools/nv/accept/origin.ts` serves it for the sweep.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// The examples that end with a status other than 0, and the status: an uncaught throw and a
/// memory limit each end the program with 1, which is what each of them is there to show.
const STATUS: &[(&str, i32)] = &[("limits.nvs", 1), ("uncaught.nvs", 1)];

/// The examples that need a server `tests/db/compose.yaml` starts, and the address it publishes.
///
/// The examples that share an address run one after another. Five of them open Postgres, and a
/// queue worker allows its handshake two seconds, which five handshakes at once beside the other
/// test binaries outran.
const SERVERS: &[(&str, &str)] = &[
    ("cache.nvs", "127.0.0.1:16379"),
    ("cache-shared-tls.nvs", "127.0.0.1:16380"),
    ("db.nvs", "127.0.0.1:15432"),
    ("pool.nvs", "127.0.0.1:15432"),
    ("queue-purge.nvs", "127.0.0.1:15432"),
    ("queue.nvs", "127.0.0.1:15432"),
    ("stream.nvs", "127.0.0.1:15432"),
    ("transaction.nvs", "127.0.0.1:15432"),
];

/// The examples whose queue tables `nvs queue migrate` creates before they run, under the same
/// configuration. `--including-risky` brings up to date a table an older build left behind.
const MIGRATE: &[&str] = &["queue-purge.nvs", "queue-sqlite.nvs", "queue.nvs"];

/// `examples/cache-shared-socket.nvs`'s store is the compose file's `redis` reached over a Unix
/// domain socket, at the path its `.toml` names.
const SOCKET_EXAMPLE: (&str, &str) = ("cache-shared-socket.nvs", "/mnt/wsl/novis-redis/redis.sock");

/// `tools/nv/accept/origin.ts`'s address and hold: long enough that the example's 1ms deadline
/// always ends its last request first, and far under the default deadline of its first.
const ORIGIN: &str = "127.0.0.1:8099";
const ORIGIN_HOLD: Duration = Duration::from_millis(25);

/// How many examples run at once: one per core, and no more than eight, since the other test
/// binaries run beside this one.
fn width() -> usize {
    thread::available_parallelism().map_or(4, |cores| cores.get().min(8))
}

fn examples(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("`examples/` is readable")
        .map(|entry| entry.expect("an entry of `examples/`").path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "nvs"))
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Whether something accepts a connection at `address` within a second.
fn answers(address: &str) -> bool {
    let address: SocketAddr = address.parse().expect("a socket address");
    TcpStream::connect_timeout(&address, Duration::from_secs(1)).is_ok()
}

/// Serves `examples/http.nvs`'s origin for the rest of the test binary's life: `/ok` answers 200
/// `ok` and every other path 404, each after [`ORIGIN_HOLD`]. A port somebody else already holds is
/// left to them.
fn serve_origin() {
    let Ok(listener) = TcpListener::bind(ORIGIN) else {
        return;
    };
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || answer(stream));
        }
    });
}

fn answer(mut stream: TcpStream) {
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 8192 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let line = String::from_utf8_lossy(&head);
    let path = line.split(' ').nth(1).unwrap_or("");
    let (status, body) = if path == "/ok" {
        ("200 OK", "ok")
    } else {
        ("404 Not Found", "not found")
    };
    thread::sleep(ORIGIN_HOLD);
    // The caller may have hung up inside the hold, which is how the example's last request ends.
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
}

/// Runs one example and returns its failure, or `None` when it ended with its status.
fn run(dir: &Path, config: &Path, name: &str) -> Option<String> {
    let data = nvs_repo::scratch_private(&format!("example-{name}"));
    let binary = env!("CARGO_BIN_EXE_nvs");
    let own = dir.join(name).with_extension("toml");
    let config = if own.exists() { own.as_path() } else { config };
    if MIGRATE.contains(&name) {
        let migrated = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .args(["queue", "migrate", "--including-risky", "--config"])
            .arg(config)
            .current_dir(dir)
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        if !migrated.status.success() {
            return Some(format!(
                "`nvs queue migrate` for examples/{name} ended with {:?}:\n{}",
                migrated.status.code(),
                String::from_utf8_lossy(&migrated.stderr)
            ));
        }
    }
    let request = dir.join(name).with_extension("nvsr");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .arg("run")
        .arg("--config")
        .arg(config);
    if request.exists() {
        command.arg("--request").arg(&request);
    }
    let output = command
        .arg(name)
        .current_dir(dir)
        .env("NVS_BIN", binary)
        .env_remove("NOVIS_NO_INIT")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let wanted = STATUS
        .iter()
        .find(|(file, _)| *file == name)
        .map_or(0, |(_, status)| *status);
    if output.status.code() == Some(wanted) {
        return None;
    }
    Some(format!(
        "examples/{name} ended with {:?}, not {wanted}:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    ))
}

// The workspace denies a print because user-facing output belongs in
// `nvs-cli`'s binary. The one print here is for whoever reads the test run: an
// example that did not run, told to nobody, reads like one that passed.
#[allow(clippy::print_stderr)]
#[test]
fn every_example_ends_with_the_status_it_is_written_to_end_with() {
    let dir: PathBuf = nvs_repo::path("examples");
    let config = nvs_repo::path("nvs.toml");
    serve_origin();

    let mut left_out = Vec::new();
    let mut queue = Vec::new();
    for name in examples(&dir) {
        let server = SERVERS
            .iter()
            .find(|(file, _)| *file == name)
            .map(|(_, address)| *address);
        let unreachable = match server {
            Some(address) => !answers(address),
            None if name == SOCKET_EXAMPLE.0 => !Path::new(SOCKET_EXAMPLE.1).exists(),
            None => false,
        };
        if unreachable {
            left_out.push(name);
        } else {
            queue.push(name);
        }
    }
    assert!(
        queue.len() > SERVERS.len(),
        "the examples found under {}: {queue:?}",
        dir.display()
    );

    let queue = Mutex::new(queue);
    let failures = Mutex::new(Vec::new());
    let mut turns: Vec<(&str, Mutex<()>)> = Vec::new();
    for (_, address) in SERVERS {
        if !turns.iter().any(|(held, _)| held == address) {
            turns.push((address, Mutex::new(())));
        }
    }
    thread::scope(|scope| {
        for _ in 0..width() {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().unwrap().pop();
                    let Some(name) = next else { break };
                    let turn = SERVERS
                        .iter()
                        .find(|(file, _)| *file == name)
                        .and_then(|(_, address)| turns.iter().find(|(held, _)| held == address))
                        .map(|(_, turn)| turn.lock().unwrap());
                    if let Some(failure) = run(&dir, &config, &name) {
                        failures.lock().unwrap().push(failure);
                    }
                    drop(turn);
                }
            });
        }
    });

    if !left_out.is_empty() {
        eprintln!(
            "not run, because the server `tests/db/compose.yaml` starts for each is not up: {}",
            left_out.join(", ")
        );
    }
    let failures = failures.into_inner().unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
