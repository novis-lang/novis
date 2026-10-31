//! The extras of an xls, read with its cells by [`super::xls::read`]: each sheet's hyperlinks,
//! notes and pictures, and the document properties.
//!
//! **Where each one is.** A hyperlink is a `HLINK` record of the sheet, its tooltip the
//! `HLINKTOOLTIP` after it. A note is a `NOTE` record, whose text is the `TXO` record that
//! follows the `OBJ` of the same id, and whose characters are in the `CONTINUE` records after the
//! `TXO`. A picture is a shape of the sheet's drawing, the `MSODRAWING` records read as one run of
//! bytes: its blip index points into the picture store of the globals' `MSODRAWINGGROUP`, and its
//! client anchor names its top-left cell. The properties are the `\x05SummaryInformation` and
//! `\x05DocumentSummaryInformation` streams of the compound file ([MS-OLEPS]).
//!
//! **Nothing a file names is followed** (`rule:core-classes/spreadsheet-has-no-io`). An external
//! hyperlink is its target's text. A picture the store links to a file has no bytes and is not
//! listed.
//!
//! What it spends: the extras of every sheet and each picture's bytes, held with the rows until
//! the reader is freed. A property that does not read is `None`, because a property is metadata
//! and not a cell.

use super::binary::{u8_at, u16_at, u32_at};
use super::extras::{Image, Link, Name, Note};
use super::{cfb, civil, days_from_civil, letters};
use crate::{Clock, Error, Properties, TimeOfDay};

/// The extras of an xls, each sheet's entries with its sheet's name.
#[derive(Debug, Default)]
pub struct Extras {
    pub links: Vec<(String, Link)>,
    pub notes: Vec<(String, Note)>,
    pub names: Vec<Name>,
    pub images: Vec<(String, Image)>,
    pub properties: Properties,
}

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid xls file: {what}."))
}

/// The cell `row` and `column`, each from 0, as A1 text.
fn cell_text(row: u16, column: u16) -> String {
    format!("{}{}", letters(u64::from(column) + 1), u32::from(row) + 1)
}

/// The UTF-16 text of `bytes`, up to its first NUL.
fn wide_text(bytes: &[u8]) -> String {
    let units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&unit| u16::from_le_bytes(unit))
        .take_while(|&unit| unit != 0);
    char::decode_utf16(units)
        .map(|c| c.unwrap_or('\u{FFFD}'))
        .collect()
}

/// The bytes of `body` from `at` for `length` bytes, or an error naming `what`.
fn take<'a>(body: &'a [u8], at: usize, length: usize, what: &str) -> Result<&'a [u8], Error> {
    at.checked_add(length)
        .and_then(|end| body.get(at..end))
        .ok_or_else(|| broken(&format!("{what} is longer than its record")))
}

/// A `HyperlinkString` at `at`: a count of UTF-16 units with its NUL, and the units. Gives the
/// text and the offset after it.
fn hyperlink_string(body: &[u8], at: usize) -> Result<(String, usize), Error> {
    let units = u32_at(body, at)? as usize;
    let bytes = take(body, at + 4, units.saturating_mul(2), "a hyperlink")?;
    Ok((wide_text(bytes), at + 4 + bytes.len()))
}

/// The class id of a URL moniker, `{79EAC9E0-BAF9-11CE-8C82-00AA004BA90B}`, as the file stores it.
const URL_MONIKER: [u8; 16] = [
    0xE0, 0xC9, 0xEA, 0x79, 0xF9, 0xBA, 0xCE, 0x11, 0x8C, 0x82, 0x00, 0xAA, 0x00, 0x4B, 0xA9, 0x0B,
];
/// The class id of a file moniker, `{00000303-0000-0000-C000-000000000046}`.
const FILE_MONIKER: [u8; 16] = [0x03, 0x03, 0, 0, 0, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0x46];

/// A `HLINK` record ([MS-XLS] 2.4.140): the range it covers, a class id, and a hyperlink object
/// ([MS-OSHARED] 2.3.7.1) of a version, flags, and then the display text, the frame, the moniker
/// that names the target and the place inside it, each present when its flag is set. A moniker
/// of a kind other than a URL or a file gives no target.
pub fn hyperlink(body: &[u8]) -> Result<Option<Link>, Error> {
    let corner = |at| u16_at(body, at);
    let (first, last) = (
        cell_text(corner(0)?, corner(4)?),
        cell_text(corner(2)?, corner(6)?),
    );
    let anchor = if first == last {
        first
    } else {
        format!("{first}:{last}")
    };
    let flags = u32_at(body, 28)?;
    let mut at = 32;
    let mut display = None;
    if flags & 0x10 != 0 {
        let (text, after) = hyperlink_string(body, at)?;
        display = Some(text);
        at = after;
    }
    if flags & 0x80 != 0 {
        at = hyperlink_string(body, at)?.1;
    }
    let mut target = None;
    if flags & 0x01 != 0 {
        if flags & 0x100 != 0 {
            let (text, after) = hyperlink_string(body, at)?;
            target = Some(text);
            at = after;
        } else {
            let class = take(body, at, 16, "a hyperlink")?;
            at += 16;
            if class == URL_MONIKER {
                let length = u32_at(body, at)? as usize;
                target = Some(wide_text(take(body, at + 4, length, "a hyperlink")?));
                at += 4 + length;
            } else if class == FILE_MONIKER {
                let up = usize::from(u16_at(body, at)?);
                let narrow = u32_at(body, at + 2)? as usize;
                let ansi = take(body, at + 6, narrow, "a hyperlink")?;
                at += 6 + narrow + 24;
                let wide = u32_at(body, at)? as usize;
                at += 4;
                let path = if wide > 0 {
                    let bytes = u32_at(body, at)? as usize;
                    let text = wide_text(take(body, at + 6, bytes, "a hyperlink")?);
                    at += wide;
                    text
                } else {
                    ansi.iter()
                        .take_while(|&&b| b != 0)
                        .map(|&b| char::from(b))
                        .collect()
                };
                target = Some(format!("{}{path}", "..\\".repeat(up.min(64))));
            } else {
                return Ok(None);
            }
        }
    }
    let location = if flags & 0x08 != 0 {
        Some(hyperlink_string(body, at)?.0)
    } else {
        None
    };
    let (target, external) = match (target, location) {
        (Some(target), Some(location)) => (format!("{target}#{location}"), true),
        (Some(target), None) => (target, true),
        (None, Some(location)) => (location, false),
        (None, None) => return Ok(None),
    };
    Ok(Some(Link {
        anchor,
        target,
        text: display,
        tooltip: None,
        external,
    }))
}

/// The text of a `HLINKTOOLTIP` record: its record type and range, then UTF-16 text.
pub fn tooltip(body: &[u8]) -> String {
    wide_text(body.get(10..).unwrap_or_default())
}

/// A `NOTE` record's cell and the id of its `OBJ`: its row, its column, flags and the id. Its
/// author's name follows at 8.
pub fn note(body: &[u8]) -> Result<(String, u16), Error> {
    Ok((
        cell_text(u16_at(body, 0)?, u16_at(body, 2)?),
        u16_at(body, 6)?,
    ))
}

/// An `OBJ` record's first subrecord, its common data: the object's type and its id.
pub fn object(body: &[u8]) -> Result<(u16, u16), Error> {
    Ok((u16_at(body, 4)?, u16_at(body, 6)?))
}

/// The number of characters a `TXO` record's text has. The characters are in the `CONTINUE`
/// records after it.
pub fn text_length(body: &[u8]) -> Result<usize, Error> {
    Ok(usize::from(u16_at(body, 10)?))
}

/// A `CONTINUE` record of a `TXO`'s text: a byte that says whether the characters are two bytes
/// each, and then the characters. Takes at most `left` of them into `out`, and gives how many.
pub fn text_part(body: &[u8], left: usize, out: &mut String) -> Result<usize, Error> {
    let wide = u8_at(body, 0)? & 1 != 0;
    let width = if wide { 2 } else { 1 };
    let count = (body.len().saturating_sub(1) / width).min(left);
    let bytes = &body[1..1 + count * width];
    if wide {
        out.extend(
            char::decode_utf16(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&unit| u16::from_le_bytes(unit)),
            )
            .map(|c| c.unwrap_or('\u{FFFD}')),
        );
    } else {
        out.extend(bytes.iter().map(|&b| char::from(b)));
    }
    Ok(count)
}

/// An OfficeArt record at `at` of `bytes`: its version, its instance, its type, and its body,
/// cut where `bytes` ends.
fn art_record(bytes: &[u8], at: usize) -> Option<(u16, u16, u16, &[u8])> {
    let head = bytes.get(at..at.checked_add(8)?)?;
    let version_instance = u16::from_le_bytes([head[0], head[1]]);
    let kind = u16::from_le_bytes([head[2], head[3]]);
    let length = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    let start = at + 8;
    let end = start.saturating_add(length).min(bytes.len());
    Some((
        version_instance & 0xF,
        version_instance >> 4,
        kind,
        &bytes[start..end],
    ))
}

/// The OfficeArt records of `bytes` in order, each its version, instance, type and body.
fn art_records(bytes: &[u8]) -> impl Iterator<Item = (u16, u16, u16, &[u8])> {
    let mut at = 0;
    std::iter::from_fn(move || {
        let record = art_record(bytes, at)?;
        at += 8 + record.3.len();
        Some(record)
    })
}

const STORE: u16 = 0xF001;
const STORE_ENTRY: u16 = 0xF007;
const SHAPE: u16 = 0xF004;
const OPTIONS: u16 = 0xF00B;
const ANCHOR: u16 = 0xF010;
/// The property that names a shape's picture by its place in the store, from 1.
const PICTURE: u16 = 0x0104;
/// How deep containers may nest before the rest of a drawing is not read.
const MOST_DEPTH: usize = 16;
/// The most bytes a compressed metafile may grow to.
const MOST_METAFILE: usize = 64 << 20;

/// The pictures of the store in the drawing group `group`, in the store's order: each one's
/// format and bytes, or `None` for an entry with no picture inside it.
pub fn store(group: &[u8]) -> Vec<Option<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut walk = vec![(group, 0)];
    while let Some((bytes, depth)) = walk.pop() {
        for (version, _, kind, body) in art_records(bytes) {
            if kind == STORE {
                for (_, _, kind, entry) in art_records(body) {
                    if kind == STORE_ENTRY {
                        out.push(store_entry(entry));
                    }
                }
                return out;
            }
            if version == 0xF && depth < MOST_DEPTH {
                walk.push((body, depth + 1));
            }
        }
    }
    out
}

/// One entry of the store ([MS-ODRAW] 2.2.32 `OfficeArtFBSE`): 36 bytes of fields, a name, and
/// the picture's record. A bitmap is one or two ids, a tag byte and the file. A metafile is one
/// or two ids, a header of 34 bytes and its data, compressed with zlib when the header's
/// compression byte is 0.
fn store_entry(entry: &[u8]) -> Option<(String, Vec<u8>)> {
    let name = usize::from(*entry.get(33)?);
    let (_, instance, kind, blip) = art_record(entry, 36 + name)?;
    let ids = if instance & 1 == 1 { 32 } else { 16 };
    let format = match kind {
        0xF01D | 0xF02A => "jpeg",
        0xF01E => "png",
        0xF01F => "dib",
        0xF029 => "tiff",
        0xF01A => "emf",
        0xF01B => "wmf",
        0xF01C => "pict",
        _ => return None,
    };
    let bytes = if kind == 0xF01A || kind == 0xF01B || kind == 0xF01C {
        let header = blip.get(ids..ids + 34)?;
        let size = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let saved = u32::from_le_bytes([header[28], header[29], header[30], header[31]]) as usize;
        let data = blip.get(ids + 34..)?;
        let data = &data[..saved.min(data.len())];
        if header[32] == 0 {
            miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, size.min(MOST_METAFILE))
                .ok()?
        } else {
            data.to_vec()
        }
    } else {
        blip.get(ids + 1..)?.to_vec()
    };
    Some((format.to_owned(), bytes))
}

/// The pictures of a sheet's drawing `drawing`, in its order: each shape's place in the store,
/// from 1, and the column and row of its anchor's top-left cell, each from 0.
pub fn placed(drawing: &[u8]) -> Vec<(u32, u16, u16)> {
    let mut out = Vec::new();
    shapes(drawing, 0, &mut out);
    out
}

fn shapes(bytes: &[u8], depth: usize, out: &mut Vec<(u32, u16, u16)>) {
    for (version, _, kind, body) in art_records(bytes) {
        if kind == SHAPE {
            let (mut picture, mut anchor) = (None, None);
            for (_, instance, kind, part) in art_records(body) {
                match kind {
                    OPTIONS => {
                        picture = part
                            .as_chunks::<6>()
                            .0
                            .iter()
                            .take(usize::from(instance))
                            .find(|option| {
                                u16::from_le_bytes([option[0], option[1]]) & 0x3FFF == PICTURE
                            })
                            .map(|option| {
                                u32::from_le_bytes([option[2], option[3], option[4], option[5]])
                            });
                    }
                    ANCHOR if part.len() >= 8 => {
                        let at = |n: usize| u16::from_le_bytes([part[n], part[n + 1]]);
                        anchor = Some((at(2), at(6)));
                    }
                    _ => {}
                }
            }
            if let (Some(picture), Some((column, row))) = (picture, anchor) {
                out.push((picture, column, row));
            }
            continue;
        }
        if version == 0xF && depth < MOST_DEPTH {
            shapes(body, depth + 1, out);
        }
    }
}

/// The picture `index`, from 1, of `store`, as an image at the cell `column` and `row`.
pub fn image(
    store: &[Option<(String, Vec<u8>)>],
    index: u32,
    column: u16,
    row: u16,
) -> Option<Image> {
    let (format, bytes) = store.get((index as usize).checked_sub(1)?)?.as_ref()?;
    Some(Image {
        at: cell_text(row, column),
        format: format.clone(),
        image: bytes.clone(),
        link: None,
    })
}

const CODEPAGE: u32 = 1;
const TEXT: u16 = 0x1E;
const WIDE_TEXT: u16 = 0x1F;
const FILE_TIME: u16 = 0x40;

/// A property value this reader keeps: text, or a time in 100-nanosecond ticks from 1601.
enum Value {
    Text(String),
    Ticks(u64),
}

/// The characters Windows-1252 gives 0x80 to 0x9F, where Latin-1 has control characters.
const WINDOWS_1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
];

/// The text of a narrow property in `codepage`: UTF-8 for 65001, and Windows-1252 otherwise, the
/// code page Excel writes. It ends at its first NUL.
fn narrow_text(bytes: &[u8], codepage: u16) -> String {
    let bytes: Vec<u8> = bytes.iter().copied().take_while(|&b| b != 0).collect();
    if codepage == 65001 {
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => WINDOWS_1252[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

/// The first property set of a property set stream ([MS-OLEPS] 2.21): a header of 28 bytes, the
/// set's id and offset, and at that offset its size, its count and each property's id and
/// offset. Each property is a type of four bytes and its value.
fn property_set(stream: &[u8]) -> Vec<(u32, Value)> {
    let read = || -> Option<Vec<(u32, Value)>> {
        let start = u32_at(stream, 44).ok()? as usize;
        let set = stream.get(start..)?;
        let size = (u32_at(set, 0).ok()? as usize).min(set.len());
        let set = &set[..size];
        let count = (u32_at(set, 4).ok()? as usize).min(size / 8);
        let mut pairs = Vec::with_capacity(count);
        for n in 0..count {
            pairs.push((
                u32_at(set, 8 + n * 8).ok()?,
                u32_at(set, 12 + n * 8).ok()? as usize,
            ));
        }
        let codepage = pairs
            .iter()
            .find(|(id, _)| *id == CODEPAGE)
            .and_then(|&(_, at)| u16_at(set, at + 4).ok())
            .unwrap_or(1252);
        let mut out = Vec::new();
        for (id, at) in pairs {
            let Ok(kind) = u16_at(set, at) else {
                continue;
            };
            let value = match kind {
                TEXT => u32_at(set, at + 4).ok().and_then(|length| {
                    let bytes = set.get(at + 8..(at + 8).checked_add(length as usize)?)?;
                    Some(Value::Text(narrow_text(bytes, codepage)))
                }),
                WIDE_TEXT => u32_at(set, at + 4).ok().and_then(|units| {
                    let end = (at + 8).checked_add((units as usize).checked_mul(2)?)?;
                    Some(Value::Text(wide_text(set.get(at + 8..end)?)))
                }),
                FILE_TIME => {
                    let low = u64::from(u32_at(set, at + 4).ok()?);
                    let high = u64::from(u32_at(set, at + 8).ok()?);
                    Some(Value::Ticks(high << 32 | low))
                }
                _ => None,
            };
            out.extend(value.map(|value| (id, value)));
        }
        Some(out)
    };
    read().unwrap_or_default()
}

/// A time in 100-nanosecond ticks from 1601-01-01, as its clock time in UTC. Zero, which a file
/// writes for no time, is `None`.
fn ticks_clock(ticks: u64) -> Option<Clock> {
    if ticks == 0 {
        return None;
    }
    let seconds = i64::try_from(ticks / 10_000_000).ok()?;
    let total = days_from_civil(1601, 1, 1) * 86_400 + seconds;
    let of_day = total.rem_euclid(86_400);
    Some(Clock {
        day: civil(total.div_euclid(86_400)),
        clock: TimeOfDay {
            hour: of_day / 3600,
            minute: of_day % 3600 / 60,
            second: of_day % 60,
            nanos: (ticks % 10_000_000) as i64 * 100,
        },
    })
}

/// The document properties of the compound file `file`: the summary stream's title (2), subject
/// (3), author (4), keywords (5) and creation time (12), and the document summary stream's
/// company (15).
pub fn properties(file: &cfb::File) -> Result<Properties, Error> {
    let mut properties = Properties::default();
    if let Some(stream) = file.stream("\u{5}SummaryInformation")? {
        for (id, value) in property_set(&stream) {
            match (id, value) {
                (2, Value::Text(text)) => properties.title = Some(text),
                (3, Value::Text(text)) => properties.subject = Some(text),
                (4, Value::Text(text)) => properties.author = Some(text),
                (5, Value::Text(text)) => properties.keywords = Some(text),
                (12, Value::Ticks(ticks)) => properties.created = ticks_clock(ticks),
                _ => {}
            }
        }
    }
    if let Some(stream) = file.stream("\u{5}DocumentSummaryInformation")? {
        for (id, value) in property_set(&stream) {
            if let (15, Value::Text(text)) = (id, value) {
                properties.company = Some(text);
            }
        }
    }
    Ok(properties)
}
