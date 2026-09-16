//! `rule:config/the-extension-set-is-in-every-unit-key`: the one `env_hash` both compiled-unit cache keys carry.
//!
//! The caches that key a compiled unit — the on-disk artifact cache (`rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`) and the in-memory
//! unit table (`rule:config/an-edit-reaches-the-next-request-without-a-restart`) — are given the same environment digest by § 4, so that a unit compiled
//! against one extension set, one CPU or one compiler is never reused against another:
//!
//! ```text
//! extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
//! env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)
//! content_hash       = BLAKE3(source)
//! artifact_key       = BLAKE3(content_hash ‖ env_hash)
//! program_id         = BLAKE3(content_hash of each unit, in program order ‖ env_hash)
//! ```
//!
//! **The source is hashed once.** Both keys carry [`content_hash`], and the on-disk key is derived
//! from that digest rather than from the source a second time, so a unit's bytes cross BLAKE3 exactly
//! once however many caches it lands in; [`artifact_key`] itself runs over 64 bytes. Both digests are
//! cryptographic on purpose — `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` says what a merely fast hash would give up.
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
//! Neither directive is refused at boot yet: an unspelled `validate` falls back to the default,
//! and [`Validate::of`] is the one place a refusal would read the word.
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

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

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

/// `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s in-memory unit key, as `rule:config/the-extension-set-is-in-every-unit-key` rekeyed it: `{ path, content_hash, env_hash }`.
///
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct UnitKey {
    path: PathBuf,
    content_hash: Digest,
    env: EnvHash,
}

impl UnitKey {
    /// The key `path` has under `env` when its source hashes to `content` — [`content_hash`]'s
    /// value, carried rather than recomputed so the source is read once.
    pub fn new(path: &Path, content: Digest, env: EnvHash) -> Self {
        Self {
            path: path.to_path_buf(),
            content_hash: content,
            env,
        }
    }

    /// The unit's path, which is what `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s revalidation `stat`s.
    ///
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The source's own digest.
    pub fn content_hash(&self) -> Digest {
        self.content_hash
    }

    /// The environment this unit was compiled against.
    pub fn env(&self) -> EnvHash {
        self.env
    }
}

/// `[opcache] validate` — what a resolve looks at when it re-checks a path it has already
/// compiled (`rule:config/an-edit-reaches-the-next-request-without-a-restart` steps 1-2).
///
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Validate {
    /// `never` — a path compiled once is answered from the unit table for the life of the process
    /// and no resolve spends a syscall. This is production's value, selected there by the run mode
    /// as an `rule:config/a-startup-default-is-never-flipped` row rather than by this type.
    ///
    Never,
    /// `mtime` — PHP's `validate_timestamps`: `stat`, and re-read the source only where the
    /// modification time or the size moved. The cheap pre-filter `rule:config/an-edit-reaches-the-next-request-without-a-restart`
    /// names, and the default.
    #[default]
    Mtime,
    /// `hash` — re-read and re-hash whenever the rate cap allows a check at all, so a file
    /// rewritten twice inside one timestamp tick is still observed.
    Hash,
}

impl Validate {
    /// `rule:config/a-startup-default-is-never-flipped`'s row for this directive: the value a host **starts** with when `[opcache]`
    /// writes none, `never` in `production` and the timestamp check in `development`.
    ///
    /// It is `mtime` rather than `hash` on the permissive side because the row's cell reads "on"
    /// and PHP's own `validate_timestamps = 1` — the thing an operator is transcribing, per
    /// [`validate_of`] — is the stamp. A mode that is neither of the two is `production`, which is
    /// § 5's answer for a host that wrote nothing at all and the fail-closed direction besides.
    ///
    /// A § 3a row is fixed at boot and never re-derived, so this is read where the policy is built
    /// and nowhere on the request path; § 4's runtime mode flip re-derives only § 3's rows.
    ///
    #[must_use]
    pub fn started_in(mode: &str) -> Self {
        if mode == crate::mode::DEVELOPMENT {
            Self::Mtime
        } else {
            Self::Never
        }
    }

    /// The value `written` names, and `None` for a word that names none of them.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        match written {
            "never" => Some(Self::Never),
            "mtime" => Some(Self::Mtime),
            "hash" => Some(Self::Hash),
            _ => None,
        }
    }
}

/// `[opcache]`'s revalidation directives, read into what one resolve asks — `rule:config/an-edit-reaches-the-next-request-without-a-restart`
/// steps 1-2.
///
/// Both are `System`-class (`rule:config/three-changeability-classes`) and that ADR says why in its own words: a request able to
/// set `validate = never` for itself could pin a version of the code past a shipped fix, and one
/// able to lower the cap could force a `stat` storm on a hot file. Nothing here is per-request, so
/// a caller holds one of these for a configuration generation and reads it on every resolve.
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Revalidation {
    /// What a check looks at, once the cap below has let one happen.
    pub validate: Validate,
    /// `revalidate_freq`: the shortest interval between two checks of one path. A resolve inside it
    /// reuses the digest the last check observed and spends no syscall, which is what bounds the
    /// overhead at `N ⁄ revalidate_freq` stats rather than at the request rate.
    pub freq: Duration,
}

impl Default for Revalidation {
    /// `mtime`, checked at most once every two seconds.
    ///
    /// `rule:config/an-edit-reaches-the-next-request-without-a-restart` states neither number, so they are decided here, in the crate that reads the
    /// block — the same place `rule:config/opcache-file-cache-directives-are-system` leaves its file-cache pair to the implementation. Both
    /// are PHP's own `opcache` defaults, which is the behaviour every deployment this runtime is
    /// migrating from already has: an edit becomes visible without a restart, and a hot path pays
    /// at most one `stat` every two seconds for it.
    ///
    /// **This is the type's own value and not what a configured host runs**: `validate`'s startup
    /// default is the run mode's ([`from_config`](Self::from_config)), and it reaches this one only
    /// for the `freq` beside it.
    ///
    fn default() -> Self {
        Self {
            validate: Validate::Mtime,
            freq: Duration::from_secs(2),
        }
    }
}

impl Revalidation {
    /// The policy `config`'s `[opcache]` block writes, with the startup default for every key it
    /// leaves out — and for a value that spells nothing, which the module doc records as the
    /// refusal this crate does not make yet.
    ///
    /// **`validate`'s fallback is the run mode's, not [`Revalidation::default`]'s**, because it is
    /// an `rule:config/a-startup-default-is-never-flipped` row: [`Validate::started_in`] over `[mode] default`, which a tree that
    /// writes no mode at all leaves at `production`. The cap beside it is deliberately *not* a row
    /// — § 3a says so in its own words, there being no value of `revalidate_freq` a developer's
    /// machine needs that an operator's does not — so it keeps [`Revalidation::default`]'s two
    /// seconds under either mode. § 3's first property is what makes the pair coherent: an
    /// `[opcache] validate` written beside `mode = "development"` still wins, because the mode
    /// supplies a default and nothing more.
    ///
    /// The mode read here is the tree's own `[mode] default`. An `[[app]]` block's mode
    /// (`rule:config/a-mount-routes-and-an-app-block-sets-policy`) is deliberately not consulted: `[opcache]` is `System`-class and one
    /// process holds one unit cache, so a per-application answer would be a second policy over a
    /// table the applications share.
    ///
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let fallback = Self {
            validate: Validate::started_in(
                config
                    .mode
                    .as_ref()
                    .and_then(|mode| mode.default.as_deref())
                    .unwrap_or(crate::mode::PRODUCTION),
            ),
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
                .and_then(freq_of)
                .unwrap_or(fallback.freq),
        }
    }
}

/// One written `validate`, as the check it names.
///
/// A boolean is read as PHP's own spelling of the same directive — `validate_timestamps = 0` is
/// `never` and `1` is the timestamp check — rather than refused, because an operator transcribing
/// an `opcache` block they already run is writing the thing this directive replaced.
fn validate_of(setting: &Setting) -> Option<Validate> {
    match setting {
        Setting::Text(written) => Validate::of(written),
        Setting::Bool(false) => Some(Validate::Never),
        Setting::Bool(true) => Some(Validate::Mtime),
        _ => None,
    }
}

/// One written `revalidate_freq`, as the interval it names.
///
/// [`Quantity`] is the one parser for a duration anywhere in this tree (`rule:config/ini-set-is-core-config-set`), so `"2s"`,
/// `"500ms"` and a bare `2` all read here exactly as they do in `[limits]`. `false` is the spelling
/// `rule:config/three-changeability-classes` gives to "no ceiling" and means no cap at all — a check on every resolve, which is
/// what a developer watching one file asks for and what the default deliberately is not.
///
fn freq_of(setting: &Setting) -> Option<Duration> {
    match Quantity::parse("opcache.revalidate_freq", Unit::Duration, setting) {
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
