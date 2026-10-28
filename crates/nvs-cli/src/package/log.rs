//! The registry's transparency log in C2SP tlog-tiles
//! (`rule:packaging/the-registry-keeps-an-append-only-log`): RFC 6962 record and node hashing,
//! the tile layout and its paths, appending to a log directory, and inclusion and consistency
//! proofs, produced and checked.
//!
//! The arithmetic is that of Go's `golang.org/x/mod/sumdb/tlog`, which C2SP's tiles are a fixed
//! height of. Every hash the tree ever has is numbered by a *stored hash index*: record `n`
//! commits its own record hash, then one more hash per trailing one bit of `n`, the complete
//! subtrees it closes, so the numbering only ever grows at the end. A tile at level `L` holds 256
//! consecutive hashes of Merkle level `8 * L`, and any hash between two tile levels is computed
//! from the tile below it. A [`HashReader`] reads stored hashes by index, from a log directory
//! ([`Log`]) or from whatever a client fetched, and every proof is built from those reads alone.
//!
//! The tree's size is not stored here. A checkpoint carries it, and [`append`] takes the size the
//! last checkpoint named. [`append`] never changes a file it already wrote: a tile or bundle
//! that is on disk with other bytes is an error, so an append from a stale size cannot rewrite
//! history. [`verify`] checks a whole log directory from scratch, as `nvs registry log verify`
//! does.
//!
//! **Memory:** an append holds the new records' hashes and the tiles it writes, a few kilobytes
//! per 256 records, and nothing that grows with the log. A proof is at most 64 hashes.

// The client's inclusion and consistency checks are the proofs' first callers outside the tests.
#![cfg_attr(not(test), expect(dead_code, reason = "no client checks a proof yet"))]

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

/// A SHA-256 hash in the tree.
pub(crate) type Hash = [u8; 32];

/// The bytes in one [`Hash`].
const HASH_SIZE: usize = 32;

/// C2SP fixes a tile's height at 8: a full tile holds 256 hashes.
const TILE_HEIGHT: u32 = 8;

/// The hashes in a full tile, and the records in a full entry bundle.
const TILE_WIDTH: u64 = 1 << TILE_HEIGHT;

/// An entry bundle prefixes each entry with a big-endian `u16` length.
pub(crate) const MAX_ENTRY_BYTES: usize = u16::MAX as usize;

/// Why a log operation failed.
#[derive(Debug)]
pub(crate) enum Error {
    /// A file of the log could not be read or written.
    Io { path: PathBuf, error: io::Error },
    /// A file of the log is not what its path says it is.
    Malformed { path: PathBuf, reason: String },
    /// An argument is outside the tree, such as a record past its size.
    OutOfRange(String),
    /// A proof does not lead to the hashes it claims.
    ProofFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Malformed { path, reason } => write!(f, "{}: {reason}", path.display()),
            Self::OutOfRange(what) => f.write_str(what),
            Self::ProofFailed => f.write_str("the proof does not match the tree"),
        }
    }
}

/// The hash of a record: SHA-256 of a zero byte and the record (RFC 6962 § 2.1).
pub(crate) fn record_hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0x00]);
    hasher.update(data);
    hasher.finalize().into()
}

/// The hash of an inner node: SHA-256 of a one byte and its two children (RFC 6962 § 2.1).
pub(crate) fn node_hash(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0x01]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// The root hash of a tree with no records: SHA-256 of nothing (RFC 6962 § 2.1).
pub(crate) fn empty_root() -> Hash {
    Sha256::digest([]).into()
}

/// The stored hash index of the hash at Merkle `level` and position `n` within that level.
pub(crate) fn stored_hash_index(level: u32, n: u64) -> u64 {
    // The hash at (level, n) is stored right after the last record under it, record
    // ((n + 1) << level) - 1, and is that record's `level`th extra hash.
    let mut n = n;
    for _ in 0..level {
        n = 2 * n + 1;
    }
    let mut index = 0;
    let mut rest = n;
    while rest > 0 {
        index += rest;
        rest >>= 1;
    }
    index + u64::from(level)
}

/// The Merkle level and position of stored hash `index`: the inverse of [`stored_hash_index`].
fn split_stored_hash_index(index: u64) -> (u32, u64) {
    // `stored_hash_index(0, n)` is below `2 * n`, so the record that stored `index` is at
    // `index / 2` or a few past it.
    let mut n = index / 2;
    let mut index_n = stored_hash_index(0, n);
    loop {
        // Record `n` stores one hash and one more per trailing one bit.
        let next = index_n + 1 + u64::from((n + 1).trailing_zeros());
        if next > index {
            break;
        }
        n += 1;
        index_n = next;
    }
    let level = u32::try_from(index - index_n).expect("a record stores at most 64 hashes");
    (level, n >> level)
}

/// How many hashes a tree of `size` records has stored.
pub(crate) fn stored_hash_count(size: u64) -> u64 {
    if size == 0 {
        return 0;
    }
    let mut count = stored_hash_index(0, size - 1) + 1;
    let mut last = size - 1;
    while last & 1 != 0 {
        count += 1;
        last >>= 1;
    }
    count
}

/// Reads stored hashes by their [`stored_hash_index`].
pub(crate) trait HashReader {
    /// The hash at each of `indexes`, in the same order.
    fn read_hashes(&mut self, indexes: &[u64]) -> Result<Vec<Hash>, Error>;
}

/// [`HashReader::read_hashes`], with a reader that returns the wrong count refused.
fn read_exact(reader: &mut dyn HashReader, indexes: &[u64]) -> Result<Vec<Hash>, Error> {
    let hashes = reader.read_hashes(indexes)?;
    if hashes.len() != indexes.len() {
        return Err(Error::OutOfRange(format!(
            "{} hashes were read where {} were asked for",
            hashes.len(),
            indexes.len()
        )));
    }
    Ok(hashes)
}

/// The hashes record `n` stores when it is appended with `data`: its record hash, then one node
/// per complete subtree it closes.
pub(crate) fn stored_hashes(
    n: u64,
    data: &[u8],
    reader: &mut dyn HashReader,
) -> Result<Vec<Hash>, Error> {
    let mut hash = record_hash(data);
    let closes = (n + 1).trailing_zeros();
    // The left sibling at each level, read lowest level last as `stored_hash_index` orders them.
    let indexes: Vec<u64> = (0..closes)
        .rev()
        .map(|level| stored_hash_index(level, (n >> level) - 1))
        .collect();
    let siblings = read_exact(reader, &indexes)?;
    let mut hashes = Vec::with_capacity(siblings.len() + 1);
    hashes.push(hash);
    for left in siblings.iter().rev() {
        hash = node_hash(left, &hash);
        hashes.push(hash);
    }
    Ok(hashes)
}

/// The largest power of two below `n`, and its base-2 logarithm. `n` is at least 2.
fn max_pow2(n: u64) -> (u64, u32) {
    let mut level = 0;
    while (1u64 << (level + 1)) < n {
        level += 1;
    }
    (1 << level, level)
}

/// Pushes the stored hash indexes of the complete subtrees that make up records `lo..hi`.
fn sub_tree_index(mut lo: u64, hi: u64, need: &mut Vec<u64>) {
    while lo < hi {
        let (k, level) = max_pow2(hi - lo + 1);
        need.push(stored_hash_index(level, lo >> level));
        lo += k;
    }
}

/// The hash of records `lo..hi` from the subtree hashes [`sub_tree_index`] named, and the hashes
/// left after them.
fn sub_tree_hash(mut lo: u64, hi: u64, hashes: &[Hash]) -> (Hash, &[Hash]) {
    let mut count = 0;
    while lo < hi {
        let (k, _) = max_pow2(hi - lo + 1);
        lo += k;
        count += 1;
    }
    let (trees, rest) = hashes.split_at(count);
    let mut trees = trees.iter().rev();
    let mut hash = *trees.next().expect("a range has at least one subtree");
    for left in trees {
        hash = node_hash(left, &hash);
    }
    (hash, rest)
}

/// The root hash of the tree's first `size` records.
pub(crate) fn tree_hash(size: u64, reader: &mut dyn HashReader) -> Result<Hash, Error> {
    if size == 0 {
        return Ok(empty_root());
    }
    let mut indexes = Vec::new();
    sub_tree_index(0, size, &mut indexes);
    let hashes = read_exact(reader, &indexes)?;
    Ok(sub_tree_hash(0, size, &hashes).0)
}

/// The inclusion proof of record `index` in the tree of `size` records (RFC 6962 § 2.1.1).
pub(crate) fn prove_record(
    size: u64,
    index: u64,
    reader: &mut dyn HashReader,
) -> Result<Vec<Hash>, Error> {
    if index >= size {
        return Err(Error::OutOfRange(format!(
            "record {index} is not in a tree of {size} records"
        )));
    }
    let mut indexes = Vec::new();
    leaf_proof_index(0, size, index, &mut indexes);
    let hashes = read_exact(reader, &indexes)?;
    Ok(leaf_proof(0, size, index, &hashes).0)
}

/// Pushes the stored hash indexes [`leaf_proof`] reads.
fn leaf_proof_index(lo: u64, hi: u64, n: u64, need: &mut Vec<u64>) {
    if lo + 1 == hi {
        return;
    }
    let (k, _) = max_pow2(hi - lo);
    if n < lo + k {
        leaf_proof_index(lo, lo + k, n, need);
        sub_tree_index(lo + k, hi, need);
    } else {
        sub_tree_index(lo, lo + k, need);
        leaf_proof_index(lo + k, hi, n, need);
    }
}

/// The inclusion proof of record `n` in records `lo..hi`, nearest the record first.
fn leaf_proof(lo: u64, hi: u64, n: u64, hashes: &[Hash]) -> (Vec<Hash>, &[Hash]) {
    if lo + 1 == hi {
        return (Vec::new(), hashes);
    }
    let (k, _) = max_pow2(hi - lo);
    let (mut proof, sibling, rest) = if n < lo + k {
        let (proof, rest) = leaf_proof(lo, lo + k, n, hashes);
        let (sibling, rest) = sub_tree_hash(lo + k, hi, rest);
        (proof, sibling, rest)
    } else {
        let (sibling, rest) = sub_tree_hash(lo, lo + k, hashes);
        let (proof, rest) = leaf_proof(lo + k, hi, n, rest);
        (proof, sibling, rest)
    };
    proof.push(sibling);
    (proof, rest)
}

/// Checks that `proof` shows record `index`, whose record hash is `leaf`, in the tree of `size`
/// records whose root is `root`.
pub(crate) fn check_record(
    proof: &[Hash],
    size: u64,
    root: &Hash,
    index: u64,
    leaf: &Hash,
) -> Result<(), Error> {
    if index >= size {
        return Err(Error::ProofFailed);
    }
    if run_record_proof(proof, 0, size, index, leaf)? == *root {
        Ok(())
    } else {
        Err(Error::ProofFailed)
    }
}

/// The root of records `lo..hi` that `proof` leads to from record `n`'s hash.
fn run_record_proof(proof: &[Hash], lo: u64, hi: u64, n: u64, leaf: &Hash) -> Result<Hash, Error> {
    if lo + 1 == hi {
        return if proof.is_empty() {
            Ok(*leaf)
        } else {
            Err(Error::ProofFailed)
        };
    }
    let Some((sibling, inner)) = proof.split_last() else {
        return Err(Error::ProofFailed);
    };
    let (k, _) = max_pow2(hi - lo);
    if n < lo + k {
        let hash = run_record_proof(inner, lo, lo + k, n, leaf)?;
        Ok(node_hash(&hash, sibling))
    } else {
        let hash = run_record_proof(inner, lo + k, hi, n, leaf)?;
        Ok(node_hash(sibling, &hash))
    }
}

/// The consistency proof from the tree's first `old` records to its first `size` (RFC 6962
/// § 2.1.2). `old` is at least 1 and at most `size`.
pub(crate) fn prove_tree(
    size: u64,
    old: u64,
    reader: &mut dyn HashReader,
) -> Result<Vec<Hash>, Error> {
    if old == 0 || old > size {
        return Err(Error::OutOfRange(format!(
            "a tree of {old} records is not a prefix of one of {size}"
        )));
    }
    let mut indexes = Vec::new();
    tree_proof_index(0, size, old, &mut indexes);
    let hashes = read_exact(reader, &indexes)?;
    Ok(tree_proof(0, size, old, &hashes).0)
}

/// Pushes the stored hash indexes [`tree_proof`] reads.
fn tree_proof_index(lo: u64, hi: u64, n: u64, need: &mut Vec<u64>) {
    if n == hi {
        if lo != 0 {
            sub_tree_index(lo, hi, need);
        }
        return;
    }
    let (k, _) = max_pow2(hi - lo);
    if n <= lo + k {
        tree_proof_index(lo, lo + k, n, need);
        sub_tree_index(lo + k, hi, need);
    } else {
        sub_tree_index(lo, lo + k, need);
        tree_proof_index(lo + k, hi, n, need);
    }
}

/// The consistency proof from records `lo..n` to records `lo..hi`.
fn tree_proof(lo: u64, hi: u64, n: u64, hashes: &[Hash]) -> (Vec<Hash>, &[Hash]) {
    if n == hi {
        if lo == 0 {
            return (Vec::new(), hashes);
        }
        let (hash, rest) = sub_tree_hash(lo, hi, hashes);
        return (vec![hash], rest);
    }
    let (k, _) = max_pow2(hi - lo);
    let (mut proof, sibling, rest) = if n <= lo + k {
        let (proof, rest) = tree_proof(lo, lo + k, n, hashes);
        let (sibling, rest) = sub_tree_hash(lo + k, hi, rest);
        (proof, sibling, rest)
    } else {
        let (sibling, rest) = sub_tree_hash(lo, lo + k, hashes);
        let (proof, rest) = tree_proof(lo + k, hi, n, rest);
        (proof, sibling, rest)
    };
    proof.push(sibling);
    (proof, rest)
}

/// Checks that `proof` shows the tree of `old` records with root `old_root` is a prefix of the
/// tree of `size` records with root `root`.
pub(crate) fn check_tree(
    proof: &[Hash],
    size: u64,
    root: &Hash,
    old: u64,
    old_root: &Hash,
) -> Result<(), Error> {
    if old == 0 || old > size {
        return Err(Error::ProofFailed);
    }
    let (got_old, got_new) = run_tree_proof(proof, 0, size, old, old_root)?;
    if got_old == *old_root && got_new == *root {
        Ok(())
    } else {
        Err(Error::ProofFailed)
    }
}

/// The roots of records `lo..n` and `lo..hi` that `proof` leads to, starting from `old`.
fn run_tree_proof(
    proof: &[Hash],
    lo: u64,
    hi: u64,
    n: u64,
    old: &Hash,
) -> Result<(Hash, Hash), Error> {
    if n == hi {
        return match (lo, proof) {
            (0, []) => Ok((*old, *old)),
            (1.., [hash]) => Ok((*hash, *hash)),
            _ => Err(Error::ProofFailed),
        };
    }
    let Some((sibling, inner)) = proof.split_last() else {
        return Err(Error::ProofFailed);
    };
    let (k, _) = max_pow2(hi - lo);
    if n <= lo + k {
        let (old_hash, hash) = run_tree_proof(inner, lo, lo + k, n, old)?;
        Ok((old_hash, node_hash(&hash, sibling)))
    } else {
        let (old_hash, hash) = run_tree_proof(inner, lo + k, hi, n, old)?;
        Ok((node_hash(sibling, &old_hash), node_hash(sibling, &hash)))
    }
}

/// What a tile holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Level {
    /// Hashes of Merkle level `8 * L`, for `L` up to 63.
    Hashes(u8),
    /// The records themselves: an entry bundle, beside the level-0 tile of the same index.
    Entries,
}

/// One tile or entry bundle of a log directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Tile {
    pub(crate) level: Level,
    /// Which run of 256 the tile is, counting from 0 within its level.
    pub(crate) index: u64,
    /// How many of the 256 it holds; below 256 it is a partial tile.
    pub(crate) width: u64,
}

impl Tile {
    /// The tile's path under the log's prefix, as C2SP writes it: `tile/<L>/<N>[.p/<W>]`, with
    /// `<N>` in groups of three digits and every group but the last prefixed with `x`.
    pub(crate) fn path(&self) -> String {
        let mut n = self.index;
        let mut index = format!("{:03}", n % 1000);
        while n >= 1000 {
            n /= 1000;
            index = format!("x{:03}/{index}", n % 1000);
        }
        let level = match self.level {
            Level::Hashes(level) => level.to_string(),
            Level::Entries => "entries".to_owned(),
        };
        if self.width == TILE_WIDTH {
            format!("tile/{level}/{index}")
        } else {
            format!("tile/{level}/{index}.p/{}", self.width)
        }
    }

    /// The tile a C2SP path names, or `None` when the path is not one [`Self::path`] writes.
    pub(crate) fn parse(path: &str) -> Option<Self> {
        let rest = path.strip_prefix("tile/")?;
        let (level, rest) = rest.split_once('/')?;
        let level = match level {
            "entries" => Level::Entries,
            level => Level::Hashes(level.parse().ok().filter(|level| *level < 64)?),
        };
        let (groups, width) = match rest.split_once(".p/") {
            Some((groups, width)) => (groups, width.parse().ok()?),
            None => (rest, TILE_WIDTH),
        };
        if width == 0 || width > TILE_WIDTH {
            return None;
        }
        let mut index: u64 = 0;
        let mut groups = groups.split('/').peekable();
        while let Some(group) = groups.next() {
            let digits = if groups.peek().is_some() {
                group.strip_prefix('x')?
            } else {
                group
            };
            if digits.len() != 3 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let group: u64 = digits.parse().ok()?;
            index = index.checked_mul(1000)?.checked_add(group)?;
        }
        let tile = Self {
            level,
            index,
            width,
        };
        // A leading `x000` group, a leading zero or a `+` sign all parse, and none is written.
        (tile.path() == path).then_some(tile)
    }

    /// The Merkle level of the hashes in a hash tile.
    fn merkle_level(self) -> u32 {
        match self.level {
            Level::Hashes(level) => u32::from(level) * TILE_HEIGHT,
            Level::Entries => 0,
        }
    }
}

/// The smallest tile that holds stored hash `index`, and the byte range in it that the hash is
/// computed from.
fn tile_for_index(index: u64) -> (Tile, std::ops::Range<usize>) {
    let (level, n) = split_stored_hash_index(index);
    let tile_level = level / TILE_HEIGHT;
    // How far above the tile's own level the hash is, and its position at the tile's level.
    let above = level - tile_level * TILE_HEIGHT;
    let tile_index = (n << above) >> TILE_HEIGHT;
    let within = n - ((tile_index << TILE_HEIGHT) >> above);
    let tile = Tile {
        level: Level::Hashes(u8::try_from(tile_level).expect("a level is below 64")),
        index: tile_index,
        width: (within + 1) << above,
    };
    let start = usize::try_from(within << above).expect("a tile has 256 hashes") * HASH_SIZE;
    let end = usize::try_from((within + 1) << above).expect("a tile has 256 hashes") * HASH_SIZE;
    (tile, start..end)
}

/// The hash of the run of hashes in `data`, whose count is a power of two.
fn tile_hash(data: &[u8]) -> Hash {
    if data.len() == HASH_SIZE {
        return data.try_into().expect("the run is one hash");
    }
    let (left, right) = data.split_at(data.len() / 2);
    node_hash(&tile_hash(left), &tile_hash(right))
}

/// The tiles and entry bundles that growing the tree from `old` records to `new` writes, lowest
/// level first. An entry bundle follows the level-0 tile it belongs to.
fn new_tiles(old: u64, new: u64) -> Vec<Tile> {
    let mut tiles = Vec::new();
    let mut level = 0u8;
    loop {
        let shift = u32::from(level) * TILE_HEIGHT;
        let (old_n, new_n) = (old >> shift, new >> shift);
        if new_n == 0 {
            break;
        }
        if old_n != new_n {
            let full = (old_n >> TILE_HEIGHT)..(new_n >> TILE_HEIGHT);
            let partial = new_n % TILE_WIDTH;
            let widths = full
                .map(|index| (index, TILE_WIDTH))
                .chain((partial > 0).then_some((new_n >> TILE_HEIGHT, partial)));
            for (index, width) in widths {
                tiles.push(Tile {
                    level: Level::Hashes(level),
                    index,
                    width,
                });
                if level == 0 {
                    tiles.push(Tile {
                        level: Level::Entries,
                        index,
                        width,
                    });
                }
            }
        }
        if shift + TILE_HEIGHT >= 64 {
            break;
        }
        level += 1;
    }
    tiles
}

/// The bytes of hash tile `tile`: its hashes, in order.
fn read_tile_data(tile: Tile, reader: &mut dyn HashReader) -> Result<Vec<u8>, Error> {
    let level = tile.merkle_level();
    let start = tile.index * TILE_WIDTH;
    let indexes: Vec<u64> = (start..start + tile.width)
        .map(|n| stored_hash_index(level, n))
        .collect();
    Ok(read_exact(reader, &indexes)?.concat())
}

/// A log directory as it stands at a known size, read through its tiles.
#[derive(Debug)]
pub(crate) struct Log {
    dir: PathBuf,
    size: u64,
    /// Each tile read so far, by level and index.
    tiles: HashMap<(u8, u64), Vec<u8>>,
    /// Hashes an [`append`] has computed and not yet written, by stored hash index.
    pending: HashMap<u64, Hash>,
}

impl Log {
    /// The log in `dir`, as the checkpoint of `size` records describes it.
    pub(crate) fn open(dir: &Path, size: u64) -> Self {
        Self {
            dir: dir.to_path_buf(),
            size,
            tiles: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    /// The width tile `(level, index)` has at the log's size.
    fn width_at_size(&self, level: u8, index: u64) -> u64 {
        let count = self.size >> (u32::from(level) * TILE_HEIGHT);
        count.saturating_sub(index * TILE_WIDTH).min(TILE_WIDTH)
    }

    /// The stored hash at `index`.
    fn read_hash(&mut self, index: u64) -> Result<Hash, Error> {
        if let Some(hash) = self.pending.get(&index) {
            return Ok(*hash);
        }
        let (tile, range) = tile_for_index(index);
        let Level::Hashes(level) = tile.level else {
            unreachable!("a stored hash is in a hash tile")
        };
        let width = self.width_at_size(level, tile.index);
        if width < tile.width {
            return Err(Error::OutOfRange(format!(
                "stored hash {index} is not in a tree of {} records",
                self.size
            )));
        }
        if !self.tiles.contains_key(&(level, tile.index)) {
            let path = self.dir.join(Tile { width, ..tile }.path());
            let data = fs::read(&path).map_err(|error| Error::Io {
                path: path.clone(),
                error,
            })?;
            if u64::try_from(data.len()).ok() != Some(width * HASH_SIZE as u64) {
                return Err(Error::Malformed {
                    path,
                    reason: format!("a tile of {width} hashes is {} bytes", data.len()),
                });
            }
            self.tiles.insert((level, tile.index), data);
        }
        Ok(tile_hash(&self.tiles[&(level, tile.index)][range]))
    }
}

impl HashReader for Log {
    fn read_hashes(&mut self, indexes: &[u64]) -> Result<Vec<Hash>, Error> {
        indexes.iter().map(|index| self.read_hash(*index)).collect()
    }
}

/// The entries of the bundle at `path`, which holds `width` records.
fn read_bundle(path: &Path, width: u64) -> Result<Vec<Vec<u8>>, Error> {
    let data = fs::read(path).map_err(|error| Error::Io {
        path: path.to_path_buf(),
        error,
    })?;
    let malformed = |reason: &str| Error::Malformed {
        path: path.to_path_buf(),
        reason: reason.to_owned(),
    };
    let mut entries = Vec::new();
    let mut rest = data.as_slice();
    while let Some((length, after)) = rest.split_first_chunk::<2>() {
        let length = usize::from(u16::from_be_bytes(*length));
        if after.len() < length {
            return Err(malformed("an entry runs past the end of the bundle"));
        }
        let (entry, after) = after.split_at(length);
        entries.push(entry.to_vec());
        rest = after;
    }
    if !rest.is_empty() || u64::try_from(entries.len()).ok() != Some(width) {
        return Err(malformed("the bundle does not hold its width of entries"));
    }
    Ok(entries)
}

/// Writes `data` to `path` unless the same bytes are already there. Other bytes at `path` are
/// an error: a file of the log never changes once written.
fn write_once(path: &Path, data: &[u8]) -> Result<(), Error> {
    let failed = |error| Error::Io {
        path: path.to_path_buf(),
        error,
    };
    match fs::read(path) {
        Ok(existing) if existing == data => return Ok(()),
        Ok(_) => {
            return Err(Error::Malformed {
                path: path.to_path_buf(),
                reason: "the file has other bytes, and a log file never changes".to_owned(),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(failed(error)),
    }
    let parent = path.parent().expect("a tile path has a parent");
    fs::create_dir_all(parent).map_err(failed)?;
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    fs::write(&partial, data).map_err(failed)?;
    fs::rename(&partial, path).map_err(failed)
}

/// Appends `entries` to the log in `dir`, whose last checkpoint has `size` records, and writes
/// every tile and entry bundle that changes. Returns the new size and root hash, which the next
/// checkpoint signs.
pub(crate) fn append(dir: &Path, size: u64, entries: &[Vec<u8>]) -> Result<(u64, Hash), Error> {
    if let Some(entry) = entries.iter().find(|entry| entry.len() > MAX_ENTRY_BYTES) {
        return Err(Error::OutOfRange(format!(
            "an entry of {} bytes is over the {MAX_ENTRY_BYTES} a bundle can hold",
            entry.len()
        )));
    }
    let mut log = Log::open(dir, size);
    let first_bundle = size >> TILE_HEIGHT;
    let base = first_bundle * TILE_WIDTH;
    // The records of the partial bundle this append extends, each checked against its tile.
    let carried = match size % TILE_WIDTH {
        0 => Vec::new(),
        width => {
            let bundle = Tile {
                level: Level::Entries,
                index: first_bundle,
                width,
            };
            let path = dir.join(bundle.path());
            let carried = read_bundle(&path, width)?;
            for (n, entry) in (base..).zip(&carried) {
                if log.read_hash(stored_hash_index(0, n))? != record_hash(entry) {
                    return Err(Error::Malformed {
                        path,
                        reason: format!("record {n} does not match its hash"),
                    });
                }
            }
            carried
        }
    };
    for (n, entry) in (size..).zip(entries) {
        let hashes = stored_hashes(n, entry, &mut log)?;
        let first = stored_hash_index(0, n);
        log.pending.extend((first..).zip(hashes));
    }
    let new_size = size + entries.len() as u64;
    let record = |n: u64| -> &[u8] {
        match n.checked_sub(size) {
            Some(new) => &entries[usize::try_from(new).expect("the entries are in memory")],
            None => &carried[usize::try_from(n - base).expect("a bundle has 256 records")],
        }
    };
    for tile in new_tiles(size, new_size) {
        let data = match tile.level {
            Level::Hashes(_) => read_tile_data(tile, &mut log)?,
            Level::Entries => {
                let start = tile.index * TILE_WIDTH;
                let mut data = Vec::new();
                for n in start..start + tile.width {
                    let entry = record(n);
                    let length = u16::try_from(entry.len()).expect("checked above");
                    data.extend_from_slice(&length.to_be_bytes());
                    data.extend_from_slice(entry);
                }
                data
            }
        };
        write_once(&dir.join(tile.path()), &data)?;
    }
    let root = tree_hash(new_size, &mut log)?;
    Ok((new_size, root))
}

/// Checks the log in `dir` from scratch against a checkpoint of `size` records with root hash
/// `root`: every record of every entry bundle against the level-0 tile beside it, every hash of
/// a higher tile against the full tile below it, and the root against the tree. `each` is given
/// every record in order, and its error names what is wrong with one.
///
/// **Memory:** two tiles at a time, whatever the log's size.
pub(crate) fn verify(
    dir: &Path,
    size: u64,
    root: &Hash,
    mut each: impl FnMut(u64, &[u8]) -> Result<(), String>,
) -> Result<(), Error> {
    let read_tile = |tile: Tile| read_hash_tile(dir, tile);
    let mut level_zero = Vec::new();
    for tile in new_tiles(0, size) {
        match tile.level {
            Level::Hashes(0) => level_zero = read_tile(tile)?.1,
            Level::Entries => {
                let path = dir.join(tile.path());
                let entries = read_bundle(&path, tile.width)?;
                for ((n, entry), hash) in (tile.index * TILE_WIDTH..)
                    .zip(&entries)
                    .zip(level_zero.as_chunks::<HASH_SIZE>().0)
                {
                    let reason = if record_hash(entry) != *hash {
                        Some(format!("record {n} does not match its hash"))
                    } else {
                        each(n, entry).err().map(|why| format!("record {n}: {why}"))
                    };
                    if let Some(reason) = reason {
                        return Err(Error::Malformed { path, reason });
                    }
                }
            }
            Level::Hashes(level) => {
                let (path, data) = read_tile(tile)?;
                for (j, hash) in (0..tile.width).zip(data.as_chunks::<HASH_SIZE>().0) {
                    let below = Tile {
                        level: Level::Hashes(level - 1),
                        index: tile.index * TILE_WIDTH + j,
                        width: TILE_WIDTH,
                    };
                    if tile_hash(&read_tile(below)?.1) != *hash {
                        return Err(Error::Malformed {
                            path,
                            reason: format!("hash {j} does not match the tile below it"),
                        });
                    }
                }
            }
        }
    }
    if tree_hash(size, &mut Log::open(dir, size))? != *root {
        return Err(Error::Malformed {
            path: dir.to_path_buf(),
            reason: "the tree's root hash is not the checkpoint's".to_owned(),
        });
    }
    Ok(())
}

/// The path and bytes of hash tile `tile` in the log in `dir`, which must hold its width of
/// hashes.
fn read_hash_tile(dir: &Path, tile: Tile) -> Result<(PathBuf, Vec<u8>), Error> {
    let path = dir.join(tile.path());
    let data = fs::read(&path).map_err(|error| Error::Io {
        path: path.clone(),
        error,
    })?;
    if u64::try_from(data.len()).ok() != Some(tile.width * HASH_SIZE as u64) {
        return Err(Error::Malformed {
            reason: format!("a tile of {} hashes is {} bytes", tile.width, data.len()),
            path,
        });
    }
    Ok((path, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree in memory: every stored hash, in stored hash index order.
    #[derive(Default)]
    struct Memory(Vec<Hash>);

    impl Memory {
        fn of(records: &[Vec<u8>]) -> Self {
            let mut memory = Self::default();
            for (n, record) in (0..).zip(records) {
                let hashes = stored_hashes(n, record, &mut memory).expect("the tree is in memory");
                memory.0.extend(hashes);
            }
            memory
        }
    }

    impl HashReader for Memory {
        fn read_hashes(&mut self, indexes: &[u64]) -> Result<Vec<Hash>, Error> {
            indexes
                .iter()
                .map(|index| {
                    usize::try_from(*index)
                        .ok()
                        .and_then(|index| self.0.get(index).copied())
                        .ok_or_else(|| Error::OutOfRange(format!("no stored hash {index}")))
                })
                .collect()
        }
    }

    /// RFC 6962 § 2.1's definition of the root, straight from the records.
    fn reference_root(records: &[Vec<u8>]) -> Hash {
        match records.len() {
            0 => empty_root(),
            1 => record_hash(&records[0]),
            len => {
                let (k, _) = max_pow2(len as u64);
                let (left, right) = records.split_at(usize::try_from(k).unwrap());
                node_hash(&reference_root(left), &reference_root(right))
            }
        }
    }

    fn hash(text: &str) -> Hash {
        let bytes: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("the vector is hex"))
            .collect();
        bytes.try_into().expect("the vector is 32 bytes")
    }

    fn hashes(value: &serde_json::Value) -> Vec<Hash> {
        value
            .as_array()
            .expect("a list of hashes")
            .iter()
            .map(|text| hash(text.as_str().expect("a hex string")))
            .collect()
    }

    fn number(value: &serde_json::Value) -> u64 {
        value.as_u64().expect("a number")
    }

    fn vectors() -> serde_json::Value {
        serde_json::from_str(include_str!("../../tests/fixtures/tlog/rfc6962.json"))
            .expect("the vectors parse")
    }

    fn leaves(vectors: &serde_json::Value) -> Vec<Vec<u8>> {
        vectors["leaves"]
            .as_array()
            .expect("a list of leaves")
            .iter()
            .map(|text| {
                let text = text.as_str().expect("a hex string");
                (0..text.len())
                    .step_by(2)
                    .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex"))
                    .collect()
            })
            .collect()
    }

    /// Every proof hash changed in one bit, one at a time.
    fn each_hash_changed(proof: &[Hash]) -> impl Iterator<Item = Vec<Hash>> + '_ {
        (0..proof.len()).map(|at| {
            let mut changed = proof.to_vec();
            changed[at][at % HASH_SIZE] ^= 1;
            changed
        })
    }

    #[test]
    fn records_and_nodes_hash_as_the_vectors_say() {
        let vectors = vectors();
        let leaves = leaves(&vectors);
        let levels: Vec<Vec<Hash>> = vectors["node_hashes"]
            .as_array()
            .expect("levels")
            .iter()
            .map(hashes)
            .collect();
        for (leaf, want) in leaves.iter().zip(&levels[0]) {
            assert_eq!(record_hash(leaf), *want);
        }
        for pair in levels.windows(2) {
            for (at, want) in pair[1].iter().enumerate() {
                assert_eq!(node_hash(&pair[0][2 * at], &pair[0][2 * at + 1]), *want);
            }
        }
    }

    #[test]
    fn each_tree_size_has_the_vectors_root() {
        let vectors = vectors();
        let leaves = leaves(&vectors);
        let mut memory = Memory::of(&leaves);
        for (size, want) in (0..).zip(hashes(&vectors["roots"])) {
            assert_eq!(tree_hash(size, &mut memory).unwrap(), want, "size {size}");
            assert_eq!(
                stored_hash_count(size),
                Memory::of(&leaves[..usize::try_from(size).unwrap()])
                    .0
                    .len() as u64
            );
        }
    }

    #[test]
    fn a_stored_hash_index_splits_back_into_its_level_and_position() {
        for index in 0..5000 {
            let (level, n) = split_stored_hash_index(index);
            assert_eq!(stored_hash_index(level, n), index);
        }
    }

    #[test]
    fn inclusion_proofs_are_the_vectors_and_one_changed_hash_fails() {
        let vectors = vectors();
        let leaves = leaves(&vectors);
        let roots = hashes(&vectors["roots"]);
        let mut memory = Memory::of(&leaves);
        for case in vectors["inclusion"].as_array().unwrap() {
            let (index, size) = (number(&case["index"]), number(&case["size"]));
            let want = hashes(&case["proof"]);
            let root = roots[usize::try_from(size).unwrap()];
            let leaf = record_hash(&leaves[usize::try_from(index).unwrap()]);
            assert_eq!(prove_record(size, index, &mut memory).unwrap(), want);
            check_record(&want, size, &root, index, &leaf).unwrap();
            for changed in each_hash_changed(&want) {
                assert!(matches!(
                    check_record(&changed, size, &root, index, &leaf),
                    Err(Error::ProofFailed)
                ));
            }
            let other = (index + 1) % size;
            if other != index {
                assert!(check_record(&want, size, &root, other, &leaf).is_err());
            }
            assert!(check_record(&want, size, &root, size, &leaf).is_err());
        }
    }

    #[test]
    fn consistency_proofs_are_the_vectors_and_one_changed_hash_fails() {
        let vectors = vectors();
        let leaves = leaves(&vectors);
        let roots = hashes(&vectors["roots"]);
        let mut memory = Memory::of(&leaves);
        for case in vectors["consistency"].as_array().unwrap() {
            let (old, size) = (number(&case["old"]), number(&case["new"]));
            let want = hashes(&case["proof"]);
            let old_root = roots[usize::try_from(old).unwrap()];
            let root = roots[usize::try_from(size).unwrap()];
            assert_eq!(prove_tree(size, old, &mut memory).unwrap(), want);
            check_tree(&want, size, &root, old, &old_root).unwrap();
            for changed in each_hash_changed(&want) {
                assert!(matches!(
                    check_tree(&changed, size, &root, old, &old_root),
                    Err(Error::ProofFailed)
                ));
            }
            // A root of another tree is a fork, whichever side it is on.
            let other = roots[usize::try_from(old - 1).unwrap()];
            assert!(check_tree(&want, size, &root, old, &other).is_err());
            assert!(check_tree(&want, size, &other, old, &old_root).is_err());
        }
    }

    #[test]
    fn every_proof_in_a_small_tree_checks_against_the_reference_root() {
        let records: Vec<Vec<u8>> = (0..40u32).map(|n| n.to_be_bytes().to_vec()).collect();
        let mut memory = Memory::of(&records);
        let roots: Vec<Hash> = (0..=records.len())
            .map(|size| reference_root(&records[..size]))
            .collect();
        for size in 1..=40u64 {
            let root = roots[usize::try_from(size).unwrap()];
            assert_eq!(tree_hash(size, &mut memory).unwrap(), root);
            for index in 0..size {
                let proof = prove_record(size, index, &mut memory).unwrap();
                let leaf = record_hash(&records[usize::try_from(index).unwrap()]);
                check_record(&proof, size, &root, index, &leaf).unwrap();
            }
            for old in 1..=size {
                let proof = prove_tree(size, old, &mut memory).unwrap();
                let old_root = roots[usize::try_from(old).unwrap()];
                check_tree(&proof, size, &root, old, &old_root).unwrap();
            }
        }
    }

    #[test]
    fn tile_paths_are_the_vectors() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/tlog/tiles.json")).unwrap();
        for case in vectors["paths"].as_array().unwrap() {
            let path = case["path"].as_str().unwrap();
            let tile = Tile {
                level: match case["level"].as_u64() {
                    Some(level) => Level::Hashes(u8::try_from(level).unwrap()),
                    None => Level::Entries,
                },
                index: number(&case["index"]),
                width: number(&case["width"]),
            };
            assert_eq!(tile.path(), path);
            assert_eq!(Tile::parse(path), Some(tile), "{path}");
        }
        for path in vectors["invalid"].as_array().unwrap() {
            let path = path.as_str().unwrap();
            assert_eq!(Tile::parse(path), None, "{path}");
        }
    }

    #[test]
    fn a_log_directory_appended_in_runs_is_the_tree_of_all_its_records() {
        let scratch = nvs_repo::scratch("tlog-append");
        let dir: &Path = &scratch;
        let records: Vec<Vec<u8>> = (0..600u32)
            .map(|n| format!("record {n}").into_bytes())
            .collect();
        let mut size = 0;
        for run in [1, 255, 1, 300, 43] {
            let end = size + run;
            let (new_size, root) = append(dir, size as u64, &records[size..end]).unwrap();
            assert_eq!(new_size, end as u64);
            assert_eq!(root, reference_root(&records[..end]), "size {end}");
            size = end;
        }
        // 600 records are two full tiles and 88 more, and two level-1 hashes.
        for path in [
            "tile/0/000",
            "tile/0/001",
            "tile/0/002.p/88",
            "tile/1/000.p/2",
        ] {
            assert!(dir.join(path).is_file(), "{path}");
        }
        let bundle = read_bundle(&dir.join("tile/entries/002.p/88"), 88).unwrap();
        assert_eq!(bundle, records[512..600]);

        let mut log = Log::open(dir, 600);
        let mut memory = Memory::of(&records);
        for index in [0, 255, 256, 511, 599] {
            assert_eq!(
                prove_record(600, index, &mut log).unwrap(),
                prove_record(600, index, &mut memory).unwrap()
            );
        }
        assert_eq!(
            prove_tree(600, 257, &mut log).unwrap(),
            prove_tree(600, 257, &mut memory).unwrap()
        );
        // Record 600 is past the log's size, so its hash is in no tile.
        assert!(matches!(
            tree_hash(601, &mut log),
            Err(Error::OutOfRange(_))
        ));
    }

    #[test]
    fn an_append_from_a_stale_size_does_not_rewrite_the_log() {
        let scratch = nvs_repo::scratch("tlog-stale");
        let dir: &Path = &scratch;
        let first: Vec<Vec<u8>> = (0..10u8).map(|n| vec![n]).collect();
        append(dir, 0, &first).unwrap();
        let before = fs::read(dir.join("tile/0/000.p/10")).unwrap();
        let other: Vec<Vec<u8>> = (0..10u8).map(|n| vec![n, n]).collect();
        assert!(matches!(
            append(dir, 0, &other),
            Err(Error::Malformed { .. })
        ));
        assert_eq!(fs::read(dir.join("tile/0/000.p/10")).unwrap(), before);
    }

    #[test]
    fn a_changed_record_in_a_partial_bundle_is_found_before_it_is_extended() {
        let scratch = nvs_repo::scratch("tlog-bundle");
        let dir: &Path = &scratch;
        let records: Vec<Vec<u8>> = (0..5u8).map(|n| vec![n]).collect();
        append(dir, 0, &records).unwrap();
        let bundle = dir.join("tile/entries/000.p/5");
        let mut data = fs::read(&bundle).unwrap();
        *data.last_mut().unwrap() ^= 1;
        fs::write(&bundle, data).unwrap();
        assert!(matches!(
            append(dir, 5, &[vec![9]]),
            Err(Error::Malformed { .. })
        ));
    }

    #[test]
    fn verify_reads_every_tile_and_finds_one_changed_byte_at_any_level() {
        let scratch = nvs_repo::scratch("tlog-verify");
        let dir: &Path = &scratch;
        let records: Vec<Vec<u8>> = (0..600u32).map(|n| n.to_be_bytes().to_vec()).collect();
        let (size, root) = append(dir, 0, &records).unwrap();
        let mut seen = 0;
        verify(dir, size, &root, |n, entry: &[u8]| {
            assert_eq!(entry, records[usize::try_from(n).unwrap()]);
            seen += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, 600);

        let refuse_300 = |n, _: &[u8]| match n {
            300 => Err("not allowed".to_owned()),
            _ => Ok(()),
        };
        assert!(verify(dir, size, &root, refuse_300).is_err());
        assert!(verify(dir, size, &empty_root(), |_, _: &[u8]| Ok(())).is_err());
        for path in ["tile/0/001", "tile/1/000.p/2"] {
            let tile = dir.join(path);
            let data = fs::read(&tile).unwrap();
            let mut changed = data.clone();
            changed[40] ^= 1;
            fs::write(&tile, changed).unwrap();
            assert!(
                matches!(
                    verify(dir, size, &root, |_, _: &[u8]| Ok(())),
                    Err(Error::Malformed { .. })
                ),
                "{path}"
            );
            fs::write(&tile, data).unwrap();
        }
    }
}
