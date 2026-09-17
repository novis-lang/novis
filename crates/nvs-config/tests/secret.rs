//! `rule:config/a-secret-is-a-file-whose-content-is-the-value`: a secret arrives as a file whose content is the value.
//!
//! Every case runs against an in-memory [`Files`] for the same reason `tests/resolve.rs` does, plus
//! one of its own: § 7's refusals are about *content* — empty, whitespace-only, oversized, not
//! UTF-8 — and a case that had to write those to a real directory would be asserting on what the
//! host's filesystem did with the bytes rather than on what the resolver made of them. The two
//! halves this reader cannot answer, whether a mode makes a file writable or readable by another
//! account, are `tests/trust.rs`'s and `crates/nvs-config/src/trust.rs`'s module doc.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

    /// The path itself: no case here has an `[[app]]` block, and § 7's own paths are absolute.
    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        if self.exists(path) {
            Ok(path.to_path_buf())
        } else {
            Err("no such file".to_string())
        }
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
fn a_password_file_yields_its_content_with_one_trailing_newline_stripped() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", " hunt er2 \n")]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(password(&resolved), Some(" hunt er2 "));
    assert_eq!(
        resolved.config.db["main"].password_file.as_deref(),
        Some("secrets/db"),
        "the file stays named after the value is read: § 9's dump renders the value `<secret>` and \
         names where it came from, which it can only do if this is still here",
    );
    let codes: Vec<_> = resolved
        .warnings
        .iter()
        .map(|warning| warning.code)
        .collect();
    assert_eq!(
        codes,
        vec![Some(code::W_CREDENTIAL_HAS_EDGE_WHITESPACE)],
        "nothing is advised about a secret file no other account can read — but the edge spaces \
         this case exists to preserve are advised about, because preserving them and saying nothing \
         is how a working credential becomes an unexplained authentication failure",
    );
}

/// § 7 keeps a credential as it was written, so the only thing left to do about an edge space is to
/// name it: `W1007`, in both spellings of the pair, with the value untouched either way.
///
/// The two halves are one case because the rule is one sentence. The file half is the reason the
/// advisory exists — there is nothing in a file for an operator to look at — and the inline half is
/// the reason it has no exception, since a space before a closing quote is missed about as easily.
#[test]
fn a_credential_with_an_edge_space_is_kept_and_advised_in_both_spellings() {
    let from_a_file = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", "hunter2 \n")]);
    let resolved = tree_of(&from_a_file, "etc/nvs.toml");

    assert_eq!(password(&resolved), Some("hunter2 "));
    let warning = resolved
        .warnings
        .iter()
        .find(|warning| warning.code == Some(code::W_CREDENTIAL_HAS_EDGE_WHITESPACE))
        .expect("the edge space is advised");
    assert!(
        warning.message.contains("db.main.password") && warning.message.contains("ends with"),
        "the advisory names the directive and which end: {}",
        warning.message,
    );
    assert!(
        !warning.message.contains("hunter2") && !warning.notes.join(" ").contains("hunter2"),
        "and never the credential itself: {} {:?}",
        warning.message,
        warning.notes,
    );

    let inline = Fake::with(&[(
        "etc/nvs.toml",
        "[db.main]\ndriver = \"postgres\"\npassword = \" hunter2\"\n",
    )]);
    let resolved = tree_of(&inline, "etc/nvs.toml");

    assert_eq!(
        password(&resolved),
        Some(" hunter2"),
        "the inline half is not repaired either",
    );
    assert!(
        resolved.warnings.iter().any(|warning| warning.code
            == Some(code::W_CREDENTIAL_HAS_EDGE_WHITESPACE)
            && warning.message.contains("begins with")),
        "and it is advised about, naming the end it is on: {:?}",
        resolved.warnings,
    );
}

/// The ordinary credential says nothing at all. An advisory an operator sees on every boot is one
/// they stop reading, so the case that matters most for `W1007` is the one where it is silent —
/// including § 7's own `echo secret > file`, whose one trailing newline is removed before this
/// question is asked.
#[test]
fn an_ordinary_credential_raises_no_advisory() {
    let fs = Fake::with(&[("etc/nvs.toml", ROOT), ("etc/secrets/db", "hunter2\n")]);

    assert!(
        tree_of(&fs, "etc/nvs.toml").warnings.is_empty(),
        "a credential with nothing wrong with it is not worth a line of a boot log",
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
/// `rule:statements/nothing-gets-a-second-name` refuses, and the refusal names both so an operator can see which to remove.
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

/// § 7 reaches a block that is not a map: `[http.client.proxy]`'s credential is the same pair in a
/// block an operator does not name, so the file half is read into the value half and setting both
/// is the same refusal a `[db.<name>]` gets.
#[test]
fn a_proxy_password_and_its_file_together_are_refused() {
    const BLOCK: &str = "[http.client.proxy]\nurl = \"http://proxy.internal:3128\"\n\
                         resolve = \"local\"\nusername = \"app\"\n";

    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            &format!("{BLOCK}password = \"inline\"\npassword_file = \"secrets/proxy\"\n"),
        ),
        ("etc/secrets/proxy", "hunter2\n"),
    ]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SECRET_FILE));
    assert!(
        diagnostic.message.contains("password")
            && diagnostic.message.contains("password_file")
            && diagnostic.message.contains("http.client.proxy"),
        "the refusal names the block and both spellings: {}",
        diagnostic.message,
    );

    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            &format!("{BLOCK}password_file = \"secrets/proxy\"\n"),
        ),
        ("etc/secrets/proxy", "hunter2\n"),
    ]);
    assert_eq!(
        tree_of(&fs, "etc/nvs.toml")
            .config
            .http
            .and_then(|http| http.client)
            .and_then(|client| client.proxy)
            .and_then(|proxy| proxy.password)
            .as_deref(),
        Some("hunter2"),
        "the file half never reached the tree",
    );
}

/// § 7's content refusals, each of which would otherwise become a credential: an empty file, a
/// whitespace-only one, and one over the 64 KiB cap. Each is `E0608` and each names the path,
/// because the operator's next move is to look at that file.
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

/// § 7 reaches the reader: the value survives into the snapshot a request reads, and does **not**
/// arrive there through the merged table.
///
/// Both halves are the finding this case was written for. The snapshot deserializes its typed tree
/// out of the table — twice, if a reload carries a `Boot` value across — and the table holds the
/// `_file` sibling and no content, so a `password` put on the resolved tree and nowhere else is
/// dropped the moment the snapshot is built. `Snapshot::secrets` is where it survives, which is also
/// why the second assertion is not merely a redaction check: there is nothing in that table to
/// redact, so `dump --toml` cannot write a credential into a file an operator diffs.
#[test]
fn a_secret_survives_into_the_snapshot_without_entering_the_table() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", ROOT),
        ("etc/secrets/db", "hunter2\n"),
        ("srv/app.nvs", "<?nvs\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");
    let snapshot = nvs_config::Snapshot::build(&resolved, &p("srv/app.nvs"), &fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes));

    assert_eq!(
        snapshot.config.db["main"].password.as_deref(),
        Some("hunter2"),
        "the typed tree a driver reads has the value, past the retype that builds it",
    );
    assert_eq!(
        nvs_config::Request::new(Arc::clone(&snapshot)).get("db.main.password"),
        Some("hunter2".to_string()),
        "and so does `Core\\Config::get`, which reads the key the value is filed under",
    );
    assert!(
        !toml::to_string(&snapshot.table)
            .expect("the merged table serializes")
            .contains("hunter2"),
        "the content is carried beside the table and never in it — `dump --toml` is that table",
    );
}

/// A root naming `secrets/mail` as `[mail.relay]`'s password file, with the rest of the block
/// `rule:programs/framework-core-half` needs so that `nvs_stdlib::mail` would accept it.
const MAIL_ROOT: &str = "[mail.relay]\nhost = \"smtp.internal\"\nfrom = \"app@example.test\"\nuser = \"app\"\npassword_file = \"secrets/mail\"\n";

/// § 7 is about the pairs the registry marks, and `[mail.<name>] password` is one of them: an
/// SMTP submission credential is written by an operator into a named block and injected into a
/// container as a file, exactly as a database password is.
///
/// The assertion on `secrets` is the one that matters most, and it is not a duplicate of the one on
/// the typed tree. That map keyed by the *value's* key is what `nvs config dump` renders `<secret>`,
/// what `Snapshot::retype` puts back, and what `Core\Config::get` — and therefore
/// `nvs_stdlib::mail`'s own `configured` — answers out of before it ever consults the table. A value
/// that reached the typed tree and not this map would work in the resolver and vanish at the first
/// reload.
#[test]
fn a_mail_endpoints_password_arrives_as_a_file_too() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", MAIL_ROOT),
        ("etc/secrets/mail", "relay-hunter2\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(
        resolved.config.mail["relay"].password.as_deref(),
        Some("relay-hunter2"),
    );
    assert_eq!(
        resolved.secrets["mail.relay.password"].value, "relay-hunter2",
        "filed under the key the value is of, which is what every reader past the resolver asks for",
    );
    assert_eq!(
        resolved.config.mail["relay"].password_file.as_deref(),
        Some("secrets/mail"),
        "and the file stays named, so § 9's dump can say where the credential came from",
    );
}

/// § 7's one-of-the-pair rule is the registry's and not `[db]`'s: the same refusal, naming the block
/// that actually set both.
#[test]
fn setting_both_halves_of_a_mail_endpoints_pair_is_refused() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[mail.relay]\npassword = \"inline\"\npassword_file = \"secrets/mail\"\n",
        ),
        ("etc/secrets/mail", "relay-hunter2\n"),
    ]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SECRET_FILE));
    assert!(
        diagnostic.message.contains("mail.relay"),
        "the refusal names the block that set both, not the pair's first registry row: {}",
        diagnostic.message,
    );
}

/// A root naming `secrets/cache` as the shared store's password file, beside the URL that says
/// which store it is the credential for.
const CACHE_ROOT: &str =
    "[cache.shared]\nurl = \"redis://cache.internal\"\npassword_file = \"secrets/cache\"\n";

/// § 7 reaches the coherent tier: `[cache.shared] password` is a pair like every other, in a block
/// an operator does not name, so `AUTH` carries a credential a container mounted as a file.
///
/// The `secrets` assertion is the one that matters, for the reason the mail case gives — that map
/// is what `Core\Config::get` answers out of, and it is where `nvs_stdlib::cache` reads the
/// credential it puts into the dial. A value that reached the typed tree alone would be dialled
/// with at boot and gone at the first reload.
#[test]
fn the_shared_stores_password_arrives_as_a_file_too() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", CACHE_ROOT),
        ("etc/secrets/cache", "cache-hunter2\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");
    let shared = resolved
        .config
        .cache
        .as_ref()
        .and_then(|cache| cache.shared.as_ref());

    assert_eq!(
        shared.and_then(|shared| shared.password.as_deref()),
        Some("cache-hunter2"),
    );
    assert_eq!(
        resolved.secrets["cache.shared.password"].value, "cache-hunter2",
        "filed under the key the value is of — the block names no segment, so that key is the \
         block's own",
    );
    assert_eq!(
        shared.and_then(|shared| shared.password_file.as_deref()),
        Some("secrets/cache"),
        "and the file stays named, so § 9's dump can say where the credential came from",
    );
}

/// § 7's one-of-the-pair rule over a block with no name in it: the same refusal a `[db.<name>]`
/// gets, naming `cache.shared` itself.
#[test]
fn setting_both_halves_of_the_shared_stores_pair_is_refused() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[cache.shared]\npassword = \"inline\"\npassword_file = \"secrets/cache\"\n",
        ),
        ("etc/secrets/cache", "cache-hunter2\n"),
    ]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SECRET_FILE));
    assert!(
        diagnostic.message.contains("cache.shared"),
        "the refusal names the block that set both: {}",
        diagnostic.message,
    );
}

/// The credential-shaped field names this census recognizes.
///
/// A list rather than a heuristic, and deliberately a short one: it is what to extend when a
/// deployment starts writing a credential under a name that is not here. Extending it is loud rather
/// than optional, because the assertion below fails the moment such a field exists without its
/// `_file` sibling.
const CREDENTIAL_FIELDS: &[&str] = &[
    "password",
    "passphrase",
    "secret",
    "token",
    "api_key",
    "access_key",
    "private_key",
    "csrf_key",
];

/// Every `pub struct` in the config tree, as its name and the field names it declares.
///
/// Read out of the source because there is no other way to ask: the tree is a set of `serde` structs
/// with no runtime field roster, and the whole point of this census is to catch a field that was
/// added without the row that reads it.
fn tree_structs() -> Vec<(String, Vec<String>)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tree.rs");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));

    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("pub struct ") {
            let name = rest
                .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .next()
                .unwrap_or_default();
            out.push((name.to_string(), Vec::new()));
        } else if line == "}" {
            // Whatever item just ended, nothing after it belongs to it.
            out.push((String::new(), Vec::new()));
        } else if let Some(rest) = line.strip_prefix("    pub ")
            && let Some((field, _)) = rest.split_once(':')
            && let Some((_, fields)) = out.last_mut()
        {
            fields.push(field.to_string());
        }
    }
    out.retain(|(name, _)| !name.is_empty());
    out
}

/// `rule:config/a-secret-is-a-file-whose-content-is-the-value` marks *directives*, plural: a credential added to the typed tree with no
/// [`SECRETS`] row is a `_file` sibling that silently does nothing.
///
/// Nothing about such an omission fails to compile, and no behavioural case can find it, because
/// the missing code and the missing case are the same absence. So this asserts both directions
/// over the tree's own source: a credential field implies a `_file` sibling and a row, and a row
/// implies a credential field that has one.
#[test]
fn every_credential_on_the_tree_has_a_file_sibling_and_a_secrets_row() {
    let structs = tree_structs();
    assert!(
        structs.iter().any(|(name, _)| name == "Database"),
        "the census read no structs at all, so it is asserting nothing: {structs:?}",
    );

    let mut found: BTreeSet<String> = BTreeSet::new();
    for (name, fields) in &structs {
        for credential in CREDENTIAL_FIELDS {
            if !fields.iter().any(|field| field == credential) {
                continue;
            }
            found.insert((*credential).to_string());
            let sibling = format!("{credential}_file");
            assert!(
                fields.contains(&sibling),
                "`{name}` holds a `{credential}` and no `{sibling}`: `rule:config/a-secret-is-a-file-whose-content-is-the-value` gives every \
                 credential a file half, and a deployment that injects secrets as files cannot \
                 reach this one at all",
            );
            assert!(
                nvs_config::secret::SECRETS
                    .iter()
                    .any(|pair| pair.value == *credential),
                "`{name}` holds a `{credential}` with no row in `secret::SECRETS`, so its \
                 `{sibling}` parses, dumps, and is never read",
            );
        }
    }

    for pair in nvs_config::secret::SECRETS {
        assert!(
            found.contains(pair.value),
            "`SECRETS` marks `{}` in `{}` and the tree declares no such credential field — either \
             the field was renamed or `CREDENTIAL_FIELDS` above no longer recognizes it",
            pair.value,
            pair.block,
        );
        let segments: Vec<&str> = pair.block.split('.').collect();
        let named = segments.iter().filter(|segment| **segment == "*").count();
        assert!(
            named <= 1,
            "a row's block names at most one operator-chosen segment — a map's, or none at all for \
             a block that is a single one: `{}`",
            pair.block,
        );
        assert_eq!(
            pair.key("main"),
            format!("{}.{}", pair.block_of("main"), pair.value)
        );
        assert!(
            named == 1 || pair.block_of("main") == pair.block,
            "a block with no `*` has nowhere to put a name, so every key it builds is its own: `{}`",
            pair.block,
        );
        assert_eq!(pair.file_key("main"), format!("{}_file", pair.key("main")));
    }
}
