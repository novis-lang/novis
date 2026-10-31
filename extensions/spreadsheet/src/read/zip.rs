//! A zip archive read through byte ranges: the directory from the end, and each entry inflated as
//! it is parsed.
//!
//! [`Directory::read`] reads the end record from the last 64 KiB of the source and then the
//! central directory, and keeps one [`Entry`] per file: its name, method, sizes and the offset of
//! its local header. [`Part`] inflates one entry a block of [`BLOCK`] compressed bytes at a time,
//! so a sheet is never held whole. [`Directory::whole`] reads a part the reader keeps, such as the shared
//! strings.
//!
//! **Bounds.** `rule:core-classes/decompression-bound`, with the [`Bound`] the host clamped to
//! the request's `[limits]`. A [`Part`] streamed and never held stops past its compressed size
//! times the ratio. A part read [`Directory::whole`] stops past the smaller of that and what the
//! archive has left of the byte ceiling, and is then charged against it, so the archive's total
//! counts held parts only. Each is a [`Error::Parse`] naming the part and the half that stopped it.
//! An entry that inflates past the size its directory line declares is refused too, though that
//! size is the file's own claim and bounds nothing. The central directory is read whole, and one
//! larger than [`DIRECTORY_MOST`] is refused. Zip64 is not read: Excel writes it only past 4 GiB.
//! Only stored (0) and deflated (8) entries are read.

use std::cell::Cell;

use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};

use super::Source;
use crate::Error;

/// The compressed bytes one read of a part asks for: the read-ahead block.
pub const BLOCK: u64 = 64 * 1024;

/// The largest central directory read, which 65,535 entries with long names fit in.
pub const DIRECTORY_MOST: u64 = 16 * 1024 * 1024;

/// The end record's fixed length, and the longest comment after it.
const END: u64 = 22;
const COMMENT_MOST: u64 = 0xFFFF;

/// The decompression bound one archive is read under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bound {
    /// The most bytes the parts held whole decompress to, together.
    pub bytes: u64,
    /// The most bytes of output per byte of input, for every part.
    pub ratio: u64,
}

impl Default for Bound {
    /// The shipped `[limits] max_decompressed` and `max_decompression_ratio`, for a call no host
    /// clamped.
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            ratio: 1000,
        }
    }
}

/// One file of the archive.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    method: u16,
    packed: u64,
    pub raw: u64,
    header: u64,
    /// The bound's ratio: the entry inflates to at most its compressed size times this.
    ratio: u64,
}

/// The archive's directory.
#[derive(Debug, Default)]
pub struct Directory {
    entries: Vec<Entry>,
    bound: Bound,
    /// The bytes of the parts read whole so far.
    held: Cell<u64>,
}

fn over_ratio(name: &str, ratio: u64) -> Error {
    Error::Parse(format!(
        "The workbook part `{name}` decompresses to more than {ratio} times its compressed size. \
         To allow a larger ratio, raise `[limits] max_decompression_ratio` in the server's \
         configuration."
    ))
}

fn over_bytes(name: &str, bytes: u64) -> Error {
    Error::Parse(format!(
        "The workbook part `{name}` makes the decompressed parts larger than {bytes} bytes. \
         To allow more, raise `[limits] max_decompressed` in the server's configuration."
    ))
}

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid workbook: {what}."))
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

impl Directory {
    /// Reads the directory of the zip archive `source` is, whose parts are then read under
    /// `bound`.
    pub fn read(source: &dyn Source, bound: Bound) -> Result<Self, Error> {
        let size = source.size()?;
        if size < END {
            return Err(broken("it is too short to be a zip archive"));
        }
        let tail_start = size.saturating_sub(END + COMMENT_MOST);
        let tail = source.range(tail_start, size - tail_start)?;
        let last = tail
            .len()
            .checked_sub(22)
            .ok_or_else(|| broken("it has no zip directory"))?;
        let at = (0..=last)
            .rev()
            .find(|&at| tail[at..at + 4] == [0x50, 0x4b, 0x05, 0x06])
            .ok_or_else(|| broken("it has no zip directory"))?;
        let count = u16_at(&tail, at + 10);
        let length = u64::from(u32_at(&tail, at + 12));
        let offset = u64::from(u32_at(&tail, at + 16));
        if count == 0xFFFF || length == 0xFFFF_FFFF || offset == 0xFFFF_FFFF {
            return Err(broken("it is a zip64 archive, which is not read"));
        }
        if length > DIRECTORY_MOST || offset + length > size {
            return Err(broken("its zip directory is larger than the file"));
        }
        let central = source.range(offset, length)?;
        let mut entries = Vec::with_capacity(usize::from(count));
        let mut at = 0;
        for _ in 0..count {
            if central.len() < at + 46 || central[at..at + 4] != [0x50, 0x4b, 0x01, 0x02] {
                return Err(broken("its zip directory is cut short"));
            }
            let names = usize::from(u16_at(&central, at + 28));
            let extra = usize::from(u16_at(&central, at + 30));
            let comment = usize::from(u16_at(&central, at + 32));
            let end = at + 46 + names;
            if central.len() < end {
                return Err(broken("its zip directory is cut short"));
            }
            entries.push(Entry {
                name: String::from_utf8_lossy(&central[at + 46..end]).into_owned(),
                method: u16_at(&central, at + 10),
                packed: u64::from(u32_at(&central, at + 20)),
                raw: u64::from(u32_at(&central, at + 24)),
                header: u64::from(u32_at(&central, at + 42)),
                ratio: bound.ratio,
            });
            at = end + extra + comment;
        }
        Ok(Self {
            entries,
            bound,
            held: Cell::new(0),
        })
    }

    /// The whole of `entry`, for a part the reader keeps: stopped past what the archive has left
    /// of the bound's bytes, and charged against it.
    pub fn whole(&self, source: &dyn Source, entry: &Entry) -> Result<Vec<u8>, Error> {
        let left = self.bound.bytes.saturating_sub(self.held.get());
        let mut part = Part::open(source, entry)?;
        let mut out = Vec::with_capacity(usize::try_from(entry.raw.min(BLOCK * 16)).unwrap_or(0));
        while part.fill(source, &mut out)? {
            if out.len() as u64 > left {
                return Err(over_bytes(&entry.name, self.bound.bytes));
            }
        }
        self.held.set(self.held.get() + out.len() as u64);
        Ok(out)
    }

    /// The entry named `name`. Names in a workbook are compared without case, as Excel does.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        let name = name.trim_start_matches('/');
        self.entries
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }

    /// Every entry, in the directory's order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

/// How a part's bytes come out.
enum Method {
    Stored,
    Deflated(Box<InflateState>),
}

/// One entry being inflated a block at a time.
pub struct Part {
    method: Method,
    name: String,
    next: u64,
    end: u64,
    raw: u64,
    most: u64,
    ratio: u64,
    out: u64,
    input: Vec<u8>,
    used: usize,
    done: bool,
}

impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Part")
            .field("next", &self.next)
            .field("end", &self.end)
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}

impl Part {
    /// Starts reading `entry`: reads its local header to find where its data begins.
    pub fn open(source: &dyn Source, entry: &Entry) -> Result<Self, Error> {
        let local = source.range(entry.header, 30)?;
        if local.len() < 30 || local[..4] != [0x50, 0x4b, 0x03, 0x04] {
            return Err(broken(&format!("the entry `{}` has no header", entry.name)));
        }
        let start =
            entry.header + 30 + u64::from(u16_at(&local, 26)) + u64::from(u16_at(&local, 28));
        let method = match entry.method {
            0 => Method::Stored,
            8 => Method::Deflated(InflateState::new_boxed(DataFormat::Raw)),
            other => {
                return Err(broken(&format!(
                    "the entry `{}` uses compression method {other}, which is not read",
                    entry.name
                )));
            }
        };
        Ok(Self {
            method,
            name: entry.name.clone(),
            next: start,
            end: start + entry.packed,
            raw: entry.raw,
            most: entry.packed.saturating_mul(entry.ratio),
            ratio: entry.ratio,
            out: 0,
            input: Vec::new(),
            used: 0,
            done: false,
        })
    }

    /// Appends the next bytes of the part to `out`. Returns `false` once the part has ended.
    pub fn fill(&mut self, source: &dyn Source, out: &mut Vec<u8>) -> Result<bool, Error> {
        if self.done {
            return Ok(false);
        }
        if self.used == self.input.len() && self.next < self.end {
            let take = BLOCK.min(self.end - self.next);
            self.input = source.range(self.next, take)?;
            if self.input.is_empty() {
                return Err(broken("a part is cut short"));
            }
            self.next += self.input.len() as u64;
            self.used = 0;
        }
        let drained = self.used == self.input.len() && self.next >= self.end;
        let before = out.len();
        match &mut self.method {
            Method::Stored if drained => {
                self.done = true;
                return Ok(false);
            }
            Method::Stored => {
                out.extend_from_slice(&self.input[self.used..]);
                self.used = self.input.len();
            }
            Method::Deflated(state) => {
                let mut chunk = vec![0; 64 * 1024];
                let result = inflate(state, &self.input[self.used..], &mut chunk, MZFlush::None);
                self.used += result.bytes_consumed;
                out.extend_from_slice(&chunk[..result.bytes_written]);
                match result.status {
                    Ok(MZStatus::StreamEnd) => self.done = true,
                    Ok(_) | Err(MZError::Buf) if drained && result.bytes_written == 0 => {
                        return Err(broken("a part ends before its data does"));
                    }
                    Ok(_) | Err(MZError::Buf) => {}
                    Err(_) => return Err(broken("a part's compressed data is damaged")),
                }
            }
        }
        self.out += (out.len() - before) as u64;
        if self.out > self.most {
            return Err(over_ratio(&self.name, self.ratio));
        }
        if self.out > self.raw {
            return Err(broken("a part is larger than the zip directory says"));
        }
        Ok(true)
    }
}
