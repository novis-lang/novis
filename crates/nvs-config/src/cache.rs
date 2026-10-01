//! `rule:config/the-extension-set-is-in-every-unit-key`: the one `env_hash` both compiled-unit cache keys carry.
//!
//! The caches that key a compiled unit — the on-disk artifact cache (`rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`) and the in-memory
//! unit table (`rule:config/an-edit-reaches-the-next-request-without-a-restart`) — are given the same environment digest by § 4, so that a unit compiled
//! against one extension set, one CPU or one compiler is never reused against another:
//!
//! ```text
//! extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
//! env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)
//! content_hash       = BLAKE3(each file's name ‖ text ‖ folder, in program order)
//! probe_hash         = BLAKE3(each path the unit's autoload resolution probed, in probe order)
//! artifact_key       = BLAKE3(content_hash ‖ env_hash)
//! program_id         = BLAKE3(content_hash of each unit, in program order ‖ env_hash)
//! ```
//!
//! **The source is hashed once.** Both keys carry [`content_hash`], and the on-disk key is derived
//! from that digest rather than from the source a second time, so a unit's bytes cross BLAKE3 exactly
//! once however many caches it lands in; [`artifact_key`] itself runs over 64 bytes. Both digests are
//! cryptographic on purpose — `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` says what a merely fast hash would give up.
//!
//! **[`ProbeHash`] is the one key field a lookup cannot compute for itself.**
//! `rule:packaging/autoload-probes-fold-into-the-cache-key` folds the paths a unit's `autoload`
//! resolution probed — the misses included — into that unit's key, and the list exists only once
//! the resolution has run. So a caller addressing a unit *before* compiling it spells
//! [`ProbeHash::unrecorded`], and the compile that follows re-keys its own result through
//! [`UnitKey::with_probes`]; the caches that hold such a key are what say which of the two a given
//! entry is under. The digest is over the trace alone, because the content and the environment are
//! already the key's other two fields.
//!
//! **Both keys are built here, in the crate that holds the extension set**, rather than each in the
//! cache that uses it. Those caches live in other crates and answer the same question; § 4's
//! whole content is that they answer it *with the same value*, and two implementations of one
//! formula is how one of them ends up hashing three pins where the other hashes four. A
//! [`UnitKey`]'s fields are private and [`UnitKey::new`] takes an [`EnvHash`] for the same reason:
//! there is no way to spell a key without one.
//!
//! **[`program_id`] is here for that same rule rather than because it is a cache key** — it is not
//! one. It is `rule:programs/no-runtime-autoload`'s `Core\Program::id()`, and it belongs to this module because its two
//! inputs do: written where its answer is *used* it would be a second spelling of a formula whose
//! whole content is that everyone spells it identically. It reads the digests a resolution has
//! already computed, so a program's identity reads no source a second time.
//!
//! **Every variable-length field is length-prefixed before it is hashed**, so a pin set of
//! `["ab", "c"]` and one of `["a", "bc"]` are different environments rather than the same one.
//!
//! **[`Revalidation`] is here for [`UnitKey`]'s own reason**, one layer up: `rule:config/an-edit-reaches-the-next-request-without-a-restart`
//! puts a `PathEntry` in front of the unit table this key addresses, and `[opcache]
//! validate` and `revalidate_freq` are what decide when a resolve looks at the file behind a path
//! at all. Reading them is a question about the configuration and not about any one cache, so it
//! lands beside the key rather than inside the crate that happens to hold the table — the same
//! separation `[server]`'s waits have from the listener that arms them ([`mod@crate::server`]).
//! `settle` is the third, and the one whose default the run mode chooses. A `validate` that is
//! neither `mtime` nor `hash` does not load ([`validate`]); a `revalidate_freq` or `settle` that is
//! not a duration is not refused yet and keeps its default.
//!
//! **`compiler_version_hash` is the running executable, not the release version.** The package
//! version alone is the same string for every build of an unreleased tree, so a developer who
//! rebuilt the compiler would address the artifacts its predecessor emitted — bytes that really are
//! this version's, with a correct checksum, and emitted by different codegen. [`build_stamp`]
//! answers with this process's own binary instead: its path, its byte length and its modification
//! time. Two builds of one version therefore key their units apart. A compiler that cannot examine
//! its own binary keys apart from every one that can, but not from another that also could not —
//! that single case is closed a layer up, by [`build_is_identified`], which is what a cache outliving
//! the process is asked before it opens a directory.
//!
//! It is deliberately not a hash of the binary's *contents*, which would be the exact identity and
//! would cost a pass over tens of megabytes at every process start. A build that can rewrite the
//! compiler in place while preserving both its length and its modification nanosecond is already
//! running as the compiler, so the cheaper stamp gives up nothing an attacker did not already have.
//!
//! Cost: one BLAKE3 pass over a few dozen bytes plus one per `[[extension]]` entry, per
//! [`env_hash`] call, and one `metadata` call for the whole process — [`build_stamp`] is computed
//! once and held. The digest is a snapshot's value, not a unit's: a caller computes it when it
//! publishes a snapshot and carries it into every key built against that snapshot. [`content_hash`]
//! is one pass over the source, per unit; [`artifact_key`] and [`UnitKey::new`] then read none of
//! the source at all.
//!

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Setting};
use crate::value::{Quantity, Unit};

/// A 32-byte BLAKE3 digest, printed as lowercase hex.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The digest's bytes, as `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`'s header field holds them.
    ///
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({self})")
    }
}

/// The environment half of both cache keys — `rule:config/the-extension-set-is-in-every-unit-key`.
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EnvHash(Digest);

impl EnvHash {
    /// The digest itself, for a header field or a directory name.
    pub fn digest(&self) -> Digest {
        self.0
    }
}

impl fmt::Display for EnvHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A whole program's identity — what `rule:programs/no-runtime-autoload`'s `Core\Program::id()` answers, computed by
/// [`program_id`] and displayed as 64 lowercase hex characters, never truncated here.
///
/// A type of its own for [`EnvHash`]'s reason: this module holds several digests over overlapping
/// inputs, and only the type system keeps a program's identity out of a cache key.
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ProgramId(Digest);

impl ProgramId {
    /// The digest itself, for a caller that wants its bytes rather than its hex.
    pub fn digest(&self) -> Digest {
        self.0
    }
}

impl fmt::Display for ProgramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The environment this process compiles for, as one digest — `rule:config/the-extension-set-is-in-every-unit-key`.
///
/// The configuration's contribution is its `[[extension]]` array and nothing else: every other
/// directive is read by a running request rather than baked into a compiled unit, so a reload that
/// changes one must *not* invalidate the caches.
///
pub fn env_hash(config: &Config) -> EnvHash {
    env_hash_of(config, build_stamp())
}

/// [`env_hash`] with the compiler build named rather than read, which is the seam a test drives:
/// two stamps are two builds, and [`None`] is a compiler that could not examine its own binary.
fn env_hash_of(config: &Config, build: Option<Digest>) -> EnvHash {
    let mut hasher = blake3::Hasher::new();
    // The triple, spelled from what the process can actually read. `rustc`'s own target string
    // needs a build script to reach, and the fields below distinguish the same set of hosts.
    feed(&mut hasher, std::env::consts::ARCH.as_bytes());
    feed(&mut hasher, std::env::consts::OS.as_bytes());
    feed(&mut hasher, target_env().as_bytes());
    hasher.update(&cpu_feature_bitset().to_le_bytes());
    feed(&mut hasher, env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(&[u8::from(cfg!(debug_assertions))]);
    // One discriminant byte, so an unstamped build is a different environment from any stamped one
    // rather than a prefix of it.
    match build {
        Some(stamp) => {
            hasher.update(&[1]);
            hasher.update(stamp.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(extension_set_hash(config).as_bytes());
    EnvHash(Digest(*hasher.finalize().as_bytes()))
}

/// Whether this process could identify its own build, which is what an on-disk cache is asked
/// before it opens a directory at all.
///
/// Every unstamped build shares one environment, so artifacts one of them wrote are addressable by
/// the next — the single case the stamp does not separate. A cache that outlives the process
/// therefore does not open: `nvs_cli::cache::from_config` reads this and answers with no cache,
/// which is the same cold-compile fallback every miss there already takes. The in-memory table is
/// unaffected, since nothing in one process's memory outlives that process.
pub fn build_is_identified() -> bool {
    build_stamp().is_some()
}

/// This compiler build, as one digest: the running executable's path, byte length and modification
/// time.
///
/// Held for the life of the process, so the `metadata` call happens once however many snapshots are
/// published. [`None`] when the binary cannot be named or examined — the module header says why
/// that keys apart from every build that can name itself rather than falling back to the version.
fn build_stamp() -> Option<Digest> {
    static STAMP: OnceLock<Option<Digest>> = OnceLock::new();
    *STAMP.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        let meta = std::fs::metadata(&exe).ok()?;
        let mut hasher = blake3::Hasher::new();
        feed(&mut hasher, exe.as_os_str().to_string_lossy().as_bytes());
        hasher.update(&meta.len().to_le_bytes());
        let modified = meta
            .modified()
            .ok()
            .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_nanos());
        hasher.update(&modified.to_le_bytes());
        Some(Digest(*hasher.finalize().as_bytes()))
    })
}

/// `BLAKE3(source)` — the one pass over a unit's bytes, which both cache keys are then built from.
pub fn content_hash(source: &[u8]) -> Digest {
    Digest(*blake3::hash(source).as_bytes())
}

/// What one unit's `autoload` resolution probed, as one value — `rule:packaging/autoload-probes-fold-into-the-cache-key`'s trace,
/// and the third field of a [`UnitKey`].
///
/// A type of its own for [`EnvHash`]'s reason: this is a third digest over the same unit, and only
/// the type system keeps it out of the content hash's argument position.
///
/// It holds no digest at all where nothing has probed this content yet ([`Self::unrecorded`]),
/// which is a different answer from the digest of an **empty** trace: a program declaring no
/// `autoload` really does probe nothing, and its unit is an ordinary hit rather than a key no
/// lookup can ever spell.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ProbeHash(Option<Digest>);

impl ProbeHash {
    /// The field a key carries when no resolution of this content has been recorded: the compile
    /// that produces a trace has not run, so this is what the lookup ahead of it addresses.
    #[must_use]
    pub fn unrecorded() -> Self {
        Self(None)
    }

    /// The digest itself, and [`None`] for [`Self::unrecorded`] — for a caller that wants the
    /// bytes rather than the key field.
    #[must_use]
    pub fn digest(&self) -> Option<Digest> {
        self.0
    }
}

/// `rule:packaging/autoload-probes-fold-into-the-cache-key`'s trace as one digest: every path the resolution probed, in probe
/// order, misses included (`nvs_hir::autoload::ProbeTrace`).
///
/// Order is significant, for the reason it is in [`program_id`] — the same paths probed in another
/// order are another resolution — and each path is length-prefixed ([`feed`]), so two probes cannot
/// run together into a third. A path is hashed as it was written down by the resolution that probed
/// it, which is what a later resolution of the same program will spell again.
///
/// Cost: one BLAKE3 pass over the trace's own bytes, once per compile, on a step that has just run
/// a whole front end. The trace is O(names × roots) and is released with the map it rode out on
/// (`rule:programs/memory-priority`); nothing of it is held here.
pub fn probe_hash(probed: &[PathBuf]) -> ProbeHash {
    discovery_hash(probed, &[])
}

/// [`probe_hash`], with every directory a discovery scan listed folded in after the probes: the
/// directory, then the sorted names it held, or nothing to list (`nvs_hir::autoload::Listing`).
///
/// So a listing whose names did not change hashes the same, and a module added under a scanned
/// directory is another key. With no listing the digest is [`probe_hash`]'s; with one, a length no
/// path can have separates the two halves, so no probe list can hash as a listing.
///
/// Cost: [`probe_hash`]'s, plus one pass over the listed names, once per compile.
pub fn discovery_hash(probed: &[PathBuf], listed: &[(PathBuf, Option<Vec<String>>)]) -> ProbeHash {
    let mut hasher = blake3::Hasher::new();
    for path in probed {
        feed(&mut hasher, path.as_os_str().to_string_lossy().as_bytes());
    }
    if !listed.is_empty() {
        hasher.update(&u64::MAX.to_le_bytes());
    }
    for (dir, names) in listed {
        feed(&mut hasher, dir.as_os_str().to_string_lossy().as_bytes());
        let count = names.as_ref().map_or(0, |names| names.len() as u64 + 1);
        hasher.update(&count.to_le_bytes());
        for name in names.iter().flatten() {
            feed(&mut hasher, name.as_bytes());
        }
    }
    ProbeHash(Some(Digest(*hasher.finalize().as_bytes())))
}

/// `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s on-disk key, as `rule:config/the-extension-set-is-in-every-unit-key` rekeyed it: `BLAKE3(content_hash ‖ env_hash)`.
///
/// Both inputs are fixed 32-byte digests, so neither needs [`feed`]'s length prefix to keep them
/// apart.
///
pub fn artifact_key(content: Digest, env: EnvHash) -> Digest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(content.as_bytes());
    hasher.update(env.0.as_bytes());
    Digest(*hasher.finalize().as_bytes())
}

/// `rule:programs/no-runtime-autoload`'s program identity: `BLAKE3(each unit's content hash, in program order ‖ env_hash)`.
///
/// Every input is a fixed 32-byte digest and the environment's is always the last one, so the
/// concatenation splits positionally and no field needs [`feed`]'s length prefix. Order is
/// significant on purpose: `units` is the resolution's own order, and the same files required in a
/// different order are a different program.
///
/// The environment is folded in for the reason `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` folds it into an artifact key — the
/// same sources compiled against another extension set, CPU or compiler are not the same running
/// program, and an id that could not tell them apart would name two of them the same thing.
///
/// Cost: one BLAKE3 pass over `32 × (units + 1)` bytes, once per program resolution and once per
/// hot-reload swap (`rule:config/an-edit-reaches-the-next-request-without-a-restart`), never per call — a first-call compute would put the whole hash on
/// one unlucky request's path. What it holds is 32 bytes per program, per `rule:programs/memory-priority`'s ledger.
///
pub fn program_id(units: &[Digest], env: EnvHash) -> ProgramId {
    let mut hasher = blake3::Hasher::new();
    for unit in units {
        hasher.update(unit.as_bytes());
    }
    hasher.update(env.0.as_bytes());
    ProgramId(Digest(*hasher.finalize().as_bytes()))
}

/// `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s in-memory unit key, as `rule:config/the-extension-set-is-in-every-unit-key` rekeyed it and
/// `rule:packaging/autoload-probes-fold-into-the-cache-key` rekeyed it again: `{ path, content_hash, probe_hash, env_hash }`.
///
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct UnitKey {
    path: PathBuf,
    content_hash: Digest,
    probes: ProbeHash,
    env: EnvHash,
}

impl UnitKey {
    /// The key `path` has under `env` when its source hashes to `content` and its `autoload`
    /// resolution probed what `probes` digests — [`content_hash`]'s and [`probe_hash`]'s values,
    /// carried rather than recomputed so neither the source nor the trace is walked twice.
    ///
    /// [`ProbeHash::unrecorded`] is the value a caller that has not compiled `content` yet has, and
    /// [`Self::with_probes`] is what the compile then answers with.
    pub fn new(path: &Path, content: Digest, probes: ProbeHash, env: EnvHash) -> Self {
        Self {
            path: path.to_path_buf(),
            content_hash: content,
            probes,
            env,
        }
    }

    /// The same key with the trace a finished resolution recorded, which is the one field the
    /// lookup ahead of that resolution could not name.
    ///
    /// The other three are read from the key rather than from the caller again, so a compile cannot
    /// publish its unit under a path, a content or an environment other than the one it claimed —
    /// a reload moving [`env_hash`] mid-compile is what that guards against.
    #[must_use]
    pub fn with_probes(self, probes: ProbeHash) -> Self {
        Self { probes, ..self }
    }

    /// The same key with the digest of the whole program a finished compile read, in place of
    /// the entry file's digest the lookup ahead of it could name
    /// (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
    ///
    /// Like [`Self::with_probes`], it keeps the path and the environment the key was claimed
    /// under.
    #[must_use]
    pub fn with_program(self, program: Digest) -> Self {
        Self {
            content_hash: program,
            ..self
        }
    }

    /// The trace this key is under — [`ProbeHash::unrecorded`] where the entry behind it was
    /// published by a compile that produced none.
    ///
    #[must_use]
    pub fn probes(&self) -> ProbeHash {
        self.probes
    }

    /// The unit's path, which is what `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s revalidation `stat`s.
    ///
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The content digest this key addresses: the whole program's once a compile has recorded
    /// it ([`Self::with_program`]), and the entry file's for a key a first compile claims under.
    pub fn content_hash(&self) -> Digest {
        self.content_hash
    }

    /// The environment this unit was compiled against.
    pub fn env(&self) -> EnvHash {
        self.env
    }
}

/// `[opcache] validate` — what a check looks at when it re-examines a path it has already
/// compiled (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
///
/// There are two values and no third: `rule:config/opcache-revalidation-is-system-class` removed
/// `never`, and [`validate`] refuses it at load with every other word neither of these spells.
///
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Validate {
    /// `mtime` — PHP's `validate_timestamps`: `stat`, and re-read the source only where the
    /// modification time or the size moved. The cheap pre-filter `rule:config/an-edit-reaches-the-next-request-without-a-restart`
    /// names, and the default in both modes.
    #[default]
    Mtime,
    /// `hash` — re-read and re-hash whenever the rate cap allows a check at all, so a file
    /// rewritten twice inside one timestamp tick is still observed.
    Hash,
}

impl Validate {
    /// The value `written` names, and `None` for a word that names none of them.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        match written {
            "mtime" => Some(Self::Mtime),
            "hash" => Some(Self::Hash),
            _ => None,
        }
    }
}

/// `[opcache]`'s revalidation directives, read into what one resolve asks — `rule:config/an-edit-reaches-the-next-request-without-a-restart`
/// steps 1-2.
///
/// All three are `System`-class (`rule:config/opcache-revalidation-is-system-class`): a request able
/// to raise the cap or the settle time for itself could hold a shipped fix back, and one able to
/// lower either could force a `stat` storm on a hot file or a compile per keystroke of a deploy. Nothing here is per-request, so
/// a caller holds one of these for a configuration generation and reads it on every resolve.
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Revalidation {
    /// What a check looks at, once the cap below has let one happen.
    pub validate: Validate,
    /// `revalidate_freq`: the interval between two passes of the background check over every loaded
    /// program. A request never checks, so the overhead is `N ⁄ revalidate_freq` stats whatever the
    /// request rate.
    pub freq: Duration,
    /// `settle`: how long no file of a changed program may have moved before it is compiled, so a
    /// deploy still being copied is compiled once, from the finished tree.
    pub settle: Duration,
}

/// `settle`'s startup row (`rule:config/a-startup-default-is-never-flipped`) in `production`, and
/// the default for a mode that is neither.
pub const SETTLE_PRODUCTION: Duration = Duration::from_secs(1);

/// `settle`'s startup row in `development`: short enough that a saved file reaches the next reload
/// of a browser tab, and long enough that an editor's write-then-rename is one change.
pub const SETTLE_DEVELOPMENT: Duration = Duration::from_millis(100);

impl Default for Revalidation {
    /// `mtime`, checked at most once every two seconds, and settled for production's one second.
    ///
    /// `rule:config/an-edit-reaches-the-next-request-without-a-restart` states neither number, so they are decided here, in the crate that reads the
    /// block — the same place `rule:config/opcache-file-cache-directives-are-system` leaves its file-cache pair to the implementation. Both
    /// are PHP's own `opcache` defaults, which is the behaviour every deployment this runtime is
    /// migrating from already has: an edit becomes visible without a restart, and a hot path pays
    /// at most one `stat` every two seconds for it. Neither is chosen by the run mode:
    /// `rule:config/opcache-revalidation-is-system-class` gives `validate` one default in both, and
    /// no value of the cap a developer's machine needs differs from an operator's.
    ///
    fn default() -> Self {
        Self {
            validate: Validate::Mtime,
            freq: Duration::from_secs(2),
            settle: SETTLE_PRODUCTION,
        }
    }
}

impl Revalidation {
    /// The policy `config`'s `[opcache]` block writes, with [`Revalidation::default`] for every key
    /// it leaves out.
    ///
    /// A tree that reached a snapshot has already passed [`validate`], so a `validate` value that
    /// spells nothing never arrives here from a load; the fallback is for a [`Config`] a caller
    /// built by hand. `revalidate_freq` is not refused at load yet, and a value that is not a
    /// duration keeps the default cap.
    ///
    /// An `[[app]]` block (`rule:config/a-mount-routes-and-an-app-block-sets-policy`) is not
    /// consulted: `[opcache]` is `System`-class and one process holds one unit cache, so a
    /// per-application answer would be a second policy over a table the applications share.
    ///
    /// An unwritten `settle` is the startup row of the mode `[mode] default` names, read here each
    /// time a configuration is published and never from a runtime flip, which only a request makes.
    ///
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let development = config
            .mode
            .as_ref()
            .and_then(|mode| mode.default.as_deref())
            == Some(crate::mode::DEVELOPMENT);
        let fallback = Self {
            settle: if development {
                SETTLE_DEVELOPMENT
            } else {
                SETTLE_PRODUCTION
            },
            ..Self::default()
        };
        let Some(opcache) = config.opcache.as_ref() else {
            return fallback;
        };
        Self {
            validate: opcache
                .validate
                .as_ref()
                .and_then(validate_of)
                .unwrap_or(fallback.validate),
            freq: opcache
                .revalidate_freq
                .as_ref()
                .and_then(|written| duration_of("opcache.revalidate_freq", written))
                .unwrap_or(fallback.freq),
            settle: opcache
                .settle
                .as_ref()
                .and_then(|written| duration_of("opcache.settle", written))
                .unwrap_or(fallback.settle),
        }
    }
}

/// One written `validate`, as the check it names, and `None` for a value that names none.
///
/// `true` is read as PHP's own spelling of the timestamp check, `validate_timestamps = 1`, because
/// an operator transcribing an `opcache` block they already run is writing the thing this
/// directive replaced. `false` was PHP's `validate_timestamps = 0`, the `never` this directive no
/// longer has, so it names nothing.
fn validate_of(setting: &Setting) -> Option<Validate> {
    match setting {
        Setting::Text(written) => Validate::of(written),
        Setting::Bool(true) => Some(Validate::Mtime),
        _ => None,
    }
}

/// `rule:config/opcache-revalidation-is-system-class`'s refusal, asked of the merged tree: an
/// `[opcache] validate` that is neither `mtime` nor `hash` does not load.
///
/// `never`, and the boolean `false` that meant it, are refused with every other word rather than
/// read as the default: a value that pins the code a process started with is the outage
/// `rule:config/an-edit-reaches-the-next-request-without-a-restart` exists to end, and a host
/// that wrote it believes it has something it does not.
///
/// # Errors
///
/// `E0601`, the code a directive with an invalid value gets, naming the value, `mtime` and `hash`,
/// and the file the value was written in.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(written) = config
        .opcache
        .as_ref()
        .and_then(|opcache| opcache.validate.as_ref())
    else {
        return Ok(());
    };
    if validate_of(written).is_some() {
        return Ok(());
    }
    let shown = match written {
        Setting::Text(word) => format!("\"{word}\""),
        Setting::Bool(flag) => flag.to_string(),
        Setting::Integer(count) => count.to_string(),
        Setting::Float(ratio) => ratio.to_string(),
        Setting::List(_) => "[...]".to_string(),
    };
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!("`[opcache] validate = {shown}` is not a value that key can hold"),
    )
    .with_note(format!(
        "a server checks every source file it compiled, so a changed file reaches the next request \
         without a restart, and `validate` only chooses how it checks{}",
        origin_note(origins.get("opcache.validate"))
    ))
    .with_help(
        "write `validate = \"mtime\"`, the default, or `validate = \"hash\"` for a file system whose \
         timestamps cannot be trusted"
            .to_string(),
    ))
}

/// One written `revalidate_freq` or `settle`, as the interval it names.
///
/// [`Quantity`] is the one parser for a duration anywhere in this tree (`rule:config/ini-set-is-core-config-set`), so `"2s"`,
/// `"500ms"` and a bare `2` all read here exactly as they do in `[limits]`. `false` is the spelling
/// `rule:config/three-changeability-classes` gives to "no ceiling" and means no wait at all: for
/// `revalidate_freq` a background check as often as the process allows, for `settle` a compile as soon as a change is seen.
///
fn duration_of(key: &str, setting: &Setting) -> Option<Duration> {
    match Quantity::parse(key, Unit::Duration, setting) {
        Ok(Quantity::Nanos(nanos)) => Some(Duration::from_nanos(nanos)),
        Ok(Quantity::Unbounded) => Some(Duration::ZERO),
        _ => None,
    }
}

/// `BLAKE3(sorted sha256 pins of the [[extension]] array)` — `rule:config/the-extension-set-is-in-every-unit-key`.
///
/// Sorted, so the set is order-independent: duplicate class names across extensions are refused at
/// load, which is what makes the *set* rather than the sequence the thing that matters. An entry
/// with no pin contributes its path instead, so an unpinned extension still rekeys every unit
/// rather than being invisible here.
///
fn extension_set_hash(config: &Config) -> Digest {
    let mut pins: Vec<&str> = config
        .extension
        .iter()
        .map(|extension| {
            extension
                .sha256
                .as_deref()
                .or(extension.path.as_deref())
                .unwrap_or("")
        })
        .collect();
    pins.sort_unstable();
    let mut hasher = blake3::Hasher::new();
    for pin in pins {
        feed(&mut hasher, pin.as_bytes());
    }
    Digest(*hasher.finalize().as_bytes())
}

/// Hashes one variable-length field, length first, so two fields cannot run together into a third.
fn feed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
}

/// The triple's environment field, which `std::env::consts` does not carry.
fn target_env() -> &'static str {
    if cfg!(target_env = "msvc") {
        "msvc"
    } else if cfg!(target_env = "gnu") {
        "gnu"
    } else if cfg!(target_env = "musl") {
        "musl"
    } else if cfg!(target_env = "sgx") {
        "sgx"
    } else {
        ""
    }
}

/// The CPU features the JIT may emit against, as a bitset.
///
/// **Detected at run time on x86_64, and read off the build on every other architecture.** The
/// question is what the compiler backend may *emit*, and on x86_64 that is a run-time decision:
/// one binary runs on a host with AVX-512 and on one without, and an artifact compiled on the
/// first is not valid on the second. Elsewhere the baseline is fixed at build time, so
/// `target_triple` already separates the hosts this would.
fn cpu_feature_bitset() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let mut bits = 0u64;
        for (index, present) in [
            std::is_x86_feature_detected!("sse4.2"),
            std::is_x86_feature_detected!("popcnt"),
            std::is_x86_feature_detected!("avx"),
            std::is_x86_feature_detected!("avx2"),
            std::is_x86_feature_detected!("bmi1"),
            std::is_x86_feature_detected!("bmi2"),
            std::is_x86_feature_detected!("fma"),
            std::is_x86_feature_detected!("avx512f"),
        ]
        .into_iter()
        .enumerate()
        {
            if present {
                bits |= 1 << index;
            }
        }
        bits
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut bits = 0u64;
        for (index, present) in [
            cfg!(target_feature = "neon"),
            cfg!(target_feature = "lse"),
            cfg!(target_feature = "fp16"),
            cfg!(target_feature = "dotprod"),
        ]
        .into_iter()
        .enumerate()
        {
            if present {
                bits |= 1 << index;
            }
        }
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in unit digest. [`program_id`] reads its inputs as opaque bytes, so a test does not
    /// need real sources to pin what the combine does with them.
    fn unit(seed: u8) -> Digest {
        Digest([seed; 32])
    }

    fn env(seed: u8) -> EnvHash {
        EnvHash(unit(seed))
    }

    #[test]
    fn two_builds_of_one_release_version_key_their_units_apart() {
        let config = Config::default();
        // Everything but the build stamp is this process's: one version, one target, one extension
        // set. Only the compiler binary differs, which is the case the package version misses.
        assert_ne!(
            env_hash_of(&config, Some(unit(1))),
            env_hash_of(&config, Some(unit(2))),
        );
        assert_eq!(
            env_hash_of(&config, Some(unit(1))),
            env_hash_of(&config, Some(unit(1))),
        );
        assert_ne!(
            env_hash_of(&config, None),
            env_hash_of(&config, Some(unit(1)))
        );
    }

    #[test]
    fn the_running_build_stamps_itself_once() {
        let first = build_stamp().expect("a test binary can name and examine itself");
        assert_eq!(
            first,
            build_stamp().expect("the stamp is held, not recomputed")
        );
        assert_eq!(
            env_hash(&Config::default()),
            env_hash_of(&Config::default(), Some(first))
        );
    }

    #[test]
    fn the_program_id_is_stable_for_identical_units_and_env() {
        let units = [unit(1), unit(2), unit(3)];
        let first = program_id(&units, env(9));
        assert_eq!(first, program_id(&units, env(9)));

        let hex = first.to_string();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn the_program_id_changes_when_one_units_content_does() {
        assert_ne!(
            program_id(&[unit(1), unit(2), unit(3)], env(9)),
            program_id(&[unit(1), unit(2), unit(4)], env(9)),
        );
    }

    #[test]
    fn the_program_id_changes_when_the_env_hash_does() {
        let units = [unit(1), unit(2)];
        assert_ne!(program_id(&units, env(9)), program_id(&units, env(10)));
    }

    #[test]
    fn the_program_id_changes_when_the_unit_order_does() {
        assert_ne!(
            program_id(&[unit(1), unit(2)], env(9)),
            program_id(&[unit(2), unit(1)], env(9)),
        );
    }
}
