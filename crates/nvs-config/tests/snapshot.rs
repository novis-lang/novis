//! `rule:config/the-config-is-an-immutable-snapshot` and `rule:config/reloadability-is-its-own-field`: the snapshot one entry file gets, and what a reload may and may not change.
//!
//! Every case runs against an in-memory [`Files`] for `tests/resolve.rs`'s reason. This reader is
//! the plain one — no symlinks and no `..` — because whether a path *matches* a block is `rule:config/an-application-is-its-entry-file-path`
//! 's claim and `tests/app.rs` pins it against a reader that resolves both. What is asked here
//! is what the matching blocks then produce.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use nvs_config::cache::{
    ProbeHash, Revalidation, UnitKey, Validate, artifact_key, content_hash, env_hash, probe_hash,
};
use nvs_config::resolve::{Files, Resolved, Roots, resolve};
use nvs_config::snapshot::{Current, Snapshot};
use nvs_config::trust::Untrusted;
use nvs_config::{Setting, tree};
use nvs_diagnostics::SourceMap;

/// A path written the way an ADR writes one, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// The filesystem the cases describe: a name-to-text map with directories implied by it.
#[derive(Default)]
struct Fake {
    files: BTreeMap<PathBuf, String>,
}

impl Fake {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            files: entries
                .iter()
                .map(|(path, text)| (p(path), (*text).to_string()))
                .collect(),
        }
    }
}

impl Files for Fake {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        if self.exists(path) {
            Ok(path.to_path_buf())
        } else {
            Err(Untrusted::Unreadable("no such file".to_string()))
        }
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other.as_os_str()),
            }
        }
        if self.exists(&out) {
            Ok(out)
        } else {
            Err("no such file or directory".to_string())
        }
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_string())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.read(path).map(String::into_bytes)
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        None
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

/// Resolves `nvs.toml` in `fs`, panicking with the refusal when it does not.
fn tree_of(fs: &Fake) -> Resolved {
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p("nvs.toml")]), &mut sources, fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The snapshot `entry` gets out of `fs`, panicking with the refusal when there is none.
fn snapshot_of(fs: &Fake, entry: &str) -> Arc<Snapshot> {
    Snapshot::build(&tree_of(fs), &p(entry), fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// `[limits]` as the snapshot has it, panicking when the block is absent.
fn limits(snapshot: &Snapshot) -> &tree::Limits {
    snapshot
        .config
        .limits
        .as_ref()
        .expect("this tree writes a `[limits]` block")
}

/// A text setting, as every size and duration in the file is written.
fn text(value: &str) -> Option<Setting> {
    Some(Setting::Text(value.to_string()))
}

/// `rule:config/every-matching-app-block-applies-least-specific-first`'s own example: a host-wide block, a shop beneath it, and one entry file inside the
/// shop, over a global tree each of them takes something from.
const SHOP: &str = "\
[limits]
memory = \"128M\"
wall_time = \"30s\"

[capabilities.process]
exec = false

[[app]]
root = \"srv/www\"
mode = \"production\"

[[app]]
root = \"srv/www/shop\"
origin = \"https://shop.example\"

[app.limits]
memory = \"512M\"

[app.capabilities.process]
exec = true

[[app]]
entry = \"srv/www/shop/bin/import.nvs\"
mode = \"development\"

[app.limits]
wall_time = \"600s\"
";

fn shop() -> Fake {
    Fake::with(&[
        ("nvs.toml", SHOP),
        ("srv/www/index.nvs", ""),
        ("srv/www/shop/index.nvs", ""),
        ("srv/www/shop/bin/import.nvs", ""),
        ("srv/other/x.nvs", ""),
    ])
}

/// § 2's closing sentence: an entry file matched by no block gets the global configuration, which
/// is the ordinary case and needs no block at all.
#[test]
fn an_entry_no_block_matches_gets_the_global_configuration() {
    let snapshot = snapshot_of(&shop(), "srv/other/x.nvs");
    assert!(snapshot.blocks.is_empty());
    assert_eq!(limits(&snapshot).memory, text("128M"));
    assert_eq!(snapshot.mode, None);
    assert_eq!(snapshot.origin, None);
}

/// [`Snapshot::host`] is for no entry file, so no block matches it whatever the tree writes: it is
/// the global configuration, and it names no entry.
#[test]
fn the_hosts_snapshot_folds_no_block_and_names_no_entry() {
    let fs = shop();
    let snapshot = Snapshot::host(&tree_of(&fs), &fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes));
    assert_eq!(snapshot.entry, None);
    assert!(snapshot.blocks.is_empty());
    assert_eq!(limits(&snapshot).memory, text("128M"));
    assert_eq!(limits(&snapshot).wall_time, text("30s"));
    assert_eq!(snapshot.mode, None);
    assert_eq!(snapshot.origin, None);
}

/// § 2's worked example, both halves at once: `/srv/www/shop/bin/import.nvs` gets a memory of
/// `512M` from the `/srv/www/shop` block and a `wall_time` of `600s` from its own, while
/// inheriting everything neither states.
#[test]
fn every_matching_block_layers_least_specific_first() {
    let snapshot = snapshot_of(&shop(), "srv/www/shop/bin/import.nvs");
    assert_eq!(
        snapshot.blocks,
        vec![
            p("srv/www"),
            p("srv/www/shop"),
            p("srv/www/shop/bin/import.nvs"),
        ]
    );
    assert_eq!(limits(&snapshot).memory, text("512M"));
    assert_eq!(limits(&snapshot).wall_time, text("600s"));
}

/// The same tree from one directory up: the innermost block does not apply to a file it does not
/// cover, so `wall_time` is still the global one. Asserted beside the case above because a fold
/// that applied every block regardless would pass that one on its own.
#[test]
fn a_block_that_does_not_cover_the_entry_contributes_nothing() {
    let snapshot = snapshot_of(&shop(), "srv/www/shop/index.nvs");
    assert_eq!(snapshot.blocks, vec![p("srv/www"), p("srv/www/shop")]);
    assert_eq!(limits(&snapshot).memory, text("512M"));
    assert_eq!(limits(&snapshot).wall_time, text("30s"));
}

/// `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling`'s widening half, which is what makes a root file that denies workable: the global
/// block withholds `process.exec` and the shop's block grants it.
#[test]
fn a_block_grants_a_capability_the_global_block_withholds() {
    let global = snapshot_of(&shop(), "srv/other/x.nvs");
    let shop = snapshot_of(&shop(), "srv/www/shop/index.nvs");
    let exec = |snapshot: &Snapshot| {
        snapshot
            .config
            .capabilities
            .as_ref()
            .and_then(|caps| caps.process.as_ref())
            .and_then(|process| process.exec.clone())
    };
    assert_eq!(exec(&global), Some(Setting::Bool(false)));
    assert_eq!(exec(&shop), Some(Setting::Bool(true)));
}

/// `mode` and `origin` sit on the block rather than in a sub-table, and the most specific block to
/// state one wins — `import.nvs`'s own `development` over the host block's `production`, while
/// `origin` comes from the middle block because neither of the other two writes one.
#[test]
fn mode_and_origin_come_from_the_most_specific_block_that_states_them() {
    let snapshot = snapshot_of(&shop(), "srv/www/shop/bin/import.nvs");
    assert_eq!(snapshot.mode.as_deref(), Some("development"));
    assert_eq!(snapshot.origin.as_deref(), Some("https://shop.example"));
    // And the global `[mode]` block is untouched by either: a string folded over that table would
    // have replaced `default` and `ceiling` together.
    assert_eq!(snapshot.config.mode, None);
}

/// The roster is what *selected* the directives, not one of them. A snapshot carrying it would
/// hand one application the host's whole list of applications.
#[test]
fn the_app_roster_is_not_in_the_snapshot() {
    let snapshot = snapshot_of(&shop(), "srv/www/shop/index.nvs");
    assert!(snapshot.config.app.is_empty());
    assert!(!snapshot.table.contains_key("app"));
    assert!(!snapshot.origins.keys().any(|key| key.starts_with("app")));
}

/// § 2 is [0103 § 3]'s later-wins in a different order, so a block taking a key from the global
/// tree is reported exactly as a later file taking one is — both origins named, and the origin of
/// the winner is the file the *block* was written in.
#[test]
fn a_block_overriding_a_global_key_is_reported_with_both_origins() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[limits]\nmemory = \"128M\"\n\n[[include]]\npath = \"conf.d/shop.toml\"\n",
        ),
        // The `root` is relative to the file that wrote it (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`), and that file is one
        // directory down — which is the half of § 5 an `[[app]]` key is easiest to get wrong.
        (
            "conf.d/shop.toml",
            "[[app]]\nroot = \"../srv/www/shop\"\n\n[app.limits]\nmemory = \"512M\"\n",
        ),
        ("srv/www/shop/index.nvs", ""),
    ]);
    let snapshot = snapshot_of(&fs, "srv/www/shop/index.nvs");
    let record = snapshot
        .overrides
        .iter()
        .find(|record| record.key == "limits.memory")
        .expect("the block took `limits.memory` from the global tree");
    assert_eq!(record.replaced.path, p("nvs.toml"));
    assert_eq!(record.winner.path, p("conf.d/shop.toml"));
    assert_eq!(
        snapshot.origins["limits.memory"].path,
        p("conf.d/shop.toml")
    );
}

/// `rule:config/the-config-is-an-immutable-snapshot`'s whole point: a request clones the `Arc` when it starts and reads that clone for
/// its whole life, so a reload landing mid-request is invisible to it.
#[test]
fn a_request_that_started_before_a_swap_reads_the_old_value_to_completion() {
    let before = Fake::with(&[("nvs.toml", "[limits]\nmemory = \"128M\"\n")]);
    let after = Fake::with(&[("nvs.toml", "[limits]\nmemory = \"512M\"\n")]);
    let current = Current::new(snapshot_of(&before, "nvs.toml"));

    let in_flight = current.load();
    let reload = current
        .publish(Arc::unwrap_or_clone(snapshot_of(&after, "nvs.toml")))
        .expect("this tree deserializes");

    assert_eq!(limits(&in_flight).memory, text("128M"));
    assert_eq!(limits(&reload.snapshot).memory, text("512M"));
    assert_eq!(limits(&current.load()).memory, text("512M"));
    assert!(reload.boot.is_empty());
}

/// `rule:config/reloadability-is-its-own-field` and m6.md's *Verify*: a changed `Boot` key is reported in the result **and does not
/// take effect** — the published snapshot still carries the running value, because a report beside
/// a snapshot holding the new one would be a report of something that had already happened.
#[test]
fn a_changed_boot_key_is_reported_and_does_not_take_effect() {
    let before = Fake::with(&[(
        "nvs.toml",
        "[opcache]\nfile_cache_dir = \"/var/cache/nvs\"\n\n[limits]\nmemory = \"128M\"\n",
    )]);
    let after = Fake::with(&[(
        "nvs.toml",
        "[opcache]\nfile_cache_dir = \"/srv/cache\"\n\n[limits]\nmemory = \"512M\"\n",
    )]);
    let current = Current::new(snapshot_of(&before, "nvs.toml"));

    let reload = current
        .publish(Arc::unwrap_or_clone(snapshot_of(&after, "nvs.toml")))
        .expect("this tree deserializes");

    assert_eq!(
        reload.boot.iter().map(|row| row.key).collect::<Vec<_>>(),
        vec!["opcache.file_cache_dir"]
    );
    let opcache = reload
        .snapshot
        .config
        .opcache
        .as_ref()
        .expect("the running `[opcache]` block was carried forward");
    assert_eq!(opcache.file_cache_dir.as_deref(), Some("/var/cache/nvs"));
    // The `Reload` half of the same file did take effect, which is what makes the refusal above a
    // property of the directive and not of the reload.
    assert_eq!(limits(&reload.snapshot).memory, text("512M"));
}

/// A `Boot` row naming a block governs every key beneath it, so a `[server]` the reload rewrote is
/// one reported directive and the whole block stays as it was bound.
// covers: directive:server
#[test]
fn a_boot_row_naming_a_block_carries_the_whole_block() {
    let before = Fake::with(&[(
        "nvs.toml",
        "[server]\nlisten = [\"127.0.0.1:8080\"]\ndispatch = \"path\"\n",
    )]);
    let after = Fake::with(&[(
        "nvs.toml",
        "[server]\nlisten = [\"0.0.0.0:80\"]\ndispatch = \"entry\"\n",
    )]);
    let current = Current::new(snapshot_of(&before, "nvs.toml"));

    let reload = current
        .publish(Arc::unwrap_or_clone(snapshot_of(&after, "nvs.toml")))
        .expect("this tree deserializes");

    assert_eq!(
        reload.boot.iter().map(|row| row.key).collect::<Vec<_>>(),
        vec!["server"]
    );
    let server = reload
        .snapshot
        .config
        .server
        .as_ref()
        .expect("the running `[server]` block was carried forward");
    assert_eq!(
        server.listen.as_deref(),
        Some(["127.0.0.1:8080".to_string()].as_slice())
    );
    assert_eq!(server.dispatch.as_deref(), Some("path"));
}

/// [ADR 0154] § 5's reason for splitting `[queue]` across two apply classes, asserted as the thing
/// an operator sees: a `workers` they edited on a running server is named by the reload and the
/// count still in force is the one the workers were started with, while the `max_attempts` in the
/// same file takes effect. `[queue]` is the block after `[deferred]` where both halves are in one
/// tree, so a registry that made the block one row would fail here in whichever direction it chose.
///
/// [ADR 0154]: ../../../docs/decisions/0154.md
#[test]
fn a_reload_that_changes_workers_carries_the_running_value_and_names_the_key() {
    let written = |workers: u32, max_attempts: u32| {
        let text = format!(
            "[db.main]\ndriver = \"pgsql\"\n\n[queue]\nconnection = \"main\"\n\
             workers = {workers}\nmax_attempts = {max_attempts}\n"
        );
        Fake::with(&[("nvs.toml", text.as_str())])
    };
    let current = Current::new(snapshot_of(&written(4, 5), "nvs.toml"));

    let reload = current
        .publish(Arc::unwrap_or_clone(snapshot_of(
            &written(16, 9),
            "nvs.toml",
        )))
        .expect("this tree deserializes");

    assert_eq!(
        reload.boot.iter().map(|row| row.key).collect::<Vec<_>>(),
        vec!["queue.workers"],
        "a worker is a spawned task, so the count an operator changed is named rather than applied",
    );
    let queue = reload
        .snapshot
        .config
        .queue
        .as_ref()
        .expect("the running `[queue]` block was carried forward");
    assert_eq!(
        queue.workers,
        Some(4),
        "the running count is still in force"
    );
    // The connection did not change, so it is not reported either — `Boot` is what applying costs,
    // not what a key is.
    assert_eq!(queue.connection.as_deref(), Some("main"));
    // And the `Reload` half of the same block did take effect, which is what makes the carry above
    // a property of the two directives rather than of the reload.
    assert_eq!(queue.max_attempts, Some(9));
}

/// The bound's other side: a reload that *adds* a `Boot` key is a change like any other, and the
/// snapshot goes back to having none of it rather than to an empty block the typed tree would read
/// as an `[opcache]` that was written.
#[test]
fn a_boot_key_a_reload_added_is_reported_and_left_unset() {
    let before = Fake::with(&[("nvs.toml", "[limits]\nmemory = \"128M\"\n")]);
    let after = Fake::with(&[("nvs.toml", "[opcache]\nfile_cache_dir = \"/srv/cache\"\n")]);
    let current = Current::new(snapshot_of(&before, "nvs.toml"));

    let reload = current
        .publish(Arc::unwrap_or_clone(snapshot_of(&after, "nvs.toml")))
        .expect("this tree deserializes");

    assert_eq!(
        reload.boot.iter().map(|row| row.key).collect::<Vec<_>>(),
        vec!["opcache.file_cache_dir"]
    );
    assert_eq!(reload.snapshot.config.opcache, None);
}

/// `rule:config/the-config-is-an-immutable-snapshot`'s validate-then-publish, and m6.md's *Verify*: a reload whose tree does not parse
/// never reaches [`Current::publish`] at all, so the snapshot serving is the one that was already
/// serving, and the refusal names the line an operator has to fix rather than a byte offset.
#[test]
fn a_malformed_file_leaves_the_previous_snapshot_serving_and_names_the_line() {
    let before = Fake::with(&[("nvs.toml", "[limits]\nmemory = \"128M\"\n")]);
    let current = Current::new(snapshot_of(&before, "nvs.toml"));

    // The `[limits]` block above it is well formed and would have raised the ceiling, so what the
    // case measures is the refusal and not a reload that had nothing to say.
    let broken = "[limits]\nmemory = \"512M\"\n\n[server\nlisten = [\"127.0.0.1:8080\"]\n";
    let after = Fake::with(&[("nvs.toml", broken)]);
    let mut sources = SourceMap::new();
    let refusal = resolve(&Roots::Files(vec![p("nvs.toml")]), &mut sources, &after)
        .expect_err("an unclosed table header is refused");

    let span = refusal
        .primary_span()
        .expect("a refusal carries the line that caused it");
    let file = sources.file(span.file);
    let (line, _) = file.line_col(span.start);
    assert_eq!(
        file.line_text(line),
        Some("[server"),
        "the refusal points at the unclosed header, not at the file",
    );

    // Nothing was published, so the request starting now reads what the request before it read.
    assert_eq!(limits(&current.load()).memory, text("128M"));
}

/// The snapshot whose `[[extension]]` array carries `pins`, over a tree that is otherwise the same
/// one every time — so the only thing two of these differ by is the extension set.
fn pinned(pins: &[&str]) -> Arc<Snapshot> {
    let mut written = String::from("[limits]\nmemory = \"128M\"\n");
    for pin in pins {
        written.push_str(&format!(
            "\n[[extension]]\npath = \"ext/{pin}.nvsx\"\nsha256 = \"{pin}\"\n"
        ));
    }
    let fs = Fake::with(&[("nvs.toml", &written), ("srv/www/index.nvs", "")]);
    snapshot_of(&fs, "srv/www/index.nvs")
}

/// `rule:config/the-extension-set-is-in-every-unit-key`: one `env_hash` over the extension set, and **both** compiled-unit cache keys carry
/// it — the on-disk `BLAKE3(content_hash ‖ env_hash)` and the in-memory
/// `UnitKey { path, content_hash, probe_hash, env_hash }`. Asked of both keys together, because § 4's whole
/// content is that they move as one: a key built out of the source digest alone still looks right
/// beside a case that only asks the other one. The last block is § 4's "hashed once": both keys
/// take the source's digest, and the on-disk key is a derivation of it rather than either input
/// passed through.
#[test]
fn env_hash_is_carried_by_both_cache_keys() {
    let one = env_hash(&pinned(&["aa", "bb"]).config);
    let reordered = env_hash(&pinned(&["bb", "aa"]).config);
    let changed = env_hash(&pinned(&["aa", "cc"]).config);
    assert_eq!(
        one, reordered,
        "§ 4 hashes the pins sorted, so the set is order-independent"
    );
    assert_ne!(one, changed, "a changed pin is a changed environment");
    assert_ne!(
        one,
        env_hash(&pinned(&["aa"]).config),
        "a dropped extension is one too",
    );

    let source = content_hash(b"<?nvs\necho 1;\n");
    let path = p("srv/www/index.nvs");

    // On disk. Same source, two environments, two entries — which is the hole § 4 closes.
    assert_ne!(
        artifact_key(source, one),
        artifact_key(source, changed),
        "the on-disk key carries env_hash",
    );
    assert_eq!(artifact_key(source, one), artifact_key(source, reordered));

    // In memory. Same path and same content, two environments, two units.
    let unprobed = ProbeHash::unrecorded();
    assert_ne!(
        UnitKey::new(&path, source, unprobed, one),
        UnitKey::new(&path, source, unprobed, changed),
        "the in-memory key carries env_hash",
    );
    assert_eq!(
        UnitKey::new(&path, source, unprobed, one),
        UnitKey::new(&path, source, unprobed, reordered)
    );

    // And it is the value itself that is carried, not a second derivation of the same inputs.
    assert_eq!(UnitKey::new(&path, source, unprobed, one).env(), one);
    assert_eq!(
        UnitKey::new(&path, source, unprobed, one).content_hash(),
        source
    );

    // The on-disk key is derived from the digest, not either input handed back.
    let key = artifact_key(source, one);
    assert_ne!(key, source, "the key is not the content hash");
    assert_ne!(key, one.digest(), "the key is not the env hash");
    assert_ne!(
        key,
        artifact_key(content_hash(b"<?nvs\necho 2;\n"), one),
        "a changed source is a changed key",
    );
}

/// `rule:packaging/autoload-probes-fold-into-the-cache-key`: what a unit's `autoload` resolution
/// probed is a field of that unit's key, so a file appearing *in front of* the one the resolution
/// reached is a different unit — with no byte of anything the first compile hashed having changed.
/// Asked as the three properties a cache rests on: the trace is ordered, an empty trace is a real
/// value rather than the absent one, and the field is carried so a compile can re-key its own
/// result without respelling the other three.
#[test]
fn a_units_key_carries_what_its_autoload_resolution_probed() {
    let env = env_hash(&pinned(&["aa", "bb"]).config);
    let source = content_hash(b"<?nvs\necho 1;\n");
    let path = p("srv/www/index.nvs");

    let reached = probe_hash(&[p("vendor/compat/Thing.nvs")]);
    let shadowed = probe_hash(&[p("src/Thing.nvs"), p("vendor/compat/Thing.nvs")]);
    assert_ne!(
        UnitKey::new(&path, source, reached, env),
        UnitKey::new(&path, source, shadowed, env),
        "a path probed in front of the one that was reached is another unit",
    );
    assert_ne!(
        shadowed,
        probe_hash(&[p("vendor/compat/Thing.nvs"), p("src/Thing.nvs")]),
        "the same two paths probed the other way round are another resolution",
    );

    // A program declaring no `autoload` probes nothing, and that is an answer rather than the
    // absence of one: its unit is an ordinary hit rather than a key no lookup can ever spell.
    assert_eq!(probe_hash(&[]), probe_hash(&[]));
    assert_ne!(probe_hash(&[]), ProbeHash::unrecorded());
    assert_eq!(ProbeHash::unrecorded().digest(), None);
    assert!(probe_hash(&[]).digest().is_some());

    let claimed = UnitKey::new(&path, source, ProbeHash::unrecorded(), env);
    assert_eq!(claimed.probes(), ProbeHash::unrecorded());
    let published = claimed.clone().with_probes(reached);
    assert_eq!(published.probes(), reached);
    assert_eq!(
        published,
        UnitKey::new(&path, source, reached, env),
        "re-keying names the same unit the trace would have been spelled into",
    );
    assert_ne!(published, claimed);
}

/// `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s two revalidation directives, read off the merged tree: what a resolve
/// looks at when it re-checks a compiled path, and how often it may look at all. Both are
/// `System`-class, so what the snapshot holds is what the process runs with — nothing re-reads
/// them per request, and `nvs_config::cache`'s own doc is where the defaults are decided.
#[test]
fn the_opcache_block_is_read_into_a_revalidation_policy() {
    let written = |block: &str| {
        let fs = Fake::with(&[("nvs.toml", block), ("srv/www/index.nvs", "")]);
        Revalidation::from_config(&snapshot_of(&fs, "srv/www/index.nvs").config)
    };

    assert_eq!(
        written("").freq,
        Revalidation::default().freq,
        "a tree with no `[opcache]` block runs the default cap"
    );
    assert_eq!(
        written("[opcache]\nvalidate = \"hash\"\n").validate,
        Validate::Hash
    );
    assert_eq!(
        written("[opcache]\nvalidate = true\n").validate,
        Validate::Mtime,
        "PHP's `validate_timestamps = 1`, which is what an operator transcribes",
    );
    assert_eq!(
        written("[opcache]\nrevalidate_freq = \"500ms\"\n").freq,
        Duration::from_millis(500),
        "the one quantity parser reads this exactly as it reads a `[limits]` duration",
    );
    assert_eq!(
        written("[opcache]\nrevalidate_freq = false\n").freq,
        Duration::ZERO,
        "`rule:config/three-changeability-classes`'s `false` removes the cap, which is a check on every resolve",
    );
}

/// `rule:config/opcache-revalidation-is-system-class`'s refusal: `validate` has two values, and a
/// tree writing anything else does not load. `never` and the boolean `false` that meant it are the
/// two an operator is likely to write — one from an older Novis, one from PHP — and a word that
/// spells nothing is refused the same way rather than read as the default.
#[test]
fn validate_never_does_not_load_and_names_mtime_and_hash() {
    for block in [
        "[opcache]\nvalidate = \"never\"\n",
        "[opcache]\nvalidate = false\n",
        "[opcache]\nvalidate = \"sometimes\"\n",
        "[mode]\ndefault = \"development\"\n\n[opcache]\nvalidate = \"never\"\n",
    ] {
        let fs = Fake::with(&[("nvs.toml", block), ("srv/www/index.nvs", "")]);
        let Err(refused) = resolve(
            &Roots::Files(vec![p("nvs.toml")]),
            &mut SourceMap::new(),
            &fs,
        ) else {
            panic!("`{block}` loaded");
        };
        assert_eq!(
            refused.code,
            Some(nvs_diagnostics::code::E_BAD_DIRECTIVE),
            "{block}"
        );
        let said = format!("{refused:?}");
        for value in ["mtime", "hash", "nvs.toml"] {
            assert!(
                said.contains(value),
                "`{block}` did not name {value}: {said}"
            );
        }
    }
}

/// `validate`'s default is `mtime`, and the run mode does not choose it: a host that writes no
/// mode, one in `production` and one in `development` all check every file by its stamp. A
/// written value still wins, and the rate cap beside it is not a mode row either
/// (`rule:config/opcache-revalidation-is-system-class`).
#[test]
fn validate_defaults_to_mtime_in_production_and_development() {
    let written = |block: &str| {
        let fs = Fake::with(&[("nvs.toml", block), ("srv/www/index.nvs", "")]);
        Revalidation::from_config(&snapshot_of(&fs, "srv/www/index.nvs").config)
    };

    for block in [
        "",
        "[mode]\ndefault = \"production\"\n",
        "[mode]\ndefault = \"development\"\n",
    ] {
        assert_eq!(written(block).validate, Validate::Mtime, "{block}");
    }
    assert_eq!(
        written("[mode]\ndefault = \"production\"\n\n[opcache]\nvalidate = \"hash\"\n").validate,
        Validate::Hash,
    );

    // The cap keeps its own default under either mode.
    assert_eq!(
        written("[mode]\ndefault = \"development\"\n").freq,
        Revalidation::default().freq,
    );
    assert_eq!(
        written("[mode]\ndefault = \"production\"\n").freq,
        Revalidation::default().freq,
    );
}
