//! The compound file (MS-CFB) an xls is: a file of fixed-size sectors, a table that chains the
//! sectors of each stream, and a directory that names the streams.
//!
//! [`File::open`] reads the header, the sector table and the directory of bytes the reader holds
//! whole, and [`File::stream`] gives one stream's bytes. A stream shorter than the header's
//! cutoff is kept in the mini stream, in sectors of 64 bytes, and is read through the mini table.
//!
//! **A chain is followed at most once round the table.** A chain longer than the table has
//! sectors, a sector number past the table, or a stream longer than its chain is
//! [`Error::Parse`], so a chain that loops back on itself ends the read and is never followed
//! again.

use crate::Error;

const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;
const HEADER_TABLE: usize = 109;
const MINI_SECTOR: usize = 64;
const STREAM: u8 = 2;
const ROOT: u8 = 5;

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid xls file: {what}."))
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, Error> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| broken("it is cut short"))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, Error> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| broken("it is cut short"))
}

/// Whether `bytes` starts as a compound file does.
pub fn is_compound(bytes: &[u8]) -> bool {
    bytes.starts_with(&MAGIC)
}

/// One stream the directory names.
#[derive(Debug)]
struct Stream {
    name: String,
    kind: u8,
    start: u32,
    size: u64,
}

/// A compound file being read.
#[derive(Debug)]
pub struct File<'a> {
    bytes: &'a [u8],
    sector: usize,
    table: Vec<u32>,
    mini_table: Vec<u32>,
    mini_stream: Vec<u8>,
    cutoff: u64,
    streams: Vec<Stream>,
}

impl<'a> File<'a> {
    /// Reads the header, the sector table and the directory of `bytes`.
    pub fn open(bytes: &'a [u8]) -> Result<Self, Error> {
        if !is_compound(bytes) || bytes.len() < 512 {
            return Err(broken("it has no compound file header"));
        }
        let sector = match u16_at(bytes, 30)? {
            9 => 512,
            12 => 4096,
            _ => return Err(broken("its sector size is not 512 or 4096 bytes")),
        };
        let table_sectors = u32_at(bytes, 44)? as usize;
        if table_sectors > bytes.len() / sector {
            return Err(broken("its sector table is larger than the file"));
        }
        let mut file = Self {
            bytes,
            sector,
            table: Vec::new(),
            mini_table: Vec::new(),
            mini_stream: Vec::new(),
            cutoff: u64::from(u32_at(bytes, 56)?),
            streams: Vec::new(),
        };
        // The sector table's own sectors: the first 109 are listed in the header, and the rest in
        // a chain of extra sectors whose last entry names the next one.
        let mut listed = Vec::new();
        for at in 0..HEADER_TABLE.min(table_sectors) {
            listed.push(u32_at(bytes, 76 + at * 4)?);
        }
        let mut next = u32_at(bytes, 68)?;
        let per = sector / 4 - 1;
        let mut extra = u32_at(bytes, 72)?;
        while listed.len() < table_sectors && next != END_OF_CHAIN && next != FREE {
            if extra == 0 {
                return Err(broken("its sector table lists more sectors than it has"));
            }
            extra -= 1;
            let block = file.sector_bytes(next)?;
            for at in 0..per.min(table_sectors - listed.len()) {
                listed.push(u32_at(block, at * 4)?);
            }
            next = u32_at(block, per * 4)?;
        }
        let mut table = Vec::with_capacity(listed.len() * sector / 4);
        for id in listed {
            let block = file.sector_bytes(id)?;
            for at in 0..sector / 4 {
                table.push(u32_at(block, at * 4)?);
            }
        }
        file.table = table;
        let directory = file.chain(u32_at(bytes, 48)?, None)?;
        for entry in directory.chunks_exact(128) {
            let length = usize::from(u16_at(entry, 64)?).min(64);
            let units = entry[..length.saturating_sub(2)]
                .chunks_exact(2)
                .map(|unit| u16::from_le_bytes([unit[0], unit[1]]));
            let name = char::decode_utf16(units)
                .map(|c| c.unwrap_or('\u{FFFD}'))
                .collect();
            let size = if sector == 512 {
                u64::from(u32_at(entry, 120)?)
            } else {
                u64::from(u32_at(entry, 120)?) | (u64::from(u32_at(entry, 124)?) << 32)
            };
            file.streams.push(Stream {
                name,
                kind: entry[66],
                start: u32_at(entry, 116)?,
                size,
            });
        }
        if let Some(root) = file.streams.iter().find(|s| s.kind == ROOT) {
            let (start, size) = (root.start, root.size);
            if size > 0 {
                file.mini_stream = file.chain(start, Some(size))?;
            }
            let mini = file.chain(u32_at(bytes, 60)?, None)?;
            file.mini_table = mini
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
        }
        Ok(file)
    }

    fn sector_bytes(&self, id: u32) -> Result<&'a [u8], Error> {
        let start = (id as usize)
            .checked_add(1)
            .and_then(|n| n.checked_mul(self.sector))
            .ok_or_else(|| broken("a sector number is past the file"))?;
        self.bytes
            .get(start..start + self.sector)
            .ok_or_else(|| broken("a sector number is past the file"))
    }

    /// The bytes of the chain that starts at `start`, cut to `size` when it is given.
    fn chain(&self, start: u32, size: Option<u64>) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        let mut id = start;
        let mut steps = 0;
        while id != END_OF_CHAIN && id != FREE {
            steps += 1;
            if steps > self.table.len() {
                return Err(broken("a sector chain loops"));
            }
            out.extend_from_slice(self.sector_bytes(id)?);
            if size.is_some_and(|size| out.len() as u64 >= size) {
                break;
            }
            id = *self
                .table
                .get(id as usize)
                .ok_or_else(|| broken("a sector number is past the sector table"))?;
        }
        cut(out, size)
    }

    /// The bytes of the mini stream's chain that starts at `start`, cut to `size`.
    fn mini_chain(&self, start: u32, size: u64) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        let mut id = start;
        let mut steps = 0;
        while id != END_OF_CHAIN && id != FREE && (out.len() as u64) < size {
            steps += 1;
            if steps > self.mini_table.len() {
                return Err(broken("a sector chain loops"));
            }
            let at = id as usize * MINI_SECTOR;
            out.extend_from_slice(
                self.mini_stream
                    .get(at..at + MINI_SECTOR)
                    .ok_or_else(|| broken("a mini sector is past the mini stream"))?,
            );
            id = *self
                .mini_table
                .get(id as usize)
                .ok_or_else(|| broken("a sector number is past the sector table"))?;
        }
        cut(out, Some(size))
    }

    /// The stream named `name`, compared without case, or `None` when the file has none.
    pub fn stream(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        let Some(stream) = self
            .streams
            .iter()
            .find(|s| s.kind == STREAM && s.name.eq_ignore_ascii_case(name))
        else {
            return Ok(None);
        };
        if stream.size < self.cutoff {
            self.mini_chain(stream.start, stream.size).map(Some)
        } else {
            self.chain(stream.start, Some(stream.size)).map(Some)
        }
    }
}

fn cut(mut bytes: Vec<u8>, size: Option<u64>) -> Result<Vec<u8>, Error> {
    if let Some(size) = size {
        let size = usize::try_from(size).map_err(|_| broken("a stream is too large"))?;
        if bytes.len() < size {
            return Err(broken("a stream is longer than its sectors"));
        }
        bytes.truncate(size);
    }
    Ok(bytes)
}
