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
    assert!(String::from_utf8_lossy(&appended.stdout).contains("The log has 5 entries."));

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
