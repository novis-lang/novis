//! The per-file store: machine code kept one file per source file under a small program list, and
//! placed again function by function (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`).
//!
//! Three payloads share the store's header, its verify and its eviction, and each opens with a
//! magic of its own so no one of them decodes as another:
//!
//! ```text
//! entry  "NVSE" | pieces: u32 | piece* | literals: u32 | literal*
//! list   "NVSL" | entries: u32 | entry key: [u8; 32]*
//! hint   "NVSH" | list key: [u8; 32]
//! piece   = name: str | key: [u8; 32] | align: u64 | bytes | relocs
//! literal = digest: [u8; 32] | align: u64 | bytes | relocs
//! relocs  = count: u32 | (offset: u32 | kind: u8 | addend: i64 | target)*
//! target  = 0 name: str (a function) | 1 digest: [u8; 32] (a literal) | 2 name: str (a symbol)
//! str, bytes = len: u32 | the bytes
//! ```
//!
//! Every integer is little-endian, for the header's reason: the target triple is in `env_hash`.
//!
//! **A source file's entry** holds the [`nvs_codegen::pieces::Piece`] of every function whose
//! first statement is in that file, in program order, and every literal those pieces reach. Its
//! key is [`entry_key`]: the keys it holds and the environment, so two programs that share a file
//! whose functions compile alike share the entry. **The program list** is each entry's key in
//! program order, under [`super::unit_key`] as the whole-program artifact was. **The hint** names
//! the last list written for one program, under [`hint_key`]. It is the one file the store
//! replaces, and it exists so a process with nothing in memory reuses functions after an edit. A
//! stale or missing hint costs reuse and never correctness: every piece reused is found under its
//! own key in an entry that passed the checksum.
//!
//! **A warm hit emits nothing.** The list under the unit key names every entry, and their pieces
//! are placed as stored. A miss reads the hint's list and its entries, gives their keys to
//! [`nvs_codegen::compile_pieces`] as the functions it need not compile, places every function,
//! and only then writes each entry whose key is new, the list and the hint. A placement that fails
//! writes nothing, so the writer never publishes code its own loader refuses.
//!
//! **The placer** maps one private region, copies every function and each literal into it once,
//! writes every relocation, and makes the region executable. A call to a runtime symbol goes
//! through a stub beside the code, so its distance never matters. An unresolved name or a field
//! too narrow for its value is a miss, never a truncated address.
//!
//! **Cost of a hit:** one read per source file and one for the list, the decode, and one copy of
//! the program's code into the mapping. **Cost on disk after an edit:** the entry of the file
//! that changed, a new list and the hint.
//!
//! **Not built: shared code between unit versions.** Every compile places the whole program's
//! code again: 29 MB at 440 files, and an estimated 2.9 GB, about 1–2 s, at 100 times that size.
//! Sharing unchanged code between versions needs calls between functions to go through a table
//! per version, which is one more load per call on the request path. It waits for a measured
//! program that needs it.

use std::collections::BTreeMap;

use memmap2::MmapOptions;
use nvs_codegen::pieces::{LiteralPiece, Piece, PieceReloc, RelocKind, Target};
use nvs_config::cache::{Digest, EnvHash, artifact_key, content_hash};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    Cache, HOST_ARCH, LANDING_LEN, Loaded, Provenance, Unloadable, stub_body, this_process,
    write_branch26, write_le,
};

/// The magic a source file's entry opens with.
const ENTRY_MAGIC: [u8; 4] = *b"NVSE";
/// The magic a program list opens with.
const LIST_MAGIC: [u8; 4] = *b"NVSL";
/// The magic a hint opens with.
const HINT_MAGIC: [u8; 4] = *b"NVSH";

/// What one call of [`super::unit_for_counted`] did: functions compiled and reused, and source-file
/// entries written to the store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Work {
    /// Functions Cranelift compiled in this call.
    pub(crate) compiled: usize,
    /// Functions placed from the store without a compile.
    pub(crate) reused: usize,
    /// Source-file entries this call published: one per file whose functions' keys moved. An
    /// entry the store already has is not written again, and neither is the list or the hint
    /// counted here.
    pub(crate) written: usize,
}

/// Each function's relocatable code for one program's unit in force, by its key: what a serving
/// process keeps between two compiles of the program under `[opcache] incremental`.
///
/// The next compile of the program reuses every piece whose key it emits again, before it reads
/// the disk at all, and replaces this with the pieces of the unit it built. A reused piece is
/// correct for the same reason a stored one is: its key covers everything its code reads.
///
/// **Memory:** one copy of the program's machine code, its relocations and one name per function,
/// per loaded program, and never per request or per edit. Freed when the program's next compile
/// replaces it, or when `incremental` is `false`.
#[derive(Debug, Default)]
pub(crate) struct Kept {
    pieces: FxHashMap<[u8; 32], Piece>,
}

impl Kept {
    /// The pieces `pieces` gives, by key.
    fn of(pieces: impl IntoIterator<Item = Piece>) -> Self {
        Self {
            pieces: pieces.into_iter().map(|piece| (piece.key, piece)).collect(),
        }
    }

    /// How many functions this holds the code of.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.pieces.len()
    }
}

/// Where a miss stores what it compiled: the cache, the program's unit key and its hint.
#[derive(Clone, Copy)]
pub(crate) struct Store<'a> {
    pub(crate) cache: &'a Cache,
    pub(crate) key: Digest,
    pub(crate) hint: Digest,
}

/// What a compile's key and its hint are taken over: the program's content digest, and the name
/// of the program that stays the same across an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Source {
    /// The whole program's content digest ([`super::program_digest`]).
    pub(crate) digest: Digest,
    /// The program's name: its kind, its entry file's name and folder.
    pub(crate) name: Digest,
}

impl From<Digest> for Source {
    /// A source with no name of its own: the digest names it, so an edit finds no hint.
    fn from(digest: Digest) -> Self {
        Self {
            digest,
            name: digest,
        }
    }
}

/// The name of the program `files` makes, for a unit of `kind`: its entry file's name and folder.
pub(crate) fn program_name(kind: &str, files: &[nvs_types::ProgramFile<'_>]) -> Digest {
    let mut bytes = b"nvs program\0".to_vec();
    bytes.extend_from_slice(kind.as_bytes());
    if let Some(entry) = files.first() {
        let folder = nvs_types::paths::base_folder(entry.src).unwrap_or_default();
        for part in [entry.src.name(), folder.as_str()] {
            bytes.extend_from_slice(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
            bytes.extend_from_slice(part.as_bytes());
        }
    }
    content_hash(&bytes)
}

/// The key one program's hint is stored under.
pub(crate) fn hint_key(name: Digest, env: EnvHash, level: nvs_codegen::OptLevel) -> Digest {
    let mut keyed = b"nvs latest\0".to_vec();
    keyed.extend_from_slice(name.as_bytes());
    keyed.extend_from_slice(level.name().as_bytes());
    artifact_key(content_hash(&keyed), env)
}

/// The key of the entry that holds the functions with `keys`, in that order.
pub(crate) fn entry_key<'a>(keys: impl IntoIterator<Item = &'a [u8; 32]>, env: EnvHash) -> Digest {
    let mut keyed = b"nvs file entry".to_vec();
    for key in keys {
        keyed.extend_from_slice(key);
    }
    artifact_key(content_hash(&keyed), env)
}

/// What a stored payload is, by its magic. `None` for any other payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A source file's entry.
    Entry,
    /// A program list.
    List,
    /// A hint.
    Hint,
}

impl Kind {
    /// The kind of `payload`, or `None` for a payload this module did not write.
    pub(crate) fn of(payload: &[u8]) -> Option<Self> {
        match payload.get(..4)? {
            magic if magic == ENTRY_MAGIC => Some(Self::Entry),
            magic if magic == LIST_MAGIC => Some(Self::List),
            magic if magic == HINT_MAGIC => Some(Self::Hint),
            _ => None,
        }
    }
}

/// A source file's entry, decoded.
#[derive(Debug, Default)]
pub(crate) struct FileEntry {
    /// The functions' pieces, in program order.
    pub(crate) pieces: Vec<Piece>,
    /// The literals they reach.
    pub(crate) literals: Vec<LiteralPiece>,
}

/// The bytes of a payload being written.
struct Writer(Vec<u8>);

impl Writer {
    fn u32(&mut self, value: usize) -> Option<()> {
        self.0
            .extend_from_slice(&u32::try_from(value).ok()?.to_le_bytes());
        Some(())
    }

    fn bytes(&mut self, bytes: &[u8]) -> Option<()> {
        self.u32(bytes.len())?;
        self.0.extend_from_slice(bytes);
        Some(())
    }

    fn relocs(&mut self, relocs: &[PieceReloc]) -> Option<()> {
        self.u32(relocs.len())?;
        for reloc in relocs {
            if reloc.kind == RelocKind::Other {
                return None;
            }
            self.0.extend_from_slice(&reloc.offset.to_le_bytes());
            self.0.push(reloc.kind.code());
            self.0.extend_from_slice(&reloc.addend.to_le_bytes());
            match &reloc.target {
                Target::Function(name) => {
                    self.0.push(0);
                    self.bytes(name.as_bytes())?;
                }
                Target::Literal(digest) => {
                    self.0.push(1);
                    self.0.extend_from_slice(digest);
                }
                Target::Symbol(name) => {
                    self.0.push(2);
                    self.bytes(name.as_bytes())?;
                }
            }
        }
        Some(())
    }
}

/// The bytes of a payload being read. Every read past the end is `None`.
struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        if len > self.bytes.len() {
            return None;
        }
        let (head, rest) = self.bytes.split_at(len);
        self.bytes = rest;
        Some(head)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn len(&mut self) -> Option<usize> {
        usize::try_from(self.u32()?).ok()
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn digest(&mut self) -> Option<[u8; 32]> {
        self.take(32)?.try_into().ok()
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let len = self.len()?;
        self.take(len)
    }

    fn string(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?.to_vec()).ok()
    }

    /// A count of items, each at least `least` bytes long, that the rest of the payload can hold:
    /// a forged count never reserves more memory than the file has bytes.
    fn count(&mut self, least: usize) -> Option<usize> {
        let count = self.len()?;
        (count.checked_mul(least)? <= self.bytes.len()).then_some(count)
    }

    fn relocs(&mut self) -> Option<Vec<PieceReloc>> {
        let count = self.count(4 + 1 + 8 + 1)?;
        let mut relocs = Vec::with_capacity(count);
        for _ in 0..count {
            let offset = self.u32()?;
            let kind = RelocKind::from_code(self.u8()?).filter(|kind| *kind != RelocKind::Other)?;
            let addend = self.i64()?;
            let target = match self.u8()? {
                0 => Target::Function(self.string()?),
                1 => Target::Literal(self.digest()?),
                2 => Target::Symbol(self.string()?),
                _ => return None,
            };
            relocs.push(PieceReloc {
                offset,
                kind,
                addend,
                target,
            });
        }
        Some(relocs)
    }

    fn end(&self) -> Option<()> {
        self.bytes.is_empty().then_some(())
    }
}

/// The entry payload for `pieces` and `literals`, or `None` when a piece has a relocation no
/// loader places ([`RelocKind::Other`]) or a length past `u32`. Such an entry is never stored.
pub(crate) fn encode_entry(pieces: &[&Piece], literals: &[&LiteralPiece]) -> Option<Vec<u8>> {
    let mut out = Writer(ENTRY_MAGIC.to_vec());
    out.u32(pieces.len())?;
    for piece in pieces {
        out.bytes(piece.name.as_bytes())?;
        out.0.extend_from_slice(&piece.key);
        out.0.extend_from_slice(&piece.align.to_le_bytes());
        out.bytes(&piece.bytes)?;
        out.relocs(&piece.relocs)?;
    }
    out.u32(literals.len())?;
    for literal in literals {
        out.0.extend_from_slice(&literal.digest);
        out.0.extend_from_slice(&literal.align.to_le_bytes());
        out.bytes(&literal.bytes)?;
        out.relocs(&literal.relocs)?;
    }
    Some(out.0)
}

/// The entry [`encode_entry`] wrote, or `None` for any other payload.
pub(crate) fn decode_entry(payload: &[u8]) -> Option<FileEntry> {
    let mut input = Reader { bytes: payload };
    (input.take(4)? == ENTRY_MAGIC).then_some(())?;
    let count = input.count(4 + 32 + 8 + 4 + 4)?;
    let mut pieces = Vec::with_capacity(count);
    for _ in 0..count {
        let name = input.string()?;
        let key = input.digest()?;
        let align = input.u64()?;
        let bytes = input.bytes()?.to_vec();
        let relocs = input.relocs()?;
        pieces.push(Piece {
            name,
            key,
            align,
            bytes,
            relocs,
        });
    }
    let count = input.count(32 + 8 + 4 + 4)?;
    let mut literals = Vec::with_capacity(count);
    for _ in 0..count {
        let digest = input.digest()?;
        let align = input.u64()?;
        let bytes = input.bytes()?.to_vec();
        let relocs = input.relocs()?;
        literals.push(LiteralPiece {
            digest,
            align,
            bytes,
            relocs,
        });
    }
    input.end()?;
    Some(FileEntry { pieces, literals })
}

/// The list payload naming `entries`, in order.
pub(crate) fn encode_list(entries: &[Digest]) -> Option<Vec<u8>> {
    let mut out = Writer(LIST_MAGIC.to_vec());
    out.u32(entries.len())?;
    for entry in entries {
        out.0.extend_from_slice(entry.as_bytes());
    }
    Some(out.0)
}

/// The entry keys [`encode_list`] wrote, or `None` for any other payload.
pub(crate) fn decode_list(payload: &[u8]) -> Option<Vec<Digest>> {
    let mut input = Reader { bytes: payload };
    (input.take(4)? == LIST_MAGIC).then_some(())?;
    let count = input.count(32)?;
    let entries = (0..count)
        .map(|_| input.digest().map(Digest::from_bytes))
        .collect::<Option<Vec<_>>>()?;
    input.end()?;
    Some(entries)
}

/// The hint payload naming the list under `list`.
pub(crate) fn encode_hint(list: Digest) -> Vec<u8> {
    let mut out = HINT_MAGIC.to_vec();
    out.extend_from_slice(list.as_bytes());
    out
}

/// The list key [`encode_hint`] wrote, or `None` for any other payload.
pub(crate) fn decode_hint(payload: &[u8]) -> Option<Digest> {
    let mut input = Reader { bytes: payload };
    (input.take(4)? == HINT_MAGIC).then_some(())?;
    let list = input.digest()?;
    input.end()?;
    Some(Digest::from_bytes(list))
}

/// Every entry the list under `key` names, each loaded and decoded, or `None` when the list or
/// one of its entries is missing or bad.
fn entries_of(cache: &Cache, key: Digest) -> Option<Vec<FileEntry>> {
    let list = decode_list(cache.load(key)?.payload())?;
    list.into_iter()
        .map(|entry| decode_entry(cache.load(entry)?.payload()))
        .collect()
}

/// Whether a relocation of `kind` against a runtime symbol goes through a stub: a call, whose
/// four-byte or 26-bit field cannot reach every address.
fn through_stub(reloc: &PieceReloc) -> bool {
    matches!(reloc.target, Target::Symbol(_))
        && matches!(reloc.kind, RelocKind::X86CallPcRel4 | RelocKind::Arm64Call)
}

/// Places `functions` (each with its program index and its name in this program) and `literals`
/// in one private mapping, writes every relocation against them and `resolve`, and makes the
/// mapping executable.
///
/// # Errors
///
/// Every [`Unloadable`] is a miss: a name nothing resolves, a value too wide for its field, or a
/// mapping the platform refuses.
pub(crate) fn place(
    functions: &[(usize, &str, &Piece)],
    literals: &[&LiteralPiece],
    resolve: &dyn Fn(&str) -> Option<*const u8>,
) -> Result<Loaded, Unloadable> {
    fn reserve(len: &mut usize, size: usize, align: u64) -> usize {
        let align = usize::try_from(align).unwrap_or(1).max(1);
        let start = len.next_multiple_of(align);
        *len = start + size;
        start
    }
    let mut len = 0;
    let mut by_name: FxHashMap<&str, usize> = FxHashMap::default();
    let mut starts = Vec::with_capacity(functions.len());
    for (_, name, piece) in functions {
        let start = reserve(&mut len, piece.bytes.len(), piece.align);
        by_name.insert(name, start);
        starts.push(start);
    }
    let mut by_digest: FxHashMap<[u8; 32], usize> = FxHashMap::default();
    let mut literal_starts = Vec::with_capacity(literals.len());
    for literal in literals {
        let start = reserve(&mut len, literal.bytes.len(), literal.align);
        by_digest.insert(literal.digest, start);
        literal_starts.push(start);
    }
    let mut stubs: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, piece) in functions {
        for reloc in piece.relocs.iter().filter(|reloc| through_stub(reloc)) {
            if let Target::Symbol(name) = &reloc.target
                && !stubs.contains_key(name.as_str())
            {
                let start = reserve(&mut len, LANDING_LEN, LANDING_LEN as u64);
                stubs.insert(name, start);
            }
        }
    }
    if len == 0 {
        return Err(Unloadable::Unreadable);
    }

    let mut pages = MmapOptions::new()
        .len(len)
        .map_anon()
        .map_err(|source| Unloadable::Mapping(source.to_string()))?;
    for ((_, _, piece), start) in functions.iter().zip(&starts) {
        pages[*start..*start + piece.bytes.len()].copy_from_slice(&piece.bytes);
    }
    for (literal, start) in literals.iter().zip(&literal_starts) {
        pages[*start..*start + literal.bytes.len()].copy_from_slice(&literal.bytes);
    }
    // `expose_provenance`, as `nvs-codegen` uses it when it publishes a descriptor: these bytes
    // are about to run, and what they reach through has to stay reachable.
    let base = pages.as_ptr().expose_provenance();
    for (name, start) in &stubs {
        let address = resolve(name).ok_or_else(|| Unloadable::Unresolved((*name).to_owned()))?;
        let body = stub_body(HOST_ARCH)?;
        pages[*start..*start + body.len()].copy_from_slice(body);
        let at = *start + body.len();
        let address =
            u64::try_from(address.expose_provenance()).map_err(|_| Unloadable::Unreadable)?;
        pages[at..at + 8].copy_from_slice(&address.to_le_bytes());
    }

    let target_of = |reloc: &PieceReloc| -> Result<usize, Unloadable> {
        match &reloc.target {
            Target::Function(name) => by_name
                .get(name.as_str())
                .map(|start| base + start)
                .ok_or_else(|| Unloadable::Unresolved(name.clone())),
            Target::Literal(digest) => by_digest
                .get(digest)
                .map(|start| base + start)
                .ok_or(Unloadable::Unreadable),
            Target::Symbol(name) if through_stub(reloc) => stubs
                .get(name.as_str())
                .map(|start| base + start)
                .ok_or(Unloadable::Unreadable),
            Target::Symbol(name) => resolve(name)
                .map(<*const u8>::expose_provenance)
                .ok_or_else(|| Unloadable::Unresolved(name.clone())),
        }
    };
    let relocated = functions
        .iter()
        .map(|(_, _, piece)| &piece.relocs)
        .zip(&starts)
        .chain(
            literals
                .iter()
                .map(|literal| &literal.relocs)
                .zip(&literal_starts),
        );
    for (relocs, start) in relocated {
        for reloc in relocs {
            let at = usize::try_from(reloc.offset)
                .ok()
                .and_then(|offset| start.checked_add(offset))
                .ok_or(Unloadable::Unreadable)?;
            let target = i64::try_from(target_of(reloc)?)
                .map_err(|_| Unloadable::Unreadable)?
                .checked_add(reloc.addend)
                .ok_or(Unloadable::Unreadable)?;
            let place = i64::try_from(base + at).map_err(|_| Unloadable::Unreadable)?;
            let width = match reloc.kind {
                RelocKind::Abs8 => 8,
                _ => 4,
            };
            let field = pages
                .get_mut(at..at + width)
                .ok_or(Unloadable::Unreadable)?;
            match reloc.kind {
                RelocKind::Abs8 | RelocKind::Abs4 => write_le(field, target)?,
                RelocKind::X86PcRel4 | RelocKind::X86CallPcRel4 => {
                    write_le(field, target - place)?;
                }
                RelocKind::Arm64Call => write_branch26(field, target - place)?,
                RelocKind::Other => {
                    return Err(Unloadable::Unrepresentable(
                        "a relocation kind no loader places".to_owned(),
                    ));
                }
            }
        }
    }

    let pages = nvs_codegen::make_executable(pages)
        .map_err(|source| Unloadable::Mapping(source.to_string()))?;
    let mut placed = BTreeMap::new();
    let mut entry = None;
    for ((index, name, _), start) in functions.iter().zip(&starts) {
        if *name == nvs_ir::lower::ENTRY_SCRIPT_LABEL {
            entry = Some(*start);
        }
        placed.insert(nvs_codegen::function_symbol(*index, name), *start);
    }
    entry.ok_or(Unloadable::NoEntry)?;
    Ok(Loaded {
        pages,
        functions: placed,
    })
}

/// The program's functions out of `pieces` by name, in program order, each with its index; `None`
/// when one is missing.
fn in_program_order<'a>(
    program: &'a nvs_ir::Program,
    pieces: &'a FxHashMap<&str, &'a Piece>,
) -> Option<Vec<(usize, &'a str, &'a Piece)>> {
    program
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            let piece = pieces.get(function.name.as_str())?;
            Some((index, function.name.as_str(), *piece))
        })
        .collect()
}

/// The read path for a program list: every entry it names, placed and bound into the unit, and
/// how many functions that placed. With `keep`, the pieces it placed come back too. `None` at the
/// first step that misses.
pub(crate) fn warm_list(
    program: &nvs_ir::Program,
    cache: &Cache,
    key: Digest,
    keep: bool,
) -> Option<(nvs_codegen::Unit, usize, Option<Kept>)> {
    let entries = entries_of(cache, key)?;
    let pieces: FxHashMap<&str, &Piece> = entries
        .iter()
        .flat_map(|entry| &entry.pieces)
        .map(|piece| (piece.name.as_str(), piece))
        .collect();
    let functions = in_program_order(program, &pieces)?;
    let mut seen = FxHashSet::default();
    let literals: Vec<&LiteralPiece> = entries
        .iter()
        .flat_map(|entry| &entry.literals)
        .filter(|literal| seen.insert(literal.digest))
        .collect();
    // Built from the IR the front end has just lowered, for the reason `warm_hit` always gave:
    // no payload carries a descriptor (`rule:packaging/a-warm-hit-skips-codegen-not-the-front-end`).
    let descriptors = nvs_codegen::Descriptors::of(program);
    let loaded = place(&functions, &literals, &this_process(&descriptors)).ok()?;
    let reused = functions.len();
    let unit = descriptors.into_unit(Box::new(loaded));
    let kept = keep.then(|| Kept::of(entries.into_iter().flat_map(|entry| entry.pieces)));
    Some((unit, reused, kept))
}

/// The miss path: reuse every function `kept` holds, or with nothing kept every function the
/// hint's entries hold, compile the rest, and place the whole program. With a `store`, then store
/// each new entry, the list under its key and the hint. With `keep`, the program's pieces come
/// back for the caller to hold until its next compile. `Err` when the pieces could not be
/// compiled or placed, which the caller answers with a whole-program compile; the count is the
/// functions this call compiled before that.
pub(crate) fn compile_and_store(
    program: &nvs_ir::Program,
    store_in: Option<Store<'_>>,
    kept: Option<&Kept>,
    keep: bool,
) -> Result<(nvs_codegen::Unit, Provenance, Work, Option<Kept>), usize> {
    // Kept code is read before the disk: with it, the hint and its entries are not opened.
    let held: Vec<FileEntry> = match (kept, store_in) {
        (None, Some(Store { cache, hint, .. })) => cache
            .load(hint)
            .and_then(|hint| decode_hint(hint.payload()))
            .and_then(|list| decode_list(cache.load(list)?.payload()))
            .map(|list| {
                list.into_iter()
                    .filter_map(|entry| decode_entry(cache.load(entry)?.payload()))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let index: FxHashMap<[u8; 32], &Piece> = match kept {
        Some(kept) => kept
            .pieces
            .iter()
            .map(|(key, piece)| (*key, piece))
            .collect(),
        None => held
            .iter()
            .flat_map(|entry| &entry.pieces)
            .map(|piece| (piece.key, piece))
            .collect(),
    };
    let compiled = nvs_codegen::compile_pieces(program, &|key| index.contains_key(key))
        .map_err(|_| 0_usize)?;
    let done = compiled.compiled.len();
    let fresh: FxHashMap<&str, &Piece> = compiled
        .compiled
        .iter()
        .map(|piece| (piece.name.as_str(), piece))
        .collect();
    if compiled.keys.len() != program.functions.len() {
        return Err(done);
    }
    // Each function of this program by its name here, from this compile or from the store. A
    // reused piece is renamed: its key is the same, and the name is the one this program uses.
    let mut owned: Vec<Piece> = Vec::with_capacity(compiled.keys.len());
    for (name, key) in &compiled.keys {
        let piece = match fresh.get(name.as_str()) {
            Some(piece) => (*piece).clone(),
            None => {
                let mut piece = (*index.get(key).ok_or(done)?).clone();
                piece.name.clone_from(name);
                piece
            }
        };
        owned.push(piece);
    }
    let functions: Vec<(usize, &str, &Piece)> = program
        .functions
        .iter()
        .zip(&owned)
        .enumerate()
        .map(|(index, (function, piece))| (index, function.name.as_str(), piece))
        .collect();
    let all: Vec<&Piece> = owned.iter().collect();
    let literals = compiled.literals_for(&all);
    let descriptors = nvs_codegen::Descriptors::of(program);
    let loaded = place(&functions, &literals, &this_process(&descriptors)).map_err(|_| done)?;

    // Only placed code is stored, so the store never holds an entry this loader refuses.
    let written = store_in.map_or(0, |Store { cache, key, hint }| {
        store(cache, program, &compiled, &all, key, hint)
    });
    let work = Work {
        compiled: done,
        reused: owned.len() - done,
        written,
    };
    let provenance = if done == 0 {
        Provenance::Loaded
    } else {
        Provenance::Compiled
    };
    let unit = descriptors.into_unit(Box::new(loaded));
    Ok((unit, provenance, work, keep.then(|| Kept::of(owned))))
}

/// Writes each source file's entry whose key is new, the list under `key`, and the hint, and
/// returns how many entries it published. A write that fails is a cache that stays cold, and
/// nothing else.
fn store(
    cache: &Cache,
    program: &nvs_ir::Program,
    compiled: &nvs_codegen::pieces::Pieces,
    pieces: &[&Piece],
    key: Digest,
    hint: Digest,
) -> usize {
    let mut files: Vec<Vec<&Piece>> = Vec::new();
    let mut by_file = FxHashMap::default();
    for (function, piece) in program.functions.iter().zip(pieces) {
        let file = function.stmt_spans.first().map(|span| span.file);
        let at = *by_file.entry(file).or_insert_with(|| {
            files.push(Vec::new());
            files.len() - 1
        });
        files[at].push(*piece);
    }
    let mut list = Vec::with_capacity(files.len());
    let mut written = 0;
    for file in &files {
        let entry = entry_key(file.iter().map(|piece| &piece.key), cache.env());
        if !cache.path(entry).exists() {
            let Some(payload) = encode_entry(file, &compiled.literals_for(file)) else {
                return written;
            };
            if cache.publish(entry, &payload).is_err() {
                return written;
            }
            written += 1;
        }
        list.push(entry);
    }
    let Some(payload) = encode_list(&list) else {
        return written;
    };
    if cache.publish(key, &payload).is_err() {
        return written;
    }
    drop(cache.replace(hint, &encode_hint(key)));
    cache.maybe_gc();
    written
}

/// Every entry key a list among `payloads` names: what eviction keeps.
pub(crate) fn named_entries<'a>(payloads: impl IntoIterator<Item = &'a [u8]>) -> FxHashSet<Digest> {
    payloads
        .into_iter()
        .filter_map(decode_list)
        .flatten()
        .collect()
}
