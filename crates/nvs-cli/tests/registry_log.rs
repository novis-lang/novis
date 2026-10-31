//! `nvs registry log append` and `verify` (`rule:packaging/the-registry-keeps-an-append-only-log`),
//! through the built binary: entries in on standard input, a log directory and a signed
//! checkpoint out, and the whole directory checked again from scratch.
//!
//! The signing key is Go's `sumdb/note` test key, from the vendored vectors in
//! `tests/fixtures/tlog/note.json`, so no case needs a random source.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SKEY: &str = "PRIVATE+KEY+PeterNeumann+c74f20a3+AYEKFALVFGyNhPJEMzD1QIDr+Y7hfZx09iUvxdXHKDFz";
const VKEY: &str = "PeterNeumann+c74f20a3+ARpc2QcUPDhMQegwxbzhKqiBfsVkmqq/LDE4izWy10TW";
const ORIGIN: &str = "example.com/log";

/// A log directory and a key file in a folder of their own, private to this account.
struct Setup {
    scratch: nvs_repo::Scratch,
    keys: PathBuf,
}

impl Setup {
    fn new(name: &str) -> Self {
        let scratch = nvs_repo::scratch_private(name);
        let keys = scratch.path().join("keys");
        std::fs::create_dir(&keys).unwrap();
        std::fs::write(keys.join("log.key"), SKEY).unwrap();
        Self { scratch, keys }
    }

    fn log(&self) -> PathBuf {
        self.scratch.path().join("log")
    }

    /// `nvs registry log append`, with `lines` on standard input.
    fn append(&self, lines: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "log", "append"])
            .arg(self.log())
            .arg("--key")
            .arg(self.keys.join("log.key"))
            .args(["--origin", ORIGIN])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary under test runs");
        let mut stdin = child.stdin.take().unwrap();
        for line in lines {
            writeln!(stdin, "{line}").unwrap();
        }
        drop(stdin);
        child.wait_with_output().unwrap()
    }

    /// `nvs registry log verify`.
    fn verify(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "log", "verify"])
            .arg(self.log())
            .args(["--vkey", VKEY, "--origin", ORIGIN])
            .output()
            .expect("the binary under test runs")
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn publish(n: u32) -> String {
    format!(
        r#"{{"v":1,"kind":"publish","package":"acme/http","version":"1.{n}.0","digest":"{n:064x}","repository":"https://example.com/acme/http","commit":"c{n}"}}"#
    )
}

#[test]
fn entries_appended_in_two_runs_verify_against_the_signed_checkpoint() {
    let setup = Setup::new("registry-log-append");
    let first: Vec<String> = (0..3).map(publish).collect();
    let first: Vec<&str> = first.iter().map(String::as_str).collect();
    let appended = setup.append(&first);
    assert!(appended.status.success(), "{}", stderr(&appended));
    let second = [
        r#"{ "kind": "unlist", "version": "1.1.0", "package": "acme/http", "v": 1 }"#,
        r#"{"v":1,"kind":"token","account":"ada","token":"t1","change":"revoked"}"#,
    ];
    let appended = setup.append(&second);
    assert!(appended.status.success(), "{}", stderr(&appended));
    let printed = String::from_utf8_lossy(&appended.stdout);
    assert!(printed.contains("The log has 5 entries."), "{printed}");
    // After the summary, each file the append wrote, and the checkpoint last.
    let written: Vec<&str> = printed.lines().skip(1).collect();
    assert_eq!(
        written,
        ["tile/0/000.p/5", "tile/entries/000.p/5", "checkpoint"],
        "{printed}"
    );

    let checkpoint = std::fs::read_to_string(setup.log().join("checkpoint")).unwrap();
    assert!(
        checkpoint.starts_with(&format!("{ORIGIN}\n5\n")),
        "{checkpoint}"
    );
    assert!(checkpoint.contains("\n— PeterNeumann "), "{checkpoint}");
    // The second run stored the unlist in its canonical form.
    let bundle = std::fs::read(setup.log().join("tile/entries/000.p/5")).unwrap();
    let canonical = br#"{"kind":"unlist","package":"acme/http","v":1,"version":"1.1.0"}"#;
    assert!(bundle.windows(canonical.len()).any(|w| w == canonical));

    let verified = setup.verify();
    assert!(verified.status.success(), "{}", stderr(&verified));
}

#[test]
fn verify_refuses_a_changed_tile() {
    let setup = Setup::new("registry-log-tamper");
    let entries: Vec<String> = (0..4).map(publish).collect();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    assert!(setup.append(&entries).status.success());
    let tile = setup.log().join("tile/0/000.p/4");
    let mut data = std::fs::read(&tile).unwrap();
    data[33] ^= 1;
    std::fs::write(&tile, data).unwrap();
    let verified = setup.verify();
    assert!(!verified.status.success());
    assert!(
        stderr(&verified).contains("does not match its hash"),
        "{}",
        stderr(&verified)
    );
}

#[test]
fn an_entry_outside_the_closed_list_adds_nothing() {
    let setup = Setup::new("registry-log-kind");
    let good = publish(0);
    let refused = setup.append(&[&good, r#"{"v":1,"kind":"delete","package":"acme/http"}"#]);
    assert!(!refused.status.success());
    assert!(stderr(&refused).contains("line 2"), "{}", stderr(&refused));
    assert!(!setup.log().join("checkpoint").exists());
    assert!(!setup.log().join("tile").exists());
}

fn advisory(n: u32, versions: &str) -> String {
    format!(
        r#"{{"v":1,"kind":"advisory","id":"acme/http-{n}","package":"acme/http","versions":"{versions}","summary":"It leaks the session cookie."}}"#
    )
}

#[test]
fn an_advisory_entry_with_a_closed_or_open_range_is_logged() {
    let setup = Setup::new("registry-log-advisory");
    let closed = advisory(1, "1.2.0..1.2.4");
    let open = advisory(2, "2.0.0..");
    let appended = setup.append(&[&publish(0), &closed, &open]);
    assert!(appended.status.success(), "{}", stderr(&appended));
    let bundle = std::fs::read(setup.log().join("tile/entries/000.p/3")).unwrap();
    for range in [
        &br#""versions":"1.2.0..1.2.4""#[..],
        br#""versions":"2.0.0..""#,
    ] {
        assert!(bundle.windows(range.len()).any(|w| w == range));
    }
    let verified = setup.verify();
    assert!(verified.status.success(), "{}", stderr(&verified));
}

#[test]
fn an_advisory_entry_with_a_malformed_range_is_refused() {
    for (n, versions) in ["1.2.4..1.2.0", "1.2.0", "1.2..", "1.2.0..1.3.0,2.0.0.."]
        .into_iter()
        .enumerate()
    {
        let setup = Setup::new(&format!("registry-log-advisory-bad-{n}"));
        let good = publish(0);
        let bad = advisory(1, versions);
        let refused = setup.append(&[&good, &bad]);
        assert!(!refused.status.success(), "{versions}");
        let why = stderr(&refused);
        assert!(why.contains("line 2"), "{why}");
        assert!(why.contains(versions), "{why}");
        assert!(!setup.log().join("checkpoint").exists(), "{versions}");
    }
}

#[test]
fn a_key_file_another_account_can_change_is_refused() {
    let setup = Setup::new("registry-log-key");
    writable_by_others(&setup.keys, true);
    let refused = setup.append(&[&publish(0)]);
    writable_by_others(&setup.keys, false);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("the key file"),
        "{}",
        stderr(&refused)
    );
    assert!(!setup.log().join("checkpoint").exists());
}

/// Gives every account on the host the right to write in `dir`, or takes it back: the state
/// `rule:config/ownership-is-the-trust-boundary` refuses. On Windows that is a grant to
/// `BUILTIN\Users` by its SID, with no inheritance flags; on Unix the write bit for others.
fn writable_by_others(dir: &Path, open: bool) {
    #[cfg(windows)]
    {
        const USERS: &str = "*S-1-5-32-545";
        let grant = format!("{USERS}:(WD)");
        let edit: [&str; 2] = if open {
            ["/grant:r", &grant]
        } else {
            ["/remove:g", USERS]
        };
        // `icacls` edits the scratch directory it is given and opens nothing in the repository.
        let ran = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .args(edit)
            .output()
            .expect("`icacls` ships with Windows");
        assert!(
            ran.status.success(),
            "icacls {edit:?} on `{}`",
            dir.display()
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if open { 0o757 } else { 0o755 };
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode))
            .expect("a scratch directory's mode is this process's to change");
    }
}
