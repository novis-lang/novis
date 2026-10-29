//! The registry's advisory feed (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`),
//! through the built binary: a log built with `nvs registry log append`, its feed written with
//! `nvs registry log advisories`, and the feed checked against the log by `nvs registry log
//! verify`, which is the check `nvs pkg audit` makes.
//!
//! `nvs pkg audit` runs against a `file:` registry built the way `nvs pkg fetch`'s tests build one:
//! packages packed by `nvs pkg pack`, index files and the root file signed by `nvs registry index
//! sign`, and a log whose feed `nvs registry log advisories` writes. `nvs pkg fetch` writes the
//! lock before the log gets its advisory, as an application that locked a version before it went
//! bad. A retracted and a withheld locked version are each reported with the release that clears
//! them, and the command fails. An unlisted one is not an advisory, and a registry that names no
//! second copy of its checkpoint gets one note. A copy of the checkpoint over another log, shorter
//! or of the same size, stops the command, and `--json` names the advisory and the copy. An
//! autoload prefix the application's program maps and no grant line matches is a warning that
//! leaves the exit at zero (`rule:security/an-unmatched-namespace-holds-the-application`).
//!
//! The signing key is Go's `sumdb/note` test key, from the vendored vectors in
//! `tests/fixtures/tlog/note.json`, so no case needs a random source. Its name is the log's origin,
//! as `nvs registry log advisories` needs.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
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

/// A `file:` registry with `Acme\Http` 1.1.0 and 1.1.1 released, and an application whose lock
/// holds 1.1.0, written by `nvs pkg fetch`.
struct Locked {
    root: nvs_repo::Scratch,
    data: nvs_repo::Scratch,
    keys: nvs_repo::Scratch,
    /// The number of records in the registry's log.
    records: std::cell::Cell<u64>,
}

impl Locked {
    fn new(name: &str) -> Self {
        let keys = nvs_repo::scratch_private(&format!("{name}-key"));
        fs::write(keys.join("log.key"), SKEY).unwrap();
        let tree = Self {
            root: nvs_repo::scratch(name),
            data: nvs_repo::scratch_private(&format!("{name}-data")),
            keys,
            records: std::cell::Cell::new(0),
        };
        fs::create_dir_all(tree.registry()).unwrap();
        tree.root_file(None);
        fs::create_dir_all(tree.app()).unwrap();
        fs::write(
            tree.app().join("package.toml"),
            format!(
                "[dependencies]\n'Acme\\Http' = \"1.1.0\"\n\n[registries]\n\
                 Acme = {{ url = \"file:../registry\", key = \"{VKEY}\" }}\n"
            ),
        )
        .unwrap();
        tree.release("1.1.0");
        let fetched = tree.nvs(&["pkg", "fetch"]);
        assert!(fetched.status.success(), "{}", shown(&fetched));
        assert!(tree.app().join("package.lock").is_file());
        tree.release("1.1.1");
        tree
    }

    fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }

    fn app(&self) -> PathBuf {
        self.root.join("app")
    }

    fn key(&self) -> PathBuf {
        self.keys.join("log.key")
    }

    /// The registry's root file, naming `copy` as its `checkpoint_copy`, then signed.
    fn root_file(&self, copy: Option<&str>) {
        let mut root = serde_json::json!({ "schema": 1, "hosts": [] });
        if let Some(copy) = copy {
            root["checkpoint_copy"] = serde_json::json!(copy);
        }
        fs::write(self.registry().join("registry.json"), root.to_string()).unwrap();
        if self.records.get() > 0 {
            self.sign();
        }
    }

    /// `entry` appended to the log in `log` by the binary.
    fn append_to(&self, log: &Path, entry: &serde_json::Value) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "log", "append"])
            .arg(log)
            .arg("--key")
            .arg(self.key())
            .args(["--origin", ORIGIN])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary under test runs");
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{entry}").unwrap();
        drop(stdin);
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", shown(&out));
    }

    /// `entry` appended to the registry's log. Its record number.
    fn append(&self, entry: &serde_json::Value) -> u64 {
        self.append_to(&self.registry().join("log"), entry);
        let n = self.records.get();
        self.records.set(n + 1);
        n
    }

    /// `Acme\Http` at `version`, released, packed by the binary, published in the log and listed
    /// in its index file.
    fn release(&self, version: &str) {
        let dir = self.root.join("packages").join(version);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("package.toml"),
            format!(
                "[package]\nname = 'Acme\\Http'\nversion = \"{version}\"\n\n\
                 [capabilities]\nrequired = []\n"
            ),
        )
        .unwrap();
        fs::write(dir.join("LICENSE"), "MIT\n").unwrap();
        fs::write(
            dir.join("src").join("Main.nvs"),
            "<?nvs\nnamespace Acme\\Http;\n\nfinal class Main\n{\n}\n",
        )
        .unwrap();
        let tar = self.root.join("packages").join(format!("{version}.tar"));
        let packed = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["pkg", "pack"])
            .arg(&dir)
            .arg("--out")
            .arg(&tar)
            .output()
            .expect("the binary under test runs");
        assert!(packed.status.success(), "{}", shown(&packed));
        let bytes = fs::read(&tar).unwrap();
        let digest = blake3::hash(&bytes).to_hex().to_string();
        let archive = self.registry().join("archive");
        fs::create_dir_all(&archive).unwrap();
        fs::write(archive.join(format!("{digest}.tar")), &bytes).unwrap();
        let repository = "https://git.example.com/acme/http";
        let commit = "0".repeat(40);
        let record = self.append(&serde_json::json!({
            "v": 1, "kind": "publish", "package": "acme/http", "version": version,
            "digest": digest, "repository": repository, "commit": commit,
        }));
        let mut index = self.index();
        index["versions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "version": version,
                "digest": digest,
                "state": "released",
                "dependencies": {},
                "capabilities": { "required": [], "optional": [] },
                "repository": repository,
                "commit": commit,
                "log": record,
            }));
        self.write_index(&index);
    }

    fn index_path(&self) -> PathBuf {
        self.registry().join("index/acme/http.json")
    }

    /// The index file's JSON, without its signature.
    fn index(&self) -> serde_json::Value {
        fs::read_to_string(self.index_path()).map_or_else(
            |_| {
                serde_json::json!({
                    "schema": 1, "name": "Acme\\Http", "owners": ["owner"], "versions": []
                })
            },
            |note| serde_json::from_str(note.split("\n\n").next().unwrap()).unwrap(),
        )
    }

    /// `index` written and signed with every other file of the registry.
    fn write_index(&self, index: &serde_json::Value) {
        fs::create_dir_all(self.index_path().parent().unwrap()).unwrap();
        fs::write(self.index_path(), index.to_string()).unwrap();
        self.sign();
    }

    /// `entry` against 1.1.0 appended to the log, 1.1.0's index line set to `state`, and the
    /// feed written.
    fn advise(&self, entry: &serde_json::Value, state: &serde_json::Value) {
        self.append(entry);
        let mut index = self.index();
        for line in index["versions"].as_array_mut().unwrap() {
            if line["version"] == "1.1.0" {
                line["state"] = state.clone();
            }
        }
        self.write_index(&index);
        let written = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "log", "advisories"])
            .arg(self.registry().join("log"))
            .arg("--key")
            .arg(self.key())
            .output()
            .expect("the binary under test runs");
        assert!(written.status.success(), "{}", shown(&written));
    }

    /// The registry's root file and index files signed with the log's key.
    fn sign(&self) {
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "index", "sign"])
            .arg(self.registry())
            .arg("--key")
            .arg(self.key())
            .output()
            .expect("the binary under test runs");
        assert!(out.status.success(), "{}", shown(&out));
    }

    /// `nvs <args>` in the application's folder, with this tree's own data folder.
    fn nvs(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*self.data)
            .arg("--no-init")
            .args(args)
            .current_dir(self.app())
            .output()
            .expect("the binary under test runs")
    }
}

/// Both streams, for a message on failure.
fn shown(out: &Output) -> String {
    format!("{}{}", stdout(out), stderr(out))
}

fn retract() -> serde_json::Value {
    serde_json::json!({
        "v": 1, "kind": "retract", "package": "acme/http", "version": "1.1.0",
        "reason": "It deletes files.", "fixed_in": "1.1.1",
    })
}

fn retracted() -> serde_json::Value {
    serde_json::json!({ "retracted": { "reason": "It deletes files.", "fixed_in": "1.1.1" } })
}

// covers: tools:packages/nvs-pkg-audit
#[test]
fn audit_names_a_locked_retracted_version_and_the_release_that_fixes_it() {
    let tree = Locked::new("audit-retracted");
    tree.advise(&retract(), &retracted());
    let out = tree.nvs(&["pkg", "audit"]);
    assert!(!out.status.success(), "{}", shown(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "`Acme\\Http` 1.1.0 is retracted: It deletes files.\n  Update to 1.1.1 or newer."
        ),
        "{}",
        shown(&out)
    );
    assert!(
        text.contains("1 locked version has an advisory."),
        "{}",
        shown(&out)
    );
}

#[test]
fn audit_names_a_locked_withheld_version_and_the_release_that_clears_it() {
    let tree = Locked::new("audit-withheld");
    let withhold = serde_json::json!({
        "v": 1, "kind": "withhold", "package": "acme/http", "version": "1.1.0",
        "reason": "Malware.",
    });
    tree.advise(
        &withhold,
        &serde_json::json!({ "withheld": { "reason": "Malware." } }),
    );
    let out = tree.nvs(&["pkg", "audit"]);
    assert!(!out.status.success(), "{}", shown(&out));
    assert!(
        stdout(&out).contains(
            "`Acme\\Http` 1.1.0 is withheld by the registry: Malware.\n  Update to 1.1.1 or newer."
        ),
        "{}",
        shown(&out)
    );
}

/// An unlisted version is not an advisory. The registry names no second copy of its checkpoint,
/// so audit says so once and still succeeds.
#[test]
fn audit_is_silent_on_a_locked_unlisted_version_and_notes_a_missing_copy() {
    let tree = Locked::new("audit-unlisted");
    let unlist = serde_json::json!({
        "v": 1, "kind": "unlist", "package": "acme/http", "version": "1.1.0",
    });
    tree.advise(&unlist, &serde_json::json!("unlisted"));
    let out = tree.nvs(&["pkg", "audit"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        stdout(&out),
        "No locked package has an advisory.\n",
        "{}",
        shown(&out)
    );
    let errors = stderr(&out);
    let notes = errors
        .lines()
        .filter(|line| {
            line.starts_with("note: ") && line.contains("names no second copy of its checkpoint")
        })
        .count();
    assert_eq!(notes, 1, "{}", shown(&out));
}

/// A second copy of the checkpoint signed with the registry's key over another log, shorter or
/// of the same size, stops the command.
#[test]
fn audit_stops_when_the_copy_of_the_checkpoint_shows_another_log() {
    for records in [1, 3] {
        let tree = Locked::new(&format!("audit-copy-{records}"));
        let other = tree.root.join("other");
        for n in 0..records {
            tree.append_to(
                &other,
                &serde_json::json!({
                    "v": 1, "kind": "unlist", "package": "acme/other", "version": format!("1.0.{n}"),
                }),
            );
        }
        fs::create_dir_all(tree.root.join("copy")).unwrap();
        fs::copy(other.join("checkpoint"), tree.root.join("copy/checkpoint")).unwrap();
        tree.root_file(Some("file:../copy/checkpoint"));
        tree.advise(&retract(), &retracted());
        let out = tree.nvs(&["pkg", "audit"]);
        assert!(!out.status.success(), "{records}: {}", shown(&out));
        assert!(
            stderr(&out).contains("shows two different logs"),
            "{records}: {}",
            shown(&out)
        );
        assert!(
            !stdout(&out).contains("is retracted"),
            "{records}: {}",
            shown(&out)
        );
    }
}

#[test]
fn audit_json_names_the_advisory_and_the_matching_copy() {
    let tree = Locked::new("audit-json");
    tree.root_file(Some("file:../copy/checkpoint"));
    tree.advise(&retract(), &retracted());
    fs::create_dir_all(tree.root.join("copy")).unwrap();
    fs::copy(
        tree.registry().join("log/checkpoint"),
        tree.root.join("copy/checkpoint"),
    )
    .unwrap();
    let out = tree.nvs(&["pkg", "audit", "--json"]);
    assert!(!out.status.success(), "{}", shown(&out));
    assert!(!stderr(&out).contains("note:"), "{}", shown(&out));
    let report: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("audit prints JSON");
    assert_eq!(
        report,
        serde_json::json!({
            "advisories": [{
                "package": "Acme\\Http",
                "version": "1.1.0",
                "kind": "retract",
                "reason": "It deletes files.",
                "log": 2,
                "fixed_in": "1.1.1",
                "bump": "1.1.1",
            }],
            "registries": [{
                "url": "file:../registry",
                "size": 3,
                "copy": "file:../copy/checkpoint",
                "copy_size": 3,
            }],
            "unmatched": [],
        })
    );
}

/// The application's program autoloads the fetched `Acme\Http`, which `nvs pkg fetch` gave a grant
/// line, and a copied `Legacy\Pdf`, which has none. Only `Legacy\Pdf` is named, as a warning, and
/// the exit stays zero. A grant line for it makes audit silent.
#[test]
fn audit_warns_on_an_autoload_prefix_no_grant_line_matches() {
    let tree = Locked::new("audit-unmatched");
    let unlist = serde_json::json!({
        "v": 1, "kind": "unlist", "package": "acme/http", "version": "1.1.0",
    });
    tree.advise(&unlist, &serde_json::json!("unlisted"));
    fs::create_dir_all(tree.app().join("lib/pdf")).unwrap();
    fs::write(
        tree.app().join("main.nvs"),
        "<?nvs\nrequire './vendor/packages.nvs';\nautoload 'Legacy\\Pdf' from './lib/pdf';\n",
    )
    .unwrap();
    let out = tree.nvs(&["pkg", "audit"]);
    assert!(out.status.success(), "{}", shown(&out));
    let errors = stderr(&out);
    let warnings: Vec<&str> = errors
        .lines()
        .filter(|line| line.starts_with("warning: "))
        .collect();
    assert_eq!(
        warnings,
        [
            "warning: no line in `[grants]` matches `Legacy\\Pdf`, loaded from `lib/pdf`. Its code \
             has every permission your application has."
        ],
        "{}",
        shown(&out)
    );
    let json = tree.nvs(&["pkg", "audit", "--json"]);
    assert!(json.status.success(), "{}", shown(&json));
    let report: serde_json::Value =
        serde_json::from_str(&stdout(&json)).expect("audit prints JSON");
    assert_eq!(
        report["unmatched"],
        serde_json::json!([{ "prefix": "Legacy\\Pdf", "roots": ["lib/pdf"] }])
    );

    let manifest = tree.app().join("package.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    assert!(text.contains("[grants]\n"), "{text}");
    fs::write(
        &manifest,
        text.replace("[grants]\n", "[grants]\n'Legacy\\Pdf\\*' = []\n"),
    )
    .unwrap();
    let granted = tree.nvs(&["pkg", "audit"]);
    assert!(granted.status.success(), "{}", shown(&granted));
    assert!(
        !stderr(&granted).contains("warning:"),
        "{}",
        shown(&granted)
    );
}
