//! `Core\Zip` — spec § 17's archive class: what an archive is allowed to
//! contain, decided when it is *read* and not when a program gets around to
//! writing it somewhere.
//!
//! **Why this is Tier 0 rather than an extension.** A `../` entry, an
//! absolute-path entry and a symlink entry are the three ways an archive
//! written by an attacker reaches a file the program never named, and a
//! decompression bomb is the fourth thing an archive can do to a server. All
//! four are *policy*, and policy must be non-optional
//! (`rule:core-api/tier-roster`, [ADR 0051](/docs/decisions/0051.md) § 3), so
//! the class that reads an archive is always present and there is no
//! configuration, argument or extension that turns one of the four off.
//!
//! **The refusals happen in the reader, which is the whole design.** PHP's
//! shape — `ZipArchive::open` answers a handle, `statIndex` hands out whatever
//! name the archive claimed, and `extractTo` is where somebody remembers to
//! check — makes safety a thing each caller does, and a caller that walks the
//! entries itself opts out of it by writing ordinary code. Here there is no
//! entry to walk that [`directory`] has not already judged: `entries` and
//! `read` both go through it, so a hostile entry has no spelling that reaches
//! a program at all. What is refused is refused for the whole archive rather
//! than skipped, because an archive carrying one traversing entry is not a
//! well-meaning archive with a bad row in it.
//!
//! **A name is judged by `Core\Path`'s grammar and never a second one.** The
//! absolute test is [`crate::path::parse`]'s own — a leading separator, or a
//! drive — on every platform in both directions, which is that module's "one
//! grammar everywhere". A copy of the predicate here would be a second
//! implementation of a comparison `rule:security/path-scope-canonicalise-then-prefix`
//! says there is exactly one of, and the way one of the two ends up accepting
//! something the other refuses.
//!
//! **The bomb is `Core\Compress`'s bound and not a second rule.**
//! [`crate::compress::Bound`] is the one arithmetic —
//! `min(input × ratio, ceiling)`, lowerable by a call and raisable by nobody —
//! and `rule:core-classes/decompression-bound` is stated once for both
//! classes. What is local here is only the read loop, so a refusal can say
//! `Core\Zip` and name the entry it stopped at; the numbers it compares
//! against are [`Bound::within`]'s and the operator's `[limits]`. Charging it
//! *across* the archive as well as per entry is [`Budget`], which hands each
//! entry a bound whose ceiling is what the archive has left — the same
//! arithmetic one level up rather than a second rule for the level above.
//!
//! **Extraction resolves before it creates, one level at a time.** Every
//! directory [`extract`](CLASS) needs is created and then resolved, and the
//! resolution is compared against the destination before anything is created
//! inside it — so a link planted between two entries is found at the level it
//! sits at and refused, rather than followed. That order is
//! `rule:security/path-scope-canonicalise-then-prefix`'s and is the opposite of
//! checking the name the archive wrote, which is a comparison against a string
//! the filesystem was never asked about. The file itself is created
//! exclusively: an entry whose name is already taken refuses rather than
//! overwriting, because what is already there may be a link somebody else wrote
//! and following it is the escape this paragraph exists to stop.
//!
//! **An extraction that refuses part way leaves what it had already written**,
//! and says so rather than pretending to unwind. The refusals that are about
//! the archive all happen in [`directory`] before a directory is created, so
//! what can stop a call mid-way is the bound running out or the disk refusing —
//! and both leave a caller who has to decide what to do with a partial
//! destination. Removing files to undo a failed call is the one thing a class
//! this defensive should not be doing on its own: the destination is the
//! program's, an unwind is a second walk over paths that may have changed
//! again, and a caller that wants an all-or-nothing extraction has one — an
//! empty directory of its own to extract into.
//!
//! **Every refusal about the archive is a `ParseError` and never an `IOError`.** A hostile
//! archive and a failing disk are different questions, and a caller that
//! cannot tell them apart retries the one it should have refused.
//!
//! **Zip64 is read from the records that carry it.** An archive past 4 GiB or
//! past 65535 entries writes `0xFFFFFFFF` in the size and offset fields this
//! reader walks, puts the real values in a zip64 extra field on each record,
//! and hands its entry count and its directory's offset to a zip64 end record
//! that a locator in front of the ordinary end record points at. Both are
//! read, so such an archive is read rather than refused as malformed, and one
//! whose narrow field is the sentinel while the wide record it names is absent
//! is refused rather than walked from the sentinel itself. The bound is still
//! measured as the output grows rather than taken from either header, so
//! nothing here trusts a claimed size whichever record wrote it.
//!
//! **An entry's octets are checked against the CRC-32 the archive recorded**,
//! on the one route every member reads through, so a corrupt entry is refused
//! rather than answered or written. There is no argument that turns it off,
//! for the reason the four refusals have none, and it costs one pass over
//! output already in hand next to the decompression that produced it. A
//! *stored* entry is the half this catches: a deflate stream that has been
//! corrupted already fails to decode.
//!
//! **What this spends** (`rule:programs/memory-priority`): one entry's output
//! at a time, bounded by the ceiling above and attributable to the request
//! that asked for it. An extraction holds one entry and not the archive, for
//! the same reason. The central directory is walked into a `Vec` of entries
//! whose size is the archive's own entry count, beside a set of the names
//! borrowed from the archive — one pointer and length per entry, dropped when
//! the walk ends — which is what keeps the repeated-name refusal one lookup per
//! entry rather than a scan of every entry before it. Nothing holds the
//! decompressed archive. Zip64 adds no allocation to that — the wide values land in the
//! fields the narrow ones would have, and what a zip64 archive costs over a
//! narrow one is a bounded walk of one record's extra area per entry — and the
//! CRC is a rolling checksum over octets already held.

use std::collections::HashSet;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, ThrownClass, Value};

use crate::compress::{Bound, DEFAULT_MAX_BYTES, DEFAULT_MAX_RATIO};
use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Zip`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Zip";

/// `Core\Zip::extract`'s member name, as the capability doors and their
/// refusals both spell it.
const EXTRACT: &str = r"Core\Zip::extract";

/// `Core\Zip`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads zip archives. `entries` lists the names in an archive, `read` returns one file \
            from it, and `extract` writes its files into a folder. Every archive is checked \
            before any name or file is returned. An archive with an unsafe name, such as \
            `../config.php`, throws a `ParseError`.",
};

/// `Core\Zip`'s registry rows — the listing and the read, both of them through
/// the one reader that judges an entry before a program sees it.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "entries",
            names: &["archive"],
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_zip_entries",
            doc: Some(&ENTRIES_DOC),
        },
        CoreMethod {
            name: "read",
            names: &["archive", "name", "maxBytes", "maxRatio"],
            params: &[
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Uint,
                CoreTy::Uint,
            ],
            defaults: &[
                Const::Uint(DEFAULT_MAX_BYTES),
                Const::Uint(DEFAULT_MAX_RATIO),
            ],
            return_ty: CoreTy::TaintedBytes,
            symbol: "nvs_core_zip_read",
            doc: Some(&READ_DOC),
        },
        CoreMethod {
            name: "extract",
            names: &["archive", "destination", "maxBytes", "maxRatio"],
            params: &[
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Text(Qual::Sink),
                CoreTy::Uint,
                CoreTy::Uint,
            ],
            defaults: &[
                Const::Uint(DEFAULT_MAX_BYTES),
                Const::Uint(DEFAULT_MAX_RATIO),
            ],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_zip_extract",
            doc: Some(&EXTRACT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Zip::entries`'s reference card — `rule:core-api/reference-card`.
const ENTRIES_DOC: MethodDoc = MethodDoc {
    short: "The names an archive carries, in the order its central directory lists them — replacing \
            `zip_read`/`zip_entry_name` and `ZipArchive::statIndex`. The archive is judged whole \
            first: an archive carrying a traversing, absolute-path or symlink entry has no listing, \
            because a program that could see such an entry could act on it.",
    params: &[ParamDoc {
        name: "archive",
        desc: "The archive's octets. Only its central directory is read, so listing a large \
               archive costs its entry count rather than its size.",
        shape: &[],
    }],
    ret: "One `tainted string` per entry. A name is tainted whatever the archive's own type was, \
          for the reason a claim out of a verified token is: the name was written by whoever built \
          the archive, and a literal archive in a test is no safer than a downloaded one.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$archive` is not a well-formed zip archive, or it carries an entry naming an \
               absolute path, traversing out of the archive with a `..` component, repeating a \
               name, or marked as a symlink. Never an `IOError`: a hostile archive and a failing \
               disk are different questions.",
    }],
};

/// `Core\Zip::read`'s reference card — `rule:core-api/reference-card`.
const READ_DOC: MethodDoc = MethodDoc {
    short: "One entry's octets, decompressed **under a bound that cannot be switched off** — \
            replacing `zip_entry_read` and `ZipArchive::getFromName`, whose only limit was the \
            memory the request had left.",
    params: &[
        ParamDoc {
            name: "archive",
            desc: "The archive's octets, judged by the same reader `entries` uses: a hostile entry \
                   anywhere in it refuses this call too, whichever entry was asked for.",
            shape: &[],
        },
        ParamDoc {
            name: "name",
            desc: "The entry to read, compared whole against the archive's own names and never \
                   resolved as a path. A name `entries` just answered is accepted as it stands.",
            shape: &[],
        },
        ParamDoc {
            name: "maxBytes",
            desc: "The most output this call will produce, in octets. A call may ask for less than \
                   `[limits] max_decompressed` and never for more; `0` is a bound of zero and not \
                   a spelling for unbounded.",
            shape: &[],
        },
        ParamDoc {
            name: "maxRatio",
            desc: "The most output per octet of the entry's compressed size. The second half of \
                   the same bound, because a bomb is small on the wire.",
            shape: &[],
        },
    ],
    ret: "The entry's octets, never a truncation — an entry that would pass either half of the \
          bound throws instead of answering the prefix it had reached. Tainted, for the reason the \
          names are: octets out of an archive are somebody else's.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "Everything `entries` refuses, plus: no entry has that name, the entry is compressed \
               by a method this class does not read, its stream is not well formed, its octets do \
               not match the CRC-32 the archive recorded, or its output would pass either half of \
               the bound.",
    }],
};

/// `Core\Zip::extract`'s reference card — `rule:core-api/reference-card`.
const EXTRACT_DOC: MethodDoc = MethodDoc {
    short: "Writes an archive's entries under `$destination` and answers how many files it wrote — \
            replacing `ZipArchive::extractTo`, which wrote whatever names it had been handed. The \
            archive is judged whole before an octet is written, and every directory is created and \
            then resolved and proved to be under the destination before anything is created inside \
            it.",
    params: &[
        ParamDoc {
            name: "archive",
            desc: "The archive's octets, judged by the same reader `entries` uses: a hostile entry \
                   anywhere in it refuses this call before it writes anything at all.",
            shape: &[],
        },
        ParamDoc {
            name: "destination",
            desc: "The directory to write under, created if it is not there. `fs.write` is shown \
                   for it and `fs.read` as well, because resolving a name is reading the \
                   directories above it.",
            shape: &[],
        },
        ParamDoc {
            name: "maxBytes",
            desc: "The most output this call will produce **across the whole archive**, in octets, \
                   and not a per-entry allowance: an archive whose entries are each within it and \
                   whose total is not is the same attack one level up. A call may ask for less \
                   than `[limits] max_decompressed` and never for more.",
            shape: &[],
        },
        ParamDoc {
            name: "maxRatio",
            desc: "The most output per octet of an entry's compressed size. This half is per \
                   entry, because a ratio is a property of the stream being decoded.",
            shape: &[],
        },
    ],
    ret: "The number of files written. A directory entry is created and not counted, since what a \
          caller compares against is the number of files it now has.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "Everything `entries` refuses, plus an entry whose octets do not match the \
                   CRC-32 the archive recorded, or whose output would pass either half of the \
                   bound — per entry or across the archive, which is one rule.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`fs.read` or `fs.write` is not granted for the destination, or a directory \
                   under it resolved to somewhere outside it, which is a link that appeared while \
                   the extraction was running.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The creation or the write itself failed: a name an entry asked for is already \
                   taken, since a file is created exclusively rather than overwritten, or the disk \
                   refused. Never a refusal this class made — those name the rule.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_zip_entries" => (nvs_core_zip_entries as *const ()).cast(),
        "nvs_core_zip_read" => (nvs_core_zip_read as *const ()).cast(),
        "nvs_core_zip_extract" => (nvs_core_zip_extract as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The container
// ============================================================================

/// The end-of-central-directory record's signature.
const END_SIGNATURE: u32 = 0x0605_4b50;

/// A central-directory file header's signature.
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;

/// A local file header's signature.
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;

/// The end-of-central-directory record's width with an empty comment, which is
/// also the smallest an archive can be.
const END_WIDTH: usize = 22;

/// The widest comment that record can carry, which is how far back from the end
/// of the archive it can sit.
const MAX_COMMENT: usize = 0xFFFF;

/// The zip64 end-of-central-directory record's signature.
const ZIP64_END_SIGNATURE: u32 = 0x0606_4b50;

/// The zip64 *locator*'s signature — the record between the central directory
/// and the ordinary end record that says where the zip64 end record begins.
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;

/// The locator's width, which is fixed: it carries neither a name nor a
/// comment, so it sits exactly this far in front of the end record.
const ZIP64_LOCATOR_WIDTH: usize = 20;

/// The zip64 extended-information extra field's header id.
const ZIP64_EXTRA_ID: u16 = 0x0001;

/// The value a 32-bit field carries when the real one is in a zip64 record or
/// extra field instead.
const WIDE_U32: u32 = 0xFFFF_FFFF;

/// The same for the 16-bit field the end record counts entries in.
const WIDE_U16: u16 = 0xFFFF;

/// The stored method: the entry's octets are the entry.
const STORED: u16 = 0;

/// The deflate method — RFC 1951, the same stream `Core\Codec::Deflate` names.
const DEFLATE: u16 = 8;

/// The high nibble of a Unix mode, and the value of it that means symlink.
const MODE_KIND: u32 = 0xF000;

/// `S_IFLNK`.
const MODE_SYMLINK: u32 = 0xA000;

/// One central-directory record, as much of it as this class reads.
///
/// The sizes are the *central* directory's rather than the local header's,
/// which is what makes an entry written with a data descriptor readable at all:
/// a streaming writer leaves the local header's sizes zeroed and records them
/// only here.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    /// The entry's name, already judged by [`refuse_hostile`].
    pub(crate) name: String,
    /// The compression method, one of [`STORED`] or [`DEFLATE`].
    method: u16,
    /// The CRC-32 the archive recorded for the entry's decompressed octets,
    /// taken from the central directory rather than the local header for the
    /// reason the sizes are: a streaming writer leaves the local header's
    /// zeroed and records them only here.
    crc: u32,
    /// The entry's compressed size — the ratio half's input.
    compressed: usize,
    /// Where the entry's local header begins.
    at: usize,
}

/// The little-endian `u16` at `at`, or `None` where the archive ends first.
fn u16_at(archive: &[u8], at: usize) -> Option<u16> {
    let raw = archive.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([raw[0], raw[1]]))
}

/// The little-endian `u32` at `at`, or `None` where the archive ends first.
fn u32_at(archive: &[u8], at: usize) -> Option<u32> {
    let raw = archive.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

/// The little-endian `u64` at `at`, or `None` where the archive ends first —
/// the width every value zip64 widens is recorded in.
fn u64_at(archive: &[u8], at: usize) -> Option<u64> {
    let raw = archive.get(at..at.checked_add(8)?)?;
    Some(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

/// Where the end-of-central-directory record begins.
///
/// Backwards from the end, because the record is last and may be followed by a
/// comment of its own length. The search stops at the widest comment the format
/// allows, so a large archive costs a bounded scan rather than its own size.
fn end_record(archive: &[u8]) -> Option<usize> {
    let last = archive.len().checked_sub(END_WIDTH)?;
    let first = last.saturating_sub(MAX_COMMENT);
    (first..=last)
        .rev()
        .find(|at| u32_at(archive, *at) == Some(END_SIGNATURE))
}

/// How many entries the central directory holds, and where it begins.
///
/// The two are read together because they are written together: an archive
/// past either 32-bit limit writes the sentinel in whichever of the end
/// record's fields needs one and the real values in a zip64 end record, which
/// the locator immediately in front of the end record points at. Taking one
/// field narrow beside another wide is how a reader walks off the end of a
/// directory it half believed.
///
/// An end record that says it is zip64 and has no zip64 record behind it is
/// refused: the sentinel is not a value, and reading it as one would put the
/// walk at offset `0xFFFFFFFF` of an archive that never claimed to be that
/// large.
fn span(archive: &[u8], end: usize) -> Result<(u64, u64), Fault> {
    let narrow = || malformed("its end record is truncated");
    let count = u16_at(archive, end + 10).ok_or_else(narrow)?;
    let begins = u32_at(archive, end + 16).ok_or_else(narrow)?;
    if count != WIDE_U16 && begins != WIDE_U32 {
        return Ok((u64::from(count), u64::from(begins)));
    }
    let locator = end
        .checked_sub(ZIP64_LOCATOR_WIDTH)
        .filter(|start| u32_at(archive, *start) == Some(ZIP64_LOCATOR_SIGNATURE))
        .ok_or_else(|| {
            malformed("its end record says it is zip64 and no zip64 locator precedes it")
        })?;
    let record = usize::try_from(
        u64_at(archive, locator + 8).ok_or_else(|| malformed("its zip64 locator is truncated"))?,
    )
    .map_err(|_| malformed("its zip64 end record sits past the end of the archive"))?;
    if u32_at(archive, record) != Some(ZIP64_END_SIGNATURE) {
        return Err(malformed(
            "its zip64 locator does not point at a zip64 end record",
        ));
    }
    let wide = || malformed("its zip64 end record is truncated");
    Ok((
        u64_at(archive, record + 32).ok_or_else(wide)?,
        u64_at(archive, record + 48).ok_or_else(wide)?,
    ))
}

/// The zip64 extended-information field's payload inside one record's extra
/// area, or `None` where the record carries no such field.
///
/// The area is a run of `(id, width, payload)` headers in no fixed order, so it
/// is walked rather than indexed. A header whose width runs past the area ends
/// the walk — a field that does not fit is not a field, and the record is then
/// read from whatever its narrow fields say, which is either a real value or a
/// sentinel [`Wide::field`] refuses.
fn zip64_extra(area: &[u8]) -> Option<&[u8]> {
    let mut at = 0;
    while let (Some(id), Some(width)) = (u16_at(area, at), u16_at(area, at + 2)) {
        let payload = area.get(at + 4..at + 4 + usize::from(width))?;
        if id == ZIP64_EXTRA_ID {
            return Some(payload);
        }
        at += 4 + usize::from(width);
    }
    None
}

/// The wide values a record's `0xFFFFFFFF` fields stand in for.
///
/// A cursor rather than a struct, because the payload holds only the fields
/// whose narrow half is the sentinel, in the order the format fixes them: the
/// uncompressed size, the compressed size, the local header's offset, then the
/// disk the entry starts on. So a caller asks for them in that order, wanted or
/// not, and one asked for out of order reads another field's octets.
struct Wide<'a> {
    /// The zip64 extra field's payload — empty where the record carries none.
    payload: &'a [u8],
    /// How much of it the fields already asked for have consumed.
    at: usize,
}

impl Wide<'_> {
    /// `narrow` where it is a value, and the next eight octets of the payload
    /// where it is the sentinel that says the value is here instead.
    fn field(&mut self, narrow: u32) -> Result<u64, Fault> {
        if narrow != WIDE_U32 {
            return Ok(u64::from(narrow));
        }
        let wide = u64_at(self.payload, self.at).ok_or_else(|| {
            malformed("a record's size or offset is a zip64 field its extra area does not carry")
        })?;
        self.at += 8;
        Ok(wide)
    }
}

/// Every entry the archive's central directory names, each already judged.
///
/// This is the only route to an entry in this module, which is what makes the
/// three refusals total: `entries` and `read` are both callers of it, so there
/// is no member through which a program reaches an entry that has not been
/// through [`refuse_hostile`].
pub(crate) fn directory(archive: &[u8]) -> Result<Vec<Entry>, Fault> {
    let end = end_record(archive)
        .ok_or_else(|| malformed("it has no end-of-central-directory record"))?;
    let (entries, begins) = span(archive, end)?;
    let count = usize::try_from(entries)
        .map_err(|_| malformed("its end record names more entries than this host can hold"))?;
    let mut at = usize::try_from(begins)
        .map_err(|_| malformed("its central directory begins past the end of the archive"))?;

    let mut out: Vec<Entry> = Vec::with_capacity(count.min(4096));
    // The names already walked, borrowed from the archive, so a repeated one
    // is found by one lookup rather than a scan of every entry before it — a
    // scan an archive of many small entries turns into a hang.
    let mut seen: HashSet<&str> = HashSet::with_capacity(count.min(4096));
    for _ in 0..count {
        if u32_at(archive, at) != Some(CENTRAL_SIGNATURE) {
            return Err(malformed(
                "a central-directory record is missing its signature",
            ));
        }
        let field = |offset: usize| u16_at(archive, at + offset);
        let method =
            field(10).ok_or_else(|| malformed("a central-directory record is truncated"))?;
        let crc = u32_at(archive, at + 16)
            .ok_or_else(|| malformed("a central-directory record is truncated"))?;
        let compressed = u32_at(archive, at + 20)
            .ok_or_else(|| malformed("a central-directory record is truncated"))?;
        let uncompressed = u32_at(archive, at + 24)
            .ok_or_else(|| malformed("a central-directory record is truncated"))?;
        let name_len = usize::from(
            field(28).ok_or_else(|| malformed("a central-directory record is truncated"))?,
        );
        let extra_len = usize::from(
            field(30).ok_or_else(|| malformed("a central-directory record is truncated"))?,
        );
        let comment_len = usize::from(
            field(32).ok_or_else(|| malformed("a central-directory record is truncated"))?,
        );
        let external = u32_at(archive, at + 38)
            .ok_or_else(|| malformed("a central-directory record is truncated"))?;
        let local = u32_at(archive, at + 42)
            .ok_or_else(|| malformed("a central-directory record is truncated"))?;
        let raw = archive
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| malformed("an entry's name runs past the end of the archive"))?;
        let name = std::str::from_utf8(raw)
            .map_err(|_| malformed("an entry's name is not UTF-8, and a `string` is"))?;
        let extra = archive
            .get(at + 46 + name_len..at + 46 + name_len + extra_len)
            .ok_or_else(|| malformed("an entry's extra field runs past the end of the archive"))?;
        // The uncompressed size is asked for and dropped rather than skipped:
        // it is this class's one unread field, and the two the class does read
        // sit behind it in the payload.
        let mut wide = Wide {
            payload: zip64_extra(extra).unwrap_or_default(),
            at: 0,
        };
        wide.field(uncompressed)?;
        let compressed = wide.field(compressed)?;
        let local = wide.field(local)?;

        refuse_hostile(name, external, &mut seen)?;
        out.push(Entry {
            name: name.to_owned(),
            method,
            crc,
            compressed: usize::try_from(compressed)
                .map_err(|_| malformed("an entry's compressed size does not fit this host"))?,
            at: usize::try_from(local).map_err(|_| {
                malformed("an entry's local header sits past the end of the archive")
            })?,
        });
        at = at + 46 + name_len + extra_len + comment_len;
    }
    Ok(out)
}

/// The four things an entry is refused for, applied to every entry before any
/// of them is visible.
///
/// The order is the order the refusals are cheapest in and carries no other
/// meaning: an entry that is two of these at once is refused for whichever is
/// tested first, and it is refused either way.
fn refuse_hostile<'a>(
    name: &'a str,
    external: u32,
    seen: &mut HashSet<&'a str>,
) -> Result<(), Fault> {
    if name.is_empty() {
        return Err(malformed("an entry has no name at all"));
    }
    // `Core\Path`'s grammar rather than a second one written here: a leading
    // separator, a drive and a UNC server are each absolute on every platform
    // in both directions, which is what makes this refusal the same on the
    // Windows leg and the WSL one.
    let parts = crate::path::parse(name);
    if parts.absolute || !matches!(parts.root, crate::path::Root::Unnamed) {
        return Err(refuses_path("it names an absolute path", name));
    }
    if parts.components.contains(&"..") {
        return Err(refuses_path(
            "it traverses out of the archive with a `..` component",
            name,
        ));
    }
    // The Unix mode an archive records in the high half of its external
    // attributes. Read without asking which host wrote the archive, because a
    // writer that means `S_IFLNK` and a writer that wrote noise there are
    // indistinguishable from here, and refusing is the safe direction.
    if (external >> 16) & MODE_KIND == MODE_SYMLINK {
        return Err(refuses_path(
            "it is a symlink, whatever its name says",
            name,
        ));
    }
    if !seen.insert(name) {
        return Err(refuses_entry(
            "the archive names it twice, and which of the two a reader answers is what the attack \
             turns on",
            name,
        ));
    }
    Ok(())
}

/// One entry's compressed octets, from its local header.
fn frame<'a>(archive: &'a [u8], entry: &Entry) -> Result<&'a [u8], Fault> {
    if u32_at(archive, entry.at) != Some(LOCAL_SIGNATURE) {
        return Err(malformed(
            "an entry's local header is missing its signature",
        ));
    }
    let name_len = usize::from(
        u16_at(archive, entry.at + 26)
            .ok_or_else(|| malformed("an entry's local header is truncated"))?,
    );
    let extra_len = usize::from(
        u16_at(archive, entry.at + 28)
            .ok_or_else(|| malformed("an entry's local header is truncated"))?,
    );
    let from = entry.at + 30 + name_len + extra_len;
    // `compressed` is the archive's own claim, up to `u64::MAX` through a
    // zip64 field, so the end is a checked sum: a wrapping one is a slice
    // somewhere else in the archive, and an unchecked one is a panic.
    from.checked_add(entry.compressed)
        .and_then(|to| archive.get(from..to))
        .ok_or_else(|| malformed("an entry's data runs past the end of the archive"))
}

/// One archive's share of the bound, spent as its entries are decompressed.
///
/// `rule:core-classes/decompression-bound` is one rule with two halves, and
/// this type is the second of them: an archive whose entries are each within
/// the bound and whose total is not is the same attack one level up. So what an
/// entry decompresses under is not the bound the call named but a [`Bound`]
/// whose `bytes` is what the archive has left, which keeps the comparison
/// [`Bound::output_ceiling`]'s and leaves no second piece of arithmetic to hold
/// in step with the first.
///
/// Only the octet ceiling is spent. The ratio is a property of the entry being
/// read rather than of the archive, and an archive of many small well-compressed
/// entries is not the attack — the octets it lands on the machine are.
///
/// Every decompression this module does goes through [`Budget::read`], so there
/// is no route by which an entry is read without being charged for.
pub(crate) struct Budget {
    /// The bound the call runs under: what it asked for, lowered to the
    /// operator's `[limits]`.
    bound: Bound,
    /// What is left of `bound.bytes` once the entries already read are paid
    /// for.
    left: u64,
}

impl Budget {
    /// The whole of `bound`, with nothing of it spent.
    pub(crate) fn new(bound: Bound) -> Self {
        Self {
            bound,
            left: bound.bytes,
        }
    }

    /// One entry's octets, decompressed under what the archive has left and
    /// charged against it.
    ///
    /// The bound is applied **while** the output grows rather than to the size
    /// the entry's own header claims, which is the difference between refusing
    /// a bomb and surviving one: the decoder is driven as a `Read` and stopped
    /// one octet past the ceiling, so a hostile entry costs the ceiling and
    /// never the gigabyte it declared.
    ///
    /// What came out is then checked against the CRC-32 the archive recorded
    /// for it, and a mismatch refuses rather than answers. That check is here
    /// rather than in each member because this is the one route: `read`
    /// answers what it returns and `extract` writes it, so a corrupt entry has
    /// no spelling that reaches either.
    pub(crate) fn read(&mut self, archive: &[u8], entry: &Entry) -> Result<Vec<u8>, Fault> {
        let frame = frame(archive, entry)?;
        let ceiling = Bound {
            bytes: self.left,
            ..self.bound
        }
        .output_ceiling(frame.len());
        let out = match entry.method {
            STORED => frame.to_vec(),
            DEFLATE => {
                let mut out = Vec::new();
                flate2::read::DeflateDecoder::new(frame)
                    .take(ceiling.saturating_add(1))
                    .read_to_end(&mut out)
                    .map_err(|_| malformed_entry(&entry.name))?;
                out
            }
            other => {
                return Err(refuses_entry(
                    &format!(
                        "it is compressed by method {other}, and this class reads stored and \
                         deflate"
                    ),
                    &entry.name,
                ));
            }
        };
        if out.len() as u64 > ceiling {
            return Err(over_bound(&entry.name, frame.len(), self, ceiling));
        }
        let mut sum = crc32fast::Hasher::new();
        sum.update(&out);
        let sum = sum.finalize();
        if sum != entry.crc {
            return Err(refuses_entry(
                &format!(
                    "its octets do not match the CRC-32 the archive recorded for them \
                     ({sum:#010x} against {:#010x}), so what is in the archive is not what was \
                     put there",
                    entry.crc
                ),
                &entry.name,
            ));
        }
        self.left = self.left.saturating_sub(out.len() as u64);
        Ok(out)
    }
}

/// The path one entry is written to, with every directory above it created and
/// proved to be inside `root` — or `None` for a directory entry, which is
/// created and has nothing to write.
///
/// **One level at a time, resolved after each one.** A directory is created
/// only inside a parent whose own resolution has already been compared against
/// `root`, so a link planted while the extraction is running is caught by the
/// resolution of the level it sits at, before anything is created beneath it.
/// Reading the name first and creating the whole chain afterwards is the order
/// that loses this race.
///
/// The components are [`crate::path::parse`]'s, which is the grammar
/// [`refuse_hostile`] judged the name under; splitting the name a second way
/// here is how the two end up disagreeing about what a component is.
fn place(ctx: &Ctx, root: &Path, entry: &Entry) -> Result<Option<PathBuf>, Fault> {
    let is_dir = entry.name.ends_with('/');
    let components = crate::path::parse(&entry.name).components;
    let mut at = root.to_path_buf();
    for (index, part) in components.iter().enumerate() {
        if index + 1 == components.len() && !is_dir {
            return Ok(Some(at.join(part)));
        }
        at.push(part);
        nvs_runtime::capability::create_dir(ctx, &at, EXTRACT)?;
        at = nvs_runtime::capability::canonicalize(ctx, &at, EXTRACT)?;
        if !at.starts_with(root) {
            return Err(escapes(&entry.name, &at));
        }
    }
    Ok(None)
}

// ============================================================================
// The refusals
// ============================================================================

/// The refusal for a directory under the destination that resolved to
/// somewhere else.
///
/// A `RuntimeError` rather than a `ParseError`, and the split is the honest
/// one: nothing about the archive is wrong here. What changed is the disk — a
/// link appeared under the destination while the extraction was running — so
/// the caller's question is about its own filesystem rather than about the
/// octets it was handed.
fn escapes(name: &str, resolved: &Path) -> Fault {
    // no case can reach this: a conformance case cannot plant a link under the
    // destination between two entries of one call, and creating one at all
    // needs a privilege the Windows leg may not have. The `#[test]`
    // `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it`
    // asserts it instead, on the hosts that allow the link.
    Fault::thrown(format!(
        "Core\\Zip::extract(): a directory this archive asked for resolved outside the \
         destination, and the extraction stopped there: \"{name}\" resolves to \"{}\" \
         (rule:security/path-scope-canonicalise-then-prefix). Each level is resolved and compared \
         before anything is created inside it, so nothing of this entry reached the disk.",
        resolved.display()
    ))
}

/// The refusal for octets that are not a zip archive, or not one this reader
/// can walk.
///
/// A `ParseError` for `Core\Compress`'s reason — input did not match a format
/// this code declared — and specifically **not** an `IOError`, so a caller
/// cannot confuse a hostile input with a failing disk.
fn malformed(why: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Parse,
        format!("Core\\Zip: not a well-formed zip archive: {why}."),
    )
}

/// The refusal for an entry whose own stream is not what its method says.
///
/// The decoder's own words for *why* are deliberately not in it: they are the
/// compression library's, they change with a version of it, and a caller that
/// could act on them would be acting on a string this language does not
/// promise. What it can act on is that the entry is not readable, which is the
/// sentence.
fn malformed_entry(name: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Parse,
        format!(
            "Core\\Zip cannot read an entry: its deflate stream is not well formed (\"{name}\")."
        ),
    )
}

/// The refusal for an entry that asks to leave the archive — the three Stage 4
/// names as one sentence, because they are one decision.
///
/// The message opens on its own words rather than on the entry, so the site is
/// one `tests/conformance_coverage.rs` can find a case for, and every caller
/// distinguishes the three by reading the sentence rather than by catching a
/// different class.
fn refuses_path(what: &str, name: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Parse,
        format!(
            "Core\\Zip refuses an entry when the archive is read: {what}: \"{name}\" \
             (rule:security/path-scope-canonicalise-then-prefix). The refusal is the archive's \
             rather than the extraction's, so a program that walks the entries itself has nothing \
             left it could have checked."
        ),
    )
}

/// The refusal for an entry this class will not read for a reason that is not
/// about its name.
fn refuses_entry(what: &str, name: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Parse,
        format!("Core\\Zip will not read an entry: {what}: \"{name}\"."),
    )
}

/// The refusal for an entry whose output passes the bound.
///
/// It names what stopped it — the ratio, the ceiling, or what the archive had
/// left of the ceiling — for [`crate::compress`]'s reason: the first question a
/// caller asks is whether to lower its own argument or ask the operator about
/// `[limits]`, and only one of those two is ever the answer. An archive that
/// ran out part way says so rather than naming the remainder alone, which would
/// read as a ceiling nobody configured.
fn over_bound(name: &str, input: usize, budget: &Budget, ceiling: u64) -> Fault {
    let half = if ceiling < budget.left {
        format!(
            "the {}:1 ratio over {input} octets of input",
            budget.bound.ratio
        )
    } else if budget.left == budget.bound.bytes {
        format!("the {} octet ceiling", budget.bound.bytes)
    } else {
        format!(
            "the {} octets this archive had left of its {} octet ceiling",
            budget.left, budget.bound.bytes
        )
    };
    Fault::thrown_as(
        ThrownClass::Parse,
        format!(
            "Core\\Zip: an entry decompressing past {half} is refused rather than truncated \
             (rule:core-classes/decompression-bound): \"{name}\". Lower `$maxBytes` or `$maxRatio` \
             to read less; neither can be raised past `[limits] max_decompressed` and \
             `[limits] max_decompression_ratio`."
        ),
    )
}

// ============================================================================
// Reading arguments
// ============================================================================

/// The archive in slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot carrying anything but `bytes`.
fn archive_of<'a>(args: &'a [Value], member: &str) -> Result<&'a [u8], Fault> {
    // unreachable from source: the parameter is `CoreTy::Blob`, so anything
    // that is not `bytes` is `E0401` at the call site.
    args[0].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Zip expected a `bytes` archive, got tag {} at {member}",
            args[0].tag_byte()
        ))
    })
}

/// The `uint` in slot `at`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot carrying anything but a non-negative integer.
fn uint_of(args: &[Value], at: usize, position: &str) -> Result<u64, Fault> {
    // unreachable from source: the parameter is `CoreTy::Uint`, so an argument
    // that is not a non-negative integer is `E0401` at the call site.
    args[at].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Zip expected a non-negative `uint` for {position}, got tag {}",
            args[at].tag_byte()
        ))
    })
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Zip::entries(bytes $archive): array<tainted string>` — replacing
    /// `zip_read`/`zip_entry_name` and `ZipArchive::statIndex`.
    ///
    /// The listing is the whole archive's or nothing: [`directory`] refuses the
    /// archive rather than skipping the entry, so a program cannot receive a
    /// short list it has no way to know was short.
    fn nvs_core_zip_entries(_ctx, args: [1]) {
        let archive = archive_of(args, "entries")?;
        let mut out = NvsArray::new();
        for entry in directory(archive)? {
            out.append(Value::str(NvsStr::new(entry.name.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Zip::read(bytes $archive, string $name, uint $maxBytes = 67108864,
    /// uint $maxRatio = 1000): tainted bytes` — replacing `zip_entry_read` and
    /// `ZipArchive::getFromName`.
    ///
    /// The two bound arguments are **asks**, exactly as
    /// `Core\Compress::decompress`'s are: what the read runs under is
    /// [`Bound::within`], the smaller of the ask and what the operator
    /// configured, on each axis independently.
    fn nvs_core_zip_read(ctx, args: [4]) {
        let archive = archive_of(args, "read")?;
        // unreachable from source: the parameter is `CoreTy::Text`, so anything
        // that is not a `string` is `E0401` at the call site.
        let Some(name) = args[1].as_str_bytes() else {
            return Err(Fault::fatal(format!(
                "Core\\Zip expected a `string` entry name, got tag {}",
                args[1].tag_byte()
            )));
        };
        let asked = Bound {
            bytes: uint_of(args, 2, "`$maxBytes`")?,
            ratio: uint_of(args, 3, "`$maxRatio`")?,
        };
        let bound = Bound::ceiling(ctx).within(asked);

        let entries = directory(archive)?;
        let wanted = String::from_utf8_lossy(name).into_owned();
        let Some(entry) = entries.iter().find(|entry| entry.name == wanted) else {
            return Err(Fault::thrown_as(
                ThrownClass::Parse,
                format!(
                    "Core\\Zip::read(): the archive has no entry named \"{wanted}\". \
                     `Core\\Zip::entries()` is the list, and a name is compared whole rather than \
                     resolved as a path."
                ),
            ));
        };
        Ok(Value::bytes(NvsStr::new(
            &Budget::new(bound).read(archive, entry)?,
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Zip::extract(bytes $archive, string $destination,
    /// uint $maxBytes = 67108864, uint $maxRatio = 1000): uint` — replacing
    /// `ZipArchive::extractTo`.
    ///
    /// The order is the whole member: the archive is judged first, so a hostile
    /// entry refuses before any file exists; the destination is created and
    /// resolved once; and then each entry is placed by [`place`], which
    /// resolves every level before it creates anything inside it. The bound is
    /// one [`Budget`] for the whole call, so what a later entry decompresses
    /// under is what the archive has left.
    fn nvs_core_zip_extract(ctx, args: [4]) {
        let archive = archive_of(args, "extract")?;
        // unreachable from source: the parameter is `CoreTy::Text`, so anything
        // that is not a `string` is `E0401` at the call site.
        let Some(destination) = args[1].as_str_bytes() else {
            return Err(Fault::fatal(format!(
                "Core\\Zip expected a `string` destination, got tag {}",
                args[1].tag_byte()
            )));
        };
        let asked = Bound {
            bytes: uint_of(args, 2, "`$maxBytes`")?,
            ratio: uint_of(args, 3, "`$maxRatio`")?,
        };
        let mut budget = Budget::new(Bound::ceiling(ctx).within(asked));

        // Before the destination is even created: an archive that is refused
        // leaves nothing behind, which is what makes "the refusal is the
        // archive's rather than the extraction's" true of this member too.
        let entries = directory(archive)?;

        let named = PathBuf::from(String::from_utf8_lossy(destination).into_owned());
        nvs_runtime::capability::create_dir(ctx, &named, EXTRACT)?;
        let root = nvs_runtime::capability::canonicalize(ctx, &named, EXTRACT)?;

        let mut written = 0_u64;
        for entry in &entries {
            let Some(target) = place(ctx, &root, entry)? else {
                continue;
            };
            let octets = budget.read(archive, entry)?;
            // Exclusively: a name already taken refuses rather than being
            // followed, and what is already there may be a link.
            let mut file = nvs_runtime::capability::create(ctx, &target, false, EXTRACT)?;
            file.write_all(&octets)
                .map_err(|err| nvs_runtime::capability::io_failure(EXTRACT, &target, &err))?;
            written += 1;
        }
        Ok(Value::uint(written))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bound roomy enough not to be the subject of a test that is about
    /// something else.
    const ROOMY: Bound = Bound {
        bytes: 1 << 20,
        ratio: 1 << 20,
    };

    /// One entry, as an archive builder takes it.
    struct Written<'a> {
        /// The entry's name, exactly as it goes on the wire.
        name: &'a str,
        /// The entry's octets, stored rather than deflated.
        data: Vec<u8>,
        /// The high half of the external attributes — the Unix mode, where the
        /// symlink bit lives.
        mode: u32,
    }

    impl<'a> Written<'a> {
        /// An ordinary stored entry.
        fn plain(name: &'a str, data: &str) -> Self {
            Self {
                name,
                data: data.as_bytes().to_vec(),
                mode: 0o100_644,
            }
        }

        /// The same entry with `S_IFLNK` set, which is how an archive says an
        /// entry is a symlink and its data is the target.
        fn symlink(name: &'a str, target: &str) -> Self {
            Self {
                name,
                data: target.as_bytes().to_vec(),
                mode: 0o120_777,
            }
        }
    }

    /// The CRC-32 an archive records for an entry's octets. A builder here has
    /// to get it right or every archive it writes is a corrupt one, which is
    /// the same fact `an_extracted_entry_whose_crc_mismatches_is_refused`
    /// asserts from the other side.
    fn crc_of(data: &[u8]) -> u32 {
        let mut sum = crc32fast::Hasher::new();
        sum.update(data);
        sum.finalize()
    }

    /// A zip archive carrying `written`, stored rather than deflated.
    ///
    /// Written here rather than fetched from a fixture directory so that a
    /// hostile archive is *derived from the attack* rather than from a blob
    /// somebody once produced: the name and the mode are the test's own
    /// arguments, so a refusal cannot pass by matching a file this repository
    /// happens to hold.
    fn archive(written: &[Written<'_>]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut directory: Vec<u8> = Vec::new();
        let mut count = 0_u16;
        for entry in written {
            let at = u32::try_from(out.len()).expect("a test archive is small");
            let name = entry.name.as_bytes();
            let size = u32::try_from(entry.data.len()).expect("a test entry is small");
            let crc = crc_of(&entry.data);
            out.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(
                &u16::try_from(name.len())
                    .expect("a test entry's name is short")
                    .to_le_bytes(),
            );
            out.extend_from_slice(&0_u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(&entry.data);

            directory.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
            directory.extend_from_slice(&[20, 3, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend_from_slice(&crc.to_le_bytes());
            directory.extend_from_slice(&size.to_le_bytes());
            directory.extend_from_slice(&size.to_le_bytes());
            directory.extend_from_slice(
                &u16::try_from(name.len())
                    .expect("a test entry's name is short")
                    .to_le_bytes(),
            );
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&(entry.mode << 16).to_le_bytes());
            directory.extend_from_slice(&at.to_le_bytes());
            directory.extend_from_slice(name);
            count += 1;
        }

        let offset = u32::try_from(out.len()).expect("a test archive is small");
        let size = u32::try_from(directory.len()).expect("a test directory is small");
        out.extend_from_slice(&directory);
        out.extend_from_slice(&END_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out
    }

    /// A refusal's message, asserting on the way past that it is a
    /// `ParseError` — the goal's "a refusal is a diagnostic and never a failed
    /// I/O error", which every case below gets for free by reading its message
    /// through here.
    fn refusal(fault: Fault) -> String {
        match fault {
            Fault::Thrown(ThrownClass::Parse, message) => message.to_string(),
            Fault::Thrown(class, message) => {
                panic!("a `Core\\Zip` refusal is a `ParseError`, not {class:?}: {message}")
            }
            _ => panic!("a `Core\\Zip` refusal is a throw a program can catch"),
        }
    }

    /// The refusal `directory` answers for `written`, or a panic where it
    /// accepted the archive.
    fn refused(written: &[Written<'_>]) -> String {
        let raw = archive(written);
        match directory(&raw) {
            Ok(entries) => panic!(
                "an archive of {:?} was accepted, listing {:?}",
                written.iter().map(|entry| entry.name).collect::<Vec<_>>(),
                entries.iter().map(|entry| &entry.name).collect::<Vec<_>>()
            ),
            Err(fault) => refusal(fault),
        }
    }

    /// The reader is what refuses, so an ordinary archive still reads — the
    /// control every refusal below is measured against.
    #[test]
    fn an_ordinary_archive_lists_and_reads_its_entries() {
        let raw = archive(&[
            Written::plain("notes/one.txt", "first"),
            Written::plain("notes/two.txt", "second"),
        ]);
        let entries = directory(&raw).expect("an ordinary archive reads");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["notes/one.txt", "notes/two.txt"]
        );
        assert_eq!(
            Budget::new(ROOMY)
                .read(&raw, &entries[1])
                .expect("a stored entry reads"),
            b"second"
        );
    }

    /// What `entries` returns for `written`, called the way a program calls it:
    /// the names in order, or `None` where the member threw.
    fn listed(ctx: &mut Ctx, written: &[Written<'_>]) -> Option<Vec<String>> {
        let raw = Value::bytes(NvsStr::new(&archive(written)));
        let names = nvs_runtime::call(nvs_core_zip_entries, ctx, &[raw])
            .ok()
            .map(|answer| {
                let names = crate::str::Elements::of(
                    answer.array_ptr().expect("`entries` returns an array"),
                )
                .map(|name| name.as_text().expect("a name is a string").to_owned())
                .collect();
                #[expect(
                    unsafe_code,
                    reason = "the case owns the one reference `entries` returned"
                )]
                unsafe {
                    answer.release();
                }
                names
            });
        #[expect(
            unsafe_code,
            reason = "the case owns the archive's one reference, and the member only borrowed it"
        )]
        unsafe {
            raw.release();
        }
        names
    }

    /// The member answers the central directory's order, which is not sorted
    /// order, lists as many entries as a narrow end record can count without a
    /// scan per entry, and answers nothing at all for an archive with one
    /// hostile entry in it — never the entries around it.
    // covers: Core\Zip::entries
    #[test]
    fn the_member_lists_the_directory_in_order_or_throws_for_the_whole_archive() {
        let mut ctx = Ctx::buffered();
        assert_eq!(
            listed(
                &mut ctx,
                &[
                    Written::plain("zeta.txt", "z"),
                    Written::plain("alpha/", ""),
                    Written::plain("alpha/beta.txt", "b"),
                ]
            ),
            Some(vec![
                "zeta.txt".to_owned(),
                "alpha/".to_owned(),
                "alpha/beta.txt".to_owned()
            ])
        );

        // Every name distinct and 65534 of them, the most a narrow end record
        // counts — `0xFFFF` there says the count is in a zip64 record. A
        // repeated-name check that scanned the entries before each one would
        // make this case the slow one in the binary.
        let names: Vec<String> = (0..WIDE_U16 - 1).map(|at| format!("{at:04x}")).collect();
        let many: Vec<Written<'_>> = names.iter().map(|name| Written::plain(name, "")).collect();
        let answer = listed(&mut ctx, &many).expect("distinct names list");
        assert_eq!(answer.len(), usize::from(WIDE_U16 - 1));
        assert_eq!(answer.last().map(String::as_str), Some("fffd"));

        for hostile in [
            Written::plain("../escape", "x"),
            Written::symlink("inside.txt", "/etc/passwd"),
            Written::plain("zeta.txt", "again"),
        ] {
            let written = [Written::plain("zeta.txt", "z"), hostile];
            assert_eq!(listed(&mut ctx, &written), None);
            assert!(
                ctx.take_pending().is_some(),
                "the refusal is a throw a program can catch"
            );
        }
    }

    /// What `read` returns for the entry `name` of `raw` under the bound
    /// `(bytes, ratio)`, called the way a program calls it: the octets, or the
    /// message of what it threw.
    fn read_from(
        ctx: &mut Ctx,
        raw: &[u8],
        name: &str,
        bytes: u64,
        ratio: u64,
    ) -> Result<Vec<u8>, String> {
        let args = [
            Value::bytes(NvsStr::new(raw)),
            Value::str(NvsStr::new(name.as_bytes())),
            Value::uint(bytes),
            Value::uint(ratio),
        ];
        let answer = match nvs_runtime::call(nvs_core_zip_read, ctx, &args) {
            Ok(answer) => {
                let octets = answer.as_bytes().expect("`read` returns bytes").to_vec();
                #[expect(
                    unsafe_code,
                    reason = "the case owns the one reference `read` returned"
                )]
                unsafe {
                    answer.release();
                }
                Ok(octets)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("the refusal is a throw a program can catch")
                .into_owned()),
        };
        for arg in args {
            #[expect(
                unsafe_code,
                reason = "the case owns each argument's one reference, and the member only borrowed it"
            )]
            unsafe {
                arg.release();
            }
        }
        answer
    }

    /// The member reads one entry whole, and each half of the bound holds on
    /// both sides of its boundary: the last accepted value reads, the first
    /// refused one throws, and `0` is a bound of zero. A name is compared whole
    /// and never resolved as a path, and an entry whose zip64 size would carry
    /// its end past the address space is refused rather than summed.
    // covers: Core\Zip::read
    #[test]
    fn the_member_reads_one_entry_under_a_bound_held_on_both_sides() {
        let mut ctx = Ctx::buffered();
        let raw = archive(&[
            Written::plain("notes/one.txt", "first"),
            Written::plain("empty.txt", ""),
        ]);
        let big = u64::MAX;
        assert_eq!(
            read_from(&mut ctx, &raw, "notes/one.txt", big, big).as_deref(),
            Ok(&b"first"[..])
        );

        // The octet half: five octets read under a bound of five, and throw
        // under four.
        assert_eq!(
            read_from(&mut ctx, &raw, "notes/one.txt", 5, big).as_deref(),
            Ok(&b"first"[..])
        );
        let short = read_from(&mut ctx, &raw, "notes/one.txt", 4, big)
            .expect_err("four octets is one short of the entry");
        assert!(short.contains("\"notes/one.txt\""), "{short}");

        // Zero is a bound of zero: an empty entry fits it and nothing else does.
        assert_eq!(
            read_from(&mut ctx, &raw, "empty.txt", 0, big).as_deref(),
            Ok(&b""[..])
        );
        assert!(read_from(&mut ctx, &raw, "notes/one.txt", 0, big).is_err());

        // The ratio half: a stored entry is one octet out per octet in.
        assert!(read_from(&mut ctx, &raw, "notes/one.txt", big, 1).is_ok());
        assert!(read_from(&mut ctx, &raw, "notes/one.txt", big, 0).is_err());

        for missing in ["missing.txt", "./notes/one.txt", "notes\\one.txt", "notes"] {
            let message = read_from(&mut ctx, &raw, missing, big, big)
                .expect_err("a name the archive does not hold throws");
            assert!(message.contains("no entry named"), "{missing}: {message}");
        }

        let entries = directory(&raw).expect("an ordinary archive reads");
        let past = Entry {
            compressed: usize::MAX - 8,
            ..entries[0].clone()
        };
        assert!(
            refusal(frame(&raw, &past).expect_err("the end would wrap"))
                .contains("runs past the end")
        );
    }

    /// The same entries written the way a zip64 writer writes them: every size
    /// and every local-header offset is the `0xFFFFFFFF` sentinel with the real
    /// value in a zip64 extra field, and the end record hands its entry count
    /// and its directory's offset to a zip64 end record a locator points at.
    ///
    /// Written whole rather than by patching [`archive`]'s output, because what
    /// is asserted is that *every* one of those values comes out of the wide
    /// record: a patch that missed one would leave the narrow field it forgot
    /// readable, and the case would pass on the field it never widened.
    fn zip64(written: &[Written<'_>]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut directory: Vec<u8> = Vec::new();
        let mut count = 0_u64;
        for entry in written {
            let at = u64::try_from(out.len()).expect("a test archive is small");
            let name = entry.name.as_bytes();
            let size = u64::try_from(entry.data.len()).expect("a test entry is small");
            let crc = crc_of(&entry.data);
            let name_len = u16::try_from(name.len()).expect("a test entry's name is short");

            // The local header, carrying a zip64 extra field of its own. The
            // sizes here are never read — the central directory's are — so what
            // this pins is that an entry's data is found past its extra area
            // rather than at a fixed offset from its name.
            out.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
            out.extend_from_slice(&[45, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&WIDE_U32.to_le_bytes());
            out.extend_from_slice(&WIDE_U32.to_le_bytes());
            out.extend_from_slice(&name_len.to_le_bytes());
            out.extend_from_slice(&20_u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(&ZIP64_EXTRA_ID.to_le_bytes());
            out.extend_from_slice(&16_u16.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&entry.data);

            // And the central record, whose extra field carries all three of
            // the values its narrow fields sentinel out, in the format's order.
            directory.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
            directory.extend_from_slice(&[45, 3, 45, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend_from_slice(&crc.to_le_bytes());
            directory.extend_from_slice(&WIDE_U32.to_le_bytes());
            directory.extend_from_slice(&WIDE_U32.to_le_bytes());
            directory.extend_from_slice(&name_len.to_le_bytes());
            directory.extend_from_slice(&28_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&(entry.mode << 16).to_le_bytes());
            directory.extend_from_slice(&WIDE_U32.to_le_bytes());
            directory.extend_from_slice(name);
            directory.extend_from_slice(&ZIP64_EXTRA_ID.to_le_bytes());
            directory.extend_from_slice(&24_u16.to_le_bytes());
            directory.extend_from_slice(&size.to_le_bytes());
            directory.extend_from_slice(&size.to_le_bytes());
            directory.extend_from_slice(&at.to_le_bytes());
            count += 1;
        }

        let offset = u64::try_from(out.len()).expect("a test archive is small");
        let size = u64::try_from(directory.len()).expect("a test directory is small");
        out.extend_from_slice(&directory);

        // The zip64 end record, which is where the count and the offset the end
        // record below sentinels out actually are. Its declared width is what
        // follows the width field itself.
        let record = u64::try_from(out.len()).expect("a test archive is small");
        out.extend_from_slice(&ZIP64_END_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&44_u64.to_le_bytes());
        out.extend_from_slice(&45_u16.to_le_bytes());
        out.extend_from_slice(&45_u16.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());

        // The locator, which is the only reason the record above can be found.
        out.extend_from_slice(&ZIP64_LOCATOR_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&record.to_le_bytes());
        out.extend_from_slice(&1_u32.to_le_bytes());

        out.extend_from_slice(&END_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&WIDE_U16.to_le_bytes());
        out.extend_from_slice(&WIDE_U16.to_le_bytes());
        out.extend_from_slice(&WIDE_U32.to_le_bytes());
        out.extend_from_slice(&WIDE_U32.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out
    }

    /// An archive written the zip64 way reads: each entry's sizes and its local
    /// header's offset come out of its own extra field, and the entry count and
    /// the directory's offset out of the zip64 end record.
    ///
    /// Two entries, so the second's offset is a value the narrow field could
    /// not have carried by accident — a reader taking the sentinel for an
    /// offset lands nowhere, but one taking a zeroed field for one would find
    /// the *first* entry's header and answer the wrong octets while looking
    /// right. The refusals and the bound are the same reader's, so a hostile
    /// entry is refused here too, and an archive that says it is zip64 with no
    /// zip64 record behind it is refused rather than read from the sentinels.
    #[test]
    fn a_zip64_archive_is_read_through_its_extra_fields_and_end_record() {
        let raw = zip64(&[
            Written::plain("notes/one.txt", "first"),
            Written::plain("notes/two.txt", "second"),
        ]);
        let entries = directory(&raw).expect("a zip64 archive reads");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["notes/one.txt", "notes/two.txt"]
        );
        assert_eq!(
            Budget::new(ROOMY)
                .read(&raw, &entries[0])
                .expect("the first entry reads"),
            b"first"
        );
        assert_eq!(
            Budget::new(ROOMY)
                .read(&raw, &entries[1])
                .expect("the second entry reads"),
            b"second"
        );

        let hostile = zip64(&[Written::plain("../escape", "x")]);
        let message = refusal(directory(&hostile).expect_err("a traversing entry is refused"));
        assert!(
            message.contains("traverses out of the archive"),
            "{message}"
        );

        let mut lying = raw.clone();
        let locator = lying.len() - END_WIDTH - ZIP64_LOCATOR_WIDTH;
        lying[locator..locator + 4].copy_from_slice(&0_u32.to_le_bytes());
        let message = refusal(
            directory(&lying).expect_err("a sentinel with no zip64 record behind it is refused"),
        );
        assert!(
            message.contains("no zip64 locator precedes it"),
            "{message}"
        );
    }

    /// An entry whose octets do not match the CRC-32 the archive recorded is
    /// refused rather than answered — and so never written, since `extract`
    /// reads through the same [`Budget::read`] this asserts on.
    ///
    /// The corruption is a flipped octet in a *stored* entry, which is the half
    /// a checksum is the only catch for: a deflate stream that has been
    /// corrupted already fails to decode, so a reader with no checksum at all
    /// still looks right on one. The archive lists either way, because its
    /// central directory is untouched.
    #[test]
    fn an_extracted_entry_whose_crc_mismatches_is_refused() {
        let sound = archive(&[Written::plain("notes/one.txt", "first")]);
        let at = sound
            .windows(5)
            .position(|window| window == b"first")
            .expect("the stored entry's octets are in the archive");
        let mut corrupt = sound.clone();
        corrupt[at] = b'F';

        let entries = directory(&corrupt).expect("the central directory is intact, so it lists");
        let message = refusal(
            Budget::new(ROOMY)
                .read(&corrupt, &entries[0])
                .expect_err("a corrupt entry is refused"),
        );
        assert!(message.contains("CRC-32"), "{message}");
        assert!(message.contains("notes/one.txt"), "{message}");

        // The same entry sound reads, so what is asserted is the mismatch and
        // not a reader that refuses whatever it is given.
        let entries = directory(&sound).expect("the archive reads");
        assert_eq!(
            Budget::new(ROOMY)
                .read(&sound, &entries[0])
                .expect("a sound entry reads"),
            b"first"
        );
    }

    /// Stage 4's first refusal: a `..` entry is refused when the archive is
    /// read, and the refusal names the rule.
    ///
    /// The archive is otherwise ordinary and the traversing entry is second, so
    /// a reader that judged only the first entry — or that judged at extraction
    /// time and let a listing through — fails here rather than passing on the
    /// happy path.
    #[test]
    fn a_traversing_entry_is_refused_at_read_time_by_the_diagnostic_that_names_the_rule() {
        let message = refused(&[
            Written::plain("notes/one.txt", "first"),
            Written::plain("../../etc/passwd", "root:x:0:0"),
        ]);
        assert!(
            message.contains("it traverses out of the archive with a `..` component"),
            "{message}"
        );
        assert!(
            message.contains("rule:security/path-scope-canonicalise-then-prefix"),
            "the refusal names the rule: {message}"
        );

        // A `..` anywhere in the name, not only at its front, and under either
        // separator — `Core\Path`'s grammar is one grammar on every platform,
        // which is what keeps this refusal the same on the Windows leg and the
        // WSL one.
        for name in ["notes/../../etc/passwd", r"notes\..\..\etc\passwd"] {
            let message = refused(&[Written {
                name,
                data: b"root:x:0:0".to_vec(),
                mode: 0o100_644,
            }]);
            assert!(message.contains("`..` component"), "{name}: {message}");
        }
    }

    /// Stage 4's second refusal, and the same way: an entry that names a root
    /// never becomes a path at all.
    ///
    /// A drive letter counts, on Linux as much as on Windows: an archive is
    /// read the same way everywhere, and an archive written to attack a Windows
    /// host is not something a Linux reader should hand over intact.
    #[test]
    fn an_absolute_path_entry_is_refused_the_same_way() {
        for name in [
            "/etc/passwd",
            r"\windows\system32\drivers\etc\hosts",
            r"C:\hosts",
        ] {
            let message = refused(&[
                Written::plain("notes/one.txt", "first"),
                Written {
                    name,
                    data: b"pwned".to_vec(),
                    mode: 0o100_644,
                },
            ]);
            assert!(
                message.contains("it names an absolute path"),
                "{name}: {message}"
            );
            assert!(
                message.contains("rule:security/path-scope-canonicalise-then-prefix"),
                "{name}: {message}"
            );
        }
    }

    /// Stage 4's third refusal, and the same way: a symlink entry is refused
    /// for what it *is*, whatever its name looks like.
    ///
    /// The name here is ordinary and would pass both path tests, so this can
    /// only pass by reading the mode — which is the point: a symlink entry
    /// whose target is `/etc/passwd` turns every later write through it into a
    /// write outside the destination, and the name gives no sign of it.
    #[test]
    fn a_symlink_entry_is_refused_the_same_way() {
        let message = refused(&[
            Written::plain("notes/one.txt", "first"),
            Written::symlink("notes/two.txt", "/etc/passwd"),
        ]);
        assert!(message.contains("it is a symlink"), "{message}");
        assert!(
            message.contains("rule:security/path-scope-canonicalise-then-prefix"),
            "{message}"
        );

        // The same entry without the symlink bit is ordinary, which is what
        // makes the assertion above about the mode rather than about the name.
        let raw = archive(&[Written::plain("notes/two.txt", "/etc/passwd")]);
        assert_eq!(
            directory(&raw).expect("an ordinary entry reads")[0].name,
            "notes/two.txt"
        );
    }

    /// An archive naming one entry twice is refused, because a reader that
    /// answers one of the two and a writer that wrote the other is the whole
    /// attack.
    #[test]
    fn an_archive_naming_the_same_entry_twice_is_refused() {
        let message = refused(&[
            Written::plain("notes/one.txt", "harmless"),
            Written::plain("notes/one.txt", "not"),
        ]);
        assert!(message.contains("the archive names it twice"), "{message}");
    }

    /// The bomb, both halves of the one rule: `Core\Compress`'s bound charged
    /// per entry, and the same bound charged across the whole archive.
    ///
    /// Each half is asserted on both sides of its boundary, so a reader that
    /// stops one octet early — or one that applies the bound to the size a
    /// header claimed rather than to what came out, or that gives every entry
    /// the whole ceiling again — fails rather than printing plausibly.
    #[test]
    fn a_decompression_bomb_is_refused_by_stage_twos_bound_per_entry_and_across_the_archive() {
        // Per entry, on both sides of the boundary: one octet past the ceiling
        // is refused and the ceiling itself reads, so a reader that stops one
        // entry early fails here rather than looking right.
        let raw = archive(&[Written::plain("big.txt", &"A".repeat(4096))]);
        let entries = directory(&raw).expect("the archive reads");

        let refused = Budget::new(Bound {
            bytes: 4095,
            ratio: 1 << 20,
        })
        .read(&raw, &entries[0])
        .expect_err("an entry past the ceiling is refused");
        let message = refusal(refused);
        assert!(message.contains("the 4095 octet ceiling"), "{message}");
        assert!(
            message.contains("rule:core-classes/decompression-bound"),
            "{message}"
        );

        assert_eq!(
            Budget::new(Bound {
                bytes: 4096,
                ratio: 1 << 20
            })
            .read(&raw, &entries[0])
            .expect("an entry at the ceiling reads")
            .len(),
            4096
        );

        // And across the archive: two entries each within a 6000 octet ceiling
        // whose total is not, which is the same attack one level up.
        let raw = archive(&[
            Written::plain("one", &"A".repeat(4096)),
            Written::plain("two", &"B".repeat(4096)),
        ]);
        let entries = directory(&raw).expect("the archive reads");
        let mut budget = Budget::new(Bound {
            bytes: 6000,
            ratio: 1 << 20,
        });
        assert_eq!(
            budget
                .read(&raw, &entries[0])
                .expect("the first entry is within the ceiling")
                .len(),
            4096
        );
        let message = refusal(
            budget
                .read(&raw, &entries[1])
                .expect_err("the archive's total passes the ceiling"),
        );
        assert!(
            message.contains("this archive had left of its 6000 octet ceiling"),
            "{message}"
        );
        assert!(
            message.contains("rule:core-classes/decompression-bound"),
            "{message}"
        );

        // What is charged is the octets read and not the entry count: the same
        // two entries under a ceiling their total fits both read.
        let mut roomy = Budget::new(Bound {
            bytes: 8192,
            ratio: 1 << 20,
        });
        assert_eq!(
            roomy
                .read(&raw, &entries[0])
                .expect("the first reads")
                .len(),
            4096
        );
        assert_eq!(
            roomy
                .read(&raw, &entries[1])
                .expect("the second reads")
                .len(),
            4096
        );
    }

    /// Every refusal this module writes is a `ParseError`, so a caller never
    /// has to tell a hostile archive from a full disk by reading a message.
    ///
    /// A sweep with a count rather than four assertions: a refusal added to
    /// [`refuse_hostile`] and thrown as something else fails here without
    /// anyone remembering to add a line.
    #[test]
    fn a_refusal_is_a_diagnostic_and_never_a_failed_io_error() {
        let hostile: [&[Written<'_>]; 5] = [
            &[Written::plain("../escape", "x")],
            &[Written::plain("/etc/passwd", "x")],
            &[Written::symlink("link", "/etc/passwd")],
            &[Written::plain("same", "x"), Written::plain("same", "y")],
            &[Written {
                name: "",
                data: b"x".to_vec(),
                mode: 0o100_644,
            }],
        ];
        let mut refusals = 0;
        for written in hostile {
            // `refused` panics on anything that is not a caught `ParseError`,
            // so reaching the increment is the assertion.
            assert!(refused(written).starts_with("Core\\Zip"));
            refusals += 1;
        }
        assert_eq!(refusals, hostile.len());

        // And octets that are not an archive at all take the same route: a
        // `ParseError` saying so, rather than an `IOError` a caller would
        // retry.
        let message = refusal(directory(b"not an archive").expect_err("refused"));
        assert!(
            message.contains("not a well-formed zip archive"),
            "{message}"
        );
    }

    /// A directory of this test's own under the host's temporary directory,
    /// removed first so a run that failed half way does not decide the next
    /// one's answer.
    fn temp_root(what: &str) -> PathBuf {
        let at = std::env::temp_dir().join(format!("nvs-zip-{what}-{}", std::process::id()));
        std::fs::remove_dir_all(&at).ok();
        at
    }

    /// A symlink at `link` pointing at the directory `target`, or `None` where
    /// the host will not make one.
    #[cfg(unix)]
    fn link_dir(target: &Path, link: &Path) -> Option<()> {
        std::os::unix::fs::symlink(target, link).ok()
    }

    /// See the `unix` twin. Windows needs `SeCreateSymbolicLinkPrivilege` or
    /// developer mode for this, and answers `None` without it.
    #[cfg(windows)]
    fn link_dir(target: &Path, link: &Path) -> Option<()> {
        std::os::windows::fs::symlink_dir(target, link).ok()
    }

    /// A context granting `fs.read` and `fs.write` under `root` and nothing
    /// else, with the roots canonicalized the way a real snapshot's are — a
    /// root still spelled the way this file typed it compares against the wrong
    /// thing.
    fn granting(root: &Path) -> Ctx {
        let spelled = root.to_string_lossy().into_owned();
        let mut caps = nvs_config::tree::Capabilities {
            fs: Some(nvs_config::tree::CapFs {
                read: Some(nvs_config::tree::Setting::List(vec![spelled.clone()])),
                write: Some(nvs_config::tree::Setting::List(vec![spelled])),
            }),
            ..nvs_config::tree::Capabilities::default()
        };
        caps.canonicalize(&nvs_config::resolve::Disk);
        let mut ctx = Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
            config: nvs_config::tree::Config {
                capabilities: Some(caps),
                ..nvs_config::tree::Config::default()
            },
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    /// The extraction's own half of
    /// `rule:security/path-scope-canonicalise-then-prefix`: a link that appears
    /// under the destination while the extraction is running is caught by the
    /// resolution of the level it sits at, and refused before anything is
    /// created inside it.
    ///
    /// **The grant is wider than the destination on purpose** — the whole
    /// temporary root, so the destination and the directory the link points at
    /// are both inside it. A grant narrowed to the destination refuses this at
    /// the door instead, which is the capability half of the same rule and is
    /// `crates/nvs-stdlib/tests/capability.rs`'s; what this asserts is the half
    /// that still has to hold once a program has been granted a tree.
    ///
    /// Creating a symlink on Windows needs a privilege the host may not have,
    /// and there is no fake to put in its place here: the door resolves through
    /// `nvs_config::resolve::Disk` and takes no resolver. So the case does the
    /// real thing where the platform allows it and says why it could not where
    /// it does not — the loop's WSL leg runs it either way.
    #[test]
    #[expect(
        clippy::print_stderr,
        reason = "a host that will not make a symlink has to say so, or the half that did not \
                  run is a silent pass"
    )]
    fn extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it() {
        let root = temp_root("escape");
        let dest = root.join("dest");
        let outside = root.join("outside");
        std::fs::create_dir_all(&dest).expect("the destination is this test's to create");
        std::fs::create_dir_all(&outside).expect("the sibling is this test's to create");
        if link_dir(&outside, &dest.join("notes")).is_none() {
            eprintln!(
                "extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it: \
                 this host does not allow creating a symlink, so the link half did not run"
            );
            std::fs::remove_dir_all(&root).ok();
            return;
        }

        let ctx = granting(&root);
        let raw = archive(&[Written::plain("notes/one.txt", "first")]);
        let entries = directory(&raw).expect("the archive reads");
        let resolved = nvs_runtime::capability::canonicalize(&ctx, &dest, EXTRACT)
            .expect("the destination resolves");

        let refused = place(&ctx, &resolved, &entries[0])
            .expect_err("a level resolving outside the destination is refused");
        let message = match refused {
            Fault::Thrown(ThrownClass::Runtime, message) => message.to_string(),
            Fault::Thrown(class, message) => panic!(
                "an escape is a `RuntimeError` — the disk changed, not the archive — not \
                 {class:?}: {message}"
            ),
            _ => panic!("an escape is a throw a program can catch"),
        };
        assert!(
            message.contains("resolved outside the destination"),
            "{message}"
        );
        assert!(
            message.contains("rule:security/path-scope-canonicalise-then-prefix"),
            "{message}"
        );

        // And it refused *before* creating anything: the link's target is as
        // empty as it was, which is the whole claim — a check after the
        // creation would have written the file and then complained.
        assert!(
            std::fs::read_dir(&outside)
                .expect("the sibling is still there")
                .next()
                .is_none(),
            "nothing is created outside the destination"
        );
        std::fs::remove_dir_all(&root).ok();
    }
}
