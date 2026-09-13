//! `rule:config/the-config-is-an-immutable-snapshot`: the immutable snapshot a request clones at start.
//!
//! A [`Snapshot`] is one entry file's whole answer — the global tree with that file's `[[app]]`
//! blocks folded over it (`rule:config/every-matching-app-block-applies-least-specific-first`) — built once at boot or reload and never mutated
//! afterwards. [`Current`] holds the published one; a request clones the [`Arc`] when it starts and
//! reads that clone for its whole life, so a reload landing mid-request is invisible to it and no
//! request ever sees half of one tree and half of another. `Core\Config::set` writes a per-request
//! overlay *over* this value and never into it (`rule:config/ini-set-is-core-config-set`).
//!
//! **The per-app fold is [`resolve`](crate::resolve)'s `merge_table`, over the global table, one
//! block at a time in [`matching`](crate::app::matching)'s order** — not [`app::layer`]'s effective
//! block folded over the global tree afterwards. The two produce the same values, and this one also
//! produces the right *origins*: a key that a block overrode has to name the file that block was
//! written in, and an effective block folded from three files has one origin for all of its keys.
//! So [`app::layer`] answers "what is the effective `[[app]]` block", which is § 9's per-app
//! `nvs config dump`, and this module does not go through it.
//!
//! **A changed `Boot` directive does not take effect** (`rule:config/reloadability-is-its-own-field`). [`Current::publish`] is
//! the only place that can know, because it holds both trees: it carries each changed `Boot` key's
//! *running* value into the incoming snapshot and names the directive in its [`Reload`]. Reporting
//! alone would not be enough — the new value would still be sitting in the snapshot everything
//! reads — so the report and the carry are one operation.
//!
//! Cost: one owned `Config` and one owned `toml::Table` per snapshot, shared by every request that
//! clones the `Arc` and dropped when the last of them finishes. That is O(in-flight snapshots),
//! which is one plus however many outlived a reload, and never O(requests). Per request it is an
//! `RwLock` read and an `Arc` clone, once, at start.
//!

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use nvs_diagnostics::{Diagnostic, code};

use crate::app;
use crate::directive::{Apply, DIRECTIVES, Directive, governs};
use crate::resolve::{Files, Origin, Override, Resolved};
use crate::secret::Secret;
use crate::tree::Config;

/// The number [`Snapshot::build`] takes for the tree it is building.
///
/// Monotonic and never reused, so that a number a store kept can be compared against a generation
/// that no longer exists. It starts at 1 because [`Default`] is 0, and a snapshot nothing built must
/// not be mistaken for the first one that was.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// One entry file's effective configuration, immutable once built — `rule:config/the-config-is-an-immutable-snapshot`.
///
/// [`Default`] is the configuration of a host with **no configuration file anywhere**, which
/// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3 makes a valid state rather than an error: no directive set, no capability
/// granted, no entry file named. It is what an embedder that has built no tree holds and what a
/// test that is about something else asks for, and it grants nothing — every capability question
/// against it is a refusal, so it cannot be the shape a permission leaks through.
///
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// The effective tree: the global configuration with every matching `[[app]]` block's
    /// directives folded over it. Its `app` roster is **empty** — the blocks that produced this
    /// snapshot are [`blocks`](Snapshot::blocks), and leaving the roster here would read as a list
    /// of applications this one request has rather than as the host's whole set.
    pub config: Config,
    /// The table [`config`](Snapshot::config) was deserialized from, kept for the reason
    /// [`Resolved::table`](crate::resolve::Resolved::table) is: § 9's `nvs config dump --origin`
    /// renders keys the typed tree has no field for, and [`Current::publish`] compares two trees
    /// key by key, which the typed tree cannot be turned back into.
    ///
    /// **This is the half a driver reads**, even though a driver never touches a `toml::Value`:
    /// [`retype`](Snapshot::retype) derives `config` from *this*, so it is authoritative and
    /// `config` is a view of it. A resolving pass that writes a path into `config` alone has
    /// written somewhere this overwrites — `retype`'s own doc § *The seam every `resolve()` pass is
    /// measured against* is the rule, and the sound answers to it.
    pub table: toml::Table,
    /// The entry file this snapshot is for, canonical.
    pub entry: PathBuf,
    /// This snapshot's number: unique in this process, never reused, and 0 for one no boot built.
    ///
    /// A generation needs an identity that a store outliving it can hold, and its address is not
    /// one — `nvs_stdlib::cache`'s process tier keys an entry on the generation it was written
    /// under, and a later snapshot allocated where a dropped one sat would read the entries the
    /// first had written. [`Snapshot::build`] takes the next number; a clone carries the same one,
    /// because a copy of a tree *is* that configuration.
    pub generation: u64,
    /// The `[[app]]` block's `mode` key, from the most specific block that set one. It sits on the block rather than in
    /// a sub-table, so it is read off directly instead of merged: the global `[mode]` is a table
    /// with `default` and `ceiling` in it, and folding a string over that would replace both.
    pub mode: Option<String>,
    /// The `[[app]]` block's `origin` key (`rule:config/origin-is-a-block-key-and-there-is-no-app-table`) — what `Core\Router::urlAbsolute` prepends (`rule:http-server/a-mount-table-expands-at-boot`), from the most
    /// specific block that set one, and read off directly for [`mode`](Snapshot::mode)'s reason.
    /// This is a URL and never a [`struct@Origin`], which is where a value was written.
    pub origin: Option<String>,
    /// The `[[app]]` blocks that matched, least-specific first, by the canonical path each is keyed
    /// on. This is `rule:config/every-matching-app-block-applies-least-specific-first`'s `info: app blocks: …` line, already in order.
    ///
    pub blocks: Vec<PathBuf>,
    /// Every file the tree was read from, in the order § 3 read them.
    pub files: Vec<PathBuf>,
    /// Every override, in the order they happened: the tree's own first (`rule:config/later-wins-and-every-override-is-recorded`), then each
    /// block's over what it replaced. Both carry both origins, because both came from one merge.
    ///
    pub overrides: Vec<Override>,
    /// Where every leaf in [`table`](Snapshot::table) was written, by dotted key. A block's key
    /// names that block's own file, which is the whole reason the fold runs a block at a time.
    pub origins: BTreeMap<String, Origin>,
    /// What the tree was only advised about — a readable secret file, `W1005`. Carried so a reload
    /// can report it again: the file it names may have been re-created between the two reads.
    pub warnings: Vec<Diagnostic>,
    /// `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secrets, by the key each is the value of — `db.main.password`. Carried
    /// rather than folded into [`table`](Snapshot::table) for [`mod@crate::secret`]'s reason, and
    /// carried rather than dropped because it is where the value *is*: the table this snapshot
    /// deserializes has the `_file` sibling and no content, so [`retype`](Snapshot::retype) puts
    /// these back every time it runs and `Core\Config::get` reads them from here.
    pub secrets: BTreeMap<String, Secret>,
}

impl Snapshot {
    /// Builds the snapshot for `entry` out of an already-resolved tree.
    ///
    /// `entry` is canonicalized first and kept that way, for [`app`]'s reason: every comparison
    /// after this line is against a path that has been through
    /// [`trust::canonical`](crate::trust::canonical), and a `..` or a symlink that has not is how an
    /// entry file inherits an application's capabilities without being inside it.
    ///
    /// # Errors
    ///
    /// `E0605` when `entry` cannot be examined, and `E0601` if the folded tree does not deserialize
    /// — which the global tree and every block having deserialized on their own already makes
    /// reachable only by two of them writing one key in two shapes.
    pub fn build(
        resolved: &Resolved,
        entry: &Path,
        files: &dyn Files,
    ) -> Result<Arc<Self>, Diagnostic> {
        let entry = files.canonical(entry).map_err(|err| {
            crate::resolve::unreadable(entry, &err, "it is the entry file being configured")
        })?;
        let mut snapshot = Self {
            config: Config::default(),
            table: resolved.table.clone(),
            entry,
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
            mode: None,
            origin: None,
            blocks: Vec::new(),
            files: resolved.files.clone(),
            overrides: resolved.overrides.clone(),
            origins: resolved.origins.clone(),
            warnings: resolved.warnings.clone(),
            secrets: resolved.secrets.clone(),
        };
        // The roster goes with the blocks it lists: it is what *selected* the directives below, and
        // a key of it left in the table would deserialize into `config.app` as the host's whole set
        // of applications sitting inside one application's configuration. Its origins go too — a
        // key naming a value no longer in the table can never be overridden and so has nothing to
        // report.
        snapshot.table.remove("app");
        snapshot.origins.retain(|key, _| !governs("app", key));
        for index in app::matching(&resolved.config.app, &snapshot.entry, files)? {
            let block = &resolved.config.app[index];
            if let Some(key) = app::key_of(block) {
                snapshot.blocks.push(key.to_path_buf());
            }
            // Least-specific first, so the last block to state one of these wins — which is the
            // same later-wins the merge below applies to everything else.
            if block.mode.is_some() {
                snapshot.mode.clone_from(&block.mode);
            }
            if block.origin.is_some() {
                snapshot.origin.clone_from(&block.origin);
            }
            let (Some(mut directives), Some(origin)) = (
                app::block_table(resolved, index),
                app::block_origin(resolved, index),
            ) else {
                continue;
            };
            // `root` and `entry` selected this block and `mode`/`origin` are above; what is left is
            // `[app.limits]` and `[app.capabilities]`, which are the global blocks' own shapes and
            // fold straight onto them.
            for key in ["root", "entry", "mode", "origin"] {
                directives.remove(key);
            }
            crate::resolve::merge_table(
                &mut snapshot.table,
                &directives,
                origin,
                "",
                &mut snapshot.origins,
                &mut snapshot.overrides,
            );
        }
        snapshot.retype()?;
        // `rule:security/path-scope-canonicalise-then-prefix`'s grant side, canonicalized once and here rather than per check, for the
        // reason an `[[app]]` key is canonicalized at this same point: a root still spelled the way
        // the operator typed it is a comparison against the wrong thing.
        //
        // [ADR 0118]: ../../../docs/decisions/0118.md
        if let Some(capabilities) = snapshot.config.capabilities.as_mut() {
            capabilities.canonicalize(files);
        }
        Ok(Arc::new(snapshot))
    }

    /// Deserializes [`table`](Snapshot::table) into [`config`](Snapshot::config), and puts
    /// [`secrets`](Snapshot::secrets) back onto it.
    ///
    /// The second half is not an afterthought: `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s value is carried beside the table
    /// and never in it, so a typed tree deserialized from the table alone has every
    /// `password_file` and no `password`. Every path that produces a [`Config`] here goes through
    /// this function for that reason — the build below and the `Boot` carry a reload does — and a
    /// second deserialization written anywhere else would silently drop the credential.
    ///
    /// # The seam every `resolve()` pass is measured against
    ///
    /// A pass that rewrites [`config`](Snapshot::config) in place has written to the half this
    /// function overwrites, so it only reaches a driver if one of the following is true — and
    /// [`resolve`](crate::resolve)'s rewriting passes use one each:
    ///
    /// - It writes the **table** as well, as [`db::canonicalize`](crate::db::canonicalize) does
    ///   through its `rewrite`. This is the default and the one a new pass should reach for.
    /// - Its value is carried **beside** the table and re-applied by the line above —
    ///   `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secrets, which may not be in the table at all.
    /// - Its key is read off `resolved.config` **before** any retype and then removed from the
    ///   table, as [`app::canonicalize`](crate::app::canonicalize)'s roster is: [`build`] reads
    ///   `resolved.config.app` directly and then drops `app` from the snapshot's table, so nothing
    ///   here re-derives it.
    ///
    /// Each mechanism answers a different question, rather than one question answered over and
    /// over: a secret must not reach the table, and the `[[app]]` roster must not
    /// survive into a per-app snapshot. What they share is the rule a new pass is checked
    /// against — **a rewrite of `config` that neither reaches the table nor is re-applied here is
    /// lost, silently, at the first thing that retypes.** The read-only passes beside them
    /// (`db::validate`, `app::bound`, `queue::validate`, `schedule::validate`, `http::validate`)
    /// rewrite nothing and so have no seam to cross.
    ///
    /// [`build`]: Snapshot::build
    ///
    /// # Errors
    ///
    /// `E0601` when the table does not fit the typed tree.
    fn retype(&mut self) -> Result<(), Diagnostic> {
        self.config = toml::Value::Table(self.table.clone())
            .try_into::<Config>()
            .map_err(|err| {
                Diagnostic::error(code::E_BAD_DIRECTIVE, err.message().to_string()).with_note(
                    "the global tree and every `[[app]]` block were typed on their own before they \
                     were merged, so this can only be two of them writing one key in two shapes"
                        .to_string(),
                )
            })?;
        crate::secret::apply(&mut self.config, &self.secrets);
        Ok(())
    }

    /// Carries this snapshot's `Boot` values into `next`, returning the directives that changed.
    ///
    /// `rule:config/reloadability-is-its-own-field`: applying one of these would rebind an OS resource or re-create the runtime,
    /// so a reload names it in its result instead. The carry is what makes "does not take effect"
    /// true rather than aspirational — without it the new value is in the snapshot every later
    /// reader sees, and only the thing already bound disagrees with it.
    ///
    /// A row naming a block governs every key beneath it, so the comparison is at the row's own
    /// path and the whole subtree moves together: a `[server]` with one address changed and one
    /// added is one `Boot` change, not two.
    ///
    /// # Errors
    ///
    /// `E0601` if the carried tree does not deserialize, which two trees that each did makes
    /// unreachable.
    ///
    fn carry_boot(&self, next: &mut Self) -> Result<Vec<&'static Directive>, Diagnostic> {
        let mut changed = Vec::new();
        for row in DIRECTIVES.iter().filter(|row| row.apply == Apply::Boot) {
            let running = value_at(&self.table, row.key);
            if running == value_at(&next.table, row.key) {
                continue;
            }
            changed.push(row);
            put_at(&mut next.table, row.key, running);
            // And the origins with it, so `nvs config dump --origin` names the file the value in
            // force was written in rather than the one that asked for a change nothing applied.
            next.origins.retain(|key, _| !governs(row.key, key));
            for (key, origin) in self.origins.iter().filter(|(key, _)| governs(row.key, key)) {
                next.origins.insert(key.clone(), origin.clone());
            }
        }
        if !changed.is_empty() {
            next.retype()?;
        }
        Ok(changed)
    }
}

/// What publishing a snapshot over a running one produced — `rule:config/the-config-is-an-immutable-snapshot` and `rule:config/reloadability-is-its-own-field`.
///
#[derive(Clone, Debug)]
pub struct Reload {
    /// The snapshot now serving. Requests that started before it still hold the previous one.
    pub snapshot: Arc<Snapshot>,
    /// The `Boot` directives the new tree changed. They are reported and **not** applied:
    /// `snapshot` still carries the running value of each, so the result is the only place the
    /// change exists until a restart.
    pub boot: Vec<&'static Directive>,
}

/// The published snapshot: what a request clones at start, and what a reload replaces whole.
///
/// A `RwLock` and not a lock-free cell because the read happens **once per request**, at start,
/// and is an `Arc` clone under a read guard — the contended case is a reload, which is rare, and
/// a dependency bought for one uncontended read is not a trade this crate makes (`rule:packaging/a-c-dependency-answers-two-questions`).
///
#[derive(Debug)]
pub struct Current(RwLock<Arc<Snapshot>>);

impl Current {
    /// The holder, serving `snapshot` from the moment it exists.
    #[must_use]
    pub fn new(snapshot: Arc<Snapshot>) -> Self {
        Self(RwLock::new(snapshot))
    }

    /// The snapshot serving now, for a request to hold for its whole life.
    ///
    /// # Panics
    ///
    /// If a thread panicked while holding the lock. Nothing this module does under that lock can
    /// panic — an `Arc` clone and an `Arc` store — so a poisoned lock here would mean the process
    /// is already unwinding through something else.
    #[must_use]
    pub fn load(&self) -> Arc<Snapshot> {
        Arc::clone(&self.0.read().expect("the snapshot lock is never poisoned"))
    }

    /// Publishes `next`, after carrying the running snapshot's `Boot` values into it.
    ///
    /// This is the last of § 1's validate-then-publish steps and the only one that mutates
    /// anything: everything that can refuse a tree has already run by the time a [`Snapshot`]
    /// exists, so a malformed file leaves the previous snapshot serving because it never reached
    /// here.
    ///
    /// # Errors
    ///
    /// `E0601` if carrying a `Boot` value produces a tree that does not deserialize.
    ///
    /// # Panics
    ///
    /// If a thread panicked while holding the lock — see [`load`](Current::load).
    pub fn publish(&self, next: Snapshot) -> Result<Reload, Diagnostic> {
        let mut next = next;
        let boot = self.load().carry_boot(&mut next)?;
        let snapshot = Arc::new(next);
        *self.0.write().expect("the snapshot lock is never poisoned") = Arc::clone(&snapshot);
        Ok(Reload { snapshot, boot })
    }
}

/// The value at a dotted key, or `None` when nothing wrote one.
///
/// Shared with [`request`](crate::request), which asks the same question of a published snapshot
/// on behalf of `Core\Config::get`.
pub(crate) fn value_at<'t>(table: &'t toml::Table, key: &str) -> Option<&'t toml::Value> {
    let mut segments = key.split('.');
    let mut value = table.get(segments.next()?)?;
    for segment in segments {
        value = value.as_table()?.get(segment)?;
    }
    Some(value)
}

/// Writes `value` at a dotted key, removing what is there when it is `None`.
///
/// Intermediate tables are created on the way down, because a `Boot` value being carried back into
/// a tree whose new file dropped the whole block still has to land somewhere — and an intermediate
/// left **empty** by a removal is removed with it. `[cache]` with no `dir` in it is a block the
/// operator wrote and this one was never written, which is a difference the override record and
/// `nvs config dump` both report.
fn put_at(table: &mut toml::Table, key: &str, value: Option<&toml::Value>) {
    let Some((head, rest)) = key.split_once('.') else {
        match value {
            Some(value) => table.insert(key.to_string(), value.clone()),
            None => table.remove(key),
        };
        return;
    };
    if value.is_none() && !table.contains_key(head) {
        return;
    }
    let slot = table
        .entry(head.to_string())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if !slot.is_table() {
        *slot = toml::Value::Table(toml::Table::new());
    }
    if let Some(into) = slot.as_table_mut() {
        put_at(into, rest, value);
        if into.is_empty() {
            table.remove(head);
        }
    }
}
