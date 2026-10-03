//! The configuration tree, resolved: which file is the root, which files it pulls in, and the one
//! ordered stream they flatten to.
//!
//! `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` is the whole of this module's specification. § 1 picks the root, § 2 expands
//! `[[include]]`, § 3 flattens the tree to one sequence where a later assignment wins, § 4 splits
//! replacing from appending, and § 5 resolves every relative path against the file it is written in.
//! § 6's ownership check is not here either: it is a property of the bytes' provenance rather than
//! of the tree's shape, so it belongs to the reader as [`Files::trust`] and [`mod@crate::trust`]
//! holds what it means on each platform. What this module owes it is the *order* — nothing is
//! compared, read or resolved against until it has passed — and the two places the tree's shape
//! decides where the check falls: the directory a `dir` include lists, and the directory an absent
//! `optional` include would have appeared in.
//!
//! **Later wins is only acceptable because every override is recorded.** § 3 states that as an
//! obligation and not a permission: without the record it is the silent-shadowing failure
//! `rule:config/the-file-is-nvs-toml-and-it-is-toml` refused INI for, in a file that grants capabilities. So the merge does not just
//! overwrite — it carries an [`Origin`] for every value it holds and emits an [`Override`] naming
//! both files whenever one replaces another. A caller that drops [`Resolved::overrides`] on the
//! floor has removed a security property, not a log line.
//!
//! **Both of `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s refusals stay per file.** Each file is deserialized into
//! [`Config`] on its own before anything is merged, so an unknown key is refused with *that* file's
//! line under it; the merge itself runs over `toml::Table`, where a key set in two files is an
//! override rather than a duplicate. That is why a file is deserialized twice — once to refuse it,
//! once to merge it — and the second pass is free next to the read.
//!
//! **§ 7's secret files are read last, over the flattened tree.** Which `password_file` is in force
//! is a question only the merge has answered, so [`mod@crate::secret`] runs once at the end rather
//! than per file — reading one a later file replaced would be a secret the configuration does not
//! use, examined for nothing.
//!
//! Cost: the whole tree's text and one merged table are held for the length of a boot or a reload,
//! then dropped once the snapshot is built. Nothing here runs per request.
//!

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use nvs_diagnostics::{Diagnostic, SourceId, SourceMap, code};

use crate::secret::Secret;
use crate::tree::Config;
use crate::trust::{self, Untrusted};

/// How deep `[[include]]` may nest — `rule:config/include-takes-a-path-or-a-dir`.
///
/// A cycle is caught by name, not by depth: [`Files::trust`] hands back the canonical path, so a
/// ring built out of symlinks closes on a name [`same_file`] has already seen. This is therefore a
/// cap on nesting and nothing else — an include tree eight files deep is one no operator can read,
/// whether or not it ever closes.
pub const MAX_INCLUDE_DEPTH: usize = 8;

/// Where a configuration file's bytes come from.
///
/// A trait rather than direct `std::fs` calls for two reasons, and neither is testing for its own
/// sake. § 6's ownership check belongs on the *reader*, so a caller that has one and a caller that
/// does not are two implementations of one interface rather than a flag threaded through the
/// resolver. And § 2's directory order is **mandated** — ascending by filename, because § 3 makes
/// order decide the answer and a capability grant settled by `readdir` order is not a design — so
/// [`list`](Files::list) is allowed to return entries in any order and the sort is this module's.
pub trait Files {
    /// `rule:config/ownership-is-the-trust-boundary`'s trust check on `path`, and the **canonical** path it names.
    ///
    /// This is the trust boundary. Everything the resolver does with a path afterwards — comparing
    /// it against the chain it was reached through, reading it, resolving an include against its
    /// directory — is a statement about a file that passed here, which is why it runs first and
    /// why a reader may not decline to implement it.
    ///
    /// # Errors
    ///
    /// [`Untrusted`]: `Unreadable` when the path could not be examined at all, which the resolver
    /// reports as `E0605` like any other unreadable file, and `Breach` when it was examined and
    /// the boundary does not hold, which is `E0607`.
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted>;

    /// The canonical path `path` names, with **no** trust check — `rule:config/an-application-is-its-entry-file-path`'s half of the
    /// `[[app]]` comparison.
    ///
    /// Separate from [`trust`](Files::trust) because an application's root is not a file the
    /// configuration reads: it is a web root owned by whoever deploys to it, and demanding § 6's
    /// ownership of it would refuse the ordinary deployment while buying nothing — nothing here
    /// reads a byte of it. What is shared is the canonicalization itself ([`trust::canonical`]),
    /// which is the half a second implementation would get wrong.
    ///
    /// # Errors
    ///
    /// Whatever the underlying reader says; [`mod@crate::app`] wraps it in `E0605`.
    fn canonical(&self, path: &Path) -> Result<PathBuf, String>;

    /// [`canonical`](Files::canonical) for the path an `[[app]]` block is keyed on, which
    /// [`crate::app::canonicalize`] asks for every block of the roster.
    ///
    /// Separate because a program uses only the blocks that match its own entry file. [`Disk`]
    /// records nothing here: a block that applies to the program is recorded where it matches, and
    /// a block path that comes or goes changes whether the roster resolves at all, which the test
    /// resolving the repository's own configuration is keyed on. The default records what
    /// [`canonical`](Files::canonical) records, which is more and never less.
    ///
    /// # Errors
    ///
    /// As [`canonical`](Files::canonical).
    fn canonical_block(&self, path: &Path) -> Result<PathBuf, String> {
        self.canonical(path)
    }

    /// The file's text, or a message describing why not.
    ///
    /// # Errors
    ///
    /// Whatever the underlying reader says; the resolver wraps it in `E0605`.
    fn read(&self, path: &Path) -> Result<String, String>;

    /// [`read`](Files::read) for a configuration file of the tree, which [`resolve`] reads through
    /// this and nothing else.
    ///
    /// Separate because a program uses only part of a configuration file. [`Disk`] records the file
    /// as a `config` line rather than a `file` line: its global tables here, and its `[[app]]` blocks
    /// where [`crate::app::matching`] matches them. The default records what [`read`](Files::read)
    /// records, which is more and never less.
    ///
    /// # Errors
    ///
    /// As [`read`](Files::read).
    fn read_config(&self, path: &Path) -> Result<String, String> {
        self.read(path)
    }

    /// The file's bytes, unvalidated — what `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secret file is read through.
    ///
    /// Separate from [`read`](Files::read) because § 7 makes "not valid UTF-8" a refusal about the
    /// *content* (`E0608`), and a reader that decoded first could only report it as a failure to
    /// read at all. A configuration file still goes through `read`: it is TOML, so it has no
    /// meaning as bytes.
    ///
    /// # Errors
    ///
    /// Whatever the underlying reader says; [`mod@crate::secret`] wraps it in `E0605`.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String>;

    /// `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s advisory: how an account other than this one may **read** `path`, or `None`.
    ///
    /// On the reader rather than beside [`trust`](Files::trust) for the same reason: whether there
    /// is a filesystem to ask is the reader's question. [`trust::exposure`] is the answer when
    /// there is one.
    fn exposure(&self, path: &Path) -> Option<String>;

    /// Every entry directly inside `dir`, in any order and unfiltered.
    ///
    /// # Errors
    ///
    /// Whatever the underlying reader says; the resolver wraps it in `E0605`.
    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String>;

    /// Whether the path is there at all. Only [`Include::optional`](crate::tree::Include::optional)
    /// and § 1's step 2 ask, and both of them treat absence as an answer rather than a failure.
    fn exists(&self, path: &Path) -> bool;

    /// Whether `path` is certainly not there, and so nothing below it is either: no such entry, or
    /// a name the platform cannot hold at all. `false` whenever that is not certain — a folder that
    /// is there and may not be read, a dangling symlink, any other failure.
    ///
    /// Separate from [`exists`](Files::exists), which is `false` for all of those alike. Only
    /// [`crate::capability::resolved`] asks, to skip the ancestors of a path that cannot be pinned,
    /// and the default skips none of them: slower, and never a different answer.
    fn missing(&self, _path: &Path) -> bool {
        false
    }
}

/// The real filesystem.
#[derive(Clone, Copy, Debug, Default)]
pub struct Disk;

// Every read and test below is recorded when `NVS_FOOTPRINT_LOG` names a log, which is how a
// configuration file, an include and a secret file reach a program's footprint.
impl Files for Disk {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        nvs_footprint::exists(path);
        trust::check(path)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        nvs_footprint::exists(path);
        trust::canonical(path).map_err(|err| err.to_string())
    }

    fn canonical_block(&self, path: &Path) -> Result<PathBuf, String> {
        trust::canonical(path).map_err(|err| err.to_string())
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        nvs_footprint::file(path);
        std::fs::read_to_string(path).map_err(|err| err.to_string())
    }

    fn read_config(&self, path: &Path) -> Result<String, String> {
        nvs_footprint::config(path);
        std::fs::read_to_string(path).map_err(|err| err.to_string())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        nvs_footprint::file(path);
        std::fs::read(path).map_err(|err| err.to_string())
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        trust::exposure(path)
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        nvs_footprint::dir(dir);
        let entries = std::fs::read_dir(dir).map_err(|err| err.to_string())?;
        let mut paths = Vec::new();
        for entry in entries {
            paths.push(entry.map_err(|err| err.to_string())?.path());
        }
        Ok(paths)
    }

    fn exists(&self, path: &Path) -> bool {
        nvs_footprint::exists(path);
        path.exists()
    }

    fn missing(&self, path: &Path) -> bool {
        nvs_footprint::exists(path);
        std::fs::symlink_metadata(path).is_err_and(|err| {
            matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidFilename
            )
        })
    }
}

/// Where one value in the resolved configuration was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    /// The file, absolute.
    pub path: PathBuf,
    /// That file in the caller's [`SourceMap`], so a report can render the line as well as name it.
    pub source: SourceId,
}

/// `, written in ...` when the merge recorded where, and nothing when it did not.
///
/// One copy, beside [`Origin`] itself: every refusal that can name a file phrases it this way, and
/// a module spelling it its own way is one more phrasing an operator has to learn.
pub(crate) fn origin_note(written_in: Option<&Origin>) -> String {
    written_in.map_or_else(String::new, |origin| {
        format!(", written in `{}`", origin.path.display())
    })
}

/// One assignment replaced by a later one — `rule:config/later-wins-and-every-override-is-recorded`'s record, carrying **both** origins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Override {
    /// The dotted key, `limits.hard.memory`.
    pub key: String,
    /// Where the value that lost was written.
    pub replaced: Origin,
    /// Where the value in force was written.
    pub winner: Origin,
}

/// What resolving the tree produced.
#[derive(Clone, Debug)]
pub struct Resolved {
    /// The flattened configuration.
    pub config: Config,
    /// Every file the tree reached, in the order § 3 read them. This is what the boot log prints.
    pub files: Vec<PathBuf>,
    /// Every path the tree looked at without reading it as a file: each `[[include]] dir` it
    /// listed, and each `optional` include that was absent. A running server checks these beside
    /// [`files`](Resolved::files), so a file added to an included directory, or an optional include
    /// that appears, is noticed like an edit.
    pub probed: Vec<PathBuf>,
    /// Every override, in the order they happened. § 9's `nvs config dump --origin` prints these in
    /// full and the boot log summarizes them; dropping them is not an option (see the module doc).
    pub overrides: Vec<Override>,
    /// What the tree is only *advised* about — § 7's readable secret file (`W1005`), its
    /// credential with an edge space (`W1007`), a configured store no capability may reach
    /// (`W1008`, [`crate::store::advise`]), a TLS key log a development host left on and a proxy
    /// that resolves this deployment's destinations (`W1009` and `W1010`, both
    /// [`crate::http::advise`]). A
    /// refusal is never here: it arrives as the `Err` of [`resolve`] instead, so a caller that
    /// ignores this field has lost a warning and never a boundary.
    pub warnings: Vec<Diagnostic>,
    /// The merged table [`config`](Resolved::config) was deserialized from, kept rather than
    /// dropped because two things still need it. `rule:config/every-matching-app-block-applies-least-specific-first` layers `[[app]]` blocks by
    /// **the same** later-wins merge this one used, reporting overrides the same way, and it
    /// cannot run at resolve time because it needs an entry file. § 9's `nvs config dump --origin`
    /// renders keys the typed tree has no field for. Cost: one table for the length of a boot or
    /// reload, dropped with the rest of this struct.
    ///
    pub table: toml::Table,
    /// Where every leaf in that table was written, by dotted key — `db.main.password_file`, and
    /// `app.1.root` for the second `[[app]]` block, whichever file appended it.
    pub origins: BTreeMap<String, Origin>,
    /// § 7's secrets, by the key each is the value of — `db.main.password`. Beside the table rather
    /// than in it, for the reason [`mod@crate::secret`]'s module doc gives: the table is what
    /// `dump --toml` serializes whole, and a content in it would have to be redacted again by every
    /// reader that walks it.
    pub secrets: BTreeMap<String, Secret>,
}

/// What `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s four steps selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Roots {
    /// Step 1 or step 2: files to read, in order, each absolute.
    Files(Vec<PathBuf>),
    /// Step 3: the shipped defaults, which are a complete and valid configuration — capabilities
    /// deny-all, `[mode] default = "production"` — and so are [`Config::default`] plus the defaults
    /// each reader applies, not a failure to find anything.
    Defaults,
}

/// The file step 2 looks for in the working directory, and so the name anything writing one gives
/// it: the lookup and the writer have to agree on the spelling or the file created is not the file
/// found.
pub const LOCAL_FILE: &str = "nvs.toml";

/// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`: every `--config` in the order given, else `./nvs.toml`, else the shipped defaults.
///
/// `flags` are resolved against `cwd` because a path given on the command line means what a shell
/// argument means (§ 5). **Any explicit `--config` disables step 2 entirely**, so an operator naming
/// files never gets a surprise merge with whatever is in the working directory — which is why this
/// does not probe `cwd` when `flags` is non-empty even if every one of them turns out to be missing.
/// A `--config` naming a file that does not exist is a hard refusal, and it is [`resolve`]'s: this
/// function reports what was *asked for*, and optionality is a property a file declares about its
/// own includes and never something argv can assert.
pub fn roots(flags: &[PathBuf], cwd: &Path, files: &dyn Files) -> Roots {
    if !flags.is_empty() {
        return Roots::Files(flags.iter().map(|flag| absolute(cwd, flag)).collect());
    }
    let local = cwd.join(LOCAL_FILE);
    if files.exists(&local) {
        // Exactly this directory, never a walk upward: what makes reading the wrong file a
        // question about one path rather than about an ancestry.
        return Roots::Files(vec![normalize(&local)]);
    }
    Roots::Defaults
}

/// Reads the tree and flattens it — `rule:config/later-wins-and-every-override-is-recorded`'s one ordered stream.
///
/// Each root's own keys land first, then its includes depth-first in list order, then the next
/// root. A later assignment wins and is recorded.
///
/// # Errors
///
/// One [`Diagnostic`]: a file that cannot be read (`E0605`), an include cycle or a nesting deeper
/// than [`MAX_INCLUDE_DEPTH`] (`E0606`), a file outside § 6's trust boundary (`E0607`), a secret
/// file § 7 will not take a value from (`E0608`), an `[[app]]` block `rule:config/an-application-is-its-entry-file-path` cannot key
/// (`E0609`), a `[[schedule]]` entry `rule:config/scheduled-work-is-a-config-block` cannot arm (`E0611`), an `[http]` pair `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`
/// refuses (`E0612`), a `[log] target` `rule:errors/engine-floor` does not spell (`E0613`), or anything either
/// of `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s per-file
/// refusals catches (`E0601`/`E0604`), which arrives already carrying its own file's line.
pub fn resolve(
    roots: &Roots,
    sources: &mut SourceMap,
    files: &dyn Files,
) -> Result<Resolved, Diagnostic> {
    let mut merge = Merge::default();
    let Roots::Files(paths) = roots else {
        return merge.finish();
    };
    for path in paths {
        read_into(&mut merge, path, sources, files, &mut Vec::new(), 0)?;
    }
    // § 7 runs over the flattened tree and not over each file, because which `password_file` is in
    // force is a question only the merge has answered — a later file may have replaced it, and
    // reading the loser would be a secret file the configuration does not use.
    let mut resolved = merge.finish()?;
    // Lifted out and put back so the two passes can hold `config` mutably while reading the
    // origins they resolve their relative paths against.
    let origins = std::mem::take(&mut resolved.origins);
    let materialized = crate::secret::materialize(&mut resolved.config, &origins, files)?;
    resolved.warnings = materialized.warnings;
    resolved.secrets = materialized.secrets;
    // `rule:core-classes/db-capabilities`'s trust anchors, beside § 7's secrets and for the same reason: a `_file` on a
    // `[db]` block is resolved against the file that wrote it, and only the merge knows which of
    // them won.
    crate::db::canonicalize(&mut resolved.config, &mut resolved.table, &origins, files)?;
    // `[storage.<name>] root`, by the same rule and for the same reason as a `[db]` path.
    crate::db::canonicalize_storage(&mut resolved.config, &mut resolved.table, &origins);
    // `[http.client.tls] roots`, immediately after the `[db]` bundles and through the same check:
    // the anchors an outbound call verifies against are a wider authority than one block's, so a
    // pass that resolved them anywhere but inside the trust boundary would be the one file in the
    // tree a stranger could rewrite.
    crate::http::canonicalize(&mut resolved.config, &mut resolved.table, &origins, files)?;
    // The path-scoped `[capabilities]` grants, by the same rule and for the same reason: a
    // relative root means the directory of the file that wrote it, and only the merge knows which
    // file that was. Before the `[[app]]` roster below, which keys `app.<n>` on the same indexes.
    crate::capability::anchor(&mut resolved.config, &mut resolved.table, &origins);
    // `[[schedule]] script` and `[log] handler`, by the same rule: each names a script an isolate
    // runs, and a fire or an escalation must run the file beside the configuration that named it.
    crate::schedule::anchor(&mut resolved.config, &mut resolved.table, &origins);
    crate::log::anchor(&mut resolved.config, &mut resolved.table, &origins);
    // `rule:security/db-pool-reset-is-a-boundary`'s pool bounds, in the same pass's second half: a `lifetime` that spells nothing
    // is a boot refusal naming its file, rather than the first acquire of the first request.
    crate::db::validate(&resolved.config, &origins)?;
    // `rule:core-classes/queue-storage-is-a-table`'s `[queue]`, immediately after the roster it names: whether `connection = "main"`
    // has a block to point at is a question only the merged `[db]` map can answer.
    crate::queue::validate(&resolved.config, &origins)?;
    // `rule:config/an-application-is-its-entry-file-path`'s keys, for the same reason: `[[app]]` blocks accumulate across the tree (§ 4),
    // so the roster only exists once the merge is done. Alone among the passes above it
    // rewrites `config` and never the table, which reaches a driver only because the roster is read
    // off `resolved.config` and then dropped — `Snapshot::retype`'s doc § *The seam every
    // `resolve()` pass is measured against* is where that rule lives, and what a later pass here
    // is checked against.
    crate::app::canonicalize(&mut resolved.config, &origins, files)?;
    // `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling`'s bound, once the roster is keyed: what a block asks for is compared against the
    // global `[limits.hard]`, which is a property of the merged tree and of nothing smaller.
    crate::app::bound(&resolved.config, &origins)?;
    // `rule:observability/memory-high-water-writes-a-warn`'s fraction, over the same keyed roster:
    // a share outside `0..=1` is refused where it was written, not read as off at the first request.
    crate::app::high_water(&resolved.config, &origins)?;
    // `rule:config/scheduled-work-is-a-config-block`, `rule:config/cron-is-five-fields-and-nothing-more` and `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`, last because it reads the `[capabilities]` the merge settled: a scheduled
    // script is checked against the `script.spawn` roots, which a later file may have replaced.
    crate::schedule::validate(&resolved.config, &origins, files)?;
    // `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`, over the merged tree for the same reason: which `[http.cors] origins` and
    // `[http.cookies] secure` are in force is a question only the whole stream has answered, and a
    // per-file check would refuse a base file an include was about to correct.
    crate::http::validate(&resolved.config, &origins)?;
    // `rule:http-server/the-server-block-is-boot-class`'s inbound waits, beside the outbound half above: what the merge settled is
    // the wait the listener will actually be started with, and a `false` there is the one spelling
    // `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` has no version of.
    crate::server::validate(&resolved.config, &origins)?;
    // `rule:errors/engine-floor`'s target, over the merged tree for the http check's reason and for one of its
    // own: the floor is the rung that reports when nothing else can, so the last place to discover
    // that its destination does not parse is the failure it was configured to report.
    crate::log::validate(&resolved.config, &origins)?;
    // `rule:config/opcache-revalidation-is-system-class`'s `validate`, which has two values: a
    // `never` read as the default would leave a host believing its code is pinned when every
    // change still reaches the next request.
    crate::cache::validate(&resolved.config, &origins)?;
    // `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`, enforcing `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`: which store holds a session is the one directive whose
    // wrong value never reports itself at run time — a per-core session store forgets people rather
    // than failing — so the merged tree is the last moment anything can say so.
    crate::session::validate(&resolved.config, &origins)?;
    // `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`, which is the session
    // check above answered the other way round: a Unix socket on a build with no `AF_UNIX` transport
    // is not waiting for this project, so the key is refused where it was written rather than read
    // as loopback TCP by something that could not tell an operator it had.
    crate::store::validate(&resolved.config, &origins)?;
    // `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s pair, beside the store
    // question above and after the `[capabilities]` the merge settled: a `[cache.shared] url` with
    // no `cache.shared` grant names a store nothing may open, which is knowable here and otherwise
    // not until the first request that touches the tier. An advisory rather than a refusal, so it
    // joins the warnings instead of returning — `W1008`'s own doc is why.
    resolved
        .warnings
        .extend(crate::store::advise(&resolved.config, &origins));
    // The outbound block's two announcements, for the same reason in the other direction:
    // `http::validate` above refused the trees that are wrong, so what is left is a development
    // host whose whole outbound traffic is decryptable (`W1009`) and a deployment that handed the
    // address question to its proxy (`W1010`) — each a tree somebody meant, and each owing the log
    // the sentence at every start.
    resolved
        .warnings
        .extend(crate::http::advise(&resolved.config, &origins));
    // `rule:observability/metrics-and-trace-blocks-are-system`'s two exporters, over the merged tree for the log check's reason and with the
    // same shape of failure as the session one: both blocks are `System`, so what is in force is
    // what this boot read, and a sink nobody can spell exports nothing while looking exactly like a
    // deployment that had nothing to export.
    crate::export::validate(&resolved.config, &origins)?;
    resolved.origins = origins;
    Ok(resolved)
}

/// One file: its own keys into the merge, then its includes, depth-first in list order.
fn read_into(
    merge: &mut Merge,
    path: &Path,
    sources: &mut SourceMap,
    files: &dyn Files,
    chain: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), Diagnostic> {
    // § 6 first, because everything after this line is a statement about a file this process has
    // decided to trust — and because the canonical path it hands back is what makes the cycle test
    // below see one file through two spellings of it rather than compare the spellings.
    let trusted = files
        .trust(path)
        .map_err(|why| untrusted(path, &why, "the configuration reads it"))?;
    let path = trusted.as_path();
    if let Some(cycle) = chain.iter().position(|seen| same_file(seen, path)) {
        return Err(cycle_refusal(&chain[cycle..], path));
    }
    if depth > MAX_INCLUDE_DEPTH {
        return Err(Diagnostic::error(
            code::E_INCLUDE_CYCLE,
            format!("`[[include]]` nested deeper than {MAX_INCLUDE_DEPTH} files"),
        )
        .with_note(chain_note(chain, path))
        .with_help("an include tree this deep is nearly always a cycle a symlink hid"));
    }

    let text = files
        .read_config(path)
        .map_err(|err| unreadable(path, &err, "the configuration reads it"))?;
    let (source, parsed) =
        crate::file::parse::<Config>(sources, &path.display().to_string(), &text);
    // `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s two refusals are per file, so this is where they run: the diagnostic carries
    // this file's own line, before anything of it has been merged into the stream.
    let file = parsed?;
    let table: toml::Table = toml::from_str(&text).map_err(|err| {
        // Unreachable in practice — the typed parse above accepted the same bytes — but a
        // configuration reader is not the place to unwrap on that reasoning.
        Diagnostic::error(code::E_BAD_DIRECTIVE, err.message().to_string())
    })?;

    let origin = Origin {
        path: path.to_path_buf(),
        source,
    };
    merge.absorb(&table, &origin);
    merge.files.push(path.to_path_buf());

    // § 5: a relative path resolves against the directory of the file it is written in.
    let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    chain.push(path.to_path_buf());
    for include in &file.include {
        for target in include_targets(include, &base, path, files, &mut merge.probed)? {
            read_into(merge, &target, sources, files, chain, depth + 1)?;
        }
    }
    chain.pop();
    Ok(())
}

/// The files one `[[include]]` entry names, in the order § 2 mandates.
///
/// `path` is one file; `dir` is every `*.toml` **directly** inside, ascending by byte order of
/// filename and not recursing — the sort is here rather than left to the reader because § 3 makes
/// order decide the answer, and a capability grant settled by directory-entry order is not a design.
///
/// A directory it lists and an optional file it finds absent are pushed onto `probed`.
fn include_targets(
    include: &crate::tree::Include,
    base: &Path,
    written_in: &Path,
    files: &dyn Files,
    probed: &mut Vec<PathBuf>,
) -> Result<Vec<PathBuf>, Diagnostic> {
    let optional = include.optional.unwrap_or(false);
    match (&include.path, &include.dir) {
        (Some(one), None) => {
            let target = absolute(base, Path::new(one));
            if !files.exists(&target) {
                if optional {
                    // § 6: absence is the whole of what `optional` covers, and an absent file
                    // offers nothing to check — so the check falls on the directory it would
                    // appear in, which is the only place a promise about a file that does not
                    // exist yet can be kept.
                    trust_slot(&target, files)?;
                    probed.push(target);
                    return Ok(Vec::new());
                }
                return Err(
                    unreadable(&target, "no such file", "an `[[include]]` names it")
                        .with_note(format!("included from `{}`", written_in.display()))
                        .with_help("write `optional = true` if the file is allowed to be absent"),
                );
            }
            Ok(vec![target])
        }
        (None, Some(dir)) => {
            let target = absolute(base, Path::new(dir));
            if !files.exists(&target) {
                if optional {
                    trust_slot(&target, files)?;
                    probed.push(target);
                    return Ok(Vec::new());
                }
                return Err(
                    unreadable(&target, "no such directory", "an `[[include]]` names it")
                        .with_note(format!("included from `{}`", written_in.display())),
                );
            }
            // § 6 on the directory itself. Every file below it is checked as it is read, but an
            // empty one is checked by nothing at all — and a directory anyone can write is a slot
            // in exactly the sense that section means, whether or not it holds a file yet.
            files.trust(&target).map_err(|why| {
                untrusted(&target, &why, "an `[[include]]` reads every `*.toml` in it")
            })?;
            let mut entries: Vec<PathBuf> = files
                .list(&target)
                .map_err(|err| unreadable(&target, &err, "an `[[include]]` names it"))?
                .into_iter()
                .filter(|entry| entry.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            probed.push(target);
            Ok(entries)
        }
        _ => Err(Diagnostic::error(
            code::E_BAD_DIRECTIVE,
            "an `[[include]]` entry carries `path` or `dir`, never both and never neither"
                .to_string(),
        )
        .with_note(format!("written in `{}`", written_in.display()))),
    }
}

/// § 6's check on the directory an absent `optional` include would have appeared in — the nearest
/// one that exists, because the promise is only as strong as the shallowest directory an attacker
/// would have to write to keep it, and a directory nobody has created is not a slot yet.
fn trust_slot(target: &Path, files: &dyn Files) -> Result<(), Diagnostic> {
    let mut slot = target.parent();
    while let Some(dir) = slot {
        if files.exists(dir) {
            files.trust(dir).map_err(|why| {
                untrusted(
                    dir,
                    &why,
                    "an absent `optional` `[[include]]` would appear in it, which is a standing \
                     slot anyone able to write that directory may later fill",
                )
            })?;
            return Ok(());
        }
        slot = dir.parent();
    }
    Ok(())
}

/// What a failed § 6 check reports: `E0605` when the path could not be examined at all, which is
/// the same "cannot read" every other reader failure gets, and `E0607` when it was examined and the
/// boundary does not hold. Keeping those apart is the whole point of the split — an operator told
/// "cannot read" goes looking for a typo, and the answer is a mode.
pub(crate) fn untrusted(path: &Path, why: &Untrusted, who: &str) -> Diagnostic {
    match why {
        Untrusted::Unreadable(message) => unreadable(path, message, who),
        Untrusted::Breach(message) => Diagnostic::error(code::E_UNTRUSTED_CONFIG, message.clone())
            .with_note(format!(
                "{who}, and `rule:config/ownership-is-the-trust-boundary` leaves no file in the tree writable by any account but \
                 this one: whoever can write one of them can grant themselves every capability the \
                 configuration carries"
            ))
            .with_help(trust::REMEDY.to_string()),
    }
}

/// `E0605`, phrased so the message says what could not be read and the note says why anyone tried.
pub(crate) fn unreadable(path: &Path, why: &str, who: &str) -> Diagnostic {
    Diagnostic::error(
        code::E_UNREADABLE_CONFIG,
        format!("cannot read `{}`: {why}", path.display()),
    )
    .with_note(format!(
        "{who}, and only an absent `optional` include is allowed to be missing"
    ))
}

/// `E0606` for a cycle, naming the whole chain rather than its last file: the cycle is a property of
/// the path, and an operator who is shown only the repeated file has to rediscover how it was reached.
fn cycle_refusal(cycle: &[PathBuf], repeated: &Path) -> Diagnostic {
    Diagnostic::error(
        code::E_INCLUDE_CYCLE,
        format!(
            "`[[include]]` cycle: `{}` includes itself",
            repeated.display()
        ),
    )
    .with_note(chain_note(cycle, repeated))
}

/// The chain as one arrow-joined line.
fn chain_note(chain: &[PathBuf], last: &Path) -> String {
    let mut note = String::from("the chain is ");
    for path in chain {
        note.push_str(&format!("`{}` -> ", path.display()));
    }
    note.push_str(&format!("`{}`", last.display()));
    note
}

/// The merge state: the table so far, where each of its leaves came from, and the record.
#[derive(Default)]
struct Merge {
    table: toml::Table,
    origins: BTreeMap<String, Origin>,
    overrides: Vec<Override>,
    files: Vec<PathBuf>,
    probed: Vec<PathBuf>,
}

impl Merge {
    /// Folds one file's table into the stream, recording what it replaced.
    fn absorb(&mut self, table: &toml::Table, origin: &Origin) {
        merge_table(
            &mut self.table,
            table,
            origin,
            "",
            &mut self.origins,
            &mut self.overrides,
        );
    }

    /// Deserializes the merged table into the typed tree.
    ///
    /// Empty tables are dropped first, so the typed tree never holds a block that says nothing:
    /// `[metrics]` with no key under it is `None`, exactly as a file that never wrote the header.
    /// That is what lets the shipped file carry live headers and still resolve to what no file
    /// resolves to, and it is decided here, once, so that no reader of a block has to treat
    /// "present and empty" and "absent" alike by remembering to.
    fn finish(mut self) -> Result<Resolved, Diagnostic> {
        drop_empty_tables(&mut self.table);
        let config = toml::Value::Table(self.table.clone())
            .try_into::<Config>()
            // Every file was typed on its own before it was merged, so nothing new can be unknown
            // here; what can still fail is a key one file wrote as a table and another as a value.
            .map_err(|err| Diagnostic::error(code::E_BAD_DIRECTIVE, err.message().to_string()))?;
        Ok(Resolved {
            config,
            files: self.files,
            probed: self.probed,
            overrides: self.overrides,
            warnings: Vec::new(),
            table: self.table,
            origins: self.origins,
            secrets: BTreeMap::new(),
        })
    }
}

/// Removes every table with no key under it, at any depth, including one left empty by the removal
/// of the tables inside it.
///
/// An `[[entry]]` is never removed, however little it says: writing one is a statement that the
/// entry exists, and `[[app]]` with no path is refused as that (`E0609`) rather than skipped. The
/// tables *inside* an entry are walked like any other.
fn drop_empty_tables(table: &mut toml::Table) {
    table.retain(|_, value| match value {
        toml::Value::Table(inner) => {
            drop_empty_tables(inner);
            !inner.is_empty()
        }
        toml::Value::Array(entries) => {
            for entry in entries {
                if let toml::Value::Table(inner) = entry {
                    drop_empty_tables(inner);
                }
            }
            true
        }
        _ => true,
    });
}

/// `rule:config/later-wins-and-every-override-is-recorded` and `rule:config/a-value-array-replaces-and-a-table-appends` as one walk: recurse into a table, append an array of tables, replace
/// anything else and say so.
///
/// The replace/append split is decided **structurally** — an array whose entries are all tables is
/// an array of tables — rather than from a list of the `[[block]]` spellings § 4 names. That is
/// the same distinction TOML itself draws, which is § 4's own argument for the split: appending is
/// what two `[[schedule]]` blocks already mean inside one file, so a further such block added by a
/// later ADR gets the right behaviour with nothing here to update. An empty array is ambiguous
/// under that rule and does not need to be: appending nothing and replacing with nothing agree.
pub(crate) fn merge_table(
    dest: &mut toml::Table,
    src: &toml::Table,
    origin: &Origin,
    prefix: &str,
    origins: &mut BTreeMap<String, Origin>,
    overrides: &mut Vec<Override>,
) {
    for (key, value) in src {
        let dotted = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match (dest.get_mut(key), value) {
            (Some(toml::Value::Table(into)), toml::Value::Table(from)) => {
                merge_table(into, from, origin, &dotted, origins, overrides);
            }
            (Some(toml::Value::Array(into)), toml::Value::Array(from))
                if is_array_of_tables(into) && is_array_of_tables(from) =>
            {
                // By index, and the index is the one in the *destination*: an appended block's
                // relative path resolves against the file that wrote that block (§ 5), which the
                // array-level origin cannot say once two files have each contributed one.
                let base = into.len();
                for (offset, entry) in from.iter().enumerate() {
                    claim(
                        entry,
                        origin,
                        &format!("{dotted}.{}", base + offset),
                        origins,
                    );
                }
                into.extend(from.iter().cloned());
                origins.insert(dotted, origin.clone());
            }
            (Some(slot), _) => {
                if let Some(replaced) = origins.insert(dotted.clone(), origin.clone()) {
                    // Only a value that some file actually wrote is an override. A key seeded by
                    // nothing has nothing to report and no origin to name.
                    overrides.push(Override {
                        key: dotted,
                        replaced,
                        winner: origin.clone(),
                    });
                }
                *slot = value.clone();
            }
            (None, _) => {
                dest.insert(key.clone(), value.clone());
                claim(value, origin, &dotted, origins);
            }
        }
    }
}

/// Records `origin` for every leaf inside a value being inserted for the first time, so that a later
/// file replacing one of them has a `replaced` origin to name.
fn claim(
    value: &toml::Value,
    origin: &Origin,
    dotted: &str,
    origins: &mut BTreeMap<String, Origin>,
) {
    match value {
        toml::Value::Table(table) => {
            for (key, nested) in table {
                claim(nested, origin, &format!("{dotted}.{key}"), origins);
            }
        }
        // An array of tables is claimed by index as well as whole, so that `app.0.root` names the
        // file *that block* was written in. The array-level key stays: it is what a later file
        // appending to the array overwrites.
        toml::Value::Array(entries) if is_array_of_tables(entries) => {
            for (index, entry) in entries.iter().enumerate() {
                claim(entry, origin, &format!("{dotted}.{index}"), origins);
            }
            origins.insert(dotted.to_string(), origin.clone());
        }
        _ => {
            origins.insert(dotted.to_string(), origin.clone());
        }
    }
}

/// Whether every entry is a table, which is what makes the array an array of tables.
fn is_array_of_tables(array: &[toml::Value]) -> bool {
    !array.is_empty() && array.iter().all(toml::Value::is_table)
}

/// Whether two paths name the same file, lexically.
///
/// **Lexical**, and it does not have to be more: every path that reaches this has come back from
/// [`Files::trust`] canonical, so a symlinked ring closes on a name already in the chain and what
/// is left here is folding away the `.` and `..` a reader is free to hand back. The check `stat`s
/// every file anyway, so canonicalizing there costs nothing this would not have cost twice.
fn same_file(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// `path` made absolute against `base` when it is relative — `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`.
pub(crate) fn absolute(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize(path)
    } else {
        normalize(&base.join(path))
    }
}

/// `.` dropped and `..` resolved lexically, with no filesystem access.
///
/// A `..` with nothing to pop is kept rather than discarded: dropping it would turn `../etc/nvs.toml`
/// into `etc/nvs.toml` and read a different file than the operator wrote.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}
