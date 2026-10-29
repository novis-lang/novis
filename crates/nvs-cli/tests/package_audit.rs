//! The registry's advisory feed (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`),
//! through the built binary: a log built with `nvs registry log append`, its feed written with
//! `nvs registry log advisories`, and the feed checked against the log by `nvs registry log
//! verify`, which is the check `nvs pkg audit` makes.
//!
//! The signing key is Go's `sumdb/note` test key, from the vendored vectors in
//! `tests/fixtures/tlog/note.json`, so no case needs a random source. Its name is the log's origin,
//! as `nvs registry log advisories` needs.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const SKEY: &str = "PRIVATE+KEY+PeterNeumann+c74f20a3+AYEKFALVFGyNhPJEMzD1QIDr+Y7hfZx09iUvxdXHKDFz";
const VKEY: &str = "PeterNeumann+c74f20a3+ARpc2QcUPDhMQegwxbzhKqiBfsVkmqq/LDE4izWy10TW";
const ORIGIN: &str = "PeterNeumann";

const RETRACT: &str = r#"{"v":1,"kind":"retract","package":"acme/http","version":"1.1.0","reason":"It deletes files.","fixed_in":"1.1.1"}"#;
const WITHHOLD: &str =
    r#"{"v":1,"kind":"withhold","package":"acme/http","version":"1.2.0","reason":"Malware."}"#;

/// A log with one retraction and one withhold among other entries, and its key file, in a folder
/// private to this account.
struct Setup {
    scratch: nvs_repo::Scratch,
}

impl Setup {
    fn new(name: &str) -> Self {
        let scratch = nvs_repo::scratch_private(name);
        let keys = scratch.path().join("keys");
        std::fs::create_dir(&keys).unwrap();
        std::fs::write(keys.join("log.key"), SKEY).unwrap();
        let setup = Self { scratch };
        let publish = r#"{"v":1,"kind":"publish","package":"acme/http","version":"1.1.0","digest":"00000000000000000000000000000000000000000000000000000000000000ff","repository":"https://example.com/acme/http","commit":"c1"}"#;
        let unlist = r#"{"v":1,"kind":"unlist","package":"acme/http","version":"1.0.0"}"#;
        let appended = setup.append(&[publish, RETRACT, unlist, WITHHOLD]);
        assert!(appended.status.success(), "{}", stderr(&appended));
        setup
    }

    fn log(&self) -> PathBuf {
        self.scratch.path().join("log")
    }

    fn key(&self) -> PathBuf {
        self.scratch.path().join("keys").join("log.key")
    }

    fn nvs(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        command.arg("registry");
        command
    }

    /// `nvs registry log append`, with `lines` on standard input.
    fn append(&self, lines: &[&str]) -> Output {
        let mut child = self
            .nvs()
            .args(["log", "append"])
            .arg(self.log())
            .arg("--key")
            .arg(self.key())
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

    /// `nvs registry log advisories`.
    fn advisories(&self) -> Output {
        self.nvs()
            .args(["log", "advisories"])
            .arg(self.log())
            .arg("--key")
            .arg(self.key())
            .output()
            .expect("the binary under test runs")
    }

    /// `nvs registry log verify`, which checks the feed against the log.
    fn verify(&self) -> Output {
        self.nvs()
            .args(["log", "verify"])
            .arg(self.log())
            .args(["--vkey", VKEY, "--origin", ORIGIN])
            .output()
            .expect("the binary under test runs")
    }

    fn feed(&self) -> PathBuf {
        self.log().join("advisories")
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_feed_lists_every_advisory_and_agrees_with_the_log() {
    let setup = Setup::new("audit-feed-valid");
    let written = setup.advisories();
    assert!(written.status.success(), "{}", stderr(&written));
    assert!(
        stdout(&written).contains("lists 2 advisories in the log of 4 entries"),
        "{}",
        stdout(&written)
    );
    let feed = std::fs::read_to_string(setup.feed()).unwrap();
    assert!(feed.contains(r#""fixed_in":"1.1.1""#), "{feed}");
    assert!(feed.contains(r#""log":3"#), "{feed}");
    let verified = setup.verify();
    assert!(verified.status.success(), "{}", stderr(&verified));
    assert!(
        stdout(&verified).contains("lists 2 advisories, every one in the first 4 entries"),
        "{}",
        stdout(&verified)
    );
}

#[test]
fn a_feed_with_one_changed_byte_does_not_open() {
    let setup = Setup::new("audit-feed-changed");
    let written = setup.advisories();
    assert!(written.status.success(), "{}", stderr(&written));
    let feed = std::fs::read_to_string(setup.feed()).unwrap();
    let changed = feed.replacen("It deletes files.", "It deletes Files.", 1);
    assert_ne!(changed, feed);
    std::fs::write(setup.feed(), changed).unwrap();
    let verified = setup.verify();
    assert!(!verified.status.success());
    assert!(
        stderr(&verified).contains("its signature does not check"),
        "{}",
        stderr(&verified)
    );
}

/// A feed signed with the log's own key and checkpoint, which leaves the retraction out. `nvs
/// registry index sign` signs it as it signs any root file, so the signature is valid.
#[test]
fn a_validly_signed_feed_that_hides_an_advisory_is_an_error() {
    let setup = Setup::new("audit-feed-hidden");
    let site = setup.scratch.path().join("site");
    std::fs::create_dir(&site).unwrap();
    let forged = format!(r#"{{"schema":1,"advisories":[{{"log":3,"entry":{WITHHOLD}}}]}}"#);
    std::fs::write(site.join("registry.json"), forged).unwrap();
    let signed = setup
        .nvs()
        .args(["index", "sign"])
        .arg(&site)
        .arg("--key")
        .arg(setup.key())
        .arg("--log")
        .arg(setup.log())
        .output()
        .expect("the binary under test runs");
    assert!(signed.status.success(), "{}", stderr(&signed));
    std::fs::copy(site.join("registry.json"), setup.feed()).unwrap();
    let verified = setup.verify();
    assert!(!verified.status.success());
    assert!(
        stderr(&verified).contains("does not list record 1"),
        "{}",
        stderr(&verified)
    );
}

/// A feed written before the log grew lists the log as it was, and still agrees with it.
#[test]
fn a_feed_from_a_shorter_log_still_agrees_with_it() {
    let setup = Setup::new("audit-feed-shorter");
    let written = setup.advisories();
    assert!(written.status.success(), "{}", stderr(&written));
    let later =
        r#"{"v":1,"kind":"withhold","package":"acme/http","version":"1.3.0","reason":"Malware."}"#;
    let appended = setup.append(&[later]);
    assert!(appended.status.success(), "{}", stderr(&appended));
    let verified = setup.verify();
    assert!(verified.status.success(), "{}", stderr(&verified));
}
