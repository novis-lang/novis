//! The zip archive an xlsx file is, written forward and never sought back into.
//!
//! [`Archive`] frames each entry the way a stream must: a local header with general-purpose flag
//! bit 3 set, so its CRC-32 and sizes are zero there, the deflated data as the compressor produces
//! it, and a data descriptor after the data with the real CRC-32 and sizes. [`Archive::close`]
//! writes the central directory and the end record. Every entry is deflated (method 8), its name
//! is UTF-8 (bit 11), and its time is the fixed DOS time 1980-01-01 00:00, so two archives of
//! equal content are equal bytes (`rule:core-classes/spreadsheet-output-is-inert`).
//!
//! **No zip64.** An entry's sizes, an offset and the central directory are 32-bit fields here, so
//! a value past `u32::MAX`, or a 65,536th entry, is a [`Error::Runtime`] at the call that would
//! write it, and the archive stops: a reader never gets a file whose sizes wrap.
//!
//! [`Archive::stored`] writes a whole entry at once, uncompressed (method 0) with its CRC-32 and
//! sizes in its local header and no data descriptor. An ods needs its `mimetype` entry so.
//!
//! The bytes produced wait in [`Archive::take`]'s buffer until the caller takes them, which every
//! writer call does before it returns, so the archive keeps no byte it has already given out. One
//! [`CompressorOxide`] serves every entry and is reset between them.

use miniz_oxide::deflate::core::{
    CompressorOxide, TDEFLFlush, TDEFLStatus, compress_to_output, create_comp_flags_from_zip_params,
};

use crate::Error;

/// The deflate level: zlib's default.
const LEVEL: i32 = 6;

/// 1980-01-01 in DOS date form: years since 1980, month and day.
const DOS_DATE: u16 = (1 << 5) | 1;
/// 00:00:00 in DOS time form.
const DOS_TIME: u16 = 0;
/// Bit 3, sizes in a data descriptor, and bit 11, a UTF-8 name.
const FLAGS: u16 = (1 << 3) | (1 << 11);
/// Version 2.0, the first with deflate.
const VERSION: u16 = 20;

/// Bit 11 alone, for an entry whose sizes are in its local header.
const STORED_FLAGS: u16 = 1 << 11;

/// An entry's line in the central directory. `stored` is an entry [`Archive::stored`] wrote.
struct Central {
    name: String,
    crc: u32,
    packed: u32,
    raw: u32,
    offset: u32,
    stored: bool,
}

/// The entry being written.
struct Open {
    name: String,
    offset: u32,
    crc: crc32fast::Hasher,
    raw: u64,
    packed: u64,
}

/// A zip archive being written as a stream.
pub struct Archive {
    out: Vec<u8>,
    written: u64,
    central: Vec<Central>,
    open: Option<Open>,
    deflate: Box<CompressorOxide>,
}

impl Default for Archive {
    fn default() -> Self {
        Self::new()
    }
}

fn too_big() -> Error {
    Error::Runtime(
        "The workbook is larger than 4 GB. This writer cannot write a file of that size."
            .to_owned(),
    )
}

fn u32_of(value: u64) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| too_big())
}

impl Archive {
    /// An empty archive.
    pub fn new() -> Self {
        Self {
            out: Vec::new(),
            written: 0,
            central: Vec::new(),
            open: None,
            deflate: Box::new(CompressorOxide::new(create_comp_flags_from_zip_params(
                LEVEL, -15, 0,
            ))),
        }
    }

    /// Every byte the archive has produced, taken or not.
    pub fn size(&self) -> u64 {
        self.written + self.out.len() as u64
    }

    /// The bytes produced since the last call, which the archive no longer keeps.
    pub fn take(&mut self) -> Vec<u8> {
        self.written += self.out.len() as u64;
        std::mem::take(&mut self.out)
    }

    /// Checks that one more entry named `name` fits, and returns its offset and its name's
    /// length.
    fn room(&self, name: &str) -> Result<(u32, u16), Error> {
        if self.central.len() >= usize::from(u16::MAX) {
            return Err(Error::Runtime(
                "The workbook has more than 65,535 parts. This writer cannot write that many."
                    .to_owned(),
            ));
        }
        let offset = u32_of(self.size())?;
        let name_len = u16::try_from(name.len())
            .map_err(|_| Error::Invalid("A part name is longer than 65,535 bytes.".to_owned()))?;
        Ok((offset, name_len))
    }

    /// Writes the whole entry `name` uncompressed, ending the open one first.
    pub fn stored(&mut self, name: &str, data: &[u8]) -> Result<(), Error> {
        self.end()?;
        let (offset, name_len) = self.room(name)?;
        let size = u32_of(data.len() as u64)?;
        let crc = crc32fast::hash(data);
        let out = &mut self.out;
        put32(out, 0x0403_4b50);
        put16(out, VERSION);
        put16(out, STORED_FLAGS);
        put16(out, 0);
        put16(out, DOS_TIME);
        put16(out, DOS_DATE);
        put32(out, crc);
        put32(out, size);
        put32(out, size);
        put16(out, name_len);
        put16(out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        self.central.push(Central {
            name: name.to_owned(),
            crc,
            packed: size,
            raw: size,
            offset,
            stored: true,
        });
        Ok(())
    }

    /// Starts the entry `name`, ending the open one first.
    pub fn begin(&mut self, name: &str) -> Result<(), Error> {
        self.end()?;
        let (offset, name_len) = self.room(name)?;
        let out = &mut self.out;
        put32(out, 0x0403_4b50);
        put16(out, VERSION);
        put16(out, FLAGS);
        put16(out, 8);
        put16(out, DOS_TIME);
        put16(out, DOS_DATE);
        put32(out, 0);
        put32(out, 0);
        put32(out, 0);
        put16(out, name_len);
        put16(out, 0);
        out.extend_from_slice(name.as_bytes());
        self.deflate.reset();
        self.open = Some(Open {
            name: name.to_owned(),
            offset,
            crc: crc32fast::Hasher::new(),
            raw: 0,
            packed: 0,
        });
        Ok(())
    }

    /// Deflates `data` into the open entry. The compressor may keep the tail of it until more
    /// comes or the entry ends.
    pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.compress(data, TDEFLFlush::None)
    }

    fn compress(&mut self, data: &[u8], flush: TDEFLFlush) -> Result<(), Error> {
        let Some(open) = self.open.as_mut() else {
            return Err(Error::Runtime("no zip entry is open".to_owned()));
        };
        open.crc.update(data);
        open.raw += data.len() as u64;
        let before = self.out.len();
        let out = &mut self.out;
        let (status, consumed) = compress_to_output(&mut self.deflate, data, flush, |chunk| {
            out.extend_from_slice(chunk);
            true
        });
        let ok = match flush {
            TDEFLFlush::Finish => status == TDEFLStatus::Done,
            _ => status == TDEFLStatus::Okay,
        };
        if !ok || consumed != data.len() {
            return Err(Error::Runtime(format!(
                "the compressor stopped with {status:?}"
            )));
        }
        open.packed += (self.out.len() - before) as u64;
        if open.raw > u64::from(u32::MAX) || open.packed > u64::from(u32::MAX) {
            return Err(too_big());
        }
        Ok(())
    }

    /// Ends the open entry, if there is one, with its data descriptor.
    pub fn end(&mut self) -> Result<(), Error> {
        if self.open.is_none() {
            return Ok(());
        }
        self.compress(&[], TDEFLFlush::Finish)?;
        let Some(open) = self.open.take() else {
            return Ok(());
        };
        let entry = Central {
            name: open.name,
            crc: open.crc.finalize(),
            packed: u32_of(open.packed)?,
            raw: u32_of(open.raw)?,
            offset: open.offset,
            stored: false,
        };
        put32(&mut self.out, 0x0807_4b50);
        put32(&mut self.out, entry.crc);
        put32(&mut self.out, entry.packed);
        put32(&mut self.out, entry.raw);
        self.central.push(entry);
        Ok(())
    }

    /// Ends the open entry and writes the central directory and the end record.
    pub fn close(&mut self) -> Result<(), Error> {
        self.end()?;
        let start = u32_of(self.size())?;
        let before = self.out.len();
        for entry in &self.central {
            let out = &mut self.out;
            put32(out, 0x0201_4b50);
            put16(out, VERSION);
            put16(out, VERSION);
            if entry.stored {
                put16(out, STORED_FLAGS);
                put16(out, 0);
            } else {
                put16(out, FLAGS);
                put16(out, 8);
            }
            put16(out, DOS_TIME);
            put16(out, DOS_DATE);
            put32(out, entry.crc);
            put32(out, entry.packed);
            put32(out, entry.raw);
            // `begin` refused a name of 65,536 bytes or more.
            put16(out, entry.name.len() as u16);
            put16(out, 0);
            put16(out, 0);
            put16(out, 0);
            put16(out, 0);
            put32(out, 0);
            put32(out, entry.offset);
            out.extend_from_slice(entry.name.as_bytes());
        }
        let directory = u32_of((self.out.len() - before) as u64)?;
        u32_of(self.size())?;
        // `begin` refused a 65,536th entry.
        let count = self.central.len() as u16;
        let out = &mut self.out;
        put32(out, 0x0605_4b50);
        put16(out, 0);
        put16(out, 0);
        put16(out, count);
        put16(out, count);
        put32(out, directory);
        put32(out, start);
        put16(out, 0);
        Ok(())
    }
}

fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use miniz_oxide::inflate::decompress_to_vec;

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn an_entry_reads_back_through_its_descriptor_and_the_directory() {
        let mut archive = Archive::new();
        archive.begin("a.xml").unwrap();
        archive.write(b"<a>hello</a>").unwrap();
        archive.write(b"<a>hello</a>").unwrap();
        archive.close().unwrap();
        let bytes = archive.take();
        assert_eq!(u32_at(&bytes, 0), 0x0403_4b50);
        let data_start = 30 + 5;
        let end = bytes.len() - 22;
        assert_eq!(u32_at(&bytes, end), 0x0605_4b50);
        let directory = u32_at(&bytes, end + 16) as usize;
        let packed = u32_at(&bytes, directory + 20) as usize;
        let inflated = decompress_to_vec(&bytes[data_start..data_start + packed]).unwrap();
        assert_eq!(inflated, b"<a>hello</a><a>hello</a>");
        assert_eq!(u32_at(&bytes, data_start + packed), 0x0807_4b50);
        assert_eq!(
            u32_at(&bytes, data_start + packed + 4),
            crc32fast::hash(b"<a>hello</a><a>hello</a>")
        );
    }
}
