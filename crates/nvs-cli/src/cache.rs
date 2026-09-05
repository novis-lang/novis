//! [ADR 0042]'s on-disk artifact cache: one immutable, content-addressed file per compiled unit,
//! and the writer that publishes one by a single atomic rename.
//!
//! § 1's layout is `<cache_dir>/<key[0..2]>/<key[2..]>.nvsc`, where the key is
//! [`nvs_config::cache::artifact_key`]'s `BLAKE3(content_hash ‖ env_hash)` spelled as 64 lowercase
//! hex characters. The fan-out exists so that a directory listing stays small enough for the
//! platform to walk cheaply; nothing reads it on the hit path, and § 6's eviction is the only caller
//! that ever will.
//!
//! **The environment is in the address, not only in the header.** An artifact built for another
//! target, another CPU-feature set, another compiler build or another `[[extension]]` set is a path
//! this process never looks up, so it costs one failed `open` rather than an open-then-reject
//! ([ADR 0078] § 4 owns the digest; this module only carries it).
//!
//! # § 2's header
//!
//! ```text
//! magic "NVSC" | format_version: u16 | env_hash: [u8; 32] | payload_len: u64 | checksum: [u8; 32]
//! ```
//!
//! 78 bytes, then the payload. The two integers are little-endian and nothing negotiates that,
//! because `env_hash` already covers the target triple: a host that would read them the other way
//! round derives a different `env_hash`, so it looks for a different path and never opens this file.
//! `env_hash` is repeated in the header even though it is folded into the path — § 2's defence in
//! depth against a `BLAKE3` collision or a file placed there by hand rather than written here.
//!
//! # § 4's write path
//!
//! Write header and payload to `.tmp-<random>` **in the shard directory the final name lives in**,
//! `fsync` it, then one `rename` onto the final path. Same directory so the rename cannot cross a
//! filesystem and degrade into a copy. **No lock file, ever, and no directory `fsync`**: losing an
//! un-synced entry to a crash costs one recompile, which is a cache miss and not a defect, so the
//! sync buys nothing this design needs. If the final path already exists when the rename would
//! happen, another writer has already published byte-identical content — the key *is* the content
//! hash — so this writer discards its own temp file instead. There is nothing to reconcile because
//! there is nothing that could differ.
//!
//! **A failure to write is not an error anyone hears about.** § 3 states that discipline for the
//! reader and it holds just as well here: the caller drops an [`Err`] and the next process
//! recompiles. [`store`](Cache::store) still returns one so a test can assert on it, and so that a
//! caller which wants to log a persistently unwritable cache directory can.
//!
//! Cost: one BLAKE3 pass over the payload, one `create_dir_all` on a shard's first write, one
//! `fsync` and one `rename`, all on a path where a real compile has just happened. A warm hit pays
//! none of it.
//!
//! # § 3's read path
//!
//! `mmap` the file read-only — never starting from an executable mapping — then check magic,
//! `format_version` and `env_hash` against what this process expects, check `payload_len` against
//! the mapping's own length, and hash the payload. Every failure is a cache miss, and none of them
//! is visible to the script being run: § 3 is explicit that a bad entry is exactly as invisible as a
//! cold cache, so nothing here returns a diagnostic, panics or throws.
//!
//! **Verification is a property of the type, not of a caller's discipline.** [`Cache::load`] hands
//! back a [`Verified`], which is constructed on exactly one code path — the far side of the
//! checksum comparison — and holds the mapping privately. A caller cannot reach the bytes without
//! going through it, so the step that would make those pages executable cannot be reached from an
//! unverified mapping even by mistake.
//!
//! **Two failures also delete the file; three do not.** A checksum mismatch does, which is § 3's own
//! instruction: the key is the content hash, so a file at that path whose contents hash differently
//! can only be corrupt or tampered and can never become a second valid version. A `payload_len` that
//! disagrees with the mapping is treated the same way, one step earlier — that field exists to catch
//! a truncation before the checksum does, the entry is corrupt on the same argument, and leaving it
//! costs a re-open on every future run. A wrong magic, `format_version` or `env_hash` is *not*
//! deleted: those three mean "not this process's file" rather than "broken", and deleting on them
//! would let one build of the compiler evict another's entries out of a shared cache directory.
//!
//! Cost of a hit: one `open`, one `mmap`, one BLAKE3 pass over the payload. The ADR's
//! *Investigation* weighs that against reading into a heap buffer and takes the mapping — the page
//! cache does the I/O once and the hash runs over it with no copy.
//!
//! # § 5's directory check
//!
//! [`Cache::new`] refuses a cache directory another local account can write, once, before anything
//! is read out of it, through [`nvs_config::trust`] — [ADR 0103] § 6's boundary, and the one
//! implementation of it. § 5 says why no checksum can stand in for this: a principal who can write
//! the directory computes a perfectly valid header over payload bytes of their own choosing, so a
//! checksum answers "is this the file I wrote" and never "should I trust whoever wrote it".
//!
//! **The check is the constructor's**, so a `Cache` value is itself the evidence that it passed —
//! the discipline [`Verified`] holds on the read path, one level up. Nothing per entry ever asks
//! again: § 5 makes this a property of the directory, checked once at process start, and it is
//! what the `unsafe` mapping in [`load`](Cache::load) rests on.
//!
//! Two details § 5 leaves here. A directory that does not exist yet is checked at the nearest
//! ancestor that does ([`existing_root`]), because that is the shallowest directory an attacker
//! would have to write in order to fill the slot the first [`store`](Cache::store) will create —
//! [ADR 0103] § 6's own answer for an absent `optional` include. And the configured spelling is
//! kept rather than the canonical path the check hands back: there is no path *comparison* here to
//! protect, unlike the config tree's cycle test, and an absent directory has no canonical spelling
//! at all, so one rule covers both.
//!
//! A refusal is the caller's to report, and it is a refusal to start naming the path rather than a
//! silent fall back to compiling every time. § 3's "invisible to the script" discipline is about a
//! bad *entry*; a cache directory anyone can write is a breach of the boundary itself, and
//! [ADR 0103] § 6 answers that the same way wherever it appears.
//!
//! # § 6's eviction
//!
//! A content-addressed entry never needs invalidating for correctness — only for growth — so the
//! whole of eviction is a size question, and § 6 hangs it off the *miss*: after a store, with a
//! configured probability, walk the directory and, if it is over its cap, delete oldest-by-`mtime`
//! entries down to a floor below that cap. [`Cache::store`] is the only caller,
//! [`gc`](Cache::gc) is the same walk without the roll — § 6's `nvs cache gc` — and nothing on the
//! read path calls either. **A warm hit performs no directory walk, checks no size and pays
//! nothing beyond § 3's verify-then-map**, which is what keeps M6's warm-start bullet reachable.
//!
//! The roll is PHP's `session.gc_probability`/`gc_divisor` shape deliberately, and the floor is
//! hysteresis: evicting to exactly the cap leaves the next miss's roll finding work again, so a
//! cache hovering at the boundary would walk on nearly every one. § 7 leaves the three defaults to
//! the implementation and [`Eviction`] states them with their reasons.
//!
//! # Known gaps
//!
//! **A payload is a relocatable object image, not a dump of the JIT's finished pages.** ADR 0042's
//! *Investigation* describes a warm hit as mapping "the bytes directly as the pages the JIT would
//! otherwise have produced", and that is not reachable from this compiler for two independent
//! reasons. First, `cranelift_jit::JITModule` has no serialization at all: it finalizes into memory
//! it owns and hands back a code pointer, and there is no API yielding the bytes plus the
//! relocations another process would need. Second, and the one that would survive such an API, a
//! byte-perfect page dump would be *wrong* in the next process, because a compiled page holds host
//! addresses — a class descriptor's, and a statically resolved call target's — that are valid only
//! for the process that allocated the descriptors and compiled the callee. Every one of them now
//! arrives as a *relocation* rather than an immediate, which is what makes the dump's replacement
//! possible: a descriptor is a named symbol `nvs-codegen`'s own `class_desc_symbol` mints and its
//! `Classes` docs own, and a call target was always a `func_addr`.
//!
//! So the payload is `cranelift-object`'s `ObjectProduct` — the same `nvs_ir` emitted a second way,
//! through a `Module` that records relocations instead of resolving them — and a warm hit maps,
//! verifies, applies those relocations, and only then makes the pages executable. **That last part
//! amends § 3's letter**, which mapped `PROT_READ` and `mprotect`ed the very same mapping, where a
//! relocated image needs a private writable one first; the checksum discipline is untouched, since
//! the hash still covers the file's bytes and the patching happens after it. Per the loop goal's
//! standing decision the redesign is recorded rather than started: it is a `nvs-codegen` slice — a
//! second `Module` implementation, and a named symbol for every address the JIT bakes in — and it
//! is in the handoff's backlog, not in this crate.
//!
//! [ADR 0048](/docs/adr/0048-portable-single-file-executables.md) is not the other half of
//! this. Its § 2 decides a bundle carries *source*, not precompiled artifacts, and feeds into this
//! cache rather than out of it, so there is no already-produced payload for the cache to ship over.
//!
//! [ADR 0042]: ../../../docs/adr/0042-on-disk-artifact-cache-format.md
//! [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md

// Nothing outside this module's own tests calls either half yet: the *Known gaps* entry above says
// what has to land in `nvs-codegen` before there is a payload to publish, and the compile-pipeline
// call sites wait on the same thing. Until then `dead_code` is naming a slice that has not happened
// rather than an item nothing will use.
#![allow(dead_code)]

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use nvs_config::cache::{Digest, EnvHash};
use nvs_config::trust::{self, Untrusted};
use rand::RngExt;

/// § 2's magic, the first four bytes of every artifact.
pub(crate) const MAGIC: [u8; 4] = *b"NVSC";

/// § 2's `format_version`. A file carrying any other value is a cache miss, never an error, so this
/// is bumped rather than migrated whenever the payload's meaning changes.
pub(crate) const FORMAT_VERSION: u16 = 1;

/// § 2's header, in bytes: magic 4, version 2, `env_hash` 32, `payload_len` 8, checksum 32.
pub(crate) const HEADER_LEN: usize = 4 + 2 + 32 + 8 + 32;

/// The extension every published artifact carries — § 1.
pub(crate) const EXTENSION: &str = "nvsc";

/// § 2's header, as the fields a reader checks rather than as bytes.
///
/// `env_hash` and `checksum` are raw arrays and not [`Digest`]s: every use of them is a byte-wise
/// comparison against what this process expects, which is exactly what § 2 asks for, and a decoder
/// that had to build a typed digest first would need a constructor whose only caller is a file it
/// has not verified yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Header {
    /// § 2's `format_version`, as written. A reader compares it to [`FORMAT_VERSION`].
    pub(crate) format_version: u16,
    /// [ADR 0078] § 4's environment digest, repeated from the path as defence in depth.
    ///
    /// [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md
    pub(crate) env_hash: [u8; 32],
    /// The payload's length in bytes, so a truncated file is caught before its checksum is.
    pub(crate) payload_len: u64,
    /// `BLAKE3` over the payload bytes alone — never over the header.
    pub(crate) checksum: [u8; 32],
}

impl Header {
    /// The header `payload` gets when it is published under `env`.
    #[must_use]
    pub(crate) fn for_payload(env: EnvHash, payload: &[u8]) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            env_hash: *env.digest().as_bytes(),
            payload_len: u64::try_from(payload.len()).expect("a payload's length fits a u64"),
            checksum: *blake3::hash(payload).as_bytes(),
        }
    }

    /// The header's on-disk bytes, magic included.
    #[must_use]
    pub(crate) fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0_u8; HEADER_LEN];
        out[0..4].copy_from_slice(&MAGIC);
        out[4..6].copy_from_slice(&self.format_version.to_le_bytes());
        out[6..38].copy_from_slice(&self.env_hash);
        out[38..46].copy_from_slice(&self.payload_len.to_le_bytes());
        out[46..78].copy_from_slice(&self.checksum);
        out
    }

    /// The header at the front of `bytes`, or [`None`] when there is not one there.
    ///
    /// The magic is checked here rather than by the caller because it is the one field with no
    /// typed home: everything else this returns is compared against what the process expects, and a
    /// `Header` that got past this function is one whose four bytes said `NVSC`.
    #[must_use]
    pub(crate) fn decode(bytes: &[u8]) -> Option<Self> {
        let head: &[u8; HEADER_LEN] = bytes.get(..HEADER_LEN)?.try_into().ok()?;
        if head[0..4] != MAGIC {
            return None;
        }
        Some(Self {
            format_version: u16::from_le_bytes([head[4], head[5]]),
            env_hash: head[6..38].try_into().expect("32 bytes of env_hash"),
            payload_len: u64::from_le_bytes(
                head[38..46].try_into().expect("8 bytes of payload_len"),
            ),
            checksum: head[46..78].try_into().expect("32 bytes of checksum"),
        })
    }
}

/// Why [`Cache::load`] is not returning an artifact — and, in the second case, why it is taking the
/// file with it.
///
/// The module doc's § 3 section owns which mismatch is which and why the split is where it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Miss {
    /// Not this process's file: absent, or headed for another format version or environment.
    Foreign,
    /// This process's file, and broken. It can never become valid, so it is deleted.
    Corrupt,
}

/// One artifact that passed every one of § 3's checks, still mapped read-only.
///
/// This type is the whole of "verified before a single page is executable": it is constructed on
/// exactly one code path, the far side of the checksum comparison in [`Cache::load`], and it owns
/// the mapping privately, so there is no way to reach an artifact's bytes without having verified
/// them. The mapping is a [`Mmap`] and not an `MmapMut`, so the pages stay read-only for its whole
/// life; the step that would change that is the module doc's *Known gaps* entry.
#[derive(Debug)]
pub(crate) struct Verified {
    /// The whole file, header included, as the page cache holds it.
    map: Mmap,
    /// The decoded header, already checked against this process.
    header: Header,
}

impl Verified {
    /// The artifact's payload — the bytes the header's checksum covers, and nothing else.
    #[must_use]
    pub(crate) fn payload(&self) -> &[u8] {
        &self.map[HEADER_LEN..]
    }

    /// § 2's header, as it was read.
    #[must_use]
    pub(crate) fn header(&self) -> Header {
        self.header
    }
}

/// What [`Cache::store`] did — § 4's two outcomes, and neither is a failure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Stored {
    /// This writer published the entry.
    Written,
    /// The entry was already there when the rename would have happened, so this writer discarded
    /// its own temp file. By construction the two are byte-identical.
    AlreadyPresent,
}

/// One cache directory, and the environment every artifact in it was compiled against.
///
/// Holding `env` here rather than passing it per call is what keeps § 2's header field and § 1's
/// key from being able to disagree inside one process.
#[derive(Clone, Debug)]
pub(crate) struct Cache {
    dir: PathBuf,
    env: EnvHash,
    eviction: Eviction,
}

/// § 6's hysteresis floor, as a percentage of the cap: a walk that evicts anything evicts down to
/// here rather than to the cap itself, so a cache sitting at the boundary does not find work on
/// every subsequent miss. A fifth of the cap is one walk's worth of headroom — small enough that
/// the cache stays near its configured size, large enough that the walks stay rare.
const GC_FLOOR_PERCENT: u64 = 80;

/// § 6's growth policy: the size the cache is kept near, and how often a miss checks.
///
/// These are § 7's `opcache.file_cache_max_size` and its
/// `opcache.file_cache_gc_probability`/`opcache.file_cache_gc_divisor` pair, which
/// [`nvs_config::tree::Opcache`] holds; § 7 leaves their defaults to the implementation and
/// [`Eviction::default`] is where they are decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Eviction {
    /// The cap in bytes. A walk finding the cache over this deletes down to [`GC_FLOOR_PERCENT`]
    /// of it; between two walks the cache may sit above it, which is § 6's accepted trade.
    pub(crate) max_size: u64,
    /// The numerator of the chance that one store walks.
    pub(crate) probability: u32,
    /// Its denominator. A zero on either side never walks, which is what a test wanting to observe
    /// the directory growing past the cap asks for.
    pub(crate) divisor: u32,
}

impl Default for Eviction {
    /// 256 MiB, walked on one store in a hundred.
    ///
    /// The cap is sized for the artifacts of every application on one host rather than for one of
    /// them: an artifact is tens of kilobytes, so this is thousands of compiled units, and a host
    /// that overruns it loses only the oldest of them to a recompile. The rate is PHP's own
    /// `session.gc_probability`/`gc_divisor` spelling, and one in a hundred is chosen against the
    /// walk's cost rather than against the clock — the walk is one `readdir` per shard on a path
    /// where a real compile has just happened, so a hundredth of that is unmeasurable, while a
    /// rarer roll would let the overshoot grow for no saving worth having.
    fn default() -> Self {
        Self {
            max_size: 256 * 1024 * 1024,
            probability: 1,
            divisor: 100,
        }
    }
}

/// The nearest ancestor of `dir` that exists, which is `dir` itself once it does.
///
/// § 5's check runs once at process start, and the cache directory is usually created by the first
/// [`store`](Cache::store) rather than by an operator, so at that moment there is often nothing to
/// examine. [ADR 0103] § 6 answers the same question for an absent `optional` include and this
/// takes its answer whole: the check falls on the directory that would hold the thing, and walks up
/// while that one does not exist either, because the promise is only as strong as the shallowest
/// directory an attacker would have to write in order to keep it.
fn existing_root(dir: &Path) -> &Path {
    let mut candidate = dir;
    while !candidate.exists() {
        match candidate.parent() {
            // A relative path runs out of components at the empty path, which names the working
            // directory: ask about that rather than about nothing.
            Some(parent) if parent.as_os_str().is_empty() => return Path::new("."),
            Some(parent) => candidate = parent,
            // An absolute path whose own root is absent: let the check say so about that root.
            None => break,
        }
    }
    candidate
}

impl Cache {
    /// The cache rooted at `dir` — `[cache] dir`, which [`nvs_config::tree::Cache`] holds — for
    /// artifacts compiled against `env`, once § 5's ownership check has passed on that directory.
    ///
    /// It neither creates nor canonicalizes it: the first [`store`](Self::store) creates it, and
    /// the module doc says why the configured spelling is the one kept.
    ///
    /// # Errors
    ///
    /// [`Untrusted`], when the directory — or, while it does not exist yet, the nearest ancestor
    /// that does — is owned by another account or writable by anyone but its owner. There is no
    /// second answer for a caller to get wrong: without this, there is no `Cache` at all.
    pub(crate) fn new(dir: impl Into<PathBuf>, env: EnvHash) -> Result<Self, Untrusted> {
        let dir = dir.into();
        trust::check(existing_root(&dir))?;
        Ok(Self {
            dir,
            env,
            eviction: Eviction::default(),
        })
    }

    /// The same cache under a different § 6 policy — `[opcache]`'s three keys, once a caller reads
    /// them out of the configuration rather than taking [`Eviction::default`].
    #[must_use]
    pub(crate) fn with_eviction(mut self, eviction: Eviction) -> Self {
        self.eviction = eviction;
        self
    }

    /// The root this cache writes under.
    #[must_use]
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// The environment every artifact under [`dir`](Self::dir) is keyed and headed with.
    #[must_use]
    pub(crate) fn env(&self) -> EnvHash {
        self.env
    }

    /// § 1's path for `key`: `<dir>/<key[0..2]>/<key[2..]>.nvsc`.
    #[must_use]
    pub(crate) fn path(&self, key: Digest) -> PathBuf {
        let hex = key.to_string();
        let (shard, rest) = hex.split_at(2);
        self.dir.join(shard).join(format!("{rest}.{EXTENSION}"))
    }

    /// § 3: the artifact published under `key`, verified whole, or [`None`].
    ///
    /// [`None`] covers every failure this can have — a missing file, a foreign one, a corrupt one,
    /// an unreadable directory — because § 3 makes a bad cache entry exactly as invisible to the
    /// running script as a cold cache is. Nothing here reports, and the caller's next move is the
    /// compile it would have done anyway.
    #[expect(
        unsafe_code,
        reason = "`Mmap::map` is unsafe because another process could rewrite the file underneath \
                  the mapping; § 4 publishes an artifact by rename and never rewrites one, and § 5's \
                  ownership check on the cache directory is what bounds who could, so this is the \
                  one call in this crate and the block below states the argument"
    )]
    pub(crate) fn load(&self, key: Digest) -> Option<Verified> {
        let path = self.path(key);
        let file = File::open(&path).ok()?;
        // SAFETY: a published artifact is immutable — § 4 writes it under a temp name and publishes
        // it by one rename, and § 5's ownership check on the cache directory is what keeps any
        // other principal from replacing or truncating it underneath this mapping. Every byte is
        // hashed below before any caller sees one.
        let map = unsafe { Mmap::map(&file) }.ok()?;
        drop(file);

        match Self::verify(self.env, &map) {
            Ok(header) => Some(Verified { map, header }),
            Err(Miss::Foreign) => None,
            Err(Miss::Corrupt) => {
                // The unmap comes first: Windows keeps a file open for as long as a mapping over it
                // is alive, so deleting while mapped fails there and would leave the entry to be
                // re-read on every future run.
                drop(map);
                drop(fs::remove_file(&path));
                None
            }
        }
    }

    /// § 3's four checks over a mapped artifact, cheapest first, hash last.
    fn verify(env: EnvHash, bytes: &[u8]) -> Result<Header, Miss> {
        let header = Header::decode(bytes).ok_or(Miss::Foreign)?;
        if header.format_version != FORMAT_VERSION || header.env_hash != *env.digest().as_bytes() {
            return Err(Miss::Foreign);
        }
        let payload = &bytes[HEADER_LEN..];
        let mapped = u64::try_from(payload.len()).expect("a mapping's length fits a u64");
        if header.payload_len != mapped {
            return Err(Miss::Corrupt);
        }
        if *blake3::hash(payload).as_bytes() != header.checksum {
            return Err(Miss::Corrupt);
        }
        Ok(header)
    }

    /// § 4: publish `payload` under `key` by one atomic rename, with no lock file.
    ///
    /// # Errors
    ///
    /// The underlying [`io::Error`] when the shard directory cannot be created, the temp file
    /// cannot be written or synced, or the rename fails for a reason other than the final path
    /// having appeared underneath it. Every one of them means "there is no cache entry", which the
    /// next process handles as an ordinary miss; the module doc says why nothing escalates.
    pub(crate) fn store(&self, key: Digest, payload: &[u8]) -> io::Result<Stored> {
        let stored = self.publish(key, payload)?;
        // § 6's roll is here and nowhere else. This line is only reached on a miss, which has just
        // paid for a real compile on the compile pool, so a directory walk is invisible against
        // it; the warm-hit path never comes near it.
        self.maybe_gc();
        Ok(stored)
    }

    /// § 4's write itself, split out so § 6's eviction can hang off the one place it completes.
    fn publish(&self, key: Digest, payload: &[u8]) -> io::Result<Stored> {
        let published = self.path(key);
        let shard = published
            .parent()
            .expect("a shard path always has the cache root above it");
        fs::create_dir_all(shard)?;

        let temp = shard.join(format!(".tmp-{:032x}", rand::rng().random::<u128>()));
        if let Err(err) = self.write_temp(&temp, payload) {
            drop(fs::remove_file(&temp));
            return Err(err);
        }

        // § 4's check, immediately before the rename rather than at entry: a race that both writers
        // pass replaces one identical file with another, which is why the ADR can call this
        // resolved rather than coordinated.
        if published.exists() {
            drop(fs::remove_file(&temp));
            return Ok(Stored::AlreadyPresent);
        }
        match fs::rename(&temp, &published) {
            Ok(()) => Ok(Stored::Written),
            Err(err) => {
                drop(fs::remove_file(&temp));
                // Windows refuses a rename onto a path another process holds open, so the loser of
                // a race arrives here rather than at the branch above. The entry it wanted is on
                // disk either way.
                if published.exists() {
                    Ok(Stored::AlreadyPresent)
                } else {
                    Err(err)
                }
            }
        }
    }

    /// § 6's probabilistic half: roll `probability` against `divisor`, and walk only on a hit.
    ///
    /// Whatever the walk found is dropped. A cache directory that cannot be read is not a failure
    /// the caller of [`store`](Self::store) hears about — the entry it asked to publish is on disk
    /// either way, and the module doc says why nothing in here escalates.
    fn maybe_gc(&self) {
        let Eviction {
            probability,
            divisor,
            ..
        } = self.eviction;
        if probability == 0 || divisor == 0 {
            return;
        }
        // The modulo's bias over a `u64` is immaterial against a divisor an operator writes by
        // hand: this decides how often a directory is walked, not anything a caller can observe.
        if u64::from(probability) <= rand::rng().random::<u64>() % u64::from(divisor) {
            return;
        }
        drop(self.gc());
    }

    /// § 6's walk: delete oldest-by-`mtime` artifacts until the cache is under the hysteresis
    /// floor, and answer what that freed.
    ///
    /// This is the deterministic half of § 6 — what `nvs cache gc` will call, and what
    /// [`maybe_gc`](Self::maybe_gc) calls with a probability in front of it. **Nothing on the read
    /// path calls it**: § 6 buys the warm-hit cost by putting every size question here.
    ///
    /// Only `.nvsc` files count and only they are deleted. A `.tmp-` file belongs to a writer that
    /// still has it open, and it is transient by construction — counting it would let a burst of
    /// concurrent writes evict published entries, and deleting it would break that writer's rename
    /// for nothing.
    ///
    /// # Errors
    ///
    /// Whatever reading the cache directory said.
    pub(crate) fn gc(&self) -> io::Result<u64> {
        let mut total = 0;
        let mut artifacts = Vec::new();
        for shard in fs::read_dir(&self.dir)? {
            let shard = shard?.path();
            if !shard.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&shard)? {
                let entry = entry?;
                if entry.path().extension().is_none_or(|ext| ext != EXTENSION) {
                    continue;
                }
                let meta = entry.metadata()?;
                total += meta.len();
                artifacts.push((meta.modified()?, meta.len(), entry.path()));
            }
        }
        if total <= self.eviction.max_size {
            return Ok(0);
        }

        // Oldest first, which is the only order § 6 names.
        artifacts.sort_unstable_by_key(|(modified, _, _)| *modified);
        let floor = self.eviction.max_size / 100 * GC_FLOOR_PERCENT;
        let mut freed = 0;
        for (_, len, path) in artifacts {
            if total <= floor {
                break;
            }
            // A file another process is reading refuses to be deleted on Windows. It is the
            // newest-but-one problem of a moment, and the next walk will find it again.
            if fs::remove_file(&path).is_ok() {
                total -= len;
                freed += len;
            }
        }
        Ok(freed)
    }

    /// The temp file, header then payload, synced and closed.
    fn write_temp(&self, temp: &Path, payload: &[u8]) -> io::Result<()> {
        let mut file = File::create(temp)?;
        file.write_all(&Header::for_payload(self.env, payload).encode())?;
        file.write_all(payload)?;
        file.sync_all()
    }
}

#[cfg(test)]
mod tests {
    //! These are the bin target's unit tests and not `tests/cache.rs`, because `nvs-cli` has no
    //! library target for an integration test to link against — `tests/meta.rs` and
    //! `tests/openapi.rs` drive the built binary instead, which is the right shape for a command's
    //! output and the wrong one for a module with no command in front of it yet.

    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, SystemTime};

    use nvs_config::Config;
    use nvs_config::cache::{artifact_key, content_hash, env_hash};

    use super::*;

    /// A private directory for one test, removed first so a crashed run does not poison the next.
    ///
    /// **Two levels below the temp dir, not one**, and that is what makes these tests runnable at
    /// all: ADR 0042 § 5's check reads the directory *and its parent*, and a Unix `/tmp` is mode
    /// `1777`, so a cache placed directly in it is refused before any test's own subject is
    /// reached. The per-process root this nests under is created by this process and carries the
    /// umask's ordinary bits, so it is the parent the check is meant to see.
    fn scratch(name: &str) -> PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("nvs-cache-{}", std::process::id()));
        let dir = root.join(format!("{unique}-{name}"));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).expect("a scratch directory under the temp dir is creatable");
        dir
    }

    /// The environment this test process compiles for, with no extensions configured.
    fn env() -> EnvHash {
        env_hash(&Config::default())
    }

    /// Every name directly inside `dir`, sorted.
    fn entries(dir: &Path) -> BTreeSet<String> {
        fs::read_dir(dir)
            .expect("the directory is readable")
            .map(|entry| {
                entry
                    .expect("the entry is readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    /// Make `dir` writable by every local account — the state § 5 refuses — in the platform's own
    /// spelling, because there is no portable one. Unix is a mode; Windows is an ACE for `Everyone`
    /// (`S-1-1-0`), added through `icacls` rather than through `windows-sys` so that a test helper
    /// does not cost this crate a dependency and an `unsafe` block. `icacls` is the same command
    /// `nvs_config::trust::REMEDY` tells an operator to undo such a grant with.
    fn open_to_the_world(dir: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(dir, fs::Permissions::from_mode(0o777))
                .expect("a world-writable mode");
        }
        #[cfg(windows)]
        {
            let granted = std::process::Command::new("icacls")
                .arg(dir)
                .arg("/grant")
                .arg("*S-1-1-0:(OI)(CI)(M)")
                .output()
                .expect("`icacls` ships with every supported Windows");
            assert!(
                granted.status.success(),
                "`Everyone` could not be granted write on {}: {}",
                dir.display(),
                String::from_utf8_lossy(&granted.stderr),
            );
        }
    }

    /// Backdate `path` by `seconds`, which is the only way a case can say which entry is oldest:
    /// § 6 evicts by `mtime` and seven files written in one loop differ by microseconds.
    fn age(path: &Path, seconds: u64) {
        File::options()
            .write(true)
            .open(path)
            .expect("the artifact is writable by the account that wrote it")
            .set_modified(SystemTime::now() - Duration::from_secs(seconds))
            .expect("the filesystem records a modification time");
    }

    /// What every file under the cache root occupies, which is the quantity § 6's cap is about.
    fn total_size(dir: &Path) -> u64 {
        let mut total = 0;
        for shard in fs::read_dir(dir).expect("the cache root is readable") {
            let shard = shard.expect("the entry is readable").path();
            if !shard.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&shard).expect("a shard is readable") {
                total += entry
                    .expect("the entry is readable")
                    .metadata()
                    .expect("an artifact reports its length")
                    .len();
            }
        }
        total
    }

    /// ADR 0042 § 6: eviction rides on a store and on nothing else, it is the roll that decides
    /// whether it happens at all, and it deletes oldest-first down to a floor under the cap. The
    /// half that is a performance property rather than a policy one is the middle assertion: a
    /// warm hit over a cache that is *already* over its cap walks nothing.
    #[test]
    fn eviction_is_piggybacked_and_off_the_request_path() {
        let dir = scratch("eviction");
        let cap = 4 * 1024;
        let always = Eviction {
            max_size: cap,
            probability: 1,
            divisor: 1,
        };
        let never = Eviction {
            probability: 0,
            ..always
        };

        // Six entries of a kilobyte each under a cache that never walks, oldest first. The
        // directory ends four times over its cap, which is what makes the next two assertions
        // measurements of the roll rather than of the writes.
        let cold = Cache::new(&dir, env())
            .expect("a scratch directory of this test's own")
            .with_eviction(never);
        let mut keys = Vec::new();
        for unit in 0..6u8 {
            let payload = vec![unit; 1024];
            let key = artifact_key(content_hash(&payload), cold.env());
            cold.store(key, &payload).expect("writable");
            age(&cold.path(key), u64::from(60 - unit));
            keys.push(key);
        }
        assert!(
            total_size(&dir) > cap,
            "a store that does not roll never evicts, whatever the directory holds",
        );

        // A hit is not a miss. § 6 hangs the roll off `store` alone, so this cannot evict however
        // the roll would have gone — and this cache's roll always fires.
        let hot = Cache::new(&dir, env())
            .expect("a scratch directory of this test's own")
            .with_eviction(always);
        let over = total_size(&dir);
        assert!(
            hot.load(keys[0]).is_some(),
            "the oldest entry is there to be hit"
        );
        assert_eq!(
            total_size(&dir),
            over,
            "a warm hit walks nothing, checks no size and deletes nothing",
        );

        // One miss later: the roll fires, the walk finds the cache over its cap, and the oldest
        // entries go — to the floor rather than to the cap, so the next store does not find work
        // again immediately.
        let payload = vec![0xEE; 1024];
        let fresh = artifact_key(content_hash(&payload), hot.env());
        hot.store(fresh, &payload).expect("writable");

        let left = total_size(&dir);
        assert!(
            left <= cap / 100 * GC_FLOOR_PERCENT,
            "the walk evicts past the cap to the hysteresis floor: {left} bytes left",
        );
        assert!(
            hot.path(fresh).exists() && !hot.path(keys[0]).exists(),
            "oldest-by-mtime goes first, and the entry this very store published stays",
        );
        assert!(
            hot.load(*keys.last().expect("six keys")).is_some(),
            "and it stops at the floor rather than clearing the cache",
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// ADR 0042 § 5, which is ADR 0103 § 6 applied to `[cache] dir`: a directory another local
    /// account can write is refused once, at construction, rather than entry by entry — that
    /// principal can compute a valid header and checksum over bytes of their own choosing, so
    /// there is nothing per entry that could catch them.
    #[test]
    fn a_world_writable_cache_directory_is_refused() {
        let root = scratch("trust");
        let dir = root.join("artifacts");
        fs::create_dir_all(&dir).expect("a cache directory to open up");

        Cache::new(&dir, env()).expect("a directory this account owns is inside the boundary");
        Cache::new(root.join("not-yet"), env())
            .expect("one the first store creates is checked at its ancestor");

        open_to_the_world(&dir);

        let why = Cache::new(&dir, env())
            .expect_err("any local account could drop a valid-looking artifact in here");
        assert!(
            matches!(why, Untrusted::Breach(_)),
            "the directory was examined and refused, not merely unreadable: {why:?}",
        );
        assert!(
            why.message().contains("artifacts"),
            "the refusal names the directory it is about: {}",
            why.message(),
        );
        assert!(
            Cache::new(dir.join("shard"), env()).is_err(),
            "a directory that does not exist yet takes the answer of the ancestor that would hold \
             it, which is the only place that promise can be kept",
        );

        drop(fs::remove_dir_all(&root));
    }

    /// ADR 0042 §§ 1-2: the address is the content, the layout is a two-character fan-out, and a
    /// published file is never rewritten in place.
    #[test]
    fn the_cache_is_a_fan_out_of_immutable_content_addressed_files() {
        let dir = scratch("fanout");
        let cache = Cache::new(&dir, env()).expect("a scratch directory of this test's own");
        assert_eq!(
            cache.dir(),
            dir,
            "the root is `[cache] dir` and nothing under it"
        );

        let first = b"; the first unit's payload".as_slice();
        let second = b"; a second, different unit".as_slice();
        let first_key = artifact_key(content_hash(first), cache.env());
        let second_key = artifact_key(content_hash(second), cache.env());

        assert_eq!(
            cache.store(first_key, first).expect("writable"),
            Stored::Written
        );
        assert_eq!(
            cache.store(second_key, second).expect("writable"),
            Stored::Written
        );

        // § 1: `<dir>/<key[0..2]>/<key[2..]>.nvsc`, and the shard is the key's own first byte.
        for (key, payload) in [(first_key, first), (second_key, second)] {
            let hex = key.to_string();
            let path = cache.path(key);
            assert_eq!(
                path,
                dir.join(&hex[..2]).join(format!("{}.nvsc", &hex[2..])),
                "the path is the key, fanned out at two characters"
            );
            assert_eq!(hex.len(), 64, "a BLAKE3 key is 64 hex characters");

            // § 2: header then payload, and the header's own fields.
            let bytes = fs::read(&path).expect("the published file is readable");
            assert_eq!(bytes.len(), HEADER_LEN + payload.len());
            assert_eq!(&bytes[..4], &MAGIC);
            assert_eq!(&bytes[HEADER_LEN..], payload);
            assert_eq!(
                Header::for_payload(cache.env(), payload)
                    .encode()
                    .as_slice(),
                &bytes[..HEADER_LEN],
                "the header is § 2's field order, little-endian"
            );
        }

        // Content-addressed: the same bytes under the same environment are the same file, and
        // different bytes are a different one.
        assert_ne!(cache.path(first_key), cache.path(second_key));
        assert_eq!(artifact_key(content_hash(first), cache.env()), first_key);

        // Immutable: a second store of the same key publishes nothing and leaves the bytes alone.
        let before = fs::read(cache.path(first_key)).expect("readable");
        assert_eq!(
            cache.store(first_key, first).expect("writable"),
            Stored::AlreadyPresent
        );
        assert_eq!(
            fs::read(cache.path(first_key)).expect("readable"),
            before,
            "a published artifact is never rewritten"
        );

        // The fan-out is directories, one per distinct leading byte, and nothing else is at the
        // root — no index, no manifest.
        let shards = entries(&dir);
        assert!(shards.iter().all(|name| name.len() == 2), "{shards:?}");
        assert!(
            shards.contains(&first_key.to_string()[..2])
                && shards.contains(&second_key.to_string()[..2])
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// ADR 0042 § 4: concurrent writers of one key resolve by rename. Exactly one file exists
    /// afterwards, it holds the right bytes, and no lock file and no temp file were left behind.
    #[test]
    fn a_concurrent_write_resolves_by_rename_with_no_lock_file() {
        let dir = scratch("concurrent");
        let cache = Cache::new(&dir, env()).expect("a scratch directory of this test's own");
        let payload = b"; the unit eight threads all compiled at once".as_slice();
        let key = artifact_key(content_hash(payload), cache.env());

        let outcomes: Vec<Stored> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| cache.store(key, payload).expect("writable")))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("no writer panics"))
                .collect()
        });

        assert!(
            outcomes.contains(&Stored::Written),
            "one writer published it: {outcomes:?}"
        );

        // The claim: the shard holds the artifact and nothing else. A lock file would be a second
        // entry here, and so would any temp file a writer failed to discard.
        let shard = cache
            .path(key)
            .parent()
            .expect("the shard is under the root")
            .to_path_buf();
        let left = entries(&shard);
        assert_eq!(
            left,
            BTreeSet::from([format!("{}.nvsc", &key.to_string()[2..])]),
            "no lock file, no leftover temp file"
        );

        let bytes = fs::read(cache.path(key)).expect("the published file is readable");
        assert_eq!(&bytes[HEADER_LEN..], payload);
        assert_eq!(
            &bytes[..HEADER_LEN],
            Header::for_payload(cache.env(), payload)
                .encode()
                .as_slice()
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// ADR 0042 § 3: an artifact is checked over its whole length before [`Cache::load`] will hand
    /// back the handle a mapper would take, and the check is the byte-exact one.
    ///
    /// What is assertable today is the ordering and the coverage, which is what the ADR's claim
    /// reduces to: a [`Verified`] exists on exactly one path, so any input that fails any check
    /// produces no handle at all, and *every* byte position of the payload is covered — a flip in
    /// the first byte, the last byte or the middle is caught alike, which is what distinguishes a
    /// whole-payload hash from a prefix check that would pass a doctored tail. The `mprotect` half
    /// of § 3 does not exist yet and the module doc's *Known gaps* entry says what it waits on; a
    /// test asserting over a step this crate cannot take would assert nothing.
    #[test]
    fn an_artifact_is_verified_whole_before_any_page_is_executable() {
        let dir = scratch("verify");
        let cache = Cache::new(&dir, env()).expect("a scratch directory of this test's own");
        let payload = b"; a compiled unit's payload, long enough to have a middle".as_slice();
        let key = artifact_key(content_hash(payload), cache.env());

        assert!(
            cache.load(key).is_none(),
            "a cold cache is a miss and not an error"
        );
        cache.store(key, payload).expect("writable");

        let hit = cache.load(key).expect("a published artifact verifies");
        assert_eq!(hit.payload(), payload, "the payload is the bytes stored");
        assert_eq!(hit.header(), Header::for_payload(cache.env(), payload));
        drop(hit);

        // Whole, not prefixed: every position is inside the hash, including the last one.
        let path = cache.path(key);
        let good = fs::read(&path).expect("readable");
        for at in [
            HEADER_LEN,
            HEADER_LEN + payload.len() / 2,
            HEADER_LEN + payload.len() - 1,
        ] {
            let mut doctored = good.clone();
            doctored[at] ^= 0x01;
            fs::write(&path, &doctored).expect("writable");
            assert!(
                cache.load(key).is_none(),
                "a flipped byte at {at} is not a hit"
            );
            assert!(
                !path.exists(),
                "a corrupt entry is deleted, never left to be re-read"
            );
        }

        // A truncated payload is caught by `payload_len`, one step before the checksum.
        fs::create_dir_all(path.parent().expect("a shard")).expect("writable");
        fs::write(&path, &good[..good.len() - 1]).expect("writable");
        assert!(cache.load(key).is_none(), "a truncated artifact is a miss");
        assert!(!path.exists(), "and it is deleted too");

        // A header alone, with no payload behind it, is not a handle either.
        fs::write(&path, &good[..HEADER_LEN / 2]).expect("writable");
        assert!(cache.load(key).is_none(), "a partial header is a miss");

        drop(fs::remove_dir_all(&dir));
    }

    /// ADR 0042 § 3: a tampered artifact is rejected, silently, and the file goes with it — while a
    /// file that is merely *foreign* is left alone.
    ///
    /// The three foreign cases are the ones § 3 calls a cache miss rather than corruption: a wrong
    /// magic, a wrong `format_version` and a wrong `env_hash`. Deleting on those would let one build
    /// of the compiler evict another's entries out of a shared cache directory, which the module doc
    /// states as the reason the split is where it is.
    #[test]
    fn a_tampered_artifact_is_rejected() {
        let dir = scratch("tampered");
        let cache = Cache::new(&dir, env()).expect("a scratch directory of this test's own");
        let payload = b"; the unit an attacker would like to replace".as_slice();
        let key = artifact_key(content_hash(payload), cache.env());
        cache.store(key, payload).expect("writable");

        let path = cache.path(key);
        let good = fs::read(&path).expect("readable");

        // Tampered: the payload is not what the header says it is. Nothing is returned, nothing is
        // reported, and the entry is gone.
        let mut swapped = good.clone();
        let chosen = vec![b'x'; payload.len()];
        swapped[HEADER_LEN..].copy_from_slice(&chosen);
        assert_eq!(swapped.len(), good.len(), "the same length, other bytes");
        fs::write(&path, &swapped).expect("writable");
        assert!(cache.load(key).is_none(), "a tampered payload is rejected");
        assert!(!path.exists(), "and the entry is deleted");

        // Foreign, three ways: each is a miss, and each file survives.
        for (label, doctor) in [("magic", 0_usize), ("format_version", 4), ("env_hash", 6)] {
            let mut foreign = good.clone();
            foreign[doctor] ^= 0xff;
            fs::create_dir_all(path.parent().expect("a shard")).expect("writable");
            fs::write(&path, &foreign).expect("writable");
            assert!(cache.load(key).is_none(), "a wrong {label} is a miss");
            assert!(
                path.exists(),
                "a wrong {label} means `not mine`, not `broken` — the file stays"
            );
        }

        // And the real entry still verifies once it is back.
        fs::write(&path, &good).expect("writable");
        assert_eq!(
            cache.load(key).expect("the original verifies").payload(),
            payload
        );

        drop(fs::remove_dir_all(&dir));
    }
}
