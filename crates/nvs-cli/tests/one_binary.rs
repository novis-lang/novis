//! `nvs` as one executable: `--version`, a `--help` on every subcommand the
//! table lists, and no short flag of PHP's, as `docs/reference/tools/10-cli.md`
//! § *One binary* states them, and the binary's exit when a stop finds a served
//! request still parked.
//!
//! Through the built binary, because what is asserted is its command line and
//! its exit status.

use std::process::{Command, Output};

/// Runs `nvs` with `args` and nothing else.
fn nvs(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// Every subcommand the chapter's table names.
const SUBCOMMANDS: &[&str] = &[
    "run", "check", "test", "build", "api", "init", "config", "info", "meta", "agent", "ast",
];

/// `--version` is one line, `nvs <version> (commit <hash>, <date>)`, and the
/// version is this crate's, with `+dev` after it unless the build is a release. Every
/// subcommand the table lists answers `--help` with status `0`. `-i`, `-a`,
/// `-r`, `-f` and `-v` are refused with a non-zero status, and so is
/// `convert`, which this build does not carry.
// covers: tools:cli/one-binary
#[test]
fn nvs_is_one_binary_with_a_subcommand_for_every_operation() {
    let version = nvs(&["--version"]);
    let stdout = String::from_utf8_lossy(&version.stdout);
    assert_eq!(version.status.code(), Some(0));
    let line = stdout.strip_suffix('\n').expect("one line");
    assert!(!line.contains('\n'), "one line: {stdout}");
    let rest = line
        .strip_prefix("nvs ")
        .expect("it starts with the binary's name");
    let (semver, build) = rest.split_once(" (commit ").expect("then the commit");
    let release = env!("CARGO_PKG_VERSION");
    assert!(
        semver == release || semver == format!("{release}+dev"),
        "this crate's version, or it with `+dev`: {line}"
    );
    let (_hash, date) = build.split_once(", ").expect("then the date");
    assert!(
        date.ends_with(')') && date.len() == "2026-09-24)".len(),
        "{line}"
    );

    for sub in SUBCOMMANDS {
        let help = nvs(&[sub, "--help"]);
        let stderr = String::from_utf8_lossy(&help.stderr);
        assert_eq!(help.status.code(), Some(0), "`nvs {sub} --help`: {stderr}");
        assert!(
            !help.stdout.is_empty(),
            "`nvs {sub} --help` prints its usage"
        );
    }

    for flag in ["-i", "-a", "-r", "-f", "-v"] {
        let refused = nvs(&[flag]);
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert_ne!(
            refused.status.code(),
            Some(0),
            "`nvs {flag}` is not PHP's: {stderr}"
        );
        assert!(
            stderr.contains(&format!("unexpected argument '{flag}'")),
            "{stderr}"
        );
    }

    let convert = nvs(&["convert", "index.php"]);
    let stderr = String::from_utf8_lossy(&convert.stderr);
    assert_ne!(convert.status.code(), Some(0));
    assert!(
        stderr.contains("unrecognized subcommand 'convert'"),
        "{stderr}"
    );
}

/// Every command `nvs --help` lists, but `help`, has a row in the command table of
/// `docs/reference/tools/10-cli.md` § *One binary*: a row whose first cell starts with
/// `` `nvs <command> `` and then a space or the closing backtick. A new command fails
/// this until the table names it.
#[test]
fn every_command_nvs_help_lists_has_a_row_in_the_cli_reference() {
    let help = nvs(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&help.stdout);
    let listed: Vec<&str> = stdout
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(' '))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .collect();
    assert!(
        listed.len() > SUBCOMMANDS.len(),
        "`nvs --help` lists its commands: {stdout}"
    );

    let chapter = nvs_repo::path("docs/reference/tools/10-cli.md");
    let text = std::fs::read_to_string(&chapter).expect("the CLI chapter is in the tree");
    let table: Vec<&str> = text
        .lines()
        .skip_while(|line| *line != "# One binary")
        .skip_while(|line| !line.starts_with("| Command |"))
        .take_while(|line| line.starts_with('|'))
        .collect();
    let missing: Vec<&str> = listed
        .iter()
        .copied()
        .filter(|name| {
            let cell = format!("| `nvs {name}");
            !table.iter().any(|row| {
                row.strip_prefix(&cell)
                    .is_some_and(|rest| rest.starts_with(' ') || rest.starts_with('`'))
            })
        })
        .collect();
    assert!(
        missing.is_empty(),
        "commands with no row in the table: {missing:?}"
    );
}

/// `nvs service --help` and the help of each of its subcommands are written as
/// `AGENTS.md` § *Text an end user reads* asks: no dash joins two sentences, and
/// none of the words below, which an operator would have to look up, appears.
#[test]
fn nvs_service_help_is_written_in_the_plain_voice() {
    let subs = [
        "install",
        "uninstall",
        "start",
        "stop",
        "status",
        "run",
        "unit",
    ];
    let pages = std::iter::once(vec!["service", "--help"])
        .chain(subs.iter().map(|sub| vec!["service", sub, "--help"]));
    for args in pages {
        let help = nvs(&args);
        let stdout = String::from_utf8_lossy(&help.stdout);
        assert_eq!(help.status.code(), Some(0), "`nvs {}`", args.join(" "));
        assert!(!stdout.contains('—'), "`nvs {}`: {stdout}", args.join(" "));
        for word in [
            "refus",
            "hands",
            "verbatim",
            "argv",
            "registration",
            "drain",
        ] {
            assert!(
                !stdout.to_lowercase().contains(word),
                "`nvs {}` says `{word}`: {stdout}",
                args.join(" ")
            );
        }
    }
}

/// A server stopped while a request is still parked in its script, and while a
/// connection has sent half a request head, exits with status `0` once
/// `drain_timeout` has passed, and long before the request's sleep would end.
///
/// The stop is the operating system's: `SIGTERM` on Unix, and a Ctrl-Break sent
/// to the server's own process group on Windows. It runs on the proof profile
/// because a native teardown at exit is where `Task::force_unwind`'s Windows
/// crash showed, as exit code `0xC0000005`.
///
/// It does not reach a forced unwind, and it stayed green with that crash's fix
/// undone. A core's serving loop turns until nothing is parked, and a task
/// parked inside a script is forgotten at the scheduler's drop, not unwound.
/// `nvs-host`'s `tests/forced_unwind.rs` is what pins the unwind itself.
#[test]
fn a_server_stopped_with_a_request_still_parked_exits_cleanly() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// The request writes one line, so the case knows it is running, and then
    /// sleeps in a loop. A sleep the drain wakes starts again, so the request
    /// is still parked when the drain runs out.
    const PARKED: &str = "<?nvs\n\
        Core\\Response\\Stream $out = Core\\Response::stream('text/plain');\n\
        $out->write(\"parked\\n\");\n\
        while (true) {\n    Core\\Time::sleep(60s);\n}\n";
    /// What the start, the request's first line and the exit may each take.
    const PATIENCE: Duration = Duration::from_secs(60);

    /// Ends the server however the case ends, and removes its directory.
    struct Running {
        child: std::process::Child,
        dir: std::path::PathBuf,
    }
    impl Drop for Running {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("one-binary-parked-stop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    std::fs::write(dir.join("app.nvs"), PARKED).expect("the program is written");
    std::fs::write(dir.join("nvs.toml"), "[server]\ndrain_timeout = \"1s\"\n")
        .expect("the configuration is written");
    let stderr = std::fs::File::create(dir.join("stderr.txt")).expect("the stderr file opens");
    let data = nvs_repo::scratch_private("nvsdata");

    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(stderr);
    // Its own process group, so the Ctrl-Break below reaches the server and
    // its children and not this test.
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(
        &mut command,
        windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP,
    );
    let mut running = Running {
        child: command
            .spawn()
            .expect("the `nvs` binary this test was built beside starts"),
        dir: dir.clone(),
    };
    let out = running
        .child
        .stdout
        .take()
        .expect("standard output is piped");
    let (sent, bound) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix("listening on http://") {
                let addr = rest.split_whitespace().next().unwrap_or_default();
                let _ = sent.send(addr.to_owned());
            }
        }
    });
    let addr: std::net::SocketAddr = bound
        .recv_timeout(PATIENCE)
        .expect("`nvs serve` starts listening")
        .parse()
        .expect("the server printed an address");

    // Half a request head and then nothing. The server's reader waits for the
    // rest with no script on its stack, and the drain has to end that wait.
    let mut half = std::net::TcpStream::connect(addr).expect("the server accepts");
    half.write_all(b"GET / HTTP/1.1\r\nHost: loc")
        .expect("half a head is sent");

    let mut stream = std::net::TcpStream::connect(addr).expect("the server accepts");
    stream
        .set_read_timeout(Some(PATIENCE))
        .expect("a read timeout is set");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("the request is sent");
    let mut seen = Vec::new();
    let mut chunk = [0_u8; 256];
    while !seen.windows(b"parked\n".len()).any(|w| w == b"parked\n") {
        let read = stream
            .read(&mut chunk)
            .expect("the request's first line arrives");
        assert_ne!(read, 0, "the connection closed before the request ran");
        seen.extend_from_slice(&chunk[..read]);
    }

    let stopped = Instant::now();
    stop(running.child.id());
    let until = stopped + PATIENCE;
    let status = loop {
        if let Some(status) = running
            .child
            .try_wait()
            .expect("the server's status is read")
        {
            break status;
        }
        assert!(
            Instant::now() < until,
            "`nvs serve` did not exit after the stop"
        );
        std::thread::park_timeout(Duration::from_millis(50));
    };
    let said = std::fs::read_to_string(dir.join("stderr.txt")).unwrap_or_default();
    assert_eq!(status.code(), Some(0), "{said}");
    // The request sleeps for sixty seconds at a time, so an exit this soon was
    // the drain's and not the request's own end.
    assert!(stopped.elapsed() < Duration::from_secs(50), "{said}");
}

/// Asks the server `pid` to stop, the way a service manager does.
#[cfg(unix)]
fn stop(pid: u32) {
    let sent = nvs_repo::spawn("kill", &[])
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("`kill` runs");
    assert!(sent.success(), "`kill -TERM {pid}` was refused");
}

/// Asks the server `pid` to stop, the way Ctrl-Break in its console does. The
/// server was started as the leader of its own process group, so `pid` is that
/// group's id.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "`GenerateConsoleCtrlEvent` is the platform's only way to send a console's stop"
)]
fn stop(pid: u32) {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};
    // SAFETY: the call takes two integers and reads no memory of ours.
    let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) };
    assert_ne!(
        sent,
        0,
        "the Ctrl-Break was not sent: {}",
        std::io::Error::last_os_error()
    );
}
