//! The names of a VBA project's modules (MS-OVBA), for `macros()`. Nothing here runs or reads a
//! module's code.
//!
//! A VBA project is a compound file ([`cfb`]): an xlsm's `xl/vbaProject.bin`, or the
//! `_VBA_PROJECT_CUR` storage of an xls. Its `VBA/dir` stream is compressed by MS-OVBA 2.4.1
//! ([`decompress`]), and is a list of records, each a type, a length and its bytes. [`names`]
//! gives each `MODULENAME` record, replaced by the `MODULENAMEUNICODE` record that follows it.
//!
//! What it spends, per call: the project's bytes, read whole, and the decompressed `dir` stream,
//! at most 4,096 bytes per chunk of the compressed stream.

use super::cfb;
use crate::Error;

const CHUNK: usize = 4096;
const PROJECT_VERSION: u16 = 0x0009;
const MODULE_NAME: u16 = 0x0019;
const MODULE_NAME_UNICODE: u16 = 0x0047;

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file's VBA project is not valid: {what}."))
}

/// The module names of the VBA project in `file`, whose `dir` stream is at `storage` and then
/// `dir`. A file with no such stream has no modules.
pub fn modules(file: &cfb::File<'_>, storage: &[&str]) -> Result<Vec<String>, Error> {
    let mut path = storage.to_vec();
    path.push("dir");
    match file.stream_at(&path)? {
        Some(dir) => names(&decompress(&dir)?),
        None => Ok(Vec::new()),
    }
}

/// The bytes of an MS-OVBA compressed container: a signature byte, then chunks of at most 4,096
/// bytes each, every chunk raw or a run of literal bytes and copies of earlier bytes.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.first() != Some(&1) {
        return Err(broken("its `dir` stream has no compression signature"));
    }
    let mut out = Vec::new();
    let mut at = 1;
    while at < data.len() {
        let header = data
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| broken("a chunk is cut short"))?;
        if header & 0x7000 != 0x3000 {
            return Err(broken("a chunk has no chunk signature"));
        }
        let end = (at + usize::from(header & 0x0FFF) + 3).min(data.len());
        at += 2;
        let start = out.len();
        if header & 0x8000 == 0 {
            let raw = (at + CHUNK).min(data.len());
            out.extend_from_slice(&data[at..raw]);
            at = raw;
            continue;
        }
        while at < end {
            let flags = data[at];
            at += 1;
            for bit in 0..8 {
                if at >= end {
                    break;
                }
                if flags >> bit & 1 == 0 {
                    out.push(data[at]);
                    at += 1;
                } else {
                    let token = data
                        .get(at..at + 2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                        .ok_or_else(|| broken("a copy token is cut short"))?;
                    at += 2;
                    let done = out.len() - start;
                    if done == 0 {
                        return Err(broken("a chunk starts with a copy"));
                    }
                    let bits = (usize::BITS - (done - 1).leading_zeros()).clamp(4, 12);
                    let offset = usize::from(token >> (16 - bits)) + 1;
                    let length = usize::from(token & (0xFFFF >> bits)) + 3;
                    if offset > done || done + length > CHUNK {
                        return Err(broken("a copy reaches outside its chunk"));
                    }
                    for _ in 0..length {
                        out.push(out[out.len() - offset]);
                    }
                }
            }
            if out.len() - start > CHUNK {
                return Err(broken("a chunk is longer than 4,096 bytes"));
            }
        }
        at = end;
    }
    Ok(out)
}

/// The module names the records of a decompressed `dir` stream give, in their order.
pub fn names(dir: &[u8]) -> Result<Vec<String>, Error> {
    let mut names: Vec<String> = Vec::new();
    let mut at = 0;
    while at + 6 <= dir.len() {
        let kind = u16::from_le_bytes([dir[at], dir[at + 1]]);
        let size = u32::from_le_bytes([dir[at + 2], dir[at + 3], dir[at + 4], dir[at + 5]]);
        at += 6;
        // The version record's length field is always 4, and six bytes follow it.
        let size = if kind == PROJECT_VERSION {
            6
        } else {
            usize::try_from(size).map_err(|_| broken("a record is too long"))?
        };
        let body = dir
            .get(at..at.saturating_add(size))
            .ok_or_else(|| broken("a record is longer than the stream"))?;
        at += size;
        match kind {
            MODULE_NAME => names.push(body.iter().copied().map(char::from).collect()),
            MODULE_NAME_UNICODE => {
                if let Some(last) = names.last_mut() {
                    let units = body
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|unit| u16::from_le_bytes([unit[0], unit[1]]));
                    *last = char::decode_utf16(units)
                        .map(|c| c.unwrap_or('\u{FFFD}'))
                        .collect();
                }
            }
            _ => {}
        }
    }
    Ok(names)
}
