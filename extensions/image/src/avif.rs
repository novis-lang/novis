//! AVIF decoding: `avif-parse` reads the container, `rav1d` decodes its AV1 items, and the YUV
//! planes are converted to RGBA8 here (`rule:core-classes/image-format-roster`).
//!
//! **The cap is read off the container first.** The primary item's sequence header declares its
//! largest frame, and that is held to the cap before the decoder exists. `rav1d` is then opened
//! with the cap as its own frame size limit, so a sequence header that lies about its size is
//! refused by the decoder too.
//!
//! `rav1d` has only the C-shaped `dav1d_*` interface, so the glue is `unsafe`: a decoder and a
//! picture are each owned by a guard that releases it on every path. It runs on one thread with a
//! frame delay of one, because a guest has no threads.
//!
//! The conversion takes 4:2:0, 4:2:2, 4:4:4 and monochrome at 8, 10 and 12 bits, full and limited
//! range, under the BT.601, BT.709 and BT.2020 matrices and the identity (GBR) one. Chroma is
//! sampled at the nearest site. An unspecified matrix is read as BT.601. An alpha item is decoded
//! the same way, its luma plane is the alpha, and a premultiplied image is divided back to
//! straight alpha.
//!
//! **The primary item's properties are read by a box walk here**, because `avif-parse` skips
//! them: `irot` and `imir`, which auto-orient applies in that order as MIAF fixes, and a `colr`
//! box of type `prof` or `rICC`, whose RGB profile to sRGB converts through `moxcms`
//! (`rule:core-classes/image-correct-by-default`). The walk reads only the `meta` box's `pitm`
//! and `iprp`, each box bounded by its parent, and allocates only the primary item's list of
//! property indices. `info` walks `iinf` and `iloc` the same way for the `Exif` item, and copies
//! that item alone, at most the file's length, and reads an image sequence's frame count off the
//! sample size box of its `moov` box's `pict` track, which allocates nothing. A box that runs past its parent returns `Parse`. An EXIF orientation inside an AVIF is not applied, because `irot` and
//! `imir` are the orientation browsers follow. Turning the frame holds a second frame beside it.

use std::io::{Cursor, Read};
use std::mem::MaybeUninit;
use std::ptr::NonNull;

use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::headers::{
    DAV1D_MC_BT709, DAV1D_MC_BT2020_CL, DAV1D_MC_BT2020_NCL, DAV1D_MC_IDENTITY,
    DAV1D_PIXEL_LAYOUT_I400, DAV1D_PIXEL_LAYOUT_I420, DAV1D_PIXEL_LAYOUT_I422,
};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_get_picture,
    dav1d_open, dav1d_picture_unref, dav1d_send_data,
};

use image::{RgbaImage, imageops};
use moxcms::{ColorProfile, DataColorSpace};

use crate::decode::{Pixels, check, srgb};
use crate::{Error, Format, Info};

fn parse(what: impl std::fmt::Display) -> Error {
    Error::Parse(format!("the AVIF file does not parse: {what}"))
}

/// The container of the AVIF file `data`, read by `avif-parse`. An image sequence's major brand
/// `avis` is read as `avif`, because `avif-parse` refuses that brand, so a sequence's primary
/// still item is read as a still file's is. The bytes are chained, not copied.
fn container(data: &[u8]) -> Result<avif_parse::AvifData, Error> {
    let read = if data.get(4..12) == Some(&b"ftypavis"[..]) {
        avif_parse::read_avif(&mut data[..8].chain(&b"avif"[..]).chain(&data[12..]))
    } else {
        avif_parse::read_avif(&mut Cursor::new(data))
    };
    read.map_err(|err| parse(format!("{err:?}")))
}

/// Decodes the AVIF file `data` to RGBA8, refusing one over `cap` pixels before a buffer exists.
/// With `auto_orient`, the primary item's `irot` and `imir` are applied; with `to_srgb`, its RGB
/// ICC profile is converted to sRGB.
pub(crate) fn decode(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
) -> Result<Pixels, Error> {
    let avif = container(data)?;
    let properties = Properties::read(data)?;
    let meta = avif
        .primary_item_metadata()
        .map_err(|err| parse(format!("{err:?}")))?;
    let frames = sequence_frames(data)?;
    check(
        u64::from(meta.max_frame_width.get()),
        u64::from(meta.max_frame_height.get()),
        frames,
        cap,
    )?;
    let color = Picture::decode(&avif.primary_item, cap)?;
    let (width, height) = color.size();
    check(u64::from(width), u64::from(height), frames, cap)?;
    let alpha = match avif.alpha_item.as_deref() {
        Some(item) => {
            let alpha = Picture::decode(item, cap)?;
            if alpha.size() != (width, height) {
                return Err(parse("the alpha item is not the size of the image"));
            }
            Some(alpha)
        }
        None => None,
    };
    let mut rgba = color.to_rgba();
    if let Some(alpha) = alpha {
        let values = alpha.luma();
        for (pixel, a) in rgba.chunks_exact_mut(4).zip(values) {
            pixel[3] = a;
            if avif.premultiplied_alpha {
                for channel in &mut pixel[..3] {
                    *channel = if a == 0 {
                        0
                    } else {
                        ((u32::from(*channel) * 255 + u32::from(a) / 2) / u32::from(a)).min(255)
                            as u8
                    };
                }
            }
        }
    }
    if to_srgb {
        let profile = properties
            .icc
            .and_then(|raw| ColorProfile::new_from_slice(raw).ok())
            .filter(|profile| profile.color_space == DataColorSpace::Rgb);
        if let Some(converted) = profile.and_then(|profile| srgb(&profile, &rgba)) {
            rgba = converted;
        }
    }
    let mut image = RgbaImage::from_raw(width, height, rgba).expect("the frame is width by height");
    if auto_orient {
        image = match properties.quarter_turns {
            1 => imageops::rotate270(&image),
            2 => imageops::rotate180(&image),
            3 => imageops::rotate90(&image),
            _ => image,
        };
        match properties.mirror {
            Some(0) => imageops::flip_horizontal_in_place(&mut image),
            Some(_) => imageops::flip_vertical_in_place(&mut image),
            None => {}
        }
    }
    Ok(Pixels {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
        exif: None,
    })
}

/// The header of the AVIF file `data`, read off its container without decoding a frame. The
/// orientation is the EXIF value that names the turn `irot` and `imir` make, and `exif` is the
/// `Exif` item's fields.
pub(crate) fn info(data: &[u8]) -> Result<Info, Error> {
    let avif = container(data)?;
    let meta = avif
        .primary_item_metadata()
        .map_err(|err| parse(format!("{err:?}")))?;
    let properties = Properties::read(data)?;
    let exif = match exif_item(data)? {
        Some(item) => {
            let offset = usize::try_from(be_u32(&item, 0)?).unwrap_or(usize::MAX);
            let tiff = item
                .get(4usize.saturating_add(offset)..)
                .ok_or_else(|| parse("the `Exif` item's TIFF header is past its end"))?;
            Some(crate::info::exif_fields(tiff)?)
        }
        None => None,
    };
    Ok(Info {
        format: Format::Avif,
        width: u64::from(meta.max_frame_width.get()),
        height: u64::from(meta.max_frame_height.get()),
        has_alpha: avif.alpha_item.is_some(),
        frames: sequence_frames(data)?,
        orientation: u64::from(properties.orientation()),
        exif,
        has_icc: properties.icc.is_some(),
    })
}

/// The primary item's properties the decoder applies, borrowed from the file.
#[derive(Debug, Default, PartialEq, Eq)]
struct Properties<'a> {
    /// `irot`'s angle: quarter turns anti-clockwise, `0` to `3`.
    quarter_turns: u8,
    /// `imir`'s axis: `0` mirrors left to right, `1` top to bottom.
    mirror: Option<u8>,
    /// The ICC profile of a `colr` box of type `prof` or `rICC`.
    icc: Option<&'a [u8]>,
}

impl<'a> Properties<'a> {
    /// Reads the primary item's properties off the `meta` box of the AVIF file `data`.
    fn read(data: &'a [u8]) -> Result<Properties<'a>, Error> {
        let mut found = Properties::default();
        let Some(meta) = find(data, b"meta")? else {
            return Ok(found);
        };
        let meta = full_box(meta)?.1;
        let primary = match find(meta, b"pitm")? {
            Some(pitm) => match full_box(pitm)? {
                (0, body) => u32::from(be_u16(body, 0)?),
                (_, body) => be_u32(body, 0)?,
            },
            None => return Ok(found),
        };
        let Some(iprp) = find(meta, b"iprp")? else {
            return Ok(found);
        };
        let Some(ipco) = find(iprp, b"ipco")? else {
            return Ok(found);
        };
        // Each `ipma` box lists, for each item, the 1-based indices into `ipco` of its properties.
        for entry in Boxes(iprp) {
            let (name, ipma) = entry?;
            if name != b"ipma" {
                continue;
            }
            for index in associations(ipma, primary)?.into_iter().flatten() {
                let Some(entry) = Boxes(ipco).nth(usize::from(index) - 1) else {
                    return Err(parse("an `ipma` index is past the end of `ipco`"));
                };
                match entry? {
                    (b"irot", body) => found.quarter_turns = u8_at(body, 0)? & 3,
                    (b"imir", body) => found.mirror = Some(u8_at(body, 0)? & 1),
                    (b"colr", body) if matches!(body.get(..4), Some(b"prof" | b"rICC")) => {
                        found.icc = Some(&body[4..]);
                    }
                    _ => {}
                }
            }
        }
        Ok(found)
    }

    /// The EXIF orientation, `1` to `8`, that turns the stored frame as `irot` then `imir` do.
    fn orientation(&self) -> u8 {
        match (self.quarter_turns, self.mirror) {
            (1, None) => 8,
            (2, None) => 3,
            (3, None) => 6,
            (0, Some(0)) | (2, Some(1)) => 2,
            (0, Some(_)) | (2, Some(_)) => 4,
            (1, Some(1)) | (3, Some(0)) => 5,
            (1, Some(_)) | (3, Some(_)) => 7,
            _ => 1,
        }
    }
}

/// The bytes of the AVIF file `data`'s first item of type `Exif`, found through `iinf` and read
/// through `iloc`, from the file or from `idat`. Its extents are joined, and together they are at
/// most the file's length.
fn exif_item(data: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    let Some(meta) = find(data, b"meta")? else {
        return Ok(None);
    };
    let meta = full_box(meta)?.1;
    let Some(iinf) = find(meta, b"iinf")? else {
        return Ok(None);
    };
    let (version, body) = full_box(iinf)?;
    let entries = body
        .get(if version == 0 { 2 } else { 4 }..)
        .ok_or_else(|| parse("an `iinf` box is shorter than its count"))?;
    let mut item = None;
    for entry in Boxes(entries) {
        let (name, infe) = entry?;
        if name != b"infe" {
            continue;
        }
        // Versions 2 and 3 name the item's type after its id and its protection index.
        let (version, body) = full_box(infe)?;
        let (id, at) = match version {
            2 => (u32::from(be_u16(body, 0)?), 4),
            3 => (be_u32(body, 0)?, 6),
            _ => continue,
        };
        if body.get(at..at + 4) == Some(b"Exif") {
            item = Some(id);
            break;
        }
    }
    let Some(item) = item else {
        return Ok(None);
    };
    let Some(iloc) = find(meta, b"iloc")? else {
        return Err(parse("the `Exif` item has no location"));
    };
    let (version, body) = full_box(iloc)?;
    let sizes = u8_at(body, 0)?;
    let more = u8_at(body, 1)?;
    let (offset_size, length_size, base_size) = (sizes >> 4, sizes & 15, more >> 4);
    let index_size = if version >= 1 { more & 15 } else { 0 };
    let mut at = 2;
    let count = if version < 2 {
        u32::from(be_u16(body, at)?)
    } else {
        be_u32(body, at)?
    };
    at += if version < 2 { 2 } else { 4 };
    for _ in 0..count {
        let id = if version < 2 {
            u32::from(be_u16(body, at)?)
        } else {
            be_u32(body, at)?
        };
        at += if version < 2 { 2 } else { 4 };
        let method = if version >= 1 {
            at += 2;
            be_u16(body, at - 2)? & 15
        } else {
            0
        };
        // The data reference index, which names this file for every item a reader can open.
        at += 2;
        let base = sized(body, &mut at, base_size)?;
        let extents = be_u16(body, at)?;
        at += 2;
        let source = match method {
            0 => data,
            1 => find(meta, b"idat")?.unwrap_or_default(),
            _ => &[],
        };
        let mut bytes = Vec::new();
        for _ in 0..extents {
            sized(body, &mut at, index_size)?;
            let offset = sized(body, &mut at, offset_size)?;
            let length = sized(body, &mut at, length_size)?;
            if id != item {
                continue;
            }
            if method > 1 {
                return Err(parse("the `Exif` item is stored in another item"));
            }
            let start = usize::try_from(base.saturating_add(offset)).unwrap_or(usize::MAX);
            let end = match length {
                0 => source.len(),
                length => start.saturating_add(usize::try_from(length).unwrap_or(usize::MAX)),
            };
            let extent = source
                .get(start..end)
                .ok_or_else(|| parse("the `Exif` item runs past the end of the file"))?;
            if bytes.len() + extent.len() > data.len() {
                return Err(parse("the `Exif` item is longer than the file"));
            }
            bytes.extend_from_slice(extent);
        }
        if id == item {
            return Ok(Some(bytes));
        }
    }
    Err(parse("the `Exif` item has no location"))
}

/// The unsigned field of `size` bytes at `at` in an `iloc` body, `0` for a size of `0`, and `at`
/// moved past it.
fn sized(body: &[u8], at: &mut usize, size: u8) -> Result<u64, Error> {
    let value = match size {
        0 => 0,
        4 => u64::from(be_u32(body, *at)?),
        8 => u64::from(be_u32(body, *at)?) << 32 | u64::from(be_u32(body, *at + 4)?),
        _ => return Err(parse("an `iloc` field is not 0, 4 or 8 bytes")),
    };
    *at += usize::from(size);
    Ok(value)
}

/// The property indices `ipma` associates with `item`, without the essential bit. Index `0`
/// means no property and is left out.
fn associations(ipma: &[u8], item: u32) -> Result<Option<Vec<u16>>, Error> {
    let (version, body) = full_box(ipma)?;
    let wide = ipma[3] & 1 == 1;
    let count = be_u32(body, 0)?;
    let mut at = 4;
    for _ in 0..count {
        let id = if version == 0 {
            let id = u32::from(be_u16(body, at)?);
            at += 2;
            id
        } else {
            let id = be_u32(body, at)?;
            at += 4;
            id
        };
        let listed = usize::from(u8_at(body, at)?);
        at += 1;
        let mut indices = Vec::new();
        for _ in 0..listed {
            let index = if wide {
                let index = be_u16(body, at)? & 0x7fff;
                at += 2;
                index
            } else {
                let index = u16::from(u8_at(body, at)? & 0x7f);
                at += 1;
                index
            };
            if index != 0 {
                indices.push(index);
            }
        }
        if id == item {
            return Ok(Some(indices));
        }
    }
    Ok(None)
}

/// The frame count of an image sequence: the sample count of the `moov` box's `pict` track, read
/// off its `stsz` or `stz2` box. A file with no `moov` is a still image, and `1`.
fn sequence_frames(data: &[u8]) -> Result<u64, Error> {
    let Some(moov) = find(data, b"moov")? else {
        return Ok(1);
    };
    for entry in Boxes(moov) {
        let (name, trak) = entry?;
        if name != b"trak" {
            continue;
        }
        let Some(mdia) = find(trak, b"mdia")? else {
            continue;
        };
        // `hdlr` is version and flags, `pre_defined`, then the handler type.
        let handler = find(mdia, b"hdlr")?.and_then(|hdlr| hdlr.get(8..12));
        if handler != Some(b"pict") {
            continue;
        }
        let Some(stbl) = find(mdia, b"minf")?
            .map(|minf| find(minf, b"stbl"))
            .transpose()?
            .flatten()
        else {
            continue;
        };
        // Both boxes give the count after their version, flags and one 32-bit field.
        let sizes = match find(stbl, b"stsz")? {
            Some(stsz) => stsz,
            None => find(stbl, b"stz2")?
                .ok_or_else(|| parse("a sequence track has no sample size box"))?,
        };
        return Ok(u64::from(be_u32(full_box(sizes)?.1, 4)?).max(1));
    }
    Ok(1)
}

/// The boxes of an ISO BMFF body, each as its type and its body. A box that runs past the end
/// of the body is an `Err`, and ends the walk.
struct Boxes<'a>(&'a [u8]);

impl<'a> Iterator for Boxes<'a> {
    type Item = Result<(&'a [u8; 4], &'a [u8]), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }
        let entry = split_box(self.0);
        self.0 = match &entry {
            Ok((_, _, rest)) => rest,
            Err(_) => &[],
        };
        Some(entry.map(|(name, body, _)| (name, body)))
    }
}

/// A box's type, its body and the bytes after it.
type Split<'a> = (&'a [u8; 4], &'a [u8], &'a [u8]);

/// The first box of `data`, its body and the bytes after it.
fn split_box(data: &[u8]) -> Result<Split<'_>, Error> {
    let truncated = || parse("a box runs past the end of the box that contains it");
    let size = be_u32(data, 0)?;
    let name: &[u8; 4] = data
        .get(4..8)
        .and_then(|name| name.try_into().ok())
        .ok_or_else(truncated)?;
    let (header, size) = match size {
        0 => (8, data.len()),
        1 => {
            let large = u64::from(be_u32(data, 8)?) << 32 | u64::from(be_u32(data, 12)?);
            (16, usize::try_from(large).map_err(|_| truncated())?)
        }
        size => (8, size as usize),
    };
    if size < header || size > data.len() {
        return Err(truncated());
    }
    Ok((name, &data[header..size], &data[size..]))
}

/// The body of the first box named `name` among the boxes of `data`.
fn find<'a>(data: &'a [u8], name: &[u8; 4]) -> Result<Option<&'a [u8]>, Error> {
    for entry in Boxes(data) {
        let (found, body) = entry?;
        if found == name {
            return Ok(Some(body));
        }
    }
    Ok(None)
}

/// A full box's version and the body after its version and flags.
fn full_box(body: &[u8]) -> Result<(u8, &[u8]), Error> {
    if body.len() < 4 {
        return Err(parse("a box is shorter than its version and flags"));
    }
    Ok((body[0], &body[4..]))
}

fn u8_at(body: &[u8], at: usize) -> Result<u8, Error> {
    body.get(at)
        .copied()
        .ok_or_else(|| parse("a box is shorter than its fields"))
}

fn be_u16(body: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_be_bytes([u8_at(body, at)?, u8_at(body, at + 1)?]))
}

fn be_u32(body: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from(be_u16(body, at)?) << 16 | u32::from(be_u16(body, at + 2)?))
}

/// An open `rav1d` decoder, closed when it is dropped.
struct Decoder(Option<Dav1dContext>);

impl Drop for Decoder {
    fn drop(&mut self) {
        // SAFETY: the context came from `dav1d_open` and is closed only here, which sets it to
        // `None`.
        unsafe { dav1d_close(Some(NonNull::from(&mut self.0))) };
    }
}

/// Input bytes handed to `rav1d`, released when they are dropped.
struct Data(Dav1dData);

impl Drop for Data {
    fn drop(&mut self) {
        // SAFETY: the data came from `dav1d_data_create`, and unref is a no-op once the decoder
        // has consumed it.
        unsafe { dav1d_data_unref(Some(NonNull::from(&mut self.0))) };
    }
}

/// One decoded AV1 frame, released when it is dropped.
struct Picture(Dav1dPicture);

impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: the picture was written by `dav1d_get_picture`, or is still its default, which
        // unref leaves alone.
        unsafe { dav1d_picture_unref(Some(NonNull::from(&mut self.0))) };
    }
}

/// The negative error code `rav1d` returns when it needs more input or has no picture yet.
const EAGAIN: i32 = -libc::EAGAIN;

impl Picture {
    /// Decodes the first frame of the AV1 bitstream `obus`, under a frame size limit of `cap`.
    fn decode(obus: &[u8], cap: u64) -> Result<Picture, Error> {
        let mut settings = MaybeUninit::<Dav1dSettings>::uninit();
        // SAFETY: `dav1d_default_settings` writes every field of the settings it is handed.
        let mut settings = unsafe {
            dav1d_default_settings(NonNull::from(&mut settings).cast());
            settings.assume_init()
        };
        settings.n_threads = 1;
        settings.max_frame_delay = 1;
        settings.all_layers = 0;
        settings.frame_size_limit = u32::try_from(cap).unwrap_or(u32::MAX);
        let mut decoder = Decoder(None);
        // SAFETY: both pointers are to live, initialized values for the length of the call.
        let opened = unsafe {
            dav1d_open(
                Some(NonNull::from(&mut decoder.0)),
                Some(NonNull::from(&mut settings)),
            )
        };
        let context = match (opened.0, decoder.0) {
            (0, Some(context)) => context,
            _ => {
                return Err(Error::Runtime("the AVIF decoder did not start".to_string()));
            }
        };
        let mut data = Data(Dav1dData::default());
        // SAFETY: `data.0` is a live value `dav1d_data_create` may overwrite.
        let buffer = unsafe { dav1d_data_create(Some(NonNull::from(&mut data.0)), obus.len()) };
        if buffer.is_null() {
            return Err(parse("the AV1 item is empty"));
        }
        // SAFETY: `dav1d_data_create` returned a buffer of `obus.len()` bytes that nothing else
        // reads or writes until it is sent.
        unsafe { std::ptr::copy_nonoverlapping(obus.as_ptr(), buffer, obus.len()) };
        let mut picture = Picture(Dav1dPicture::default());
        let mut drained = false;
        loop {
            if data.0.sz > 0 {
                // SAFETY: the context is open and `data.0` is the value `dav1d_data_create` wrote.
                let sent =
                    unsafe { dav1d_send_data(Some(context), Some(NonNull::from(&mut data.0))) };
                if sent.0 != 0 && sent.0 != EAGAIN {
                    return Err(parse(format!("the AV1 data was refused ({})", sent.0)));
                }
            }
            // SAFETY: the context is open, and `picture.0` holds no reference yet.
            let got =
                unsafe { dav1d_get_picture(Some(context), Some(NonNull::from(&mut picture.0))) };
            match got.0 {
                0 => break,
                EAGAIN if data.0.sz > 0 => {}
                EAGAIN if !drained => drained = true,
                EAGAIN => return Err(parse("the AV1 item holds no frame")),
                code => return Err(parse(format!("the AV1 frame does not decode ({code})"))),
            }
        }
        let p = &picture.0.p;
        if p.w <= 0 || p.h <= 0 || !matches!(p.bpc, 8 | 10 | 12) || picture.0.seq_hdr.is_none() {
            return Err(parse("the AV1 frame has no usable size or bit depth"));
        }
        for plane in 0..picture.planes() {
            if picture.0.data[plane].is_none() {
                return Err(parse("the AV1 frame is missing a plane"));
            }
        }
        drop(decoder);
        Ok(picture)
    }

    fn size(&self) -> (u32, u32) {
        (self.0.p.w as u32, self.0.p.h as u32)
    }

    fn planes(&self) -> usize {
        if self.0.p.layout == DAV1D_PIXEL_LAYOUT_I400 {
            1
        } else {
            3
        }
    }

    /// The horizontal and vertical chroma shifts.
    fn shifts(&self) -> (usize, usize) {
        match self.0.p.layout {
            DAV1D_PIXEL_LAYOUT_I420 => (1, 1),
            DAV1D_PIXEL_LAYOUT_I422 => (1, 0),
            _ => (0, 0),
        }
    }

    /// The `(matrix, full range)` the sequence header declares.
    fn colour(&self) -> (u32, bool) {
        let header = self
            .0
            .seq_hdr
            .expect("checked when the picture was decoded");
        // SAFETY: the sequence header lives as long as the picture that references it.
        let header = unsafe { header.as_ref() };
        (header.mtrx, header.color_range != 0)
    }

    /// The sample at column `x`, row `y` of `plane`, as a value in `0..1 << bpc`.
    fn sample(&self, plane: usize, x: usize, y: usize) -> u32 {
        let stride = self.0.stride[usize::from(plane > 0)];
        let base = self.0.data[plane].expect("checked when the picture was decoded");
        let wide = self.0.p.bpc > 8;
        let offset = y as isize * stride + (x << usize::from(wide)) as isize;
        // SAFETY: `x` and `y` are inside the plane, whose rows are `stride` bytes apart, and the
        // picture keeps the plane alive. A wide sample is a native-endian `u16`.
        unsafe {
            let at = base.as_ptr().cast::<u8>().offset(offset);
            if wide {
                u32::from(at.cast::<u16>().read_unaligned())
            } else {
                u32::from(at.read())
            }
        }
    }

    /// The luma plane as 8-bit values, row by row: an alpha item's alpha.
    fn luma(&self) -> Vec<u8> {
        let (width, height) = self.size();
        let (_, full) = self.colour();
        let range = Range::new(self.0.p.bpc, full);
        let mut out = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height as usize {
            for x in 0..width as usize {
                out.push(byte(range.luma(self.sample(0, x, y))));
            }
        }
        out
    }

    /// The frame as RGBA8 with an opaque alpha, row by row.
    fn to_rgba(&self) -> Vec<u8> {
        let (width, height) = self.size();
        let (matrix, full) = self.colour();
        let range = Range::new(self.0.p.bpc, full);
        let (shift_x, shift_y) = self.shifts();
        let grey = self.planes() == 1;
        let (kr, kb) = match matrix {
            DAV1D_MC_BT709 => (0.2126, 0.0722),
            DAV1D_MC_BT2020_NCL | DAV1D_MC_BT2020_CL => (0.2627, 0.0593),
            _ => (0.299, 0.114),
        };
        let kg = 1.0 - kr - kb;
        let mut out = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height as usize {
            for x in 0..width as usize {
                let luma = self.sample(0, x, y);
                if grey {
                    let v = byte(range.luma(luma));
                    out.extend_from_slice(&[v, v, v, 255]);
                    continue;
                }
                let u = self.sample(1, x >> shift_x, y >> shift_y);
                let v = self.sample(2, x >> shift_x, y >> shift_y);
                let (r, g, b) = if matrix == DAV1D_MC_IDENTITY {
                    (range.luma(v), range.luma(luma), range.luma(u))
                } else {
                    let (yy, cb, cr) = (range.luma(luma), range.chroma(u), range.chroma(v));
                    let r = yy + 2.0 * (1.0 - kr) * cr;
                    let b = yy + 2.0 * (1.0 - kb) * cb;
                    (r, (yy - kr * r - kb * b) / kg, b)
                };
                out.extend_from_slice(&[byte(r), byte(g), byte(b), 255]);
            }
        }
        out
    }
}

/// How a sample maps onto `0..=1` for luma and `-0.5..=0.5` for chroma.
struct Range {
    luma_offset: f32,
    luma_scale: f32,
    chroma_offset: f32,
    chroma_scale: f32,
}

impl Range {
    fn new(bpc: i32, full: bool) -> Range {
        let max = ((1u32 << bpc) - 1) as f32;
        let unit = (1u32 << (bpc - 8)) as f32;
        let half = (1u32 << (bpc - 1)) as f32;
        if full {
            Range {
                luma_offset: 0.0,
                luma_scale: max,
                chroma_offset: half,
                chroma_scale: max,
            }
        } else {
            Range {
                luma_offset: 16.0 * unit,
                luma_scale: 219.0 * unit,
                chroma_offset: half,
                chroma_scale: 224.0 * unit,
            }
        }
    }

    fn luma(&self, sample: u32) -> f32 {
        (sample as f32 - self.luma_offset) / self.luma_scale
    }

    fn chroma(&self, sample: u32) -> f32 {
        (sample as f32 - self.chroma_offset) / self.chroma_scale
    }
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
