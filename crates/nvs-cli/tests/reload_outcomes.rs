//! `rule:config/a-reload-command-checks-now-and-waits`'s exit table, for the servers that do not
//! reply as they should: the built `nvs reload` against an endpoint this test serves itself.
//!
//! Each case binds one endpoint in a fresh data folder's `run/`, answers the command's connection
//! the way the case needs, and checks the command's exit code and what it printed. Nothing waits
//! on time: the endpoint's thread ends when the command's connection does.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;

use nvs_host::endpoint::{self, Connection, Endpoint};

/// A private data folder whose `run/` exists, named so an endpoint fits a socket path on every
/// runner.
fn data_folder() -> (nvs_repo::Scratch, PathBuf) {
    let data = nvs_repo::scratch_private_for_socket("o", "run/serve-99999-1700000000000.new");
    let run = data.join("run");
    nvs_config::data::create_private_root(&run).expect("`run/` is made private");
    (data, run)
}

/// Binds an endpoint named for `pid` in `run` and answers one connection with `answer` on a
/// thread of its own. The endpoint stays bound until the thread ends.
fn serve(
    run: &Path,
    pid: u32,
    answer: impl FnOnce(Connection) + Send + 'static,
) -> (PathBuf, JoinHandle<()>) {
    let path = run.join(endpoint::name(pid, 1));
    let bound = Endpoint::bind(&path).expect("the test's endpoint binds");
    let thread = std::thread::spawn(move || {
        if let Ok(connection) = bound.accept() {
            answer(connection);
        }
    });
    (path, thread)
}

/// Reads the command's request line from `connection`.
fn read_request(connection: &mut Connection) {
    let mut line = String::new();
    let _ = BufReader::new(connection).read_line(&mut line);
}

/// What `nvs reload` printed and how it exited.
struct Ran {
    code: Option<i32>,
    out: String,
}

/// Runs `nvs --data <data> reload` and waits for it.
fn nvs_reload(data: &Path) -> Ran {
    let output = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .arg("reload")
        .stdin(Stdio::null())
        .output()
        .expect("the `nvs` binary this test was built beside starts");
    Ran {
        code: output.status.code(),
        out: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

/// A server that reads the request and closes with no line is a failed server: exit `1`.
// covers: tools:server/nvs-reload
#[test]
fn a_server_that_closes_without_a_reply_fails_the_reload() {
    let (data, run) = data_folder();
    let (_path, thread) = serve(&run, std::process::id(), |mut connection| {
        read_request(&mut connection);
    });
    let ran = nvs_reload(data.path());
    thread.join().expect("the endpoint's thread ends");
    assert_eq!(ran.code, Some(1), "`nvs reload` printed {}", ran.out);
    assert!(
        ran.out.contains("closed the connection without a reply"),
        "`nvs reload` printed {}",
        ran.out
    );
}

/// A server whose check panicked replies a refusal of kind `server`, and the command exits `1`.
/// The line is the one `reload_endpoint`'s panic reply writes.
// covers: tools:server/nvs-reload
#[test]
fn a_server_whose_check_panicked_fails_the_reload() {
    let (data, run) = data_folder();
    let (_path, thread) = serve(&run, std::process::id(), |mut connection| {
        read_request(&mut connection);
        let _ = connection.write_all(
            b"{\"config\":\"error\",\"restart\":[],\"programs\":0,\"failed\":0,\"refusals\":\
              [{\"kind\":\"server\",\"entry\":null,\"file\":null,\"line\":null,\
              \"message\":\"the server stopped its check with an internal error. \
              Its log has the details.\"}]}\n",
        );
    });
    let ran = nvs_reload(data.path());
    thread.join().expect("the endpoint's thread ends");
    assert_eq!(ran.code, Some(1), "`nvs reload` printed {}", ran.out);
    assert!(
        ran.out.contains("internal error"),
        "`nvs reload` printed {}",
        ran.out
    );
}

/// A pipe served by a process other than the one its file names is not reached: exit `1`, and
/// the file stays where it is.
// covers: tools:server/nvs-reload
#[cfg(windows)]
#[test]
fn a_pipe_another_process_serves_fails_the_reload_and_keeps_its_file() {
    let (data, run) = data_folder();
    let (path, thread) = serve(&run, std::process::id() + 1, |mut connection| {
        read_request(&mut connection);
    });
    let ran = nvs_reload(data.path());
    thread.join().expect("the endpoint's thread ends");
    assert_eq!(ran.code, Some(1), "`nvs reload` printed {}", ran.out);
    assert!(
        ran.out.contains("could not be reached") && ran.out.contains("is served by process"),
        "`nvs reload` printed {}",
        ran.out
    );
    assert!(path.exists(), "the endpoint's file is left alone");
}

/// A reply of 1 MiB with no newline is read only up to its 64 KiB limit, and fails the reload.
// covers: tools:server/nvs-reload
#[test]
fn a_reply_of_one_mebibyte_fails_the_reload() {
    let (data, run) = data_folder();
    let (_path, thread) = serve(&run, std::process::id(), |mut connection| {
        read_request(&mut connection);
        // The command stops reading at its limit, so this write fails once it closes.
        let _ = connection.write_all(&vec![b'x'; 1024 * 1024]);
    });
    let ran = nvs_reload(data.path());
    thread.join().expect("the endpoint's thread ends");
    assert_eq!(ran.code, Some(1), "`nvs reload` printed {}", ran.out);
    assert!(
        ran.out.contains("longer than 64 KiB"),
        "`nvs reload` printed {}",
        ran.out
    );
}
