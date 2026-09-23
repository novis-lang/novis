//! `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s on-disk artifact cache: one immutable, content-addressed file per compiled unit,
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
//! (`rule:config/the-extension-set-is-in-every-unit-key` owns the digest; this module only carries it).
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
//! **The mapping itself is overturned, not yet replaced.** `rule:packaging/the-artifact-cache-is-read-not-mapped`
//! decides the entry is read into a heap buffer — a file replaced under a mapping raises `SIGBUS`,
//! which nothing in-process contains — and that read path is not built; until it is, the hit cost
//! below is the mapped one.
//!
//! **Verification is a property of the type, not of a caller's discipline.** [`Cache::load`] hands
//! back a [`Verified`], which is constructed on exactly one code path — the far side of the
//! checksum comparison — and holds the mapping privately. A caller cannot reach the bytes without
//! going through it, so the step that would make those pages executable cannot be reached from an
//! unverified mapping even by mistake.
//!
//! **Which failures also delete the file.** A checksum mismatch does, which is § 3's own
//! instruction: the key is the content hash, so a file at that path whose contents hash differently
//! can only be corrupt or tampered and can never become a second valid version. A `payload_len` that
//! disagrees with the mapping is treated the same way, one step earlier — that field exists to catch
//! a truncation before the checksum does, the entry is corrupt on the same argument, and leaving it
//! costs a re-open on every future run. A wrong magic, `format_version` or `env_hash` is *not*
//! deleted: each of them means "not this process's file" rather than "broken", and deleting on them
//! would let one build of the compiler evict another's entries out of a shared cache directory.
//!
//! Cost of a hit: one `open`, one `mmap`, one BLAKE3 pass over the payload. The ADR's
//! *Investigation* weighs that against reading into a heap buffer and takes the mapping — the page
//! cache does the I/O once and the hash runs over it with no copy. The loader below adds one more
//! pass, and § 3 is where that trade is argued.
//!
//! # § 3's loader
//!
//! [`Verified::relocate`] is the half that makes a payload runnable. It maps a private, anonymous,
//! writable region of its own, copies each of the object's allocatable sections into it at the
//! alignment that section runs at, resolves every symbol the object left undefined against the
//! addresses its caller supplies, and only then calls `make_exec`. That call consumes the `MmapMut`
//! which was the one writable view of those pages, so § 3's one-way W^X transition is a move here
//! rather than a rule someone has to remember.
//!
//! **The pages that run are never the file's.** § 3 owns the argument — a relocatable object's
//! sections are laid out for a linker to place, not for a header of any length to leave aligned.
//! What it means in this module is that [`Cache::load`]'s mapping is read-only for its whole life
//! and exists only to be hashed.
//!
//! **A landing area is the answer to the >2 GB relocation, and to the GOT.** A symbol reached
//! through one gets [`LANDING_LEN`] bytes inside that same mapping: eight holding the symbol's full
//! address and, for a call, [`JUMP_THROUGH_NEXT_EIGHT`] in front of them. So every displacement
//! this loader writes names a target inside its own mapping, whatever distance the host image sits
//! at. [`landing_for`] decides who gets one, and the distinction it draws is not
//! defined-versus-undefined: a *call* needs a landing only when the symbol is undefined, because
//! everything the payload defines is already inside the mapping, but a **GOT-relative** field needs
//! one either way — the instruction loads through the address the field computes, so a payload's
//! own literal reached that way needs a slot as much as an imported helper does. On Windows the
//! area stays nearly empty — a COFF object reaches an import through a `.rdata$.refptr` cell of its
//! own, which is the same indirection one layer up — and it is ELF's `PltRelative`/`GotRelative`
//! pair the area exists for.
//!
//! **Both architectures, and one place they differ.** [`HOST_ARCH`] names x86-64 and aarch64, which
//! between them are every row of `docs/plan/design.md`'s platform table, and everything above is
//! the same for each: the same placement, the same landings, the same one-way protection change.
//! What differs is only how a resolved value reaches the bytes that hold it, and [`Form`] is that
//! difference in one type — x86-64 writes a flat little-endian field, while aarch64's fixed-width
//! instructions carry a call, a page and an offset inside a page as bit ranges within one word,
//! and reach a symbol through a pair of instructions that have to agree. [`form_of`] is where a
//! container format's own relocation number becomes one of those, and it is the only place either
//! format's numbering is read.
//!
//! Every failure is an [`Unloadable`], and every variant of that is a cache miss on § 3's terms: an
//! unresolvable name says the artifact was written against a runtime this process no longer
//! matches, which is "not mine" rather than "broken", so nothing is deleted and nothing is
//! reported.
//!
//! # § 5's directory check
//!
//! [`Cache::new`] refuses a cache directory another local account can write, once, before anything
//! is read out of it, through [`nvs_config::trust`] — `rule:config/ownership-is-the-trust-boundary`'s boundary, and the one
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
//! `rule:config/ownership-is-the-trust-boundary`'s own answer for an absent `optional` include. And the configured spelling is
//! kept rather than the canonical path the check hands back: there is no path *comparison* here to
//! protect, unlike the config tree's cycle test, and an absent directory has no canonical spelling
//! at all, so one rule covers both.
//!
//! A refusal is the caller's to report, and it is a refusal to start naming the path rather than a
//! silent fall back to compiling every time. § 3's "invisible to the script" discipline is about a
//! bad *entry*; a cache directory anyone can write is a breach of the boundary itself, and
//! `rule:config/ownership-is-the-trust-boundary` answers that the same way wherever it appears.
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
//! cache hovering at the boundary would walk on nearly every one. § 7 leaves the defaults to
//! the implementation and [`Eviction`] states them with their reasons.
//!
//! Every producer of a unit is wired. [`unit_for`] is the compile site's whole decision, and
//! every call site makes it: `main.rs`'s `run_run`, `runner.rs`'s suite compile for `nvs test`,
//! and `script.rs`'s [`Compiler`](crate::script::Compiler), which is the one a `spawn script`
//! isolate and the server both resolve through. Each of them resolves the configuration snapshot
//! *above* its compile, because both halves of the key are configuration — that ordering is the
//! wiring, and [`from_config`] is the only place it is read.
//!
//! `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` is not the other half of
//! this. Its § 2 decides a bundle carries *source*, not precompiled artifacts, and feeds into this
//! cache rather than out of it, so there is no already-produced payload for the cache to ship over.
//!

// Every compile site calls `unit_for` and `from_config`, and nothing outside this module calls
// anything else here. What is left over is the vocabulary § 3's own steps are stated in —
// `Cache::dir`, `Verified::header`, `Provenance::Loaded` — reachable only from this module's tests,
// which is where a format decision is asserted rather than used. Narrowing this to those items
// would be a list to maintain for no reader.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};
use nvs_config::cache::{Digest, EnvHash, artifact_key, content_hash, env_hash};
use nvs_config::trust::{self, Untrusted};
use object::read::{Object, ObjectSection, ObjectSymbol};
use object::{
    Architecture, BinaryFormat, RelocationEncoding, RelocationFlags, RelocationKind,
    RelocationTarget, SectionKind, SymbolIndex, SymbolKind, SymbolSection,
};
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
    /// `rule:config/the-extension-set-is-in-every-unit-key`'s environment digest, repeated from the path as defence in depth.
    ///
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

/// Why [`Cache::load`] is not returning an artifact — and, where the file is corrupt, why it is
/// taking that file with it.
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

    /// § 3's second half: place this payload's sections, resolve every symbol it left undefined
    /// against `resolve`, and only then make the pages executable.
    ///
    /// `resolve` is the whole of "this process's own addresses" — a runtime helper's, and a
    /// descriptor's for an `nvs_class_desc_*`, which [`nvs_codegen::class_desc_symbol`] spells at
    /// both ends. A name it answers [`None`] for is a miss on § 3's own terms and not a failure:
    /// it says the artifact was written against a runtime this process no longer matches.
    ///
    /// # Errors
    ///
    /// Every variant of [`Unloadable`] is a cache miss; the caller's next move is the compile it
    /// would have done anyway. They are distinguished only so a test can say which wall it hit.
    pub(crate) fn relocate(
        &self,
        resolve: &dyn Fn(&str) -> Option<*const u8>,
    ) -> Result<Loaded, Unloadable> {
        let payload = self.payload();
        let object = object::File::parse(payload).map_err(|_| Unloadable::Unreadable)?;
        if object.architecture() != HOST_ARCH {
            return Err(Unloadable::ForeignArchitecture);
        }

        let layout = Layout::of(&object)?;
        let mut pages = MmapOptions::new()
            .len(layout.len)
            .map_anon()
            .map_err(|source| Unloadable::Mapping(source.to_string()))?;
        for section in object.sections() {
            let Some(&start) = layout.sections.get(&section.index().0) else {
                continue;
            };
            if section.kind() == SectionKind::UninitializedData {
                // An anonymous mapping is already zero, which is the whole of this section's
                // contents; `data()` has nothing to hand over for it.
                continue;
            }
            let data = section.data().map_err(|_| Unloadable::Unreadable)?;
            let end = start
                .checked_add(data.len())
                .filter(|end| *end <= layout.len)
                .ok_or(Unloadable::Unreadable)?;
            pages[start..end].copy_from_slice(data);
        }

        // The address the relocations are computed against. `expose_provenance` rather than
        // `addr`, for the same reason `nvs-codegen` uses it when it publishes a descriptor: these
        // bytes are about to be executed, and what they reach through has to stay reachable.
        let base = pages.as_ptr().expose_provenance();
        layout.fill_landings(&object, &mut pages, base, resolve)?;
        for section in object.sections() {
            let Some(&start) = layout.sections.get(&section.index().0) else {
                continue;
            };
            for (offset, relocation) in section.relocations() {
                let at = usize::try_from(offset)
                    .ok()
                    .and_then(|offset| start.checked_add(offset))
                    .ok_or(Unloadable::Unreadable)?;
                layout.apply(&object, &mut pages, base, at, &relocation, resolve)?;
            }
        }

        // Every function this payload defines, by the name `nvs-codegen` gave it. The entry frame
        // is one of them, and the rest are what § 3's binding step asks for by name once the pages
        // are executable — one walk answers both questions, and neither is answerable later,
        // since `object` borrows the file mapping and [`Loaded`] does not hold it.
        let mut functions = BTreeMap::new();
        for symbol in object.symbols() {
            if symbol.kind() != SymbolKind::Text {
                continue;
            }
            let (Ok(name), Some(offset)) = (symbol.name(), layout.symbol_offset(&object, &symbol))
            else {
                continue;
            };
            functions.insert(linkage_name(layout.format, name).to_owned(), offset);
        }
        let entry = *functions
            .iter()
            .find(|(name, _)| {
                nvs_codegen::is_function_symbol(name, nvs_ir::lower::ENTRY_SCRIPT_LABEL)
            })
            .map(|(_, offset)| offset)
            .ok_or(Unloadable::NoEntry)?;

        // W^X, one way and once: nothing holds a writable view of these bytes after this line,
        // because [`nvs_codegen::make_executable`] consumes the `MmapMut` that was the only one.
        // That function rather than `make_exec` directly — publishing written bytes as code has a
        // platform-specific half that a protection change does not imply, and it has one home for
        // both of the ways Novis publishes code.
        let pages = nvs_codegen::make_executable(pages)
            .map_err(|source| Unloadable::Mapping(source.to_string()))?;
        Ok(Loaded {
            pages,
            entry,
            functions,
        })
    }
}

/// The architecture this loader knows how to relocate for, or [`Architecture::Unknown`] on a host
/// where it does not.
///
/// A payload's architecture is already in `env_hash` and therefore in its path, so this can only
/// disagree with the file for a hand-placed one — but the check is also what keeps the loader off
/// a host it has no answer for. `x86-64` and `aarch64` are the two, which between them are every
/// row of `docs/plan/design.md`'s platform table.
#[cfg(target_arch = "x86_64")]
const HOST_ARCH: Architecture = Architecture::X86_64;
#[cfg(target_arch = "aarch64")]
const HOST_ARCH: Architecture = Architecture::Aarch64;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const HOST_ARCH: Architecture = Architecture::Unknown;

/// Whether § 4's writer publishes anything on this host — [`HOST_ARCH`]'s question asked of the
/// *host* rather than of a file, because an artifact no loader will ever read is a second
/// Cranelift walk every cold run pays for nothing.
///
/// `aarch64` publishes through ELF and Mach-O, which is every host that platform table names it
/// on. A COFF `aarch64` host is not on it and the object backend has no aarch64 relocation spelled
/// for COFF at all, so the artifact there is one that could never be written rather than one this
/// loader declines to read: such a host compiles every run, exactly as an architecture with no
/// backend does.
#[cfg(any(target_arch = "x86_64", all(target_arch = "aarch64", not(windows))))]
const HOST_PUBLISHES: bool = true;
#[cfg(not(any(target_arch = "x86_64", all(target_arch = "aarch64", not(windows)))))]
const HOST_PUBLISHES: bool = false;

/// x86-64's `jmp qword ptr [rip + 0]`, whose eight-byte operand follows it — [`Landing::Stub`]'s
/// whole body there, and the reason no relocation this loader applies can be out of range.
const JUMP_THROUGH_NEXT_EIGHT: [u8; 6] = [0xFF, 0x25, 0x00, 0x00, 0x00, 0x00];

/// aarch64's spelling of that jump: `ldr x16, #8` then `br x16`, so the eight bytes the branch
/// reads again follow the instructions that read them. `x16` is the procedure-call standard's
/// first scratch register, reserved for exactly this — a linker's own veneer uses it, and no
/// callee may assume anything about it.
const LOAD_NEXT_EIGHT_AND_BRANCH: [u8; 8] = [0x50, 0x00, 0x00, 0x58, 0x00, 0x02, 0x1F, 0xD6];

/// Bytes reserved for one [`Landing`], and the alignment it gets. Sixteen for both kinds and both
/// architectures: a slot needs eight, x86-64's stub fourteen and aarch64's exactly sixteen, and one
/// size keeps the arithmetic in [`Layout`] to one branch.
const LANDING_LEN: usize = 16;

/// The instructions a [`Landing::Stub`] opens with here, with the eight bytes holding the address
/// they jump through immediately behind them — which is what both spellings above are chosen for,
/// so [`Layout::fill_landings`] writes the address at one offset it derives rather than at one it
/// has to know per architecture.
fn stub_body(arch: Architecture) -> Result<&'static [u8], Unloadable> {
    match arch {
        Architecture::X86_64 => Ok(&JUMP_THROUGH_NEXT_EIGHT),
        Architecture::Aarch64 => Ok(&LOAD_NEXT_EIGHT_AND_BRANCH),
        arch => Err(Unloadable::Unrepresentable(format!("a stub for {arch:?}"))),
    }
}

/// Why a verified artifact still could not be made runnable in this process.
///
/// **Every one of these is a cache miss**, never a diagnostic and never a throw — § 3 puts an
/// unresolvable symbol on exactly the footing of a wrong `env_hash`: it says the file was written
/// against a runtime this process no longer matches, which is "not this process's file" rather
/// than "broken", so nothing here deletes anything either. They are distinguished at all so a
/// test can name the wall it hit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Unloadable {
    /// The payload is not an object file, or names a section this host's reader cannot follow.
    Unreadable,
    /// The payload was written for another architecture — or for this one on a host whose loader
    /// is not written yet. See [`HOST_ARCH`].
    ForeignArchitecture,
    /// A symbol the payload leaves undefined has no address in this process.
    Unresolved(String),
    /// A relocation whose form this loader cannot represent. A `format_version` bump is what
    /// would stop an older file reaching this at all.
    Unrepresentable(String),
    /// The payload defines no entry frame, so there is nothing for a run to enter.
    NoEntry,
    /// The mapping could not be made, or could not be made executable.
    Mapping(String),
}

/// What a relocation needs placed beside the code, when the field cannot hold the target itself.
///
/// Two different reasons put something here, and only one of them is about distance. A
/// PC-relative *call* from mapped pages to a helper in this process's image can be more than 2 GB
/// away, so it goes through a [`Stub`](Self::Stub) — a linker's veneer, for a linker's reason, and
/// only an undefined symbol needs one because everything the payload defines is inside this same
/// mapping. A **GOT-relative** field is not about distance at all: the instruction the compiler
/// emitted *loads through* the address the field computes, so it needs a [`Slot`](Self::Slot)
/// holding the target whether or not the payload defines the symbol. Substituting the target's own
/// address there does not shorten the indirection, it reads the target's first eight bytes as a
/// pointer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Landing {
    /// Eight bytes holding the symbol's address, for a GOT-relative reference to it.
    Slot,
    /// A jump through such an address, for a PC-relative call to it.
    Stub,
}

/// Where everything the loader places goes, decided before a page is allocated.
#[derive(Debug)]
struct Layout {
    /// Where each allocatable section starts, by the payload's own section index. Keyed by the
    /// index's number rather than by [`object::SectionIndex`], which `object` does not order.
    sections: BTreeMap<usize, usize>,
    /// Where each landing starts, keyed by the symbol it stands for — [`SymbolIndex`]'s number,
    /// for the same reason — **and by its kind**, because one symbol can be reached both ways: a
    /// helper that is called and also has its address taken needs a stub and a slot, and a field
    /// handed the wrong one of those is off by an indirection rather than off by an address.
    landings: BTreeMap<(usize, Landing), usize>,
    /// Total bytes to map.
    len: usize,
    /// The payload's own container format, which is all [`linkage_name`] needs to know to undo the
    /// spelling that format gave a symbol.
    format: BinaryFormat,
    /// The payload's own architecture, which is what [`form_of`] reads a relocation's number
    /// against and what [`stub_body`] writes for. Taken from the file rather than from
    /// [`HOST_ARCH`] for [`linkage_name`]'s reason: a loader that reads the object it is holding
    /// cannot disagree with it.
    arch: Architecture,
}

impl Layout {
    /// Walks `object` twice — once for the sections, once for the relocations that need a landing
    /// — and hands back the placement both walks agreed on.
    fn of(object: &object::File<'_>) -> Result<Self, Unloadable> {
        let mut layout = Self {
            sections: BTreeMap::new(),
            landings: BTreeMap::new(),
            len: 0,
            format: object.format(),
            arch: object.architecture(),
        };
        for section in object.sections() {
            if !allocatable(section.kind()) {
                continue;
            }
            let size = usize::try_from(section.size()).map_err(|_| Unloadable::Unreadable)?;
            let align = usize::try_from(section.align()).unwrap_or(1);
            let start = layout.reserve(size, align);
            layout.sections.insert(section.index().0, start);
        }
        for section in object.sections() {
            if !layout.sections.contains_key(&section.index().0) {
                continue;
            }
            for (_, relocation) in section.relocations() {
                let RelocationTarget::Symbol(index) = relocation.target() else {
                    continue;
                };
                let symbol = object
                    .symbol_by_index(index)
                    .map_err(|_| Unloadable::Unreadable)?;
                let name = linkage_name(
                    layout.format,
                    symbol.name().map_err(|_| Unloadable::Unreadable)?,
                );
                let (reach, form) = form_of(layout.arch, &relocation)?;
                let Some(landing) =
                    landing_for(reach, form, symbol.kind(), !symbol.is_undefined(), name)?
                else {
                    continue;
                };
                if layout.landings.contains_key(&(index.0, landing)) {
                    continue;
                }
                let start = layout.reserve(LANDING_LEN, LANDING_LEN);
                layout.landings.insert((index.0, landing), start);
            }
        }
        if layout.len == 0 {
            return Err(Unloadable::Unreadable);
        }
        Ok(layout)
    }

    /// Takes `size` bytes at `align`, and says where they start.
    fn reserve(&mut self, size: usize, align: usize) -> usize {
        let start = self.len.next_multiple_of(align.max(1));
        self.len = start + size;
        start
    }

    /// Writes every landing's target address, which is the only place `resolve` is asked for a
    /// *function*'s address — after this, a call relocation names a stub and nothing else.
    ///
    /// A landing for a symbol the payload **defines** — which only a GOT-relative field asks for —
    /// holds an address inside this same mapping, so it is filled from `base` and never from
    /// `resolve`: this process has no name for a literal the compiling process minted.
    fn fill_landings(
        &self,
        object: &object::File<'_>,
        pages: &mut [u8],
        base: usize,
        resolve: &dyn Fn(&str) -> Option<*const u8>,
    ) -> Result<(), Unloadable> {
        for ((index, landing), start) in &self.landings {
            let symbol = object
                .symbol_by_index(SymbolIndex(*index))
                .map_err(|_| Unloadable::Unreadable)?;
            let address = if symbol.is_undefined() {
                let name = linkage_name(
                    self.format,
                    symbol.name().map_err(|_| Unloadable::Unreadable)?,
                );
                resolve(name)
                    .ok_or_else(|| Unloadable::Unresolved(name.to_owned()))?
                    .expose_provenance()
            } else {
                base + self
                    .symbol_offset(object, &symbol)
                    .ok_or(Unloadable::Unreadable)?
            };
            let at = match landing {
                Landing::Slot => *start,
                Landing::Stub => {
                    let body = stub_body(self.arch)?;
                    pages[*start..*start + body.len()].copy_from_slice(body);
                    *start + body.len()
                }
            };
            let address = u64::try_from(address).map_err(|_| Unloadable::Unreadable)?;
            pages[at..at + 8].copy_from_slice(&address.to_le_bytes());
        }
        Ok(())
    }

    /// Applies one relocation at `at`, an offset into `pages` whose address is `base + at`.
    fn apply(
        &self,
        object: &object::File<'_>,
        pages: &mut [u8],
        base: usize,
        at: usize,
        relocation: &object::Relocation,
        resolve: &dyn Fn(&str) -> Option<*const u8>,
    ) -> Result<(), Unloadable> {
        let (reach, form) = form_of(self.arch, relocation)?;
        let width = form.width();
        let end = at.checked_add(width).ok_or(Unloadable::Unreadable)?;
        let field = pages.get(at..end).ok_or(Unloadable::Unreadable)?;
        // A format with implicit addends — COFF is one — keeps the addend in the field itself,
        // and `object` reports the part it knows separately. The two add. An instruction field is
        // never one of those, whatever the format says: Mach-O carries a non-zero aarch64 addend
        // in an `ARM64_RELOC_ADDEND` record of its own and stops calling the addend implicit once
        // it has, and ELF spells every aarch64 addend in the `RELA` entry, so the bit range about
        // to be overwritten has nothing in it to read back.
        let mut addend = relocation.addend();
        if relocation.has_implicit_addend() && form.is_flat() {
            addend = addend
                .checked_add(read_le(field).ok_or_else(|| {
                    Unloadable::Unrepresentable(format!("an implicit addend {width} bytes wide"))
                })?)
                .ok_or(Unloadable::Unreadable)?;
        }

        let target = match relocation.target() {
            RelocationTarget::Symbol(index) => {
                let symbol = object
                    .symbol_by_index(index)
                    .map_err(|_| Unloadable::Unreadable)?;
                let defined = !symbol.is_undefined();
                let name = linkage_name(
                    self.format,
                    symbol.name().map_err(|_| Unloadable::Unreadable)?,
                );
                // The same question [`Layout::of`] asked, asked again rather than remembered: the
                // landing a *field* wants is decided by that field's own kind, so a symbol reached
                // both ways cannot be handed the other one's.
                match landing_for(reach, form, symbol.kind(), defined, name)? {
                    // A landing is the target now: the stub jumps to the symbol, the slot holds
                    // it, and either way what this field encodes is a displacement inside the
                    // loader's own mapping.
                    Some(landing) => {
                        base + *self
                            .landings
                            .get(&(index.0, landing))
                            .ok_or(Unloadable::Unreadable)?
                    }
                    None if defined => {
                        base + self
                            .symbol_offset(object, &symbol)
                            .ok_or(Unloadable::Unreadable)?
                    }
                    None => resolve(name)
                        .ok_or_else(|| Unloadable::Unresolved(name.to_owned()))?
                        .expose_provenance(),
                }
            }
            // A field naming a section names something already inside this mapping, so it needs no
            // landing. A *GOT* field naming one would need a slot nobody reserved — [`Layout::of`]
            // walks symbols — so it is a miss rather than a field pointed at the section itself.
            RelocationTarget::Section(index) if reach != Reach::Got => {
                base + *self.sections.get(&index.0).ok_or(Unloadable::Unreadable)?
            }
            target => {
                return Err(Unloadable::Unrepresentable(format!(
                    "a relocation against {target:?}"
                )));
            }
        };

        let target = i64::try_from(target)
            .map_err(|_| Unloadable::Unreadable)?
            .checked_add(addend)
            .ok_or(Unloadable::Unreadable)?;
        let place = i64::try_from(base + at).map_err(|_| Unloadable::Unreadable)?;
        let field = &mut pages[at..end];
        match form {
            Form::Absolute { .. } => write_le(field, target),
            Form::Displacement { .. } => write_le(field, target - place),
            Form::Branch26 => write_branch26(field, target - place),
            // `ADRP` reaches a page, and the two ends of the distance it covers are the page the
            // target is in and the page this instruction is in — never the addresses themselves,
            // so the low twelve bits of each are dropped before they are subtracted rather than
            // after.
            Form::Page21 => write_page21(field, page_of(target) - page_of(place)),
            Form::PageOffset12 => write_page_offset12(field, target & 0xFFF),
        }
    }

    /// Where a symbol defined by the payload landed, as an offset into the mapping.
    fn symbol_offset(
        &self,
        object: &object::File<'_>,
        symbol: &object::read::Symbol<'_, '_>,
    ) -> Option<usize> {
        let SymbolSection::Section(index) = symbol.section() else {
            return None;
        };
        let section = object.section_by_index(index).ok()?;
        let start = *self.sections.get(&index.0)?;
        let within = symbol.address().checked_sub(section.address())?;
        start.checked_add(usize::try_from(within).ok()?)
    }
}

/// The name `nvs-codegen` emitted a symbol under, out of the name this container format spells it
/// with.
///
/// Mach-O prefixes every global symbol with an underscore, so a payload in that format defines
/// `_nvs_fn_…` and leaves `_nvs_helper_throw` undefined where ELF and COFF carry the bare name.
/// Both of this loader's ends speak the bare form and neither can be taught otherwise: `resolve`
/// answers for [`nvs_runtime::symbols`]' own spellings and for the descriptor names
/// [`nvs_codegen::class_desc_symbol`] mints, and the [`Loaded::functions`] map is read back by
/// [`nvs_codegen::Placed::address_of`] under the label that crate derived. So the prefix comes off
/// here, at each of the few places a name is *read*, and nothing downstream has to know the format.
///
/// The question is asked of the **file**, not of the host. A payload is the host's format by
/// construction — `env_hash` covers the target triple — but a loader that reads the object it is
/// actually holding cannot disagree with it, and this is one comparison.
fn linkage_name(format: BinaryFormat, name: &str) -> &str {
    match format {
        BinaryFormat::MachO => name.strip_prefix('_').unwrap_or(name),
        _ => name,
    }
}

/// Whether a section is one the loader places, rather than one only a linker or a debugger reads.
fn allocatable(kind: SectionKind) -> bool {
    matches!(
        kind,
        SectionKind::Text
            | SectionKind::Data
            | SectionKind::ReadOnlyData
            | SectionKind::ReadOnlyDataWithRel
            | SectionKind::ReadOnlyString
            | SectionKind::UninitializedData
    )
}

/// What a relocation's field is computed against, once the container format's own spelling of it
/// has been read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reach {
    /// The symbol's own address.
    Symbol,
    /// A word holding the symbol's address, which the instruction the field belongs to loads
    /// *through* — the GOT, and a [`Landing::Slot`] whether or not the payload defines the symbol.
    Got,
    /// The symbol as the destination of a call, which an undefined symbol reaches through a
    /// [`Landing::Stub`] whatever the distance.
    Call,
}

/// Where a relocation's resolved value goes, and how what it lands in spells it.
///
/// **This is where the two architectures differ, and it is the only place they do.** x86-64 gives
/// every relocation a flat little-endian field of its own, so the value and the bytes are the same
/// number. aarch64 has fixed-width instructions and no flat field to spare, so a call's
/// displacement, a page and an offset inside a page are each a bit range within one four-byte
/// word, and one symbol reference is a pair of instructions that have to agree.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Form {
    /// A flat little-endian field `width` bytes wide, holding the target's own address.
    Absolute { width: usize },
    /// A flat little-endian field `width` bytes wide, holding the distance from the field to the
    /// target.
    Displacement { width: usize },
    /// That distance again, in `B`/`BL`'s 26-bit immediate, which counts instructions rather than
    /// bytes and so reaches ±128 MiB.
    Branch26,
    /// The distance from the 4 KiB page the instruction sits in to the page the target sits in, in
    /// `ADRP`'s 21-bit immediate — which counts pages, and which the encoding splits in two.
    Page21,
    /// The target's offset inside its own 4 KiB page, in the 12-bit immediate of the `ADD` or the
    /// load that completes an `ADRP` pair, scaled by the access width that instruction names.
    PageOffset12,
}

impl Form {
    /// Bytes of the mapping this form reads and writes.
    fn width(self) -> usize {
        match self {
            Self::Absolute { width } | Self::Displacement { width } => width,
            // One aarch64 instruction, whatever the bit range inside it.
            Self::Branch26 | Self::Page21 | Self::PageOffset12 => 4,
        }
    }

    /// Whether the field is a whole little-endian integer, which is the only shape an implicit
    /// addend can be read back out of.
    fn is_flat(self) -> bool {
        matches!(self, Self::Absolute { .. } | Self::Displacement { .. })
    }
}

/// What a relocation asks for, out of the number its container format recorded it as.
///
/// `object` maps the relocations that every architecture spells the same way onto a
/// [`RelocationKind`] and reports the rest as [`RelocationKind::Unknown`], with the format's own
/// number left intact in [`RelocationFlags`]. aarch64's instruction-field relocations are all in
/// that second group, so they are read from the flags here — and read **only when the payload is
/// aarch64**, because a Mach-O `r_type` is one byte and `ARM64_RELOC_PAGE21` is numerically
/// `X86_64_RELOC_SIGNED`.
///
/// # Errors
///
/// [`Unloadable::Unrepresentable`] for a relocation this loader has no encoding for, which is a
/// cache miss like every other variant.
fn form_of(
    arch: Architecture,
    relocation: &object::Relocation,
) -> Result<(Reach, Form), Unloadable> {
    if arch == Architecture::Aarch64 {
        let paired = match relocation.flags() {
            RelocationFlags::Elf { r_type } => match r_type {
                object::elf::R_AARCH64_ADR_PREL_PG_HI21 => Some((Reach::Symbol, Form::Page21)),
                object::elf::R_AARCH64_ADD_ABS_LO12_NC => Some((Reach::Symbol, Form::PageOffset12)),
                object::elf::R_AARCH64_ADR_GOT_PAGE => Some((Reach::Got, Form::Page21)),
                object::elf::R_AARCH64_LD64_GOT_LO12_NC => Some((Reach::Got, Form::PageOffset12)),
                _ => None,
            },
            RelocationFlags::MachO { r_type, .. } => match r_type {
                object::macho::ARM64_RELOC_PAGE21 => Some((Reach::Symbol, Form::Page21)),
                object::macho::ARM64_RELOC_PAGEOFF12 => Some((Reach::Symbol, Form::PageOffset12)),
                object::macho::ARM64_RELOC_GOT_LOAD_PAGE21 => Some((Reach::Got, Form::Page21)),
                object::macho::ARM64_RELOC_GOT_LOAD_PAGEOFF12 => {
                    Some((Reach::Got, Form::PageOffset12))
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(paired) = paired {
            return Ok(paired);
        }
    }

    let width = usize::from(relocation.size()) / 8;
    // A call is named three ways between the backend's writer and the two formats' readers —
    // `Relative` on the way out, `PltRelative` on the way back in — and the encoding is the one
    // thing all three agree on.
    let call = relocation.encoding() == RelocationEncoding::AArch64Call;
    match relocation.kind() {
        RelocationKind::Absolute => Ok((Reach::Symbol, Form::Absolute { width })),
        RelocationKind::GotRelative => Ok((Reach::Got, Form::Displacement { width })),
        RelocationKind::Relative | RelocationKind::PltRelative if call => {
            Ok((Reach::Call, Form::Branch26))
        }
        RelocationKind::PltRelative => Ok((Reach::Call, Form::Displacement { width })),
        RelocationKind::Relative => Ok((Reach::Symbol, Form::Displacement { width })),
        kind => Err(Unloadable::Unrepresentable(format!("{kind:?}"))),
    }
}

/// What a symbol needs placed for a relocation of this shape, if anything — `defined` says whether
/// the payload defines it or leaves it to `resolve`.
///
/// A [`Landing::Slot`] and a [`Landing::Stub`] are the two shapes a 64-bit address can be reached
/// through from a field too narrow to hold one. An absolute field is already wide enough to hold
/// the address itself, so it needs neither, and neither does any field naming a symbol this payload
/// defines — with **one** exception, which is the whole reason `defined` is not simply a filter on
/// the caller's side.
///
/// That exception is [`Reach::Got`]. Its slot is an *indirection the instruction performs*, not a
/// way of reaching something far away: x86-64's `movq sym@GOTPCREL(%rip), %r` and aarch64's
/// `adrp`/`ldr [x, #:got_lo12:]` pair both load eight bytes from wherever the field points.
/// Cranelift emits that form for every non-`colocated` global, which is every literal
/// `nvs-codegen` puts in a data section (see that crate's `clear_colocated`), so the payload's own
/// `nvs_bytes_*` come through here defined and still needing a slot. Pointing such a field at the
/// literal instead loads the literal's first eight bytes as an address — for an immortal
/// `StrHeader` that is `nvs_runtime::IMMORTAL_REFCOUNT`, and `echo` of it dereferences
/// `usize::MAX`.
///
/// **An `ADRP` pair naming a symbol the payload does not define is a miss, not a landing.** The
/// pair computes an address rather than loading one, so there is nowhere to put an indirection the
/// compiler did not emit — the same answer this function has always given a PC-relative field
/// naming undefined *data*. Cranelift reaches an imported symbol through the GOT, so what would
/// arrive here is a payload some other backend wrote.
fn landing_for(
    reach: Reach,
    form: Form,
    symbol: SymbolKind,
    defined: bool,
    name: &str,
) -> Result<Option<Landing>, Unloadable> {
    match (reach, form) {
        (Reach::Got, _) => Ok(Some(Landing::Slot)),
        (_, Form::Absolute { .. }) => Ok(None),
        // Everything the payload defines is inside this one mapping, so a displacement to it is
        // always representable and a veneer would only be a hop.
        _ if defined => Ok(None),
        (Reach::Call, _) => Ok(Some(Landing::Stub)),
        // A PC-relative field naming code is a call whatever the format called it, and a stub
        // answers it whatever the distance.
        (Reach::Symbol, Form::Displacement { .. }) if symbol == SymbolKind::Text => {
            Ok(Some(Landing::Stub))
        }
        (_, form) => Err(Unloadable::Unrepresentable(format!(
            "{form:?} against `{name}`"
        ))),
    }
}

/// A relocation field's own contents, as the signed addend they stand for.
fn read_le(field: &[u8]) -> Option<i64> {
    match field.len() {
        8 => Some(i64::from_le_bytes(field.try_into().ok()?)),
        4 => Some(i64::from(i32::from_le_bytes(field.try_into().ok()?))),
        2 => Some(i64::from(i16::from_le_bytes(field.try_into().ok()?))),
        1 => Some(i64::from(i8::from_le_bytes(field.try_into().ok()?))),
        _ => None,
    }
}

/// Writes a resolved relocation value into its field, or refuses because it does not fit.
///
/// A value too wide for its field is [`Unloadable::Unrepresentable`] and therefore a miss — never
/// a truncation, which would be an address that is merely wrong.
fn write_le(field: &mut [u8], value: i64) -> Result<(), Unloadable> {
    let too_wide = || Unloadable::Unrepresentable(format!("{value} in {} bytes", field.len()));
    match field.len() {
        8 => field.copy_from_slice(&value.to_le_bytes()),
        4 => field.copy_from_slice(&i32::try_from(value).map_err(|_| too_wide())?.to_le_bytes()),
        2 => field.copy_from_slice(&i16::try_from(value).map_err(|_| too_wide())?.to_le_bytes()),
        1 => field.copy_from_slice(&i8::try_from(value).map_err(|_| too_wide())?.to_le_bytes()),
        _ => return Err(too_wide()),
    }
    Ok(())
}

/// The 4 KiB page an address sits in — `ADRP`'s unit, and the one thing it and the `ADD` or load
/// paired with it divide between them: the page is what the first instruction reaches, and the
/// offset inside it is what the second adds back.
fn page_of(address: i64) -> i64 {
    address & !0xFFF
}

/// The four bytes of `field` as the one aarch64 instruction they are.
fn instruction(field: &[u8]) -> Result<u32, Unloadable> {
    let word: [u8; 4] = field.try_into().map_err(|_| Unloadable::Unreadable)?;
    Ok(u32::from_le_bytes(word))
}

/// Writes a resolved displacement into `B`/`BL`'s 26-bit immediate, which counts instructions.
///
/// The reach is ±128 MiB, and every call this loader writes names a target inside its own mapping
/// — the payload's own code, or a [`Landing::Stub`] beside it — so the range is a bound on how
/// large a unit may be rather than a constraint on where the host image sits.
fn write_branch26(field: &mut [u8], displacement: i64) -> Result<(), Unloadable> {
    let word = instruction(field)?;
    let out_of_reach = || Unloadable::Unrepresentable(format!("{displacement} in a 26-bit branch"));
    if displacement % 4 != 0 {
        return Err(out_of_reach());
    }
    let immediate = i32::try_from(displacement / 4).map_err(|_| out_of_reach())?;
    if !(-(1 << 25)..(1 << 25)).contains(&immediate) {
        return Err(out_of_reach());
    }
    let patched = (word & 0xFC00_0000) | (immediate.cast_unsigned() & 0x03FF_FFFF);
    field.copy_from_slice(&patched.to_le_bytes());
    Ok(())
}

/// Writes a resolved page distance into `ADRP`'s 21-bit immediate, which the encoding splits in
/// two: the low two bits sit at 30:29 and the other nineteen at 23:5.
///
/// The reach is ±4 GiB. Unlike a branch's, this one is never stretched by where the host image
/// sits: [`landing_for`] only ever lets an `ADRP` name a symbol the payload defines or a
/// [`Landing::Slot`], and both of those are inside this mapping.
fn write_page21(field: &mut [u8], distance: i64) -> Result<(), Unloadable> {
    let word = instruction(field)?;
    let out_of_reach = || Unloadable::Unrepresentable(format!("{distance} in a 21-bit page field"));
    let pages = i32::try_from(distance >> 12).map_err(|_| out_of_reach())?;
    if !(-(1 << 20)..(1 << 20)).contains(&pages) {
        return Err(out_of_reach());
    }
    let pages = pages.cast_unsigned();
    let patched = (word & 0x9F00_001F) | ((pages & 0x3) << 29) | (((pages >> 2) & 0x7_FFFF) << 5);
    field.copy_from_slice(&patched.to_le_bytes());
    Ok(())
}

/// Writes a resolved page offset into the 12-bit immediate at 21:10 of the `ADD` or the load that
/// completes an `ADRP` pair.
///
/// **The scale is the instruction's, not the relocation's.** A load's immediate counts units of
/// its own access width, so the same twelve bits mean bytes after an `ADD`, eight-byte words after
/// an `ldr x`, and sixteen after a 128-bit vector load. Mach-O has one `ARM64_RELOC_PAGEOFF12` for
/// all of them, so the instruction is read for its size field exactly as a linker reads it, and an
/// offset that is not a whole number of those units is a miss rather than a truncation.
fn write_page_offset12(field: &mut [u8], offset: i64) -> Result<(), Unloadable> {
    let word = instruction(field)?;
    // A load or store with an unsigned immediate: 29:27 are `111` and 25:24 are `01`. Its scale is
    // the size field at 31:30 — except for a 128-bit access, which spells `size` as zero and says
    // so by setting the SIMD bit at 26 together with the high `opc` bit at 23.
    let scale = if word & 0x3B00_0000 == 0x3900_0000 {
        match word >> 30 {
            0 if word & 0x0480_0000 == 0x0480_0000 => 4,
            size => size,
        }
    } else {
        0
    };
    let unit = 1_i64 << scale;
    if offset % unit != 0 {
        return Err(Unloadable::Unrepresentable(format!(
            "{offset} in a page offset scaled by {unit}"
        )));
    }
    let immediate = u32::try_from(offset >> scale).map_err(|_| Unloadable::Unreadable)? & 0xFFF;
    let patched = (word & !(0xFFF << 10)) | (immediate << 10);
    field.copy_from_slice(&patched.to_le_bytes());
    Ok(())
}

/// One artifact's payload, placed, relocated and executable — § 3's end state.
///
/// The pages are this loader's own mapping and never the file's: § 2's header leaves a payload's
/// sections at whatever offset the object writer chose, which is not the alignment they have to
/// run at, and a relocation against a far-away helper needs a [`Landing`] placed beside the code
/// that no file mapping has room for. What the file mapping is for is the checksum, and it stays
/// read-only for its whole life so no patch can reach the bytes that were hashed.
#[derive(Debug)]
pub(crate) struct Loaded {
    /// The mapping, `PROT_READ | PROT_EXEC` since [`Verified::relocate`] returned and never
    /// writable again — the `MmapMut` that was the only writable view is consumed by `make_exec`.
    pages: Mmap,
    /// Where the entry frame starts inside [`Self::pages`].
    entry: usize,
    /// Every function symbol the payload defines, to its offset inside [`Self::pages`] — what this
    /// type's [`nvs_codegen::Placed`] impl answers from, and the only thing that survives the
    /// object reader, which borrows a mapping this type does not keep.
    ///
    /// **Costs** one `String` and one `usize` per function in the unit, for the life of the loaded
    /// artifact: the same scale as the JIT's own `functions` table, and the price of § 3's binding
    /// step being answerable at all after `mprotect`.
    functions: BTreeMap<String, usize>,
}

impl Loaded {
    /// This artifact's entry frame, ready to run — `nvs_codegen::Unit::script`'s counterpart for a
    /// unit that was compiled by another process.
    #[expect(
        unsafe_code,
        reason = "an address in relocated, executable pages becomes the ABI's own function type; \
                  every claim that makes it callable — the pages are executable, the symbol is \
                  the entry frame, the frame has `rule:errors/propagation`'s one signature — was established by \
                  `relocate` before this type existed, and the returned `Entry` borrows the pages \
                  so the mapping outlives the pointer"
    )]
    #[must_use]
    pub(crate) fn script(&self) -> Entry<'_> {
        let address = self.pages.as_ptr().wrapping_add(self.entry);
        // SAFETY: `address` is inside `self.pages`, which `relocate` made executable and this type
        // keeps mapped; `entry` is the offset of the symbol `nvs_ir::lower::ENTRY_SCRIPT_LABEL`
        // names, and every frame `nvs-codegen` emits has `NvsFn`'s signature —
        // `rule:errors/propagation` is the one home of that convention and both ends of this
        // file agree on it.
        let function = unsafe { std::mem::transmute::<*const u8, nvs_runtime::NvsFn>(address) };
        Entry {
            function,
            pages: PhantomData,
        }
    }
}

/// § 3's last step, from this side: the addresses this loader placed, by the names
/// `nvs-codegen` emitted them under.
///
/// `nvs_codegen::Descriptors::bind` derives the symbol a method's code was emitted under and asks
/// this for it, and `Descriptors::into_unit` asks again for every function the unit defines. An
/// absent name is skipped there rather than being an error, which is why this answers [`None`] and
/// not an [`Unloadable`].
impl nvs_codegen::Placed for Loaded {
    fn address_of(&self, symbol: &str) -> Option<*const u8> {
        let offset = *self.functions.get(symbol)?;
        Some(self.pages.as_ptr().wrapping_add(offset))
    }
}

/// A loaded artifact's entry frame, borrowed from the pages it lives in.
///
/// The borrow is the point: a frame is a bare address, and the mapping it points into is dropped
/// by whoever owns the [`Loaded`]. Tying the two together here is what keeps a caller from
/// outliving the pages it is about to jump into.
#[derive(Debug)]
pub(crate) struct Entry<'a> {
    function: nvs_runtime::NvsFn,
    pages: PhantomData<&'a Loaded>,
}

impl Entry<'_> {
    /// Runs the frame on `ctx`.
    ///
    /// # Errors
    ///
    /// The status the run reported, with its message left on `ctx` — [`nvs_runtime::call`]'s own
    /// result, unchanged, exactly as `nvs_codegen::ScriptFn::call` hands it back.
    pub(crate) fn call(&self, ctx: &mut nvs_runtime::Ctx) -> Result<nvs_runtime::Value, i32> {
        nvs_runtime::call(self.function, ctx, &[])
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
/// examine. `rule:config/ownership-is-the-trust-boundary` answers the same question for an absent `optional` include and this
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
    /// The cache rooted at `dir` — `opcache.file_cache_dir`, which [`nvs_config::tree::Opcache`]
    /// holds and `docs/decisions/0175.md` makes the only spelling of — for artifacts compiled
    /// against `env`, once § 5's ownership check has passed on that directory.
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

    /// The same cache under a different § 6 policy — `[opcache]`'s own keys, once a caller reads
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

    /// Keys and heads every artifact read or written from now on with `env`. A reload that
    /// changes the `[[extension]]` array calls it, so the next compile misses the artifacts
    /// written under the old set.
    pub(crate) fn rekey(&mut self, env: EnvHash) {
        self.env = env;
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

    /// § 3's checks over a mapped artifact, cheapest first, hash last.
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

/// § 1's content hash for a whole **program**: every file the entry point's `require`/`autoload`
/// graph reached, in the order `nvs_hir::resolve_program` handed them back.
///
/// A unit is that graph and not one file (`front_end`'s own doc), so a key over the entry file
/// alone would answer a stale artifact for a program whose `require`d file was the one edited —
/// § 3's verification cannot catch that, because such a payload is this toolchain's and its
/// checksum is correct. Each file goes in as its name and its text, both behind their lengths, so
/// no two file sets can hash alike by running together and a file *renamed* moves the key: a
/// diagnostic's path and a throw's frame name it, which makes it observable.
pub(crate) fn program_digest(files: &[nvs_types::ProgramFile<'_>]) -> Digest {
    let mut bytes = Vec::new();
    for file in files {
        for part in [file.src.name(), file.src.text()] {
            bytes.extend_from_slice(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
            bytes.extend_from_slice(part.as_bytes());
        }
    }
    content_hash(&bytes)
}

/// Each file's own content hash, in the same order — what `rule:programs/no-runtime-autoload`'s program id combines, where
/// [`program_digest`] folds the same graph into the one digest a cache key needs.
///
/// The two read the same files and are deliberately not the same value. A cache key hashes each
/// file's *name* as well as its text, because a rename is observable in a diagnostic and in a
/// throw's frame; an id is a fact about the code that runs, and `nvs_config::cache::program_id`
/// states it as the unit content hashes alone. Both walk `nvs_hir::resolve_program`'s order.
pub(crate) fn unit_digests(files: &[nvs_types::ProgramFile<'_>]) -> Vec<Digest> {
    files
        .iter()
        .map(|file| content_hash(file.src.text().as_bytes()))
        .collect()
}

/// Which half of `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` this run's unit came out of.
///
/// Nothing a script can observe turns on this — § 3 makes a miss exactly as invisible as a cold
/// cache — so it exists for the tests that have to tell the two paths apart, and for a caller that
/// wants to say which one ran.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Provenance {
    /// A cold compile: this process walked Cranelift itself, and published what it built if it
    /// could.
    Compiled,
    /// § 3 end to end: a payload this process verified, placed, relocated, protected and bound.
    Loaded,
}

/// The unit for `program` — out of `cache` when it holds a usable artifact for `source`, and out
/// of Cranelift when it does not. `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` and `rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock`, at the one call site a run makes.
///
/// **Every way this can fail to use the cache is a cold compile and nothing else.** No cache at
/// all (the directory is absent, unwritable, or another account's — [`from_config`] answers
/// [`None`] for each), no artifact under the key, a header from another toolchain, a checksum that
/// does not match, a symbol this process cannot resolve: each falls through to the same compile
/// the run would have done anyway, and none of them reaches the script.
///
/// A cold run pays codegen **twice** — once through the JIT for the unit it is about to run, and
/// once through the object backend for the artifact it publishes. That is deliberate: the run in
/// hand is not made to wait on a file being written, and the alternative — running the payload
/// this process just wrote, through place-and-relocate — would put the loader on the path of every
/// cold run to save a Cranelift walk on none of them. What it costs is one extra walk on the run
/// that populates a key, and nothing at all on every run after it.
///
/// # Errors
///
/// Only [`nvs_codegen::CodegenError`], and only from the cold compile: the cache contributes no
/// error of its own, which is § 3's rule stated in the signature.
pub(crate) fn unit_for(
    program: &nvs_ir::Program,
    source: Digest,
    cache: Option<&Cache>,
) -> Result<(nvs_codegen::Unit, Provenance), nvs_codegen::CodegenError> {
    let Some(cache) = cache else {
        return Ok((nvs_codegen::compile(program)?, Provenance::Compiled));
    };
    let key = artifact_key(source, cache.env());
    if let Some(unit) = warm_hit(program, cache, key) {
        return Ok((unit, Provenance::Loaded));
    }
    let unit = nvs_codegen::compile(program)?;
    // § 4's writer, on the path that has just paid for a compile: a failure to publish is a cache
    // that stays cold, which is the one thing this whole module promises can never be worse. A
    // host [`HOST_PUBLISHES`] says no for writes nothing, because the object walk would be paid on
    // every cold run for a file no loader will ever read.
    if HOST_PUBLISHES && let Ok(payload) = nvs_codegen::compile_object(program) {
        drop(cache.store(key, &payload));
    }
    Ok((unit, Provenance::Compiled))
}

/// § 3 in one expression: the artifact under `key`, verified, placed, relocated, protected, bound
/// and assembled — or [`None`] at the first step that says this is not this process's file.
fn warm_hit(program: &nvs_ir::Program, cache: &Cache, key: Digest) -> Option<nvs_codegen::Unit> {
    let verified = cache.load(key)?;
    // § 2: the descriptors a warm hit resolves against are built here, out of the IR the front end
    // has just lowered, because a run that skipped codegen allocated none and no payload carries
    // one.
    let descriptors = nvs_codegen::Descriptors::of(program);
    let loaded = verified.relocate(&this_process(&descriptors)).ok()?;
    Some(descriptors.into_unit(Box::new(loaded)))
}

/// § 3's "this process's own addresses", in full.
///
/// Every runtime and `Core` helper by the address this process really calls it at — the same
/// tables `nvs-codegen`'s JIT resolves through — and every `nvs_class_desc_*` out of
/// `descriptors`, which holds one per class the program declared and one per `Core` class a
/// folded constant can name.
///
/// **Costs** one map of every exported helper name, built per warm hit and dropped with the
/// relocation. That is once per unit loaded, against a walk of the payload's relocations that is
/// itself proportional to the unit, so it is a constant factor on a path that has just skipped a
/// compile.
fn this_process(descriptors: &nvs_codegen::Descriptors) -> impl Fn(&str) -> Option<*const u8> + '_ {
    let table: BTreeMap<&'static str, *const u8> = nvs_runtime::symbols()
        .into_iter()
        .chain(nvs_stdlib::symbols())
        .collect();
    move |name| {
        table
            .get(name)
            .copied()
            .or_else(|| descriptors.resolve(name))
    }
}

/// § 7's directives, resolved into the cache a run consults — or [`None`] for a run that consults
/// none.
///
/// [`None`] is `opcache.file_cache = false`, a build that cannot identify itself, a host with no
/// cache root to default to, and a directory § 5 refuses. A run without a cache is a run that
/// compiles, which is the fallback every miss in this module already takes, so none of them stops
/// anything.
///
/// **One of them is reported: a `file_cache_dir` somebody wrote and § 5 refused.** The default
/// directory is nobody's decision and its refusal is nobody's to act on, but an operator who named
/// a directory asked for a cache there, and a server that silently compiles every unit on every
/// start is that answer withheld. It is one `warning:` line per process however many callers
/// resolve a cache, and never a refusal to start.
///
/// The build question is asked here rather than beside the default directory, because it is the one
/// case `rule:config/the-extension-set-is-in-every-unit-key`'s key does not separate and a
/// *configured* `file_cache_dir` is exactly as exposed to it as the default one.
pub(crate) fn from_config(config: &nvs_config::Config) -> Option<Cache> {
    placed(config).unwrap_or_else(|(written, why)| {
        static REPORTED: std::sync::Once = std::sync::Once::new();
        REPORTED.call_once(|| eprintln!("{}", refused_dir_warning(&written, &why)));
        None
    })
}

/// [`from_config`]'s answer with its one reported case left to the caller: the written
/// `file_cache_dir` and why § 5 refused it. A reload reports that case differently from a boot,
/// because a reload keeps the cache already in use (`crate::control`).
///
/// # Errors
///
/// A `file_cache_dir` somebody wrote, and § 5's reason for refusing it.
pub(crate) fn placed(config: &nvs_config::Config) -> Result<Option<Cache>, (String, Untrusted)> {
    let opcache = config.opcache.as_ref();
    if opcache.and_then(|opcache| opcache.file_cache) == Some(false) {
        return Ok(None);
    }
    if !nvs_config::cache::build_is_identified() {
        return Ok(None);
    }
    let written = opcache.and_then(|opcache| opcache.file_cache_dir.as_deref());
    let dir = match written {
        Some(written) => PathBuf::from(written),
        None => match default_dir() {
            Some(dir) => dir,
            None => return Ok(None),
        },
    };
    match Cache::new(dir, env_hash(config)) {
        Ok(cache) => Ok(Some(cache.with_eviction(eviction_of(opcache)))),
        Err(why) => match written {
            Some(written) => Err((written.to_owned(), why)),
            None => Ok(None),
        },
    }
}

/// Whether `now` places the artifact cache anywhere `was` does not: another `file_cache`,
/// `file_cache_dir` or § 6 policy. A reload that moves none of them keeps the cache it has.
pub(crate) fn placement_moved(was: &nvs_config::Config, now: &nvs_config::Config) -> bool {
    let keys = |config: &nvs_config::Config| {
        config.opcache.as_ref().map(|opcache| {
            (
                opcache.file_cache,
                opcache.file_cache_dir.clone(),
                opcache.file_cache_max_size.clone(),
                opcache.file_cache_gc_probability,
                opcache.file_cache_gc_divisor,
            )
        })
    };
    keys(was) != keys(now)
}

/// The line [`from_config`] prints for a written `file_cache_dir` § 5 refused.
///
/// It says what the run does instead, because "not used" alone reads as harmless and the cost is
/// a compile of every unit on every start. A breach carries the remedy; a directory that could
/// not be examined carries only what the reader said, since the remedy answers a DACL or a mode
/// that was read.
fn refused_dir_warning(written: &str, why: &Untrusted) -> String {
    let head = format!(
        "warning: `[opcache] file_cache_dir = \"{written}\"` is not used, so every program is \
         compiled again on each start: {}",
        why.message()
    );
    match why {
        Untrusted::Breach(_) => format!(
            "{head}\n  \
             note: the ownership check covers the cache directory and the directory that \
             contains it\n  \
             help: {}",
            trust::REMEDY
        ),
        Untrusted::Unreadable(_) => head,
    }
}

/// § 7's "a fixed system location", read as *this account's* rather than the host's, and one
/// directory per running binary.
///
/// `%LOCALAPPDATA%\novis\opcache` on Windows, `$XDG_CACHE_HOME`'s or `~/.cache`'s `novis/opcache`
/// elsewhere. A host-wide `/var/cache/novis` would be a directory some other account owns for every
/// account but one, and § 5 refuses exactly that — so the default that works everywhere is the one
/// inside the account already running the compile. An operator wanting one shared location writes
/// `opcache.file_cache_dir`, which is `System`-class for the reason § 7 gives.
///
/// **One directory, not one per compiler build.** `rule:config/the-extension-set-is-in-every-unit-key`'s
/// `env_hash` names the running executable, so a rebuilt compiler addresses none of the artifacts
/// its predecessor emitted however they are filed — and a directory per build would instead leave
/// each dead build's artifacts in a directory of their own, outside the one § 6 budget
/// [`eviction_of`] reads and with nothing that ever reclaims them.
fn default_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
    Some(root?.join("novis").join("opcache"))
}

/// § 6's policy as `[opcache]` writes it, with [`Eviction::default`] for every key it leaves out.
///
/// A value that spells nothing readable is left at the default rather than refused, which is the
/// treatment `nvs_config::cache::Revalidation::from_config` already gives `[opcache]`'s other
/// keys — the block's refusals are one decision and this is not the place to make a second.
fn eviction_of(opcache: Option<&nvs_config::tree::Opcache>) -> Eviction {
    let mut eviction = Eviction::default();
    let Some(opcache) = opcache else {
        return eviction;
    };
    if let Some(written) = opcache.file_cache_max_size.as_ref()
        && let Ok(nvs_config::value::Quantity::Bytes(bytes)) = nvs_config::value::Quantity::parse(
            "opcache.file_cache_max_size",
            nvs_config::value::Unit::Bytes,
            written,
        )
    {
        eviction.max_size = bytes;
    }
    if let Some(probability) = opcache.file_cache_gc_probability {
        eviction.probability = probability;
    }
    if let Some(divisor) = opcache.file_cache_gc_divisor {
        eviction.divisor = divisor;
    }
    eviction
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
    use crate::testing::open_to_the_world;

    /// A private directory for one test, removed first so a crashed run does not poison the next.
    ///
    /// **Two levels below the temp dir, not one**, and that is what makes these tests runnable at
    /// all: `rule:packaging/the-checksum-proves-integrity-and-ownership-proves-trust`'s check reads the directory *and its parent*, and a Unix `/tmp` is mode
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

    /// Backdate `path` by `seconds`, which is the only way a case can say which entry is oldest:
    /// § 6 evicts by `mtime` and files written in one loop differ by microseconds.
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

    /// `rule:packaging/eviction-rides-the-cold-miss-at-a-probability`: eviction rides on a store and on nothing else, it is the roll that decides
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

        // Entries of a kilobyte each under a cache that never walks, oldest first. The directory
        // ends well over its cap, which is what makes the assertions below measurements of the
        // roll rather than of the writes.
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

    /// `rule:packaging/the-checksum-proves-integrity-and-ownership-proves-trust`, which is `rule:config/ownership-is-the-trust-boundary` applied to `opcache.file_cache_dir`: a directory another local
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

        let warning = refused_dir_warning("artifacts", &why);
        assert!(
            warning.starts_with("warning:") && warning.contains("file_cache_dir"),
            "a written directory that is refused is said so, naming the key: {warning}",
        );
        assert!(
            warning.contains("compiled again") && warning.contains("help:"),
            "with what the run does instead and what to do about it: {warning}",
        );

        drop(fs::remove_dir_all(&root));
    }

    /// The typed tree `text` deserializes to, or the [`Diagnostic`] refusing it — the two answers
    /// the pair of cases below are about, both being claims about which key an operator writes.
    ///
    /// [`Diagnostic`]: nvs_diagnostics::Diagnostic
    fn parsed(text: &str) -> Result<Config, nvs_diagnostics::Diagnostic> {
        let mut sources = nvs_diagnostics::SourceMap::new();
        nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text).1
    }

    /// `docs/decisions/0175.md` § 2: `opcache.file_cache_dir` is where the artifact cache lives and
    /// [`from_config`] reads that key alone. The path arrives through the parser rather than a
    /// struct literal so the case covers the whole chain — the text an operator writes, the field
    /// the tree documents, and the root [`Cache::dir`] hands back.
    #[test]
    fn the_configured_cache_directory_is_read_from_the_key_the_tree_documents() {
        let root = scratch("configured");
        let dir = root.join("artifacts");
        // A TOML literal string, since a Windows path is mostly backslashes.
        let written = format!("[opcache]\nfile_cache_dir = '{}'\n", dir.display());

        let config = parsed(&written).expect("the key the tree documents is the key it parses");
        let cache = from_config(&config).expect("a directory this account owns is inside § 5");

        assert_eq!(
            cache.dir(),
            dir,
            "the written path is the cache root itself, not a per-build directory under it",
        );

        drop(fs::remove_dir_all(&root));
    }

    /// `docs/decisions/0175.md` § 4: the retired `[cache] dir` is not a field of the typed tree, so
    /// a file still carrying it is refused as a directive that does not exist rather than parsed
    /// and ignored. Silence is the defect the record is about — an operator who moved the cache and
    /// restarted was served by a process still writing where it always had.
    #[test]
    fn a_cache_directory_written_in_the_retired_spelling_is_reported_not_ignored() {
        let refusal = parsed("[cache]\ndir = '/srv/novis/artifacts'\n")
            .expect_err("`dir` is no longer a field of `[cache]`, which denies an unknown key");

        assert_eq!(
            refusal.code,
            Some(nvs_diagnostics::code::E_BAD_DIRECTIVE),
            "a key that does not exist is `E0601`: {}",
            refusal.message,
        );
        assert!(
            refusal.message.contains("dir"),
            "the refusal names the key that has to move: {}",
            refusal.message,
        );
        assert!(
            refusal.notes.iter().any(|note| note.contains("[cache]")),
            "and the block holding it, which is where an operator looks: {:?}",
            refusal.notes,
        );
        assert!(
            parsed("[opcache]\nfile_cache_dir = '/srv/novis/artifacts'\n").is_ok(),
            "the spelling that survives takes the same path in the same file",
        );
    }

    /// `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` and `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`: the address is the content, the layout is a two-character fan-out, and a
    /// published file is never rewritten in place.
    #[test]
    fn the_cache_is_a_fan_out_of_immutable_content_addressed_files() {
        let dir = scratch("fanout");
        let cache = Cache::new(&dir, env()).expect("a scratch directory of this test's own");
        assert_eq!(
            cache.dir(),
            dir,
            "the root is `opcache.file_cache_dir` and nothing under it"
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

    /// `rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock`: concurrent writers of one key resolve by rename. Exactly one file exists
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

    /// `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`: an artifact is checked over its whole length before [`Cache::load`] will hand
    /// back the handle a mapper would take, and the check is the byte-exact one.
    ///
    /// What is assertable today is the ordering and the coverage, which is what the ADR's claim
    /// reduces to: a [`Verified`] exists on exactly one path, so any input that fails any check
    /// produces no handle at all, and *every* byte position of the payload is covered — a flip in
    /// the first byte, the last byte or the middle is caught alike, which is what distinguishes a
    /// whole-payload hash from a prefix check that would pass a doctored tail. That the `mprotect`
    /// on the far side of it is reached only through a [`Verified`] is
    /// [`a_warm_hit_maps_private_writable_relocates_then_makes_the_pages_executable`]'s claim, and
    /// this case stops at the handle on purpose: its payload is bytes no loader could place.
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

    /// `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`: a tampered artifact is rejected, silently, and the file goes with it — while a
    /// file that is merely *foreign* is left alone.
    ///
    /// The foreign cases are the ones § 3 calls a cache miss rather than corruption: a wrong
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

        // Foreign in each way the header can be: each is a miss, and each file survives.
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

    /// `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`: what a writer publishes is the object `nvs_codegen::compile_object` wrote,
    /// and the name § 3's loader will resolve a descriptor against is derived on *this* side of
    /// the crate boundary, from the class's label alone.
    ///
    /// The two halves are one case because neither says much alone. A round trip over bytes is
    /// already pinned by [`an_artifact_is_verified_whole_before_any_page_is_executable`], and the
    /// spelling of the symbol is `nvs-codegen`'s own unit test. What is new is that they meet: the
    /// artifact this crate stored carries, *undefined*, exactly the name
    /// [`nvs_codegen::class_desc_symbol`] mints for a class the program declared — which is the
    /// whole of what a warm hit has to resolve before a page of it may run.
    #[test]
    fn a_published_artifact_is_the_object_the_compiler_wrote() {
        use object::{Object, ObjectSymbol};

        let dir = scratch("payload");
        let source = dir.join("program.nvs");
        fs::write(
            &source,
            "<?nvs\nclass Widget { public int $n = 1; }\nWidget $w = new Widget();\necho $w->n;\n",
        )
        .expect("a scratch directory of this test's own is writable");

        let checked = crate::front_end(&source).expect("a program with no error diagnostics");
        let program = nvs_ir::lower::lower_program(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &checked.program_files(),
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        let payload = nvs_codegen::compile_object(&program).expect("a host-format object");

        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        assert_eq!(
            cache.store(key, &payload).expect("writable"),
            Stored::Written
        );

        let hit = cache.load(key).expect("a published artifact verifies");
        assert_eq!(
            hit.payload(),
            payload.as_slice(),
            "a payload is published and read back byte for byte — nothing here transforms it"
        );

        let artifact = object::File::parse(hit.payload()).expect("a relocatable object");
        // Mach-O prefixes every linker-visible name with `_` and ELF and COFF do not, so what the
        // symbol table holds is the object format's spelling of the name
        // `nvs_codegen::class_desc_symbol` mints rather than that name itself. Deriving it from
        // the artifact's own format is what keeps this a claim about the *symbol* on every host.
        let minted = nvs_codegen::class_desc_symbol("Widget");
        let wanted = match artifact.format() {
            object::BinaryFormat::MachO => format!("_{minted}"),
            _ => minted,
        };
        let descriptor = artifact
            .symbols()
            .find(|symbol| symbol.name() == Ok(wanted.as_str()))
            .unwrap_or_else(|| {
                panic!("the artifact names no `{wanted}`, so nothing would relocate a descriptor")
            });
        assert!(
            descriptor.is_undefined(),
            "`{wanted}` is defined by the artifact, so it carries an address from the compiling \
             process instead of a relocation"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// Both halves of a warm hit for the program at `source`: § 2's payload, written through the
    /// object backend which is the only producer § 2 recognises, and the descriptors § 2 says the
    /// *loading* process builds for itself out of the same lowered IR.
    ///
    /// The front end runs **once** here, which is what a warm hit does too — § 2's decision is that
    /// a hit skips codegen and not the front end, so a fixture that ran it twice would be measuring
    /// a pipeline this cache does not have.
    fn unit_of(source: &Path) -> (Vec<u8>, nvs_codegen::Descriptors) {
        let checked = crate::front_end(source).expect("a program with no error diagnostics");
        let program = nvs_ir::lower::lower_program(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &checked.program_files(),
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        (
            nvs_codegen::compile_object(&program).expect("a host-format object"),
            nvs_codegen::Descriptors::of(&program),
        )
    }

    /// The environment § 4's digest would call another toolchain's.
    ///
    /// Its `[[extension]]` array is the only contribution to that digest a test in this process
    /// can move — the triple, the CPU feature bitset and the compiler build are all read off the
    /// running binary — and moving it is enough, because § 4 folds them all into one value.
    fn other_toolchain() -> EnvHash {
        let config: Config =
            toml::from_str("[[extension]]\npath = \"an-extension-this-one-lacks\"")
                .expect("a tree carrying one extension");
        env_hash(&config)
    }

    /// The whole front end over `source`, lowered, beside § 1's content hash of the program it
    /// turned out to be — exactly what a run holds at the moment it decides whether to consult
    /// the cache, and in the same order.
    fn lowered(source: &Path) -> (nvs_ir::Program, Digest) {
        let checked = crate::front_end(source).expect("a program with no error diagnostics");
        let files = checked.program_files();
        let digest = program_digest(&files);
        let program = nvs_ir::lower::lower_program(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &files,
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        (program, digest)
    }

    /// Runs a unit's entry frame on a context it armed, and hands back what it echoed.
    fn ran(unit: &nvs_codegen::Unit) -> String {
        let mut ctx = nvs_runtime::Ctx::buffered();
        unit.install_in(&mut ctx);
        unit.script()
            .expect("a compiled program has an entry frame")
            .call(&mut ctx)
            .expect("the entry frame ran to completion");
        String::from_utf8(
            ctx.take_buffered_output()
                .expect("a context this test made buffered"),
        )
        .expect("the script echoed UTF-8")
    }

    /// Runs a loaded artifact's entry frame and hands back what it echoed.
    fn output_of(loaded: &Loaded) -> String {
        let mut ctx = nvs_runtime::Ctx::buffered();
        loaded
            .script()
            .call(&mut ctx)
            .expect("the entry frame ran to completion");
        String::from_utf8(
            ctx.take_buffered_output()
                .expect("a context this test made buffered"),
        )
        .expect("the script echoed UTF-8")
    }

    /// The other side of every warm-hit gate below: on an architecture this loader has no
    /// relocation vocabulary for, a verified payload is a **miss**, and that is a decision rather
    /// than a shortfall.
    ///
    /// [`HOST_ARCH`] is where it is spelled, and it names the two architectures
    /// `docs/plan/design.md`'s platform table does. What a third one costs is one compile per run
    /// and nothing a script can observe, which is exactly what every other miss in this file
    /// costs, so the cases that assert a *warm hit* are the ones that carry the gate and this one
    /// carries the claim.
    #[test]
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    fn a_payload_is_a_miss_elsewhere() {
        let dir = scratch("foreign-arch");
        let source = dir.join("program.nvs");
        fs::write(&source, "<?nvs\necho 42;\n")
            .expect("a scratch directory of this test's own is writable");

        let (payload, descriptors) = unit_of(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        cache.store(key, &payload).expect("writable");

        // The artifact itself is sound — it verified — so nothing before `relocate` objects. The
        // refusal is the loader saying it has no answer for this host, which `Cache::load`'s
        // caller reads as "compile it", never as an error.
        let hit = cache.load(key).expect("a published artifact verifies");
        assert!(
            matches!(
                hit.relocate(&this_process(&descriptors)),
                Err(Unloadable::ForeignArchitecture)
            ),
            "a host this loader does not relocate for is a miss, not a failure"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// § 3 end to end: a payload this process verified becomes pages it can run.
    ///
    /// The assertion is the program's own output, and it is the strongest one available: reaching
    /// it means the sections were placed at the alignments they run at, every relocation resolved
    /// against *this* process's addresses — `nvs_echo_str` wrote into this test's own buffered
    /// context, which is the one in this address space — and the pages were executable by the
    /// time the entry frame was entered. A loader that mapped without relocating, or that
    /// relocated against the compiling process's addresses, cannot get here.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_warm_hit_maps_private_writable_relocates_then_makes_the_pages_executable() {
        let dir = scratch("warm-hit");
        let source = dir.join("program.nvs");
        fs::write(&source, "<?nvs\nint $n = 40;\necho $n + 2;\n")
            .expect("a scratch directory of this test's own is writable");

        let (payload, descriptors) = unit_of(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        assert_eq!(
            cache.store(key, &payload).expect("writable"),
            Stored::Written
        );

        let hit = cache.load(key).expect("a published artifact verifies");
        let loaded = hit
            .relocate(&this_process(&descriptors))
            .expect("every symbol the payload leaves undefined has an address here");
        assert_eq!(
            output_of(&loaded),
            "42",
            "the pages a warm hit relocated ran, and reached this process's own `nvs_echo_str`"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// § 2's descriptors, at the one place a test can tell a real one from a placeholder: a payload
    /// whose program allocates an instance, names its class and reads a field back.
    ///
    /// [`a_warm_hit_maps_private_writable_relocates_then_makes_the_pages_executable`] would pass
    /// against *any* address for an `nvs_class_desc_*`, because nothing in that fixture ever
    /// dereferences one. Everything this fixture prints comes out of the descriptor instead:
    /// `$box::class` reads the name off it, and allocating the instance reads the slot count that
    /// decides how much of it `$box->n` may touch. So reaching `Box=42` means the address the
    /// loader resolved that symbol to was a real `ClassDesc` for this very class — built by *this*
    /// process out of the IR its own front end just lowered, and never carried in the file. That is
    /// § 2's decision stated as an observation rather than as prose.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_class_in_a_payload_reaches_the_descriptor_this_process_built() {
        let dir = scratch("descriptors");
        let source = dir.join("program.nvs");
        fs::write(
            &source,
            "<?nvs\nclass Box {\n    public int $n = 0;\n}\n\nvar $box = new Box();\n\
             $box->n = 42;\necho $box::class, \"=\", $box->n;\n",
        )
        .expect("a scratch directory of this test's own is writable");

        let (payload, descriptors) = unit_of(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        cache.store(key, &payload).expect("writable");

        let loaded = cache
            .load(key)
            .expect("a published artifact verifies")
            .relocate(&this_process(&descriptors))
            .expect("every symbol the payload leaves undefined has an address here");
        assert_eq!(
            output_of(&loaded),
            "Box=42",
            "the relocated code reached a descriptor this process built from the same IR"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// § 3's last step, at the one place a test can tell a bound method table from an empty one:
    /// a call that *must* go through it, whose fallback runs a different body.
    ///
    /// `static::speak()` inside `Base::shout` is late static binding, so `nvs_ir` lowers it to an
    /// `InstKind::CallVirtual` against the descriptor of the class the call was made on, with
    /// `Base::speak` as the fallback a descriptor answering no `speak` gets. `Derived::shout()`
    /// therefore prints `derived` only if `Derived`'s descriptor holds a row bound to the address
    /// the *loader* placed `Derived::speak` at — and prints `base` if the table is empty, which is
    /// exactly the state every fixture above leaves it in. The assertion is `derived`, so nothing
    /// but § 3's build-place-relocate-protect-*bind* order can produce it, and the bind runs after
    /// `relocate` returned executable pages because a row is written into this process's
    /// descriptor and never back into the mapping.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_virtual_call_in_a_payload_reaches_the_method_row_the_loader_bound() {
        let dir = scratch("bind-methods");
        let source = dir.join("program.nvs");
        fs::write(
            &source,
            "<?nvs\nclass Base {\n    public static function speak(): string { return \"base\"; }\n\
             \n    public static function shout(): string { return static::speak(); }\n}\n\n\
             class Derived extends Base {\n    \
             public static function speak(): string { return \"derived\"; }\n}\n\n\
             echo Derived::shout();\n",
        )
        .expect("a scratch directory of this test's own is writable");

        let (payload, mut descriptors) = unit_of(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        cache.store(key, &payload).expect("writable");

        let loaded = cache
            .load(key)
            .expect("a published artifact verifies")
            .relocate(&this_process(&descriptors))
            .expect("every symbol the payload leaves undefined has an address here");
        descriptors.bind(&loaded);

        assert_eq!(
            output_of(&loaded),
            "derived",
            "the virtual call dispatched through a row bound to the placed payload's own code"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// § 3's end state, as the *type* the rest of the CLI takes: a warm hit hands back a
    /// `nvs_codegen::Unit`, and it answers everything a cold compile's does.
    ///
    /// [`a_virtual_call_in_a_payload_reaches_the_method_row_the_loader_bound`] pins the binding
    /// step against the loader's own `script`. What is new here is that the assembled unit carries
    /// the rest of it — a named function it can be asked for by label, an entry frame reached
    /// through `Unit::script` rather than through this module's `Loaded::script`, and a context
    /// armed by `Unit::install_in` — because that is what lets the compile site hold one variable
    /// whichever path produced it, which is the whole point of § 3 ending in a `Unit`.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_warm_hit_assembles_the_unit_a_cold_compile_would_have() {
        let dir = scratch("loaded-unit");
        let source = dir.join("program.nvs");
        fs::write(
            &source,
            "<?nvs\nclass Base {\n    public static function speak(): string { return \"base\"; }\n\
             \n    public static function shout(): string { return static::speak(); }\n}\n\n\
             class Derived extends Base {\n    \
             public static function speak(): string { return \"derived\"; }\n}\n\n\
             echo Derived::shout();\n",
        )
        .expect("a scratch directory of this test's own is writable");

        let (payload, descriptors) = unit_of(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        cache.store(key, &payload).expect("writable");

        let loaded = cache
            .load(key)
            .expect("a published artifact verifies")
            .relocate(&this_process(&descriptors))
            .expect("every symbol the payload leaves undefined has an address here");
        let unit = descriptors.into_unit(Box::new(loaded));

        assert!(
            unit.has_function("Derived::speak"),
            "a loaded unit knows every function the payload defines, by its Novis label"
        );
        let mut ctx = nvs_runtime::Ctx::buffered();
        unit.install_in(&mut ctx);
        unit.script()
            .expect("a loaded unit has the entry frame the payload defines")
            .call(&mut ctx)
            .expect("the entry frame ran to completion");
        assert_eq!(
            String::from_utf8(
                ctx.take_buffered_output()
                    .expect("a context this test made buffered")
            )
            .expect("the script echoed UTF-8"),
            "derived",
            "the unit's own entry frame ran, dispatching through the rows it bound"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// §§ 3 and 4 as a run performs them: the second run of one program reads what the first
    /// published, and compiles nothing.
    ///
    /// This is the decision [`unit_for`] exists to make, and [`Provenance`] is how a test sees it
    /// — nothing a script can observe distinguishes the two paths, which is § 3's own rule. Both
    /// runs are asserted to print the same thing as well, because "did not compile it" is only
    /// worth anything beside "and ran the same program".
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_second_run_of_the_same_program_does_not_compile_it() {
        let dir = scratch("second-run");
        let source = dir.join("program.nvs");
        fs::write(
            &source,
            "<?nvs\nclass Greeter {\n    \
             public static function greet(): string { return \"hi\"; }\n}\n\n\
             echo Greeter::greet(), 41 + 1;\n",
        )
        .expect("a scratch directory of this test's own is writable");
        let (program, digest) = lowered(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");

        let (cold, first) =
            unit_for(&program, digest, Some(&cache)).expect("a program that compiles");
        assert_eq!(
            first,
            Provenance::Compiled,
            "an empty cache holds nothing under this key"
        );
        assert_eq!(ran(&cold), "hi42");

        let (warm, second) =
            unit_for(&program, digest, Some(&cache)).expect("a program that compiles");
        assert_eq!(
            second,
            Provenance::Loaded,
            "the second run reads the artifact the first one published"
        );
        assert_eq!(ran(&warm), "hi42", "and runs the same program");

        drop(fs::remove_dir_all(&dir));
    }

    /// The goal's standing decision, pinned: `nvs run` keeps working with no cache to consult and
    /// with one it can never write.
    ///
    /// The first half is a run that was handed no [`Cache`] at all — what [`from_config`] answers
    /// for `file_cache = false`, for a host with no cache root, and for a directory § 5 refuses.
    /// The second is a cache whose directory can never be created, because its parent is a regular
    /// file: every store fails, so every run is a miss, and the only thing that reaches the caller
    /// is the unit it asked for.
    #[test]
    fn an_absent_or_unwritable_cache_directory_is_a_miss_and_the_run_succeeds() {
        let dir = scratch("no-cache");
        let source = dir.join("program.nvs");
        fs::write(&source, "<?nvs\necho 6 * 7;\n")
            .expect("a scratch directory of this test's own is writable");
        let (program, digest) = lowered(&source);

        let (unit, provenance) = unit_for(&program, digest, None).expect("a program that compiles");
        assert_eq!(provenance, Provenance::Compiled);
        assert_eq!(
            ran(&unit),
            "42",
            "a run with no cache is a run that compiles"
        );

        let blocked = dir.join("a-file");
        fs::write(&blocked, "not a directory").expect("the scratch directory is writable");
        let cache =
            Cache::new(blocked.join("cache"), env()).expect("a path under this test's own file");
        for _ in 0..2 {
            let (unit, provenance) =
                unit_for(&program, digest, Some(&cache)).expect("a program that compiles");
            assert_eq!(
                provenance,
                Provenance::Compiled,
                "a cache that cannot be written is a miss on every run, not on the first only"
            );
            assert_eq!(ran(&unit), "42", "and the run succeeds regardless");
        }
        assert!(
            !blocked.join("cache").exists(),
            "nothing was published, and nothing was reported either"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// § 1's key is the program's content, so editing the program moves it — the artifact under
    /// the old key is not consulted, and cannot be.
    ///
    /// [`program_digest`] is what makes that true, and this asserts the whole loop rather than the
    /// digest alone: the unedited program is a hit *first*, so the miss below is the edit's doing
    /// and not an empty cache's, and the edited program is a hit on the run after it.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn an_edited_source_file_is_a_miss_on_the_next_run() {
        let dir = scratch("edited");
        let source = dir.join("program.nvs");
        fs::write(&source, "<?nvs\necho 1 + 1;\n")
            .expect("a scratch directory of this test's own is writable");
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");

        let (program, digest) = lowered(&source);
        let (unit, provenance) =
            unit_for(&program, digest, Some(&cache)).expect("a program that compiles");
        assert_eq!(provenance, Provenance::Compiled);
        assert_eq!(ran(&unit), "2");
        assert_eq!(
            unit_for(&program, digest, Some(&cache))
                .expect("a program that compiles")
                .1,
            Provenance::Loaded,
            "the unedited program is a hit, which is what makes the miss below the edit's doing"
        );

        fs::write(&source, "<?nvs\necho 1 + 2;\n").expect("the same file, edited");
        let (edited, moved) = lowered(&source);
        assert_ne!(moved, digest, "an edited program hashes to another key");
        let (unit, provenance) =
            unit_for(&edited, moved, Some(&cache)).expect("a program that compiles");
        assert_eq!(
            provenance,
            Provenance::Compiled,
            "no artifact stands under the key the edit moved to"
        );
        assert_eq!(ran(&unit), "3", "and what runs is the edited program");
        assert_eq!(
            unit_for(&edited, moved, Some(&cache))
                .expect("a program that compiles")
                .1,
            Provenance::Loaded,
            "the recompile published under the new key, so the run after it is a hit"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`: a payload whose checksum fails is a miss, and the run that asked for it sees
    /// no error at all.
    ///
    /// [`a_tampered_artifact_is_rejected`] pins the same rule over bytes a test invented. What is
    /// new here is the far end of it: this payload is a real compiled unit, so a reader that
    /// checked the header and then relocated would have had something runnable to hand back, and
    /// only the ordering stops it. The recompile afterwards is the "not an error" half — a corrupt
    /// entry costs one compile and nothing a script can observe.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_payload_whose_checksum_fails_is_a_miss_and_not_an_error() {
        let dir = scratch("checksum");
        let source = dir.join("program.nvs");
        fs::write(&source, "<?nvs\necho 7;\n")
            .expect("a scratch directory of this test's own is writable");
        let (payload, descriptors) = unit_of(&source);
        let cache = Cache::new(dir.join("cache"), env()).expect("a directory of this test's own");
        let key = artifact_key(content_hash(&payload), cache.env());
        cache.store(key, &payload).expect("writable");

        // One flipped bit inside the object's own bytes — a header check cannot see it, and the
        // file is exactly as long and exactly as well-formed as it was.
        let path = cache.path(key);
        let mut doctored = fs::read(&path).expect("readable");
        doctored[HEADER_LEN + payload.len() / 2] ^= 0x01;
        fs::write(&path, &doctored).expect("writable");
        assert!(
            cache.load(key).is_none(),
            "a payload whose checksum fails never becomes a `Verified`"
        );
        assert!(
            !path.exists(),
            "and § 3 deletes it: at this key it can only be corrupt"
        );

        // The compile the caller would have done anyway, and its result runs.
        assert_eq!(
            cache.store(key, &payload).expect("writable"),
            Stored::Written
        );
        let loaded = cache
            .load(key)
            .expect("the republished artifact verifies")
            .relocate(&this_process(&descriptors))
            .expect("every symbol resolves");
        assert_eq!(
            output_of(&loaded),
            "7",
            "a miss cost one recompile, no more"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`: an artifact another toolchain wrote is a miss, and the environment is in the
    /// *address* rather than only in the header.
    ///
    /// Both halves, because § 2 keeps both on purpose. The address is what makes a foreign
    /// artifact cost one failed `open` instead of an open-then-reject; the header is the defence
    /// in depth for a file that reached this key's path some other way, and being foreign rather
    /// than broken it survives being read.
    ///
    /// **This is `rule:expressions/preparation-preserves-behaviour`'s cache bullet too**, which
    /// therefore owes no test of its own. A prepared pattern or format plan has no address here:
    /// the cache holds one file per compiled unit, and a prepared artifact is stored in that
    /// unit's payload — `crates/nvs-types/src/intrinsics.rs` gap 2 is the channel that carries one
    /// down to `nvs-ir`, and until it is built nothing is prepared at all. So the compiler build a
    /// prepared entry came from is in the key exactly as the payload's is, and a foreign one misses
    /// on both checks below rather than being read as a mismatch.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn a_payload_written_by_a_different_toolchain_is_a_miss() {
        let dir = scratch("toolchain");
        let source = dir.join("program.nvs");
        fs::write(&source, "<?nvs\necho 9;\n")
            .expect("a scratch directory of this test's own is writable");
        let (payload, descriptors) = unit_of(&source);

        let theirs =
            Cache::new(dir.join("cache"), other_toolchain()).expect("a directory of this test's");
        let ours = Cache::new(dir.join("cache"), env()).expect("the same directory, this process");
        let their_key = artifact_key(content_hash(&payload), theirs.env());
        let our_key = artifact_key(content_hash(&payload), ours.env());
        assert_ne!(
            their_key, our_key,
            "one payload, two environments, two addresses — § 2's whole point"
        );
        theirs.store(their_key, &payload).expect("writable");
        assert!(
            ours.load(our_key).is_none(),
            "a foreign artifact costs one failed open and is never read"
        );

        // Placed at this process's own address by hand, the header's repeated `env_hash` is what
        // catches it — and it stays, because `not mine` is not `broken`.
        let ours_path = ours.path(our_key);
        fs::create_dir_all(ours_path.parent().expect("a shard")).expect("writable");
        fs::copy(theirs.path(their_key), &ours_path).expect("writable");
        assert!(
            ours.load(our_key).is_none(),
            "§ 2's header field is the second, independent check"
        );
        assert!(
            ours_path.exists(),
            "a foreign artifact is never deleted: one build must not evict another's entries"
        );

        // And nothing here damaged the entry for the toolchain that wrote it.
        let loaded = theirs
            .load(their_key)
            .expect("the artifact still verifies for its own environment")
            .relocate(&this_process(&descriptors))
            .expect("every symbol resolves");
        assert_eq!(output_of(&loaded), "9");

        drop(fs::remove_dir_all(&dir));
    }

    /// `rule:packaging/a-warm-hit-skips-codegen-not-the-front-end`'s warm-versus-cold margin — the measurement that says this cache
    /// is worth having, and the one that would say it is not.
    ///
    /// **The front end is inside neither arm.** [`lowered`] runs parse, check and lower once, above
    /// both, which is § 2's own accounting: a warm hit pays for them in full and only codegen comes
    /// off the disk. So the margin below is codegen against place-relocate-bind and nothing else,
    /// and reading it as a whole-run speedup would be reading it wrong — a real run's parse and
    /// check sit on top of both numbers alike.
    ///
    /// The cold arm is a *whole* cold run and carries § 4's publish with it: the second Cranelift
    /// walk [`unit_for`]'s doc names, the temp file, the `fsync` and the rename. That is what a
    /// first run of a program actually costs, and pricing the cold arm at less than it would
    /// flatter the cache.
    ///
    /// The margin is read off the **best pair** of runs rather than off each arm's own fastest,
    /// because the two halves of a pair are measured microseconds apart and so meet the same
    /// machine: a minimum taken over the whole sweep is free to pair a cold arm measured while the
    /// box was idle with a warm arm that waited on a disk something else was writing. Neither arm
    /// can be measured faster than the work it did, so the best pair is still a lower bound on the
    /// ratio; what it buys is that one stalled sample decides nothing, which is what a guard that
    /// shares its machine with a release build needs. The named margin is written well under the
    /// ratio a development machine actually prints, and a slow *disk* moves it the safe way: the
    /// `fsync` is in the cold arm. What is asserted
    /// is "codegen dominates place and relocate", not a benchmark's own number; a change that
    /// brought the two within this factor of each other would mean the loader had grown expensive
    /// enough to reopen the decision, which is exactly what § *Revisiting* asks this test to detect.
    ///
    /// **Skipped in the debug profile**, for the reason `benches/abi-probe`'s guards are: a cost
    /// margin holds only on an idle machine, and `tools/verify.py` runs this binary beside every
    /// other test binary in the workspace. The driver runs it under `--release` once its sweep has
    /// finished and the box is idle — `tools/loop.py`'s release checks — and both margins hold
    /// there by a wider factor than here.
    #[test]
    // A warm hit is a loaded host's; see `a_payload_is_a_miss_elsewhere`.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[cfg_attr(
        debug_assertions,
        ignore = "a cost margin needs an idle machine; the driver's release slot is one"
    )]
    fn a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names() {
        use std::time::{Duration, Instant};

        /// The factor a warm start must clear. Under it, § *Revisiting* is owed a look.
        const MARGIN: u32 = 4;
        /// Sample pairs. The best of nine clears the margin with room to spare on an idle
        /// machine, and nine pairs stall together only on a machine nothing can measure.
        const RUNS: u32 = 9;

        let dir = scratch("warm-margin");
        let source = dir.join("program.nvs");
        // Enough functions that Cranelift's own walk is the bulk of a cold compile — one class of
        // four methods is a rounding error against process startup, and would measure that instead.
        let mut text = String::from("<?nvs\n");
        for class in 0..40 {
            text.push_str(&format!("class C{class} {{\n"));
            for method in 0..4 {
                text.push_str(&format!(
                    "    public static function m{method}(int $n): int \
                     {{ return $n * {method} + {class} + $n; }}\n"
                ));
            }
            text.push_str("}\n");
        }
        text.push_str("echo C39::m3(2);\n");
        fs::write(&source, &text).expect("a scratch directory of this test's own is writable");
        let (program, digest) = lowered(&source);

        let mut best: Option<(Duration, Duration)> = None;
        for run in 0..RUNS {
            // A cache of this sample's own, so every cold arm is a genuinely empty one and the
            // directory it publishes into is not one another sample already filled.
            let cache = Cache::new(dir.join(format!("cache-{run}")), env())
                .expect("a directory of this test's own");

            let at = Instant::now();
            let (built, provenance) =
                unit_for(&program, digest, Some(&cache)).expect("a program that compiles");
            let cold = at.elapsed();
            assert_eq!(
                provenance,
                Provenance::Compiled,
                "an empty cache holds nothing under this key"
            );

            let at = Instant::now();
            let (loaded, provenance) =
                unit_for(&program, digest, Some(&cache)).expect("a program that compiles");
            let warm = at.elapsed();
            assert_eq!(
                provenance,
                Provenance::Loaded,
                "the artifact the cold arm published is under the same key"
            );

            // A cross-product rather than a division, so a warm arm that lands on zero
            // nanoseconds is still a pair this loop can rank.
            let better = match best {
                Some((held_cold, held_warm)) => {
                    cold.as_nanos() * held_warm.as_nanos() > held_cold.as_nanos() * warm.as_nanos()
                }
                None => true,
            };
            if better {
                best = Some((cold, warm));
            }

            // Once, and only for the work's sake: a margin over two arms that ran different
            // programs — or no program — would measure nothing.
            if run == 0 {
                assert_eq!(ran(&built), "47");
                assert_eq!(ran(&loaded), "47", "and the warm arm is the same program");
            }
        }

        let (cold, warm) = best.expect("one pair per run, and there is at least one run");
        println!("cold {cold:?}, warm {warm:?} (the best of {RUNS} pairs)");
        assert!(
            warm * MARGIN <= cold,
            "§ Verification's margin: a warm start must be at least {MARGIN}x a cold one, and this \
             machine measured warm {warm:?} against cold {cold:?}"
        );

        drop(fs::remove_dir_all(&dir));
    }

    /// The one thing about [`linkage_name`] a non-Mach-O host can still assert: that it undoes
    /// exactly the prefix that format adds and leaves every other format's name alone.
    ///
    /// Asserted rather than left to the Mach-O host because the branch is unreachable here — this
    /// tree's own tests place COFF and ELF payloads — so nothing else in the suite would notice it
    /// stripping a character off a name that never had one.
    #[test]
    fn a_macho_name_loses_its_underscore_and_no_other_format_s_does() {
        assert_eq!(
            linkage_name(BinaryFormat::MachO, "_nvs_helper_throw"),
            "nvs_helper_throw"
        );
        assert_eq!(
            linkage_name(BinaryFormat::Elf, "nvs_helper_throw"),
            "nvs_helper_throw"
        );
        assert_eq!(
            linkage_name(BinaryFormat::Coff, "nvs_helper_throw"),
            "nvs_helper_throw"
        );
        // A bare name under Mach-O keeps every character: the prefix is stripped when it is there,
        // never one character unconditionally.
        assert_eq!(
            linkage_name(BinaryFormat::MachO, "nvs_helper_throw"),
            "nvs_helper_throw"
        );
        // And only the first one. A label `nvs-codegen` minted with its own leading underscore
        // would come back with it.
        assert_eq!(linkage_name(BinaryFormat::MachO, "__nvs_fn_0"), "_nvs_fn_0");
    }

    /// One instruction, written back the way an assembler would have written it.
    ///
    /// Every case below is stated as the encoding of the instruction it produces, because that is
    /// the claim: a bit range put in the wrong place still writes a plausible word, and only the
    /// whole word compared against what the mnemonic assembles to says which one it is.
    fn assembled(word: u32) -> String {
        format!("{word:#010x}")
    }

    /// [`write_branch26`] puts a call's displacement in `B`/`BL`'s immediate, in instructions, and
    /// leaves the opcode alone.
    ///
    /// Asserted here rather than left to an aarch64 host for [`linkage_name`]'s reason — this
    /// tree's own tests place x86-64 payloads, so nothing else in the suite would notice the field
    /// being written a bit out of place — and the range checks are the half no payload exercises
    /// at all: a unit large enough to need one would take minutes to compile.
    #[test]
    fn an_aarch64_branch_carries_its_displacement_in_instructions() {
        // `bl #0`, and the same instruction reaching forward a kilobyte: 0x400 bytes is 0x100
        // instructions, and nothing outside the low 26 bits moves.
        let mut field = 0x9400_0000_u32.to_le_bytes();
        write_branch26(&mut field, 0x400).expect("a kilobyte is well inside ±128 MiB");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x9400_0100),
            "`bl #1024`"
        );

        // Backwards, which is where a field written as an unsigned number goes wrong: -8 bytes is
        // -2 instructions, and the immediate is the low 26 bits of it.
        let mut field = 0x9400_0000_u32.to_le_bytes();
        write_branch26(&mut field, -8).expect("two instructions back is in reach");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x97FF_FFFE),
            "`bl #-8`"
        );

        // An unconditional `b` is the same field under a different opcode, and keeps it.
        let mut field = 0x1400_0000_u32.to_le_bytes();
        write_branch26(&mut field, 0x400).expect("in reach");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x1400_0100),
            "`b #1024`"
        );

        // Neither a displacement that is not a whole number of instructions nor one past ±128 MiB
        // is written narrowed: both are misses, which is what every refusal in this module is.
        let mut field = 0x9400_0000_u32.to_le_bytes();
        assert!(matches!(
            write_branch26(&mut field, 6),
            Err(Unloadable::Unrepresentable(_))
        ));
        assert!(matches!(
            write_branch26(&mut field, 1 << 27),
            Err(Unloadable::Unrepresentable(_))
        ));
        assert_eq!(
            u32::from_le_bytes(field),
            0x9400_0000,
            "a refused relocation leaves the instruction as it found it"
        );
    }

    /// [`write_page21`] splits `ADRP`'s immediate the way the encoding does — two bits at 30:29 and
    /// nineteen at 23:5 — and counts pages rather than bytes.
    #[test]
    fn an_aarch64_page_field_is_split_across_the_instruction() {
        // One page forward lands entirely in the low half of the immediate, which is the bit range
        // an encoder that wrote all twenty-one bits contiguously would miss.
        let mut field = 0x9000_0000_u32.to_le_bytes();
        write_page21(&mut field, 0x1000).expect("one page is in reach");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0xB000_0000),
            "`adrp x0, #4096`"
        );

        // Four pages carries into the high half and leaves the low one empty.
        let mut field = 0x9000_0000_u32.to_le_bytes();
        write_page21(&mut field, 0x4000).expect("four pages is in reach");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x9000_0020),
            "`adrp x0, #16384`"
        );

        // Backwards, sign-extended across both halves — and the destination register is not part
        // of the immediate.
        let mut field = 0x9000_0003_u32.to_le_bytes();
        write_page21(&mut field, -0x1000).expect("one page back is in reach");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0xF0FF_FFE3),
            "`adrp x3, #-4096`"
        );

        // ±4 GiB is the whole reach, and past it is a miss.
        let mut field = 0x9000_0000_u32.to_le_bytes();
        assert!(matches!(
            write_page21(&mut field, 1 << 33),
            Err(Unloadable::Unrepresentable(_))
        ));
    }

    /// [`write_page_offset12`] scales the offset by the access width the *instruction* names, which
    /// is the one thing `ARM64_RELOC_PAGEOFF12` does not say.
    #[test]
    fn an_aarch64_page_offset_is_scaled_by_the_instruction_it_lands_in() {
        // `add x0, x0, #0` counts bytes.
        let mut field = 0x9100_0000_u32.to_le_bytes();
        write_page_offset12(&mut field, 0x123).expect("a byte offset needs no scaling");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x9104_8C00),
            "`add x0, x0, #291`"
        );

        // `ldr x0, [x0]` counts eight-byte words, which is the form a GOT slot is read through.
        let mut field = 0xF940_0000_u32.to_le_bytes();
        write_page_offset12(&mut field, 0x40).expect("64 is eight words");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0xF940_2000),
            "`ldr x0, [x0, #64]`"
        );

        // `ldr q0, [x0]` counts sixteens, and says so by setting the vector bit together with the
        // high half of `opc` rather than through the size field, which reads as zero.
        let mut field = 0x3DC0_0000_u32.to_le_bytes();
        write_page_offset12(&mut field, 0x20).expect("32 is two quadwords");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x3DC0_0800),
            "`ldr q0, [x0, #32]`"
        );

        // `ldrsb x0, [x0]` sets that same high `opc` bit with the vector bit clear, and counts
        // bytes. Reading bit 23 alone would scale this one by sixteen.
        let mut field = 0x3980_0000_u32.to_le_bytes();
        write_page_offset12(&mut field, 0x7).expect("a signed byte load counts bytes");
        assert_eq!(
            assembled(u32::from_le_bytes(field)),
            assembled(0x3980_1C00),
            "`ldrsb x0, [x0, #7]`"
        );

        // An offset that is not a whole number of the instruction's own units is a miss, never an
        // address rounded down to one that fits.
        let mut field = 0xF940_0000_u32.to_le_bytes();
        assert!(matches!(
            write_page_offset12(&mut field, 0x44),
            Err(Unloadable::Unrepresentable(_))
        ));
    }

    /// A [`Landing::Stub`] is the same two facts on both architectures: a jump through the eight
    /// bytes immediately behind it, inside [`LANDING_LEN`].
    #[test]
    fn a_stub_jumps_through_the_eight_bytes_behind_it() {
        for arch in [Architecture::X86_64, Architecture::Aarch64] {
            let body = stub_body(arch).expect("both architectures this loader relocates for");
            assert!(
                body.len() + 8 <= LANDING_LEN,
                "{arch:?}'s stub and the address it reads fit in one landing"
            );
        }
        assert_eq!(
            stub_body(Architecture::X86_64).expect("x86-64"),
            &JUMP_THROUGH_NEXT_EIGHT
        );
        // `ldr x16, #8` then `br x16`: the literal offset is counted from the load, which sits at
        // the stub's own start, so the address lands exactly where the body ends.
        let body = stub_body(Architecture::Aarch64).expect("aarch64");
        assert_eq!(
            assembled(instruction(&body[..4]).expect("one instruction")),
            assembled(0x5800_0050),
            "`ldr x16, #8`"
        );
        assert_eq!(
            assembled(instruction(&body[4..]).expect("one instruction")),
            assembled(0xD61F_0200),
            "`br x16`"
        );
        assert_eq!(body.len(), 8, "and the address follows immediately");

        assert!(matches!(
            stub_body(Architecture::Riscv64),
            Err(Unloadable::Unrepresentable(_))
        ));
    }

    /// [`landing_for`]'s two answers that are not about distance: a GOT field gets a slot even for
    /// a symbol the payload defines, and an `ADRP` pair naming one it does not is a miss.
    #[test]
    fn a_got_field_gets_a_slot_and_an_adrp_pair_to_an_import_gets_nothing() {
        for form in [
            Form::Page21,
            Form::PageOffset12,
            Form::Displacement { width: 4 },
        ] {
            assert_eq!(
                landing_for(Reach::Got, form, SymbolKind::Data, true, "nvs_bytes_0"),
                Ok(Some(Landing::Slot)),
                "a {form:?} loaded through needs its slot however near the target is"
            );
        }
        // Defined, and reached directly: already inside this mapping.
        assert_eq!(
            landing_for(
                Reach::Symbol,
                Form::Page21,
                SymbolKind::Data,
                true,
                "nvs_bytes_0"
            ),
            Ok(None)
        );
        // Undefined, and reached directly: there is nowhere to put an indirection the compiler did
        // not emit, so the artifact is not this process's.
        assert!(matches!(
            landing_for(
                Reach::Symbol,
                Form::Page21,
                SymbolKind::Data,
                false,
                "nvs_helper_throw"
            ),
            Err(Unloadable::Unrepresentable(_))
        ));
        // A call to an import is the case a stub exists for.
        assert_eq!(
            landing_for(
                Reach::Call,
                Form::Branch26,
                SymbolKind::Text,
                false,
                "nvs_helper_throw"
            ),
            Ok(Some(Landing::Stub))
        );
    }
}
