//! [ADR 0078] § 4: the one `env_hash` both compiled-unit cache keys carry.
//!
//! Two caches key a compiled unit — the on-disk artifact cache ([ADR 0042] § 2) and the in-memory
//! unit table ([ADR 0017]) — and § 4 gives them the same environment digest so that a unit compiled
//! against one extension set, one CPU or one compiler is never reused against another:
//!
//! ```text
//! extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
//! env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)
//! content_hash       = BLAKE3(source)
//! artifact_key       = BLAKE3(content_hash ‖ env_hash)
//! ```
//!
//! **The source is hashed once.** Both keys carry [`content_hash`], and the on-disk key is derived
//! from that digest rather than from the source a second time, so a unit's bytes cross BLAKE3 exactly
//! once however many caches it lands in; [`artifact_key`] itself runs over 64 bytes. Both digests are
//! cryptographic on purpose — [ADR 0042] § 1 says what a merely fast hash would give up.
//!
//! **Both keys are built here, in the crate that holds the extension set**, rather than each in the
//! cache that uses it. The two caches live in two other crates and answer the same question; § 4's
//! whole content is that they answer it *with the same value*, and two implementations of one
//! formula is how one of them ends up hashing three pins where the other hashes four. A
//! [`UnitKey`]'s fields are private and [`UnitKey::new`] takes an [`EnvHash`] for the same reason:
//! there is no way to spell the pre-§ 4 key.
//!
//! **Every variable-length field is length-prefixed before it is hashed**, so a pin set of
//! `["ab", "c"]` and one of `["a", "bc"]` are different environments rather than the same one.
//!
//! Cost: one BLAKE3 pass over a few dozen bytes plus one per `[[extension]]` entry, per
//! [`env_hash`] call. It is a snapshot's value, not a unit's — a caller computes it when it
//! publishes a snapshot and carries it into every key built against that snapshot. [`content_hash`]
//! is one pass over the source, per unit; [`artifact_key`] and [`UnitKey::new`] then read none of
//! the source at all.
//!
//! **Known gap: `compiler_version_hash` is the release version, so two builds of the same version
//! share it.** A developer who rebuilds the compiler without bumping [`CARGO_PKG_VERSION`](env)
//! keeps every artifact keyed against the old one; `debug_assertions` separates a debug build from
//! a release build and nothing separates two debug builds. Closing it needs a build stamp that
//! moves with the source, which is a `build.rs` this crate does not have yet and which nothing can
//! use until the caches themselves are on disk.
//!
//! [ADR 0017]: ../../../docs/adr/0017-hot-reload-without-restart.md
//! [ADR 0042]: ../../../docs/adr/0042-on-disk-artifact-cache-format.md
//! [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md

use std::fmt;
use std::path::{Path, PathBuf};

use crate::tree::Config;

/// A 32-byte BLAKE3 digest, printed as lowercase hex.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The digest's bytes, as [ADR 0042] § 2's header field holds them.
    ///
    /// [ADR 0042]: ../../../docs/adr/0042-on-disk-artifact-cache-format.md
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

/// The environment half of both cache keys — [ADR 0078] § 4.
///
/// [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md
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

/// The environment this process compiles for, as one digest — [ADR 0078] § 4.
///
/// The configuration's contribution is its `[[extension]]` array and nothing else: every other
/// directive is read by a running request rather than baked into a compiled unit, so a reload that
/// changes one must *not* invalidate the caches.
///
/// [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md
pub fn env_hash(config: &Config) -> EnvHash {
    let mut hasher = blake3::Hasher::new();
    // The triple, spelled from what the process can actually read. `rustc`'s own target string
    // needs a build script to reach, and these three fields distinguish the same set of hosts.
    feed(&mut hasher, std::env::consts::ARCH.as_bytes());
    feed(&mut hasher, std::env::consts::OS.as_bytes());
    feed(&mut hasher, target_env().as_bytes());
    hasher.update(&cpu_feature_bitset().to_le_bytes());
    feed(&mut hasher, env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(&[u8::from(cfg!(debug_assertions))]);
    hasher.update(extension_set_hash(config).as_bytes());
    EnvHash(Digest(*hasher.finalize().as_bytes()))
}

/// `BLAKE3(source)` — the one pass over a unit's bytes, which both cache keys are then built from.
pub fn content_hash(source: &[u8]) -> Digest {
    Digest(*blake3::hash(source).as_bytes())
}

/// [ADR 0042] § 1's on-disk key, as [ADR 0078] § 4 rekeyed it: `BLAKE3(content_hash ‖ env_hash)`.
///
/// Both inputs are fixed 32-byte digests, so neither needs [`feed`]'s length prefix to keep them
/// apart.
///
/// [ADR 0042]: ../../../docs/adr/0042-on-disk-artifact-cache-format.md
/// [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md
pub fn artifact_key(content: Digest, env: EnvHash) -> Digest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(content.as_bytes());
    hasher.update(env.0.as_bytes());
    Digest(*hasher.finalize().as_bytes())
}

/// [ADR 0017]'s in-memory unit key, as [ADR 0078] § 4 rekeyed it: `{ path, content_hash, env_hash }`.
///
/// [ADR 0017]: ../../../docs/adr/0017-hot-reload-without-restart.md
/// [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md
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

    /// The unit's path, which is what [ADR 0017]'s revalidation `stat`s.
    ///
    /// [ADR 0017]: ../../../docs/adr/0017-hot-reload-without-restart.md
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

/// `BLAKE3(sorted sha256 pins of the [[extension]] array)` — [ADR 0078] § 4.
///
/// Sorted, so the set is order-independent: duplicate class names across extensions are refused at
/// load, which is what makes the *set* rather than the sequence the thing that matters. An entry
/// with no pin contributes its path instead, so an unpinned extension still rekeys every unit
/// rather than being invisible here.
///
/// [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md
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
