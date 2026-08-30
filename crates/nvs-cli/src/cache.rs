//! [ADR 0042]'s on-disk artifact cache: one immutable, content-addressed file per compiled unit,
//! and the writer that publishes one by a single atomic rename.
//!
//! § 1's layout is `<cache_dir>/<key[0..2]>/<key[2..]>.nvsc`, where the key is
//! [`nvs_config::cache::artifact_key`]'s `BLAKE3(source_content ‖ env_hash)` spelled as 64 lowercase
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
//! # Known gaps
//!
//! **A payload is a relocatable object image, not a dump of the JIT's finished pages.** ADR 0042's
//! *Investigation* describes a warm hit as mapping "the bytes directly as the pages the JIT would
//! otherwise have produced", and that is not reachable from this compiler for two independent
//! reasons. First, `cranelift_jit::JITModule` has no serialization at all: it finalizes into memory
//! it owns and hands back a code pointer, and there is no API yielding the bytes plus the
//! relocations another process would need. Second, and the one that would survive such an API, a
//! byte-perfect page dump would be *wrong* in the next process — `nvs-codegen`'s `emit.rs` bakes
//! host addresses in as `iconst` immediates carrying no relocation record: a class descriptor's
//! address in `class_desc` and again in the `instanceof` lowering, and a statically resolved
//! target's code address through `method_address`. Those are valid only for the process that
//! allocated the descriptors and compiled the callee.
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
//! [ADR 0048](../../../docs/adr/0048-portable-single-file-executables.md) is not the other half of
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
}

impl Cache {
    /// The cache rooted at `dir` — `[cache] dir`, which
    /// [`nvs_config::tree::Cache`] holds — for artifacts compiled against `env`.
    ///
    /// This does not create, canonicalize or check the directory: § 5's ownership check is the
    /// caller's, done once at process start, and a directory that does not exist yet is created by
    /// the first [`store`](Self::store).
    #[must_use]
    pub(crate) fn new(dir: impl Into<PathBuf>, env: EnvHash) -> Self {
        Self {
            dir: dir.into(),
            env,
        }
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

    use nvs_config::Config;
    use nvs_config::cache::{artifact_key, env_hash};

    use super::*;

    /// A private directory for one test, removed first so a crashed run does not poison the next.
    fn scratch(name: &str) -> PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("nvs-cache-{}-{unique}-{name}", std::process::id()));
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

    /// ADR 0042 §§ 1-2: the address is the content, the layout is a two-character fan-out, and a
    /// published file is never rewritten in place.
    #[test]
    fn the_cache_is_a_fan_out_of_immutable_content_addressed_files() {
        let dir = scratch("fanout");
        let cache = Cache::new(&dir, env());
        assert_eq!(
            cache.dir(),
            dir,
            "the root is `[cache] dir` and nothing under it"
        );

        let first = b"; the first unit's payload".as_slice();
        let second = b"; a second, different unit".as_slice();
        let first_key = artifact_key(first, cache.env());
        let second_key = artifact_key(second, cache.env());

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
        assert_eq!(artifact_key(first, cache.env()), first_key);

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
        let cache = Cache::new(&dir, env());
        let payload = b"; the unit eight threads all compiled at once".as_slice();
        let key = artifact_key(payload, cache.env());

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
        let cache = Cache::new(&dir, env());
        let payload = b"; a compiled unit's payload, long enough to have a middle".as_slice();
        let key = artifact_key(payload, cache.env());

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
        let cache = Cache::new(&dir, env());
        let payload = b"; the unit an attacker would like to replace".as_slice();
        let key = artifact_key(payload, cache.env());
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
