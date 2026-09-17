//! `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s tree: which file is the root, which files it reaches, and what the one ordered stream
//! they flatten to says.
//!
//! Every case runs against an in-memory [`Files`], not a temporary directory. That is not a
//! convenience: § 2 **mandates** the order a `dir` include is read in rather than inheriting
//! `readdir`'s, so a case that got its order from a real filesystem would pass by accident on the
//! host that happens to return entries sorted and prove nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nvs_config::resolve::{Files, MAX_INCLUDE_DEPTH, Resolved, Roots, resolve, roots};
use nvs_config::trust::Untrusted;
use nvs_config::{Setting, tree};
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// A path written the way an ADR writes one, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// The filesystem the cases describe: a name-to-text map, with directories implied by it.
#[derive(Default)]
struct Fake {
    files: BTreeMap<PathBuf, String>,
    untrusted: BTreeSet<PathBuf>,
    unreadable: BTreeSet<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

impl Fake {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            files: entries
                .iter()
                .map(|(path, text)| (p(path), (*text).to_string()))
                .collect(),
            untrusted: BTreeSet::new(),
            unreadable: BTreeSet::new(),
            links: BTreeMap::new(),
        }
    }

    /// The paths whose § 6 check fails, the way a group-writable file's does.
    fn untrusting(mut self, paths: &[&str]) -> Self {
        self.untrusted = paths.iter().map(|path| p(path)).collect();
        self
    }

    /// The paths that are **there** and still cannot be read, the way a file whose mode denies this
    /// account is. It has to exist for the distinction to mean anything: `optional` is a statement
    /// about absence, so only a present file can tell absence and unreadability apart.
    fn unreadable(mut self, paths: &[&str]) -> Self {
        self.unreadable = paths.iter().map(|path| p(path)).collect();
        self
    }

    /// `link` is another name for `target`, the way a symlink is.
    fn linking(mut self, link: &str, target: &str) -> Self {
        self.links.insert(p(link), p(target));
        self
    }

    /// What a name resolves to, which is the only thing the resolver ever compares: the real
    /// [`Files::trust`] hands back a canonical path, so a fake that skipped this would make every
    /// case here a statement about paths no symlink was involved in.
    fn target(&self, path: &Path) -> PathBuf {
        self.links
            .get(path)
            .cloned()
            .unwrap_or_else(|| path.to_path_buf())
    }
}

impl Files for Fake {
    /// Exactly the paths a case named, and never their parents: the real reader checks the
    /// containing directory too, so a fake that inferred it would leave a case unable to say which
    /// of the two it meant.
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        let path = self.target(path);
        if self.untrusted.contains(&path) {
            return Err(Untrusted::Breach(format!(
                "`{}` is group-writable (mode 0775, gid 1000)",
                path.display()
            )));
        }
        if !self.exists(&path) {
            return Err(Untrusted::Unreadable("no such file".to_string()));
        }
        Ok(path)
    }

    /// A link followed, and nothing else: a case here writes paths with no `..` in them, and
    /// `tests/app.rs` is where the whole resolving walk is exercised.
    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        let path = self.target(path);
        if self.exists(&path) {
            Ok(path)
        } else {
            Err("no such file".to_string())
        }
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        if self.unreadable.contains(path) {
            return Err("permission denied".to_string());
        }
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_string())
    }

    /// Whatever `read` says, as bytes: § 7's content refusals are `tests/secret.rs`'s, and a case
    /// here only ever names configuration files, which are text by construction.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.read(path).map(String::into_bytes)
    }

    /// Nothing is exposed: § 7's advisory is about a secret file and no case here has one.
    fn exposure(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Deliberately **reverse** sorted, so a case that passes is one where the resolver did the
    /// ordering § 2 mandates rather than one where the reader happened to.
    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut entries: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|path| path.parent() == Some(dir))
            .cloned()
            .collect();
        entries.reverse();
        Ok(entries)
    }

    /// A directory exists when the map holds anything under it, which is every directory a case has
    /// a reason to name — and a link exists when its target does, the way `Path::exists` is: it
    /// `stat`s rather than `lstat`s.
    fn exists(&self, path: &Path) -> bool {
        let path = self.target(path);
        self.files.contains_key(&path) || self.files.keys().any(|file| file.starts_with(&path))
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

/// `[http.client.tls] roots` out of a resolved tree, which four blocks of `and_then` otherwise
/// spell at each of its cases.
fn roots_of(resolved: &Resolved) -> Option<Vec<String>> {
    resolved
        .config
        .http
        .as_ref()?
        .client
        .as_ref()?
        .tls
        .as_ref()?
        .roots
        .clone()
}

/// `[limits] memory` out of a resolved tree, which is the value most cases assert on.
fn memory(resolved: &Resolved) -> Option<&Setting> {
    resolved.config.limits.as_ref()?.memory.as_ref()
}

/// § 1's whole ladder: `--config` is repeatable and ordered, and **any** explicit `--config`
/// disables the `./nvs.toml` step entirely — an operator naming files never gets a surprise merge
/// with whatever is in the working directory — while a directory holding no file falls through to
/// the shipped defaults rather than to a failure.
#[test]
fn a_root_is_named_by_config_else_found_else_defaulted() {
    let fs = Fake::with(&[("app/nvs.toml", ""), ("etc/base.toml", "")]);

    assert_eq!(
        roots(&[p("etc/base.toml"), p("app/nvs.toml")], Path::new(""), &fs),
        Roots::Files(vec![p("etc/base.toml"), p("app/nvs.toml")]),
        "the flags are the list, in the order given",
    );
    assert_eq!(
        roots(&[], &p("app"), &fs),
        Roots::Files(vec![p("app/nvs.toml")]),
        "with no flag, exactly this directory's file",
    );
    assert_eq!(
        roots(&[p("etc/base.toml")], &p("app"), &fs),
        Roots::Files(vec![p("app/etc/base.toml")]),
        "a flag disables step 2 even though `app/nvs.toml` is right there — and the flag itself \
         resolves against the working directory, because that is what a shell argument means (§ 5)",
    );
    assert_eq!(
        roots(&[], &p("elsewhere"), &fs),
        Roots::Defaults,
        "step 3: no flag and no file here is a configuration, not a refusal",
    );
}

/// § 1 steps 2 and 3: `./nvs.toml` is **exactly one directory, never a walk upward**, so a nested
/// directory falls through to the shipped defaults rather than finding its parent's file.
#[test]
fn the_local_file_is_not_searched_for_upward() {
    let fs = Fake::with(&[("app/nvs.toml", "")]);

    assert_eq!(roots(&[], &p("app/sub"), &fs), Roots::Defaults);
}

/// § 1 step 3: nothing found is a complete and valid configuration, not a failure.
#[test]
fn the_shipped_defaults_resolve_to_the_default_tree() {
    let mut sources = SourceMap::new();
    let resolved = resolve(&Roots::Defaults, &mut sources, &Fake::default())
        .expect("the shipped defaults are a configuration");

    assert_eq!(resolved.config, nvs_config::Config::default());
    assert!(resolved.files.is_empty());
    assert!(resolved.overrides.is_empty());
}

/// § 1 step 3 from the other side: the file this project ships is the shipped defaults written out,
/// so a directory that took it resolves to what that directory resolved to while it was empty. That
/// equality is the whole licence for a command to write the file without being asked, and it holds
/// only while every key in it is commented out — one live key moves this assertion and nothing else
/// in the tree would notice.
///
/// `files` is where the two part company, and deliberately: a tree that read a file names it, which
/// is what `nvs config dump` reports and what the boot line announces.
#[test]
fn the_default_file_resolves_to_the_same_snapshot_as_no_file_at_all() {
    let fs = Fake::with(&[("app/nvs.toml", nvs_config::default_file())]);
    let took_the_file = tree_of(&fs, "app/nvs.toml");

    let mut sources = SourceMap::new();
    let took_nothing = resolve(&Roots::Defaults, &mut sources, &Fake::default())
        .expect("the shipped defaults are a configuration");

    assert_eq!(
        took_the_file.config, took_nothing.config,
        "the written file says exactly what no file says",
    );
    assert_eq!(took_the_file.config, nvs_config::Config::default());
    assert!(
        took_the_file.overrides.is_empty(),
        "a file that assigns nothing overrides nothing: {:?}",
        took_the_file.overrides,
    );
}

/// § 3: an include overrides the file that pulled it in — the base-plus-local shape, with the
/// include line at the point the operator wants overridden — and **both origins are recorded**.
/// That record is the whole of what makes later-wins acceptable here, so it is asserted, not the
/// winning value alone.
#[test]
fn a_key_set_in_two_files_resolves_to_the_later_one_with_both_origins_reported() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"local.toml\"\n\n[limits]\nmemory = \"128M\"\n",
        ),
        ("etc/local.toml", "[limits]\nmemory = \"512M\"\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(memory(&resolved), Some(&Setting::Text("512M".to_string())));
    assert_eq!(resolved.files, vec![p("etc/nvs.toml"), p("etc/local.toml")]);

    let record = resolved
        .overrides
        .first()
        .expect("§ 3 requires the override to be recorded");
    assert_eq!(record.key, "limits.memory");
    assert_eq!(record.replaced.path, p("etc/nvs.toml"));
    assert_eq!(record.winner.path, p("etc/local.toml"));
    assert_eq!(
        resolved.overrides.len(),
        1,
        "a key written once in each of two files is one override, not two",
    );
}

/// § 3: the stream is the root's own keys, *then* its includes depth-first in list order. Asserted
/// on the file list rather than on a value, because a resolver that read the right files in the
/// wrong order still answers plausibly for any single key.
#[test]
fn includes_are_depth_first_in_list_order_after_the_files_own_keys() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"a.toml\"\n\n[[include]]\npath = \"c.toml\"\n",
        ),
        ("etc/a.toml", "[[include]]\npath = \"b.toml\"\n"),
        ("etc/b.toml", ""),
        ("etc/c.toml", ""),
    ]);

    assert_eq!(
        tree_of(&fs, "etc/nvs.toml").files,
        vec![
            p("etc/nvs.toml"),
            p("etc/a.toml"),
            p("etc/b.toml"),
            p("etc/c.toml"),
        ],
    );
}

/// § 2: a `dir` include reads every `*.toml` **directly** inside, ascending by filename, without
/// recursing and without touching anything that is not a `.toml`. The fake reader hands them back
/// reversed on purpose.
#[test]
fn a_dir_include_is_sorted_shallow_and_toml_only() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\ndir = \"conf.d\"\n"),
        ("etc/conf.d/20-db.toml", ""),
        ("etc/conf.d/10-limits.toml", ""),
        ("etc/conf.d/README.md", "not toml"),
        ("etc/conf.d/nested/30-more.toml", ""),
    ]);

    assert_eq!(
        tree_of(&fs, "etc/nvs.toml").files,
        vec![
            p("etc/nvs.toml"),
            p("etc/conf.d/10-limits.toml"),
            p("etc/conf.d/20-db.toml"),
        ],
    );
}

/// § 4, both sides of the split named together: `key = [...]` is one value and is replaced
/// wholesale, so the last file that mentions a grant states the whole grant; `[[table]]` entries
/// accumulate, because two of them in one file already mean two.
///
/// The appending block is `[[extension]]` rather than `[[schedule]]` because `rule:config/scheduled-work-is-a-config-block` refuses a
/// half-written schedule entry at boot ([`nvs_config::schedule`]), and a case about the merge must
/// not be able to fail for a reason the merge had no part in.
#[test]
fn a_value_array_replaces_where_a_table_appends() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"local.toml\"\n\n[capabilities]\nscript.spawn = [\"/srv/a\", \"/srv/b\"]\n\n[[extension]]\npath = \"one.nvsx\"\n",
        ),
        (
            "etc/local.toml",
            "[capabilities]\nscript.spawn = [\"/srv/c\"]\n\n[[extension]]\npath = \"two.nvsx\"\n",
        ),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(
        resolved
            .config
            .capabilities
            .and_then(|caps| caps.script)
            .and_then(|script| script.spawn),
        Some(Setting::List(vec!["/srv/c".to_string()])),
        "no reader should have to assemble an effective root list out of four files",
    );
    assert_eq!(
        resolved
            .config
            .extension
            .iter()
            .filter_map(|entry| entry.path.as_deref())
            .collect::<Vec<_>>(),
        vec!["one.nvsx", "two.nvsx"],
    );
}

/// § 5: a relative path resolves against the directory of the file it is written in, which is the
/// only rule under which a config directory survives being copied or relocated whole.
#[test]
fn a_relative_path_resolves_against_the_file_it_is_written_in() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"conf.d/db.toml\"\n"),
        (
            "etc/conf.d/db.toml",
            "[[include]]\npath = \"deeper/more.toml\"\n",
        ),
        (
            "etc/conf.d/deeper/more.toml",
            "[limits]\nmemory = \"64M\"\n",
        ),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(memory(&resolved), Some(&Setting::Text("64M".to_string())));
    assert_eq!(resolved.files.len(), 3);
}

/// § 5 again, on the key `rule:core-classes/db-capabilities` makes a whole database out of: a `[db.<name>] path` written in
/// an included file names a file beside *that* file, not beside whatever directory a request happens
/// to be running in. It is asserted here rather than in `tests/db.rs` because the rule under test is
/// § 5's and not § 3's — `crates/nvs-config/src/db.rs`'s module doc owns why the bundle beside it is
/// additionally trust-checked and this one is not.
#[test]
fn a_db_blocks_path_resolves_against_the_file_it_is_written_in() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"conf.d/db.toml\"\n"),
        (
            "etc/conf.d/db.toml",
            "[db.main]\ndriver = \"sqlite\"\npath = \"main.db\"\n\
             [db.memory]\ndriver = \"sqlite\"\npath = \":memory:\"\n",
        ),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");
    let want = p("etc/conf.d/main.db");

    assert_eq!(
        resolved.config.db["main"].path.as_deref(),
        Some(want.to_string_lossy().as_ref())
    );
    // The table as well as the typed tree, and this half is the one a driver actually reads:
    // `Snapshot` deserializes itself out of the table, so a pass that rewrote only the tree above
    // would resolve nothing that any connection ever saw.
    assert_eq!(
        resolved.table["db"]["main"]["path"].as_str(),
        Some(want.to_string_lossy().as_ref())
    );
    // The other side of the bound, because a pass that resolved everything spelled into `path`
    // would look right on the line above and open nothing: `:memory:` is not a file name, and a
    // directory in front of it is a file name nothing can open.
    assert_eq!(
        resolved.table["db"]["memory"]["path"].as_str(),
        Some(":memory:")
    );
}

/// § 5 for the other path a `[db]` block can carry, and § 6 for what makes it the sharper case: a
/// `tls_ca_file` is trust-checked at the *resolved* path, so a pass that rewrote the typed tree alone
/// would prove one file safe and hand the driver a different one to open.
#[test]
fn a_db_blocks_tls_ca_file_reaches_the_table_resolved_too() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"conf.d/db.toml\"\n"),
        (
            "etc/conf.d/db.toml",
            "[db.main]\ndriver = \"postgres\"\nhost = \"db.internal\"\n\
             tls_ca_file = \"ca.pem\"\n",
        ),
        ("etc/conf.d/ca.pem", "-----BEGIN CERTIFICATE-----\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");
    let want = p("etc/conf.d/ca.pem");

    assert_eq!(
        resolved.config.db["main"].tls_ca_file.as_deref(),
        Some(want.to_string_lossy().as_ref())
    );
    // The half a driver reads: `Snapshot::retype` deserializes `Config` back out of the table, so
    // the tree above is a view that a reload rebuilds and this line is the one that says the
    // handshake verifies against the bundle the trust check actually examined.
    assert_eq!(
        resolved.table["db"]["main"]["tls_ca_file"].as_str(),
        Some(want.to_string_lossy().as_ref())
    );
}

/// `[http.client.tls] roots` resolved: a file entry becomes the path the trust check examined, in
/// the table as well as in the typed tree, and `"bundled"` is passed over untouched.
///
/// The table half is the one that matters — `Snapshot::retype` deserializes the tree back out of it,
/// so a pass that rewrote only the typed side would prove one bundle safe and hand `nvs_host::tls` a
/// relative path to open against whatever directory the process happened to start in.
#[test]
fn http_client_tls_roots_reach_the_table_resolved_and_leave_bundled_alone() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"conf.d/tls.toml\"\n"),
        (
            "etc/conf.d/tls.toml",
            "[http.client.tls]\nroots = [\"bundled\", \"corp-ca.pem\"]\n",
        ),
        ("etc/conf.d/corp-ca.pem", "-----BEGIN CERTIFICATE-----\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");
    let want = p("etc/conf.d/corp-ca.pem");
    let want = want.to_string_lossy().into_owned();

    assert_eq!(
        roots_of(&resolved),
        Some(vec!["bundled".to_string(), want.clone()]),
        "the typed tree kept a relative bundle path or lost the compiled-in entry",
    );
    let table = resolved.table["http"]["client"]["tls"]["roots"]
        .as_array()
        .expect("`roots` left the table as something other than an array")
        .iter()
        .map(|entry| entry.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(table, vec!["bundled".to_string(), want]);
}

/// The trust boundary over the anchors, which is the sharper half of the case above:
/// `rule:config/ownership-is-the-trust-boundary` asks who *else* may write a file, and a `roots`
/// entry decides which servers every outbound call in the process may be talking to.
///
/// So it is refused exactly as a `[db]` block's `tls_ca_file` is, and by the same `E0607`: a boot
/// that accepted it would leave one file in the tree a stranger could rewrite to redirect the
/// deployment's traffic to a server they hold a certificate for.
#[test]
fn a_group_writable_roots_file_is_refused_at_boot() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[http.client.tls]\nroots = [\"bundled\", \"corp-ca.pem\"]\n",
        ),
        ("etc/corp-ca.pem", "-----BEGIN CERTIFICATE-----\n"),
    ])
    .untrusting(&["etc/corp-ca.pem"]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_UNTRUSTED_CONFIG));
    assert!(
        diagnostic.message.contains("corp-ca.pem"),
        "the refusal did not name the file: {}",
        diagnostic.message
    );
}

/// An empty `roots` is refused rather than read as "trust nothing", because trusting nothing is not
/// a policy a deployment can run: every outbound `https` call would die at the handshake, naming an
/// unknown issuer and sending an operator to look at the origin.
#[test]
fn an_empty_roots_list_is_refused_rather_than_trusting_nobody() {
    let fs = Fake::with(&[("etc/nvs.toml", "[http.client.tls]\nroots = []\n")]);

    assert_eq!(
        refusal(&fs, "etc/nvs.toml").code,
        Some(code::E_TLS_ROOTS_EMPTY)
    );
}

/// A `min_version` beneath what the client implements is refused rather than clamped: accepting
/// `"1.0"` would leave the real floor at 1.2 while the file told an operator they had chosen
/// otherwise, which is the one outcome worse than a boot that stops.
#[test]
fn a_min_version_below_what_the_client_speaks_is_refused() {
    let fs = Fake::with(&[("etc/nvs.toml", "[http.client.tls]\nmin_version = \"1.0\"\n")]);

    assert_eq!(
        refusal(&fs, "etc/nvs.toml").code,
        Some(code::E_TLS_MIN_VERSION)
    );

    let raised = Fake::with(&[("etc/nvs.toml", "[http.client.tls]\nmin_version = \"1.3\"\n")]);
    assert_eq!(
        tree_of(&raised, "etc/nvs.toml")
            .config
            .http
            .as_ref()
            .unwrap()
            .client
            .as_ref()
            .unwrap()
            .tls
            .as_ref()
            .unwrap()
            .min_version
            .as_deref(),
        Some("1.3"),
        "the floor an operator raised did not survive the merge",
    );
}

/// `[http.client.tls] keylog` on a `production` host is refused, and the mode with nothing written
/// is `production` — so the tree below names no mode at all.
///
/// That is the pairing the key needs: the file it names decrypts everything this deployment sends,
/// and a deployment that never wrote `[mode]` is the one most likely to have inherited the key from
/// a development tree it copied.
#[test]
fn keylog_is_refused_at_boot_in_production() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[http.client.tls]\nkeylog = \"/tmp/tls-secrets.log\"\n",
    )]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_KEYLOG_IN_PRODUCTION));
    assert!(
        diagnostic.message.contains("tls-secrets.log"),
        "the refusal did not name the file it writes: {}",
        diagnostic.message
    );
}

/// The other side of it: a `development` host boots with the key, and says so.
///
/// A warning rather than silence because the state is one somebody switched on for an afternoon and
/// nobody reports having switched off, and `W1009` is what an operator reads at every start until
/// they remove it.
#[test]
fn keylog_is_accepted_in_development() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[mode]\ndefault = \"development\"\n\n\
         [http.client.tls]\nkeylog = \"/tmp/tls-secrets.log\"\n",
    )]);

    let resolved = tree_of(&fs, "etc/nvs.toml");

    let warned = resolved
        .warnings
        .iter()
        .find(|warning| warning.code == Some(code::W_TLS_KEYLOG_ON))
        .expect("a development host took the key log and announced nothing");
    assert!(
        warned.message.contains("tls-secrets.log"),
        "the announcement did not name the file: {}",
        warned.message
    );
}

/// A `[http.client.proxy]` block that never says who resolves the destination is refused, and the
/// refusal names both words.
///
/// Neither is a default, because both are commonly right: `local` fails outright where only the
/// proxy can resolve a name, and `proxy` gives up the address pin everywhere else. Naming both is
/// what makes the refusal actionable — the deployment's own network is what picks between them.
#[test]
fn a_proxy_block_without_resolve_refuses_the_boot_naming_both_words() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[http.client.proxy]\nurl = \"http://proxy.internal:3128\"\n",
    )]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_PROXY_RESOLVE_MISSING));
    let said = format!("{} {}", diagnostic.message, diagnostic.notes.join(" "));
    assert!(
        said.contains("local") && said.contains("proxy"),
        "the refusal did not name both words: {said}",
    );
}

/// A third word is refused rather than read as either of the two, because the choice between them
/// is a security posture and neither is a spelling to recover from a typo.
#[test]
fn a_third_resolve_word_is_refused() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[http.client.proxy]\nurl = \"http://proxy.internal:3128\"\nresolve = \"auto\"\n",
    )]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_PROXY_RESOLVE_UNKNOWN));
    assert!(
        diagnostic.message.contains("auto"),
        "the refusal did not name the word written: {}",
        diagnostic.message
    );
}

/// Every `url` this client cannot dial, each refused with the value named.
///
/// The block with no `url` at all is in the list on purpose: leaving the whole block out is how a
/// deployment asks for no proxy, so a written block that names no address is one believing its
/// egress is tunnelled when nothing tunnels it. A credential in the URL is refused for the sibling
/// reason — `username` and `password` are where one is read from, so this one would be dropped in
/// silence.
#[test]
fn a_proxy_url_this_client_cannot_dial_is_refused() {
    for written in [
        "",
        "url = \"https://proxy.internal:3128\"\n",
        "url = \"socks5://proxy.internal:1080\"\n",
        "url = \"http://\"\n",
        "url = \"http://app:s3cret@proxy.internal:3128\"\n",
        "url = \"http://proxy.internal:0\"\n",
        "url = \"http://proxy.internal:http\"\n",
    ] {
        let fs = Fake::with(&[(
            "etc/nvs.toml",
            &format!("[http.client.proxy]\nresolve = \"local\"\n{written}"),
        )]);

        assert_eq!(
            refusal(&fs, "etc/nvs.toml").code,
            Some(code::E_PROXY_URL_UNDIALABLE),
            "`{written}` was taken as a proxy this client can dial",
        );
    }

    // The two shapes that are dialable, one of them an IPv6 literal with no port.
    for written in ["http://proxy.internal:3128", "http://[::1]"] {
        let fs = Fake::with(&[(
            "etc/nvs.toml",
            &format!("[http.client.proxy]\nresolve = \"local\"\nurl = \"{written}\"\n"),
        )]);

        assert_eq!(
            tree_of(&fs, "etc/nvs.toml")
                .config
                .http
                .and_then(|http| http.client)
                .and_then(|client| client.proxy)
                .and_then(|proxy| proxy.url)
                .as_deref(),
            Some(written),
        );
    }
}

/// A `bypass` entry that is not a host name is refused, naming the entry.
///
/// Matching happens against the URL's own text before anything is resolved, so an entry carrying a
/// port or a scheme can never equal a host — a bypass an operator believes is in force — and a
/// wildcard or a range is more traffic leaving unproxied than they can see they asked for.
#[test]
fn a_bypass_entry_that_is_not_a_host_name_is_refused() {
    for entry in [
        "internal.example.com:443",
        "http://internal.example.com",
        "*.example.com",
        "10.0.0.0/8",
        "",
    ] {
        let fs = Fake::with(&[(
            "etc/nvs.toml",
            &format!(
                "[http.client.proxy]\nurl = \"http://proxy.internal:3128\"\n\
                 resolve = \"local\"\nbypass = [\"{entry}\"]\n"
            ),
        )]);

        let diagnostic = refusal(&fs, "etc/nvs.toml");

        assert_eq!(
            diagnostic.code,
            Some(code::E_PROXY_BYPASS_ENTRY),
            "`{entry}` was taken as a host name",
        );
        assert!(
            diagnostic.message.contains(entry),
            "the refusal did not name the entry: {}",
            diagnostic.message
        );
    }

    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[http.client.proxy]\nurl = \"http://proxy.internal:3128\"\n\
         resolve = \"local\"\nbypass = [\"localhost\", \".internal.example.com\"]\n",
    )]);
    assert!(
        tree_of(&fs, "etc/nvs.toml")
            .warnings
            .iter()
            .all(|warning| warning.code != Some(code::W_PROXY_RESOLVES_THE_DESTINATION)),
        "a `local` tree announced a narrowing it did not make",
    );
}

/// `resolve = "proxy"` boots, and says at every start that the address question has moved.
///
/// At every start and not once per deployment: a tree that has run this way for a year still owes
/// today's log the sentence, because the alternative is an auditor reading a boot that claims
/// `rule:security/net-address-policy` is in force when what is in force is the proxy's answer.
#[test]
fn resolve_proxy_writes_one_warn_at_every_boot() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[http.client.proxy]\nurl = \"http://proxy.internal:3128\"\nresolve = \"proxy\"\n",
    )]);

    for boot in 0..2 {
        let resolved = tree_of(&fs, "etc/nvs.toml");
        let announced: Vec<&Diagnostic> = resolved
            .warnings
            .iter()
            .filter(|warning| warning.code == Some(code::W_PROXY_RESOLVES_THE_DESTINATION))
            .collect();

        assert_eq!(announced.len(), 1, "boot {boot} announced {announced:?}");
        let said = format!("{} {}", announced[0].message, announced[0].notes.join(" "));
        assert!(
            said.contains("resolve")
                && said.contains("proxy")
                && said.contains("net-address-policy"),
            "the announcement named neither the word nor the rule it narrows: {said}",
        );
    }
}

/// § 2: a cycle is refused with the chain named. The chain and not merely the repeated file, because
/// an operator shown only the file that repeated has to rediscover how it was reached.
#[test]
fn an_include_cycle_is_refused_with_the_chain_named() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"a.toml\"\n"),
        ("etc/a.toml", "[[include]]\npath = \"b.toml\"\n"),
        ("etc/b.toml", "[[include]]\npath = \"a.toml\"\n"),
    ]);
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_INCLUDE_CYCLE));
    let chain = diagnostic.notes.join(" ");
    for file in ["a.toml", "b.toml"] {
        assert!(
            chain.contains(file),
            "the chain should name {file}: {chain}"
        );
    }
}

/// The same refusal when the ring is built out of a **symlink**, which is the case a lexical path
/// comparison cannot see: `link.toml` and `a.toml` share no spelling, so nothing here closes unless
/// the name compared is the one [`Files::trust`] handed back. That is why `same_file` is allowed to
/// be lexical — the canonicalization happened at the boundary, one file earlier.
#[test]
fn a_cycle_built_out_of_a_symlink_is_refused_with_the_chain_named() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"a.toml\"\n"),
        ("etc/a.toml", "[[include]]\npath = \"link.toml\"\n"),
    ])
    .linking("etc/link.toml", "etc/a.toml");
    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_INCLUDE_CYCLE));
    let chain = diagnostic.notes.join(" ");
    assert!(
        chain.contains("a.toml"),
        "the chain names the file the link resolved to, which is the one that repeated: {chain}",
    );
    assert!(
        !diagnostic.message.contains(&MAX_INCLUDE_DEPTH.to_string()),
        "a symlinked ring is caught by name and not by the depth cap: {}",
        diagnostic.message,
    );
}

/// § 2's depth cap: an include tree too deep to read, whether or not it ever closes. A ring is not
/// what this catches — the case above is — so a tree that nests past the cap without repeating a
/// file is the one shape left for it.
#[test]
fn nesting_past_the_cap_is_refused() {
    let mut entries: Vec<(String, String)> = Vec::new();
    for depth in 0..=MAX_INCLUDE_DEPTH + 2 {
        entries.push((
            format!("etc/{depth}.toml"),
            format!("[[include]]\npath = \"{}.toml\"\n", depth + 1),
        ));
    }
    entries.push((format!("etc/{}.toml", MAX_INCLUDE_DEPTH + 3), String::new()));
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();

    let diagnostic = refusal(&Fake::with(&borrowed), "etc/0.toml");
    assert_eq!(diagnostic.code, Some(code::E_INCLUDE_CYCLE));
    assert!(
        diagnostic.message.contains(&MAX_INCLUDE_DEPTH.to_string()),
        "the refusal should say what the cap is: {}",
        diagnostic.message,
    );
}

/// § 6: `optional = true` covers **absence and nothing else**, and every half is asserted
/// together — a resolver that skipped every failing include would pass on the absent-and-optional
/// one alone. The unreadable file is the one a naive implementation gets wrong: a file that is
/// *there* and cannot be read is a hard refusal even under `optional`, because otherwise a stray
/// `chmod` silently drops half a configuration and the server comes up looking healthy.
#[test]
fn optional_covers_absence_and_not_unreadability() {
    let present = Fake::with(&[(
        "etc/nvs.toml",
        "[[include]]\npath = \"gone.toml\"\noptional = true\n\n[limits]\nmemory = \"1M\"\n",
    )]);
    let resolved = tree_of(&present, "etc/nvs.toml");
    assert_eq!(memory(&resolved), Some(&Setting::Text("1M".to_string())));
    assert_eq!(resolved.files, vec![p("etc/nvs.toml")]);

    let absent = Fake::with(&[("etc/nvs.toml", "[[include]]\npath = \"gone.toml\"\n")]);
    let diagnostic = refusal(&absent, "etc/nvs.toml");
    assert_eq!(diagnostic.code, Some(code::E_UNREADABLE_CONFIG));
    assert!(
        diagnostic.message.contains("gone.toml"),
        "the refusal names the path: {}",
        diagnostic.message,
    );

    let denied = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"local.toml\"\noptional = true\n",
        ),
        ("etc/local.toml", "[limits]\nmemory = \"128M\"\n"),
    ])
    .unreadable(&["etc/local.toml"]);
    let diagnostic = refusal(&denied, "etc/nvs.toml");
    assert_eq!(diagnostic.code, Some(code::E_UNREADABLE_CONFIG));
    assert!(
        diagnostic
            .message
            .contains(&p("etc/local.toml").display().to_string()),
        "`optional` does not cover a file that is there and unreadable: {}",
        diagnostic.message,
    );
}

/// § 1: a `--config` naming a file that does not exist is **always** a hard refusal. Optionality is
/// a property a file declares about its own includes, never something argv can assert.
#[test]
fn a_config_flag_naming_a_missing_file_is_a_hard_refusal() {
    let diagnostic = refusal(&Fake::default(), "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_UNREADABLE_CONFIG));
}

/// § 2: an entry carries `path` **or** `dir`, never both and never neither. A shape TOML cannot
/// express, so it is a refusal rather than a type.
#[test]
fn an_include_entry_carrying_both_or_neither_is_refused() {
    for entry in [
        "[[include]]\npath = \"a.toml\"\ndir = \"conf.d\"\n",
        "[[include]]\noptional = true\n",
    ] {
        let fs = Fake::with(&[("etc/nvs.toml", entry)]);
        let diagnostic = refusal(&fs, "etc/nvs.toml");
        assert_eq!(
            diagnostic.code,
            Some(code::E_BAD_DIRECTIVE),
            "for {entry:?}"
        );
    }
}

/// `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s refusals stay **per file** across an include: the same key in two files is an
/// override, and twice in one file is still an error — asserted as one pair, because a resolver that
/// merged first and typed afterwards would get the first half right and lose the second entirely.
#[test]
fn the_per_file_refusals_survive_the_merge() {
    let across = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"local.toml\"\n\n[limits]\nmemory = \"128M\"\n",
        ),
        ("etc/local.toml", "[limits]\nmemory = \"256M\"\n"),
    ]);
    assert_eq!(
        memory(&tree_of(&across, "etc/nvs.toml")),
        Some(&Setting::Text("256M".to_string())),
    );

    let within = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"local.toml\"\n"),
        (
            "etc/local.toml",
            "[limits]\nmemory = \"128M\"\nmemory = \"256M\"\n",
        ),
    ]);
    let diagnostic = refusal(&within, "etc/nvs.toml");
    assert_eq!(diagnostic.code, Some(code::E_DUPLICATE_DIRECTIVE));
}

/// An unknown key inside an *included* file is refused against that file's own text, so the line and
/// the block named belong to the file the operator has to edit.
#[test]
fn an_unknown_key_in_an_included_file_names_that_file() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"local.toml\"\n"),
        ("etc/local.toml", "[limits]\nmemry = \"128M\"\n"),
    ]);
    let mut sources = SourceMap::new();
    let diagnostic = resolve(&Roots::Files(vec![p("etc/nvs.toml")]), &mut sources, &fs)
        .expect_err("an unknown key is refused wherever it is written");

    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("[limits]")),
        "the block travels with the refusal: {:?}",
        diagnostic.notes,
    );
    let span = diagnostic
        .primary_span()
        .expect("and so does the line it was written on");
    assert_eq!(
        sources.file(span.file).name(),
        p("etc/local.toml").display().to_string(),
        "the span points into the included file, not the root",
    );
}

/// A block written in one file and extended in another merges rather than replacing: two files each
/// setting a different key of `[limits]` leave both set.
#[test]
fn two_files_setting_different_keys_of_one_block_both_survive() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"local.toml\"\n\n[limits]\nmemory = \"128M\"\n",
        ),
        ("etc/local.toml", "[limits]\ncpu_time = \"5s\"\n"),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");
    let limits: tree::Limits = resolved.config.limits.expect("the block survives");

    assert_eq!(limits.memory, Some(Setting::Text("128M".to_string())));
    assert_eq!(limits.cpu_time, Some(Setting::Text("5s".to_string())));
    assert!(
        resolved.overrides.is_empty(),
        "neither key replaced the other, so there is nothing to report",
    );
}

/// § 6: a file some other account can write refuses the boot rather than being read, and the
/// refusal is `E0607` naming **that** file — not the `E0605` an unreadable one gets, because an
/// operator told "cannot read" goes looking for a typo when the answer is a mode. **A directory an
/// `[[include]]` reads is asked the same question**, and it is asserted here beside the file
/// because the two are one boundary: a check that held only for files would leave every `dir`
/// include a slot, and it would still look right on the file half alone.
#[test]
fn a_group_writable_file_or_directory_refuses_the_boot() {
    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\npath = \"local.toml\"\n"),
        ("etc/local.toml", "[limits]\nmemory = \"128M\"\n"),
    ])
    .untrusting(&["etc/local.toml"]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_UNTRUSTED_CONFIG));
    assert!(
        diagnostic
            .message
            .contains(&p("etc/local.toml").display().to_string()),
        "the refusal names the file that fails the check, not the root that pulled it in: {}",
        diagnostic.message,
    );

    let fs = Fake::with(&[
        ("etc/nvs.toml", "[[include]]\ndir = \"conf.d\"\n"),
        ("etc/conf.d/host.toml", "[limits]\nmemory = \"128M\"\n"),
    ])
    .untrusting(&["etc/conf.d"]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_UNTRUSTED_CONFIG));
    assert!(
        diagnostic
            .message
            .contains(&p("etc/conf.d").display().to_string()),
        "the directory is checked before anything in it is read: {}",
        diagnostic.message,
    );
}

/// § 6: an absent `optional` include puts the check on the directory that would hold it — asserted
/// on both sides, because a check that refused every optional include would pass the first half of
/// this alone. The directory is a standing slot, and the promise that a file appearing there later
/// will be trusted is only keepable where the file is not.
#[test]
fn an_absent_optional_include_checks_the_directory_that_would_hold_it() {
    let tree = &[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"conf.d/local.toml\"\noptional = true\n\n\
             [limits]\nmemory = \"128M\"\n",
        ),
        ("etc/conf.d/other.toml", ""),
    ];

    let resolved = tree_of(&Fake::with(tree), "etc/nvs.toml");
    assert_eq!(
        memory(&resolved),
        Some(&Setting::Text("128M".to_string())),
        "with the directory inside the boundary, the absent file is simply absent",
    );

    let diagnostic = refusal(
        &Fake::with(tree).untrusting(&["etc/conf.d"]),
        "etc/nvs.toml",
    );
    assert_eq!(diagnostic.code, Some(code::E_UNTRUSTED_CONFIG));
    assert!(
        diagnostic
            .message
            .contains(&p("etc/conf.d").display().to_string()),
        "the refusal is about the slot, so it names the directory: {}",
        diagnostic.message,
    );
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("standing slot")),
        "and says why a directory is being checked for an absent file: {:?}",
        diagnostic.notes,
    );
}

/// § 6, continued: when the directory an `optional` include names does not exist either, the check
/// walks up to the nearest one that does. The promise is only as strong as the shallowest directory
/// an attacker would have to write to keep it.
#[test]
fn an_optional_include_below_an_absent_directory_checks_the_nearest_one_that_exists() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[[include]]\npath = \"conf.d/local.toml\"\noptional = true\n",
    )])
    .untrusting(&["etc"]);

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_UNTRUSTED_CONFIG));
    assert!(
        diagnostic.message.contains(&p("etc").display().to_string()),
        "`etc/conf.d` is not there to check, so the check is on `etc`: {}",
        diagnostic.message,
    );
}

/// A tree whose only variable is its `[[schedule]]` block: one granted root with a script under it,
/// and a second script outside every root for the cases that need somewhere to point at.
fn scheduling(block: &str) -> Fake {
    let root = format!("[capabilities]\nscript.spawn = [\"etc/jobs\"]\n\n{block}");
    Fake::with(&[
        ("etc/nvs.toml", root.as_str()),
        ("etc/jobs/report.nvs", "<?nvs\n"),
        ("etc/elsewhere/report.nvs", "<?nvs\n"),
    ])
}

/// One entry with every key § 1 requires, `scope` last so a case can replace or drop it.
fn entry(scope: &str) -> String {
    format!(
        "[[schedule]]\nname = \"nightly\"\ncron = \"0 3 * * *\"\nscript = \"jobs/report.nvs\"\n{scope}"
    )
}

/// `rule:config/scheduled-work-is-a-config-block` and `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`: `scope` has no default, so an entry without one refuses the boot rather than
/// having this file pick a coordination model for the operator. Asserted on both sides and with the
/// third answer beside them — a check that only refused the *absent* key would pass just as well if
/// `scope` were read as free text, and `"cluster"` is what that bug would look like.
#[test]
fn a_schedule_entry_with_no_scope_refuses_the_boot() {
    let fs = scheduling(&entry(""));

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SCHEDULE));
    assert!(
        diagnostic.message.contains("nightly") && diagnostic.message.contains("scope"),
        "the refusal names the entry and the key it wants: {}",
        diagnostic.message,
    );

    let fs = scheduling(&entry("scope = \"cluster\"\n"));
    assert_eq!(
        refusal(&fs, "etc/nvs.toml").code,
        Some(code::E_BAD_SCHEDULE),
        "`fleet` and `host` are the whole set, so a third word is not a scope either",
    );

    let fs = scheduling(&entry("scope = \"host\"\n"));
    assert_eq!(
        tree_of(&fs, "etc/nvs.toml").config.schedule.len(),
        1,
        "the same entry with the key it was missing is armed",
    );
}

/// § 2: the dialect is five-field POSIX cron plus the five shorthands, and the exclusions are the
/// decision. Asserted by **counting** over both tables rather than by reading one expression, so a
/// validator that happens to answer plausibly for the first row still fails here.
#[test]
fn a_malformed_cron_refuses_the_boot() {
    let refused = [
        "0 3 * * * *",   // a seconds field: the dialect is five, and a sixth makes it a timer
        "0 3 * *",       // and four is not the dialect either
        "60 3 * * *",    // minutes are 0-59, so 60 is the off-by-one a bound catches
        "0 3 * * 7",     // Sunday is 0; `7` is Vixie's second spelling for it
        "*/0 3 * * *",   // a step of nothing never comes round
        "5/15 3 * * *",  // Quartz's "every 15 from 5"; § 2 takes the range it is short for
        "0 3 * * MON#2", // and none of Quartz's `L`/`W`/`#`/`?`
        "@fortnightly",  // not one of the five shorthands
    ];
    let accepted = [
        "0 3 * * *",
        "*/15 * * * *",
        "0 0 1-5,10 JAN-MAR MON-FRI",
        "59 23 31 12 6",
        "@daily",
    ];

    let caught = refused
        .iter()
        .filter(|expression| {
            let fs = scheduling(&format!(
                "[[schedule]]\nname = \"j\"\ncron = \"{expression}\"\nscript = \"jobs/report.nvs\"\nscope = \"host\"\n"
            ));
            refusal(&fs, "etc/nvs.toml").code == Some(code::E_BAD_SCHEDULE)
        })
        .count();
    assert_eq!(
        caught,
        refused.len(),
        "every expression outside the dialect refuses the boot, and refuses it as E0611",
    );

    let armed = accepted
        .iter()
        .filter(|expression| {
            let fs = scheduling(&format!(
                "[[schedule]]\nname = \"j\"\ncron = \"{expression}\"\nscript = \"jobs/report.nvs\"\nscope = \"host\"\n"
            ));
            tree_of(&fs, "etc/nvs.toml").config.schedule.len() == 1
        })
        .count();
    assert_eq!(
        armed,
        accepted.len(),
        "the dialect is what § 2 says it is, names and steps and bounds included",
    );
}

/// § 6: the zone is pinned because implementations differ about DST and the difference is a
/// production incident, so a `timezone` no IANA database knows refuses the boot rather than falling
/// back to UTC and running the job at the wrong hour — which nothing observes, since a schedule
/// fails silently. Asserted on both sides and with the absent key beside them: a check that only
/// refused the unknown name would pass just as well if `timezone` were never read at all.
#[test]
fn an_unknown_timezone_refuses_the_boot() {
    let fs = scheduling(&entry("scope = \"host\"\ntimezone = \"Mars/Olympus\"\n"));

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SCHEDULE));
    assert!(
        diagnostic.message.contains("nightly") && diagnostic.message.contains("Mars/Olympus"),
        "the refusal names the entry and the zone it cannot find: {}",
        diagnostic.message,
    );

    let fs = scheduling(&entry("scope = \"host\"\ntimezone = \"Europe/Vienna\"\n"));
    assert_eq!(
        tree_of(&fs, "etc/nvs.toml").config.schedule.len(),
        1,
        "a zone the database carries is armed",
    );

    let fs = scheduling(&entry("scope = \"host\"\n"));
    assert_eq!(
        tree_of(&fs, "etc/nvs.toml").config.schedule.len(),
        1,
        "and an entry that names no zone is UTC, which needs no database at all",
    );
}

/// § 1: a scheduled script is checked against the same `[capabilities] script.spawn` roots a `spawn
/// script` target is, at boot rather than at the first fire. The `..` row is the one that matters:
/// the comparison is canonicalize-then-prefix, so a path that *spells* itself inside a root and
/// resolves outside one is refused on where it lands.
#[test]
fn a_scheduled_script_outside_the_spawn_roots_refuses_the_boot() {
    let inside = scheduling(&entry("scope = \"host\"\n"));
    assert_eq!(tree_of(&inside, "etc/nvs.toml").config.schedule.len(), 1);

    for script in ["elsewhere/report.nvs", "jobs/../elsewhere/report.nvs"] {
        let fs = scheduling(&format!(
            "[[schedule]]\nname = \"nightly\"\ncron = \"@daily\"\nscript = \"{script}\"\nscope = \"host\"\n"
        ));

        let diagnostic = refusal(&fs, "etc/nvs.toml");

        assert_eq!(diagnostic.code, Some(code::E_BAD_SCHEDULE));
        assert!(
            diagnostic.message.contains("script.spawn"),
            "`{script}` is refused for being outside the roots, and the refusal says which list it \
             failed: {}",
            diagnostic.message,
        );
    }
}

/// § 3: `fleet` fires once across the deployment under a lease in the shared store, and a tree with
/// no store configured refuses rather than degrading to one run per host — which is the exact
/// failure the key exists to prevent. The store is `[cache.shared] url`, `rule:core-api/two-cache-tiers`'s coherent
/// tier and the one `Core\Cache::shared()` opens, so the two sides are asserted together: a tree
/// that writes it accepts the same entry the tree without it refused. Asserted on both sides
/// because a check that only refused would pass just as well if `fleet` were refused
/// unconditionally, which is what `nvs_config::schedule::configures_a_shared_store` answered while
/// no block spelled the store.
#[test]
fn a_fleet_scope_with_no_shared_store_refuses_the_boot() {
    let fs = scheduling(&entry("scope = \"fleet\"\n"));

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SCHEDULE));
    assert!(
        diagnostic.message.contains("shared store"),
        "the refusal names what is missing rather than the key that asked for it: {}",
        diagnostic.message,
    );

    let fs = scheduling(&entry("scope = \"host\"\n"));
    assert_eq!(
        tree_of(&fs, "etc/nvs.toml").config.schedule.len(),
        1,
        "the store is `fleet`'s dependency and nothing else's",
    );

    let with_store = format!(
        "[cache.shared]\nurl = \"redis://127.0.0.1:6379\"\n\n{}",
        entry("scope = \"fleet\"\n")
    );
    assert_eq!(
        tree_of(&scheduling(&with_store), "etc/nvs.toml")
            .config
            .schedule
            .len(),
        1,
        "`[cache.shared] url` is the store § 3's lease lives in, so a `fleet` entry beside one boots",
    );

    let empty_url = format!(
        "[cache.shared]\nurl = \"\"\n\n{}",
        entry("scope = \"fleet\"\n")
    );
    assert!(
        refusal(&scheduling(&empty_url), "etc/nvs.toml")
            .message
            .contains("shared store"),
        "a block written with an empty `url` has named no store, and saying nothing is not \
         configuring one",
    );
}

/// The index the shared store's entries live in is a key of its own beside the URL, so the one
/// value has one spelling: `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s
/// block says where the store is, and a path on the URL would be a second answer to which database
/// this deployment means.
///
/// Asserted beside a block that writes only the URL, because a check that read a configured index
/// back would pass just as well if every tree carried one — and a deployment that names no index is
/// every deployment on this chain today, dialling a store that switches to nothing.
#[test]
fn a_cache_shared_database_index_is_read_beside_the_url() {
    let fs = Fake::with(&[(
        "etc/nvs.toml",
        "[cache.shared]\nurl = \"redis://127.0.0.1:6379\"\ndatabase = 3\n",
    )]);
    let shared = |resolved: &Resolved| {
        resolved
            .config
            .cache
            .as_ref()
            .and_then(|cache| cache.shared.as_ref())
            .expect("the block was written")
            .database
    };

    assert_eq!(shared(&tree_of(&fs, "etc/nvs.toml")), Some(3));

    let bare = Fake::with(&[(
        "etc/nvs.toml",
        "[cache.shared]\nurl = \"redis://127.0.0.1:6379\"\n",
    )]);
    assert_eq!(
        shared(&tree_of(&bare, "etc/nvs.toml")),
        None,
        "a block that names no index reaches the store's own default and sends no `SELECT`",
    );
}

/// `rule:config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once`: `overlap` has a default, and a word that is not one of its three still refuses the
/// boot. Asserted with the default and both named modes beside the refusal, because a check that
/// only refused would pass just as well if the key were refused whenever it was written at all —
/// and `queue` and `kill` are configurations the ADR states, so refusing them would be the same
/// silent failure in the other direction.
#[test]
fn an_overlap_that_is_none_of_the_three_refuses_the_boot() {
    let fs = scheduling(&entry("scope = \"host\"\noverlap = \"replace\"\n"));

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_BAD_SCHEDULE));
    assert!(
        diagnostic.message.contains("nightly") && diagnostic.message.contains("overlap"),
        "the refusal names the entry and the key that cannot be read: {}",
        diagnostic.message,
    );

    for written in [
        "",
        "overlap = \"skip\"\n",
        "overlap = \"queue\"\n",
        "overlap = \"kill\"\n",
    ] {
        let fs = scheduling(&entry(&format!("scope = \"host\"\n{written}")));
        assert_eq!(
            tree_of(&fs, "etc/nvs.toml").config.schedule.len(),
            1,
            "§ 6 names three modes and gives an entry that writes none of them `skip`: {written:?}",
        );
    }
}

/// A tree whose only content is `block`, for the `[http]` pairs `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` refuses.
fn http(block: &str) -> Fake {
    Fake::with(&[("etc/nvs.toml", block)])
}

/// `rule:http-server/cors-is-closed-until-origins-are-named`: `origins = ["*"]` is permitted **only** with `credentials = false`, so the pair is
/// what refuses and neither half does alone. Asserted on all four combinations, because a check
/// reading one key would refuse the wildcard public API § 2 explicitly allows and still pass a case
/// that only tried the bad pair.
#[test]
fn cors_star_origins_with_credentials_true_is_refused() {
    let fs = http("[http.cors]\norigins = [\"*\"]\ncredentials = true\n");

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_MEANINGLESS_HTTP_PAIR));
    assert!(
        diagnostic.message.contains("credentials") && diagnostic.message.contains('*'),
        "the refusal names both halves, because either one is the one to change: {}",
        diagnostic.message,
    );

    let allowed = [
        // § 2's own sentence: the wildcard is a legitimate public API without credentials.
        "[http.cors]\norigins = [\"*\"]\ncredentials = false\n",
        // Credentials are legitimate too, for origins named exactly.
        "[http.cors]\norigins = [\"https://app.example\"]\ncredentials = true\n",
        // And the shipped default is closed, which is neither half of the pair.
        "[http.cors]\ncredentials = true\n",
    ];
    let accepted = allowed
        .iter()
        .filter(|block| tree_of(&http(block), "etc/nvs.toml").config.http.is_some())
        .count();
    assert_eq!(
        accepted,
        allowed.len(),
        "the combination is refused and neither half of it is",
    );

    // The wildcard among named origins is the same wildcard: a browser reads the list, not its
    // length, so a check looking only at a single-element `[\"*\"]` would let this through.
    assert_eq!(
        refusal(
            &http("[http.cors]\norigins = [\"https://app.example\", \"*\"]\ncredentials = true\n"),
            "etc/nvs.toml",
        )
        .code,
        Some(code::E_MEANINGLESS_HTTP_PAIR),
    );
}

/// `rule:http-server/cookies-are-secure-httponly-and-lax`: `same_site = "None"` needs `secure = true`, refused by § 2's mechanism and for §
/// 2's reason. The absent-key half is the one that matters most — § 3 ships `secure = true`, so a
/// tree that never writes the key has it in force, and reading an absent boolean as `false` would
/// refuse a correct configuration.
#[test]
fn same_site_none_without_secure_is_refused() {
    let fs = http("[http.cookies]\nsame_site = \"None\"\nsecure = false\n");

    let diagnostic = refusal(&fs, "etc/nvs.toml");

    assert_eq!(diagnostic.code, Some(code::E_MEANINGLESS_HTTP_PAIR));
    assert!(
        diagnostic.message.contains("same_site") && diagnostic.message.contains("secure"),
        "the refusal names both halves: {}",
        diagnostic.message,
    );

    // The attribute a browser parses is case-insensitive, so `none` configures the same cookie and
    // must reach the same refusal — otherwise the spelling is the way around the rule.
    assert_eq!(
        refusal(
            &http("[http.cookies]\nsame_site = \"none\"\nsecure = false\n"),
            "etc/nvs.toml",
        )
        .code,
        Some(code::E_MEANINGLESS_HTTP_PAIR),
    );

    let allowed = [
        "[http.cookies]\nsame_site = \"None\"\nsecure = true\n",
        // § 3's default is `secure = true`, so the key not being written is not `secure = false`.
        "[http.cookies]\nsame_site = \"None\"\n",
        // And `secure = false` is a decision an operator may make for a `Lax` cookie.
        "[http.cookies]\nsame_site = \"Lax\"\nsecure = false\n",
        "[http.cookies]\nsecure = false\n",
    ];
    let accepted = allowed
        .iter()
        .filter(|block| tree_of(&http(block), "etc/nvs.toml").config.http.is_some())
        .count();
    assert_eq!(
        accepted,
        allowed.len(),
        "the pair is refused, and neither key is refused on its own or by being absent",
    );
}

/// `rule:http-server/cookies-are-secure-httponly-and-lax`: `same_site` is one of three, and a fourth spelling is refused rather than read as
/// the default. The pair above cannot decide a value nobody can parse — a browser drops the
/// attribute and falls back to *its* default — and `nvs_config::http::Cookies` would otherwise have
/// to choose between repairing it and failing inside a request.
#[test]
fn a_same_site_that_is_none_of_the_three_is_refused() {
    let diagnostic = refusal(
        &http("[http.cookies]\nsame_site = \"Strictly\"\n"),
        "etc/nvs.toml",
    );

    assert_eq!(diagnostic.code, Some(code::E_BAD_SAME_SITE));
    assert!(
        diagnostic.message.contains("Strictly"),
        "the refusal names what was written, that being the thing to change: {}",
        diagnostic.message,
    );

    // The three, in a spelling the tree did not use, because the attribute a browser parses is
    // case-insensitive and refusing `lax` would make the block's own casing load-bearing.
    let allowed = [
        "[http.cookies]\nsame_site = \"Lax\"\n",
        "[http.cookies]\nsame_site = \"strict\"\n",
        "[http.cookies]\nsame_site = \"NONE\"\n",
        // And the key absent, which is `Lax` and not a fourth spelling.
        "[http.cookies]\nsecure = true\n",
    ];
    let accepted = allowed
        .iter()
        .filter(|block| tree_of(&http(block), "etc/nvs.toml").config.http.is_some())
        .count();
    assert_eq!(
        accepted,
        allowed.len(),
        "all three spellings are admitted whatever their case, and so is the absent key",
    );
}

/// `rule:http-server/cors-is-closed-until-origins-are-named`: the lists reach a preflight's answer as one header line each and `max_age`
/// reaches it as a number of seconds, so a boot refuses here what `nvs_server::cors` would
/// otherwise have to repair while answering — an entry the wire cannot carry, and a duration that
/// is not one. Both halves are one case because they are one rule: § 2's block is resolved into
/// header lines at boot, and every value that cannot become one is refused before a server starts.
#[test]
fn a_cors_value_a_preflight_cannot_be_answered_with_is_refused() {
    for key in ["methods", "headers", "expose"] {
        assert_eq!(
            refusal(
                &http(&format!(
                    "[http.cors]\n{key} = [\"GET\\r\\nX-Injected: yes\"]\n"
                )),
                "etc/nvs.toml",
            )
            .code,
            Some(code::E_UNCARRIABLE_HEADER),
            "`[http.cors] {key}` reaches a header line verbatim",
        );
    }

    let diagnostic = refusal(&http("[http.cors]\nmax_age = \"soon\"\n"), "etc/nvs.toml");
    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        diagnostic.message.contains("http.cors.max_age"),
        "the refusal names the directive whose value is not a duration: {}",
        diagnostic.message,
    );

    let allowed = [
        "[http.cors]\norigins = [\"https://a.example\"]\nmethods = [\"GET\", \"DELETE\"]\n",
        "[http.cors]\nheaders = [\"Authorization\"]\nexpose = []\n",
        "[http.cors]\nmax_age = \"10m\"\n",
        // A bare number of seconds is the same duration written the other way, and § 2's own
        // block writes neither key at all.
        "[http.cors]\nmax_age = \"600\"\n",
        "[http.cors]\norigins = []\n",
    ];
    let accepted = allowed
        .iter()
        .filter(|block| tree_of(&http(block), "etc/nvs.toml").config.http.is_some())
        .count();
    assert_eq!(
        accepted,
        allowed.len(),
        "a spelling § 2 states was refused by the check on the ones it does not",
    );
}

/// `rule:security/csrf-is-on-by-default`: the key is what arms the token half of the door's check,
/// so a value the door could not read is refused at boot.
///
/// The alternative is the one direction a security directive must not fail in — a deployment whose
/// tree names a key, whose door therefore verifies nothing, and whose operator has no signal at all.
/// The accepted half is asserted through `csrf_key` rather than through the tree's raw string,
/// because what the door reads is the decode and not the spelling.
#[test]
fn an_http_csrf_key_that_is_not_a_key_is_refused_and_a_written_one_decodes() {
    // 32 octets of `0x07`, in the alphabet a key is written in and in the one another tool would
    // have produced it in.
    let unpadded = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";
    let padded = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";

    for written in ["not base64 at all!", "BwcH", ""] {
        let diagnostic = refusal(
            &http(&format!("[http]\ncsrf_key = \"{written}\"\n")),
            "etc/nvs.toml",
        );
        assert_eq!(
            diagnostic.code,
            Some(code::E_BAD_CSRF_KEY),
            "`{written}` is not a key",
        );
        assert!(
            !diagnostic.message.contains(written) || written.is_empty(),
            "a key does not belong in a message: {}",
            diagnostic.message,
        );
    }

    for written in [unpadded, padded] {
        let tree = tree_of(
            &http(&format!("[http]\ncsrf_key = \"{written}\"\n")),
            "etc/nvs.toml",
        );
        assert_eq!(
            nvs_config::http::csrf_key(&tree.config),
            Some(vec![7_u8; 32]),
            "`{written}` is the same key either way it is written",
        );
    }

    // Nothing written is no key, which is the tree every deployment starts from: the door's origin
    // half still refuses, and there is nothing for its token half to verify against.
    assert_eq!(
        nvs_config::http::csrf_key(&tree_of(&http(""), "etc/nvs.toml").config),
        None,
    );
}

/// `rule:http-server/secure-headers-with-nothing-written`: the free-text policies go onto every response verbatim, so a byte a header
/// line cannot carry is refused at boot. `nvs_server::secure` declines to spell such a value and
/// emits the shipped default instead, which is right for a request in flight and is exactly what
/// makes the boot refusal necessary — otherwise the deployment's policy is silently not the one in
/// force.
#[test]
fn an_http_headers_value_the_wire_cannot_carry_is_refused() {
    let split = "no-referrer\\r\\nX-Injected: yes";
    let diagnostic = refusal(
        &http(&format!("[http.headers]\nreferrer_policy = \"{split}\"\n")),
        "etc/nvs.toml",
    );

    assert_eq!(diagnostic.code, Some(code::E_UNCARRIABLE_HEADER));
    assert!(
        diagnostic.message.contains("referrer_policy"),
        "the refusal names the key, there being three that could hold one: {}",
        diagnostic.message,
    );

    // Every one of them, because a check written over one key would leave the rest open — and
    // the policies are the values most likely to be assembled from somewhere else.
    for key in [
        "referrer_policy",
        "content_security_policy",
        "permissions_policy",
    ] {
        assert_eq!(
            refusal(
                &http(&format!("[http.headers]\n{key} = \"a\\nb\"\n")),
                "etc/nvs.toml",
            )
            .code,
            Some(code::E_UNCARRIABLE_HEADER),
            "{key} reaches a header line verbatim",
        );
    }

    let allowed = [
        "[http.headers]\nreferrer_policy = \"no-referrer\"\n",
        "[http.headers]\ncontent_security_policy = \"default-src 'self'; frame-ancestors 'none'\"\n",
        "[http.headers]\npermissions_policy = \"geolocation=(), camera=()\"\n",
        // Empty is § 1's "emit nothing", which is a decision and not an unspellable value.
        "[http.headers]\nreferrer_policy = \"\"\n",
    ];
    let accepted = allowed
        .iter()
        .filter(|block| tree_of(&http(block), "etc/nvs.toml").config.http.is_some())
        .count();
    assert_eq!(
        accepted,
        allowed.len(),
        "an ordinary policy carries semicolons, quotes and parentheses, and empty turns it off",
    );
}
