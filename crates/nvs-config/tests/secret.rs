//! ADR 0103 § 7: a secret arrives as a file whose content is the value.
//!
//! Every case runs against an in-memory [`Files`] for the same reason `tests/resolve.rs` does, plus
//! one of its own: § 7's refusals are about *content* — empty, whitespace-only, oversized, not
//! UTF-8 — and a case that had to write those to a real directory would be asserting on what the
//! host's filesystem did with the bytes rather than on what the resolver made of them. The two
//! halves this reader cannot answer, whether a mode makes a file writable or readable by another
//! account, are `tests/trust.rs`'s and `crates/nvs-config/src/trust.rs`'s module doc.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nvs_config::resolve::{Files, Resolved, Roots, resolve};
use nvs_config::secret::MAX_SECRET_BYTES;
use nvs_config::trust::Untrusted;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// A path written the way an ADR writes one, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// The filesystem the cases describe, holding **bytes**: a secret file is not text until § 7 has
/// decided it is one.
#[derive(Default)]
struct Fake {
    files: BTreeMap<PathBuf, Vec<u8>>,
    untrusted: BTreeSet<PathBuf>,
    exposed: BTreeMap<PathBuf, String>,
}

impl Fake {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            files: entries
                .iter()
                .map(|(path, text)| (p(path), (*text).as_bytes().to_vec()))
                .collect(),
            ..Self::default()
        }
    }

    /// A file whose content is not text, which is the only way to ask § 7's UTF-8 question.
    fn raw(mut self, path: &str, bytes: &[u8]) -> Self {
        self.files.insert(p(path), bytes.to_vec());
        self
    }

    /// The paths whose § 6 check fails, the way a group-writable file's does.
    fn untrusting(mut self, paths: &[&str]) -> Self {
        self.untrusted = paths.iter().map(|path| p(path)).collect();
        self
    }

    /// The paths § 7's advisory has something to say about — a mode another account can read.
    fn exposing(mut self, path: &str, how: &str) -> Self {
        self.exposed.insert(p(path), how.to_string());
        self
    }
}

impl Files for Fake {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        if self.untrusted.contains(path) {
            return Err(Untrusted::Breach(format!(
                "`{}` is group-writable (mode 0775, gid 1000)",
                path.display()
            )));
        }
        if !self.exists(path) {
            return Err(Untrusted::Unreadable("no such file".to_string()));
        }
        Ok(path.to_path_buf())
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        let bytes = self.read_bytes(path)?;
        String::from_utf8(bytes).map_err(|_| "stream did not contain valid UTF-8".to_string())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_string())
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        self.exposed.get(path).cloned()
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(self
            .files
            .keys()
            .filter(|path| path.parent() == Some(dir))
            .cloned()
            .collect())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path) || self.files.keys().any(|file| file.starts_with(path))
    }
}

/// Resolves `root` in `fs`, panicking with the refusal when it does not.
fn tree_of(fs: &Fake, root: &str) -> Resolved {
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p(root)]), &mut sources, fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The refusal resolving `root` produces, panicking when it is accepted instead.
fn refusal(fs: &Fake, root: &str) -> Diagnostic {
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p(root)]), &mut sources, fs)
        .expect_err("this tree should have been refused")
}

/// `[db.main] password` out of a resolved tree.
fn password(resolved: &Resolved) -> Option<&str> {
    resolved.config.db.get("main")?.password.as_deref()
}

/// A root naming `secrets/db` as `[db.main]`'s password file, and nothing else.
const ROOT: &str = "[db.main]\ndriver = \"postgres\"\npassword_file = \"secrets/db\"\n";

/// § 7's central claim, and both halves of its one trimming rule: the file's **whole** content is
/// the value, minus one trailing newline, with the leading and trailing spaces a password may
/// legitimately carry left exactly where they were.
#[test]
fn the_files_whole_content_is_the_value_minus_one_trailing_newline() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", " hunt er2 \n")]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(password(&resolved), Some(" hunt er2 "));
    assert_eq!(
        resolved.config.db["main"].password_file.as_deref(),
        Some("secrets/db"),
        "the file stays named after the value is read: § 9's dump renders the value `<secret>` and \
         names where it came from, which it can only do if this is still here",
    );
    assert!(
        resolved.warnings.is_empty(),
        "nothing is advised about a secret file no other account can read",
    );
}

/// § 7 strips *one* newline, so a credential that genuinely ends in one is written with two. The
/// value-carrying half of this is a unit test in `src/secret.rs`; this is the same rule asserted
/// through the resolver, where the bytes actually arrive.
#[test]
fn a_second_trailing_newline_is_part_of_the_value() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", "hunter2\n\n")]);

    assert_eq!(password(&tree_of(&fs, "etc/nvs.toml")), Some("hunter2\n"));
}

/// § 7: exactly one of the pair may be set. Two sources for one value is the second spelling
/// ADR 0015 refuses, and the refusal names both so an operator can see which to remove.
#[test]
fn setting_both_halves_of_the_pair_is_refused() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[db.main]\npassword = \"inline\"\npassword_file = \"secrets/db\"\n",
        ),
        ("etc/secrets/db", "hunter2\n"),
    ]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SECRET_FILE));
    assert!(
        diagnostic.message.contains("password")
            && diagnostic.message.contains("password_file")
            && diagnostic.message.contains("db.main"),
        "the refusal names the block and both spellings: {}",
        diagnostic.message,
    );
    assert!(
        diagnostic.notes.join(" ").contains("etc"),
        "and the file it was written in: {:?}",
        diagnostic.notes,
    );
}

/// § 7's three content refusals, each of which would otherwise become a credential: an empty file,
/// a whitespace-only one, and one over the 64 KiB cap. All three are `E0608` and all three name the
/// path, because the operator's next move is to look at that file.
#[test]
fn an_empty_a_whitespace_only_and_an_oversized_file_are_all_refused() {
    let oversized = "x".repeat(MAX_SECRET_BYTES + 1);
    for (content, expected) in [
        ("", "is empty"),
        ("\n", "is empty"),
        ("   \t \n", "holds only whitespace"),
        (oversized.as_str(), "is larger than 64 KiB"),
    ] {
        let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", content)]);
        let diagnostic = refusal(&fs, "etc/nvs.toml");

        assert_eq!(diagnostic.code, Some(code::E_BAD_SECRET_FILE));
        assert!(
            diagnostic.message.contains(expected) && diagnostic.message.contains("db"),
            "expected `{expected}` naming the file, got: {}",
            diagnostic.message,
        );
    }
}

/// § 7: the content must be valid UTF-8 for a string-typed directive — and that is a refusal about
/// the file's *content*, `E0608`, rather than a failure to read it at all. Which is why the secret
/// file goes through `read_bytes`: a reader that decoded first could only report the second.
#[test]
fn a_secret_file_that_is_not_utf8_is_refused_as_content() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT)]).raw("etc/secrets/db", &[0xff, 0xfe, 0x00]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SECRET_FILE));
    assert!(
        diagnostic.message.contains("is not valid UTF-8"),
        "the message says what is wrong with the content: {}",
        diagnostic.message,
    );
}

/// § 7's permission split, enforced half: the secret file is a configuration input like any other,
/// so a file another account can write is `E0607` — whoever can rewrite it chooses the credential
/// the server connects with. Nothing in this module implements that; it is `Files::trust`, which is
/// the whole point of reading the secret through it.
#[test]
fn a_secret_file_another_account_can_write_refuses_the_boot() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", "hunter2\n")])
        .untrusting(&["etc/secrets/db"]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_UNTRUSTED_CONFIG));
    assert!(
        diagnostic.notes.join(" ").contains("credential"),
        "the note says why a secret file is held to the boundary: {:?}",
        diagnostic.notes,
    );
}

/// § 7's permission split, advised half: readable by another account is `W1005` and the boot goes
/// on with the value, because a Compose secret is mounted `0444`. A warning that stopped the boot
/// would refuse the normal deployment of every container secret store there is.
#[test]
fn a_secret_file_another_account_can_read_warns_and_the_value_still_resolves() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", "hunter2\n")])
        .exposing("etc/secrets/db", "is world-readable (mode 0644)");
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(password(&resolved), Some("hunter2"));
    let warning = resolved
        .warnings
        .first()
        .expect("§ 7 advises about a readable secret file");
    assert_eq!(warning.code, Some(code::W_SECRET_FILE_READABLE));
    assert!(
        warning.message.contains("mode 0644"),
        "the advisory names the mode it found: {}",
        warning.message,
    );
}

/// § 5 reaches § 7: a relative `password_file` resolves against the directory of the file it was
/// written in, not the working directory and not the root file's — so an included file may name a
/// secret beside itself.
#[test]
fn a_relative_secret_path_resolves_against_the_file_that_wrote_it() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"conf.d/db.toml\"\n"),
        (
            "etc/conf.d/db.toml",
            "[db.main]\npassword_file = \"secret\"\n",
        ),
        ("etc/conf.d/secret", "hunter2\n"),
        ("etc/secret", "the root's, which is not the one meant\n"),
    ]);

    assert_eq!(password(&tree_of(&fs, "etc/nvs.toml")), Some("hunter2"));
}

/// § 3 reaches § 7: the secret pass runs over the **flattened** tree, so a `password_file` a later
/// file replaced is never read. The loser here names a file that would fail the boundary if it were
/// opened, which is what makes this case say more than "the winning value won".
#[test]
fn only_the_password_file_that_won_the_merge_is_read() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"local.toml\"\n\n[db.main]\npassword_file = \"loser\"\n",
        ),
        ("etc/local.toml", "[db.main]\npassword_file = \"winner\"\n"),
        ("etc/loser", "never read\n"),
        ("etc/winner", "hunter2\n"),
    ])
    .untrusting(&["etc/loser"]);

    assert_eq!(password(&tree_of(&fs, "etc/nvs.toml")), Some("hunter2"));
}
