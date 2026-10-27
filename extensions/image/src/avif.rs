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
//! sample size box of its `moov` box's `pict` track, which allocates nothing. A box that runs past
//! its parent returns `Parse`. An EXIF orientation inside an AVIF is not applied, because `irot`
//! and `imir` are the orientation browsers follow. Turning the frame holds a second frame beside
//! it.
//!
//! **An image sequence decodes every frame** through [`decode_frames`]: the `pict` track's
//! samples, found through its `stsz` or `stz2`, `stsc` and `stco` or `co64` boxes, go through one
//! decoder in order, and an `auxv` track's the same way through a second for the alpha. Each
//! frame shows for its `stts` delta over the `mdhd` timescale. The tables are read as the walk
//! goes, so it allocates nothing beyond the frames, and a sample outside the file returns
//! `Parse`. The primary item's `irot`, `imir` and `colr` apply to every frame, and a frame of
//! another size than the first returns `Parse`.

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

use crate::decode::{Frame, Pixels, check, srgb};
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
    let alpha = match avif.alpha_item.as_deref() {
        Some(item) => Some(Picture::decode(item, cap)?),
        None => None,
    };
    let profile = profile(&properties, to_srgb);
    let rgba = composite(
        &color,
        alpha.as_ref(),
        avif.premultiplied_alpha,
        frames,
        cap,
    )?;
    Ok(upright(
        rgba,
        &color,
        profile.as_ref(),
        &properties,
        auto_orient,
    ))
}

/// Decodes every frame of the AVIF file `data` as [`decode`] decodes the first. An image
/// sequence decodes its `pict` track sample by sample on one decoder, and its `auxv` track the
/// same way for the alpha, each frame shown for its `stts` delta over the `mdhd` timescale. The
/// primary item's `irot`, `imir` and `colr` apply to every frame, so each frame is turned and
/// converted as the first is. Each frame is held to the cap times the frame count, and has the
/// first one's size. A still file gives its one frame.
pub(crate) fn decode_frames(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
) -> Result<Vec<Frame>, Error> {
    let Some(colour_track) = track(data, b"pict")? else {
        return Ok(vec![Frame {
            pixels: decode(data, cap, auto_orient, to_srgb)?,
            delay_ms: 0,
        }]);
    };
    let avif = container(data)?;
    let properties = Properties::read(data)?;
    let meta = avif
        .primary_item_metadata()
        .map_err(|err| parse(format!("{err:?}")))?;
    let frames = colour_track.count()?;
    check(
        u64::from(meta.max_frame_width.get()),
        u64::from(meta.max_frame_height.get()),
        frames,
        cap,
    )?;
    let profile = profile(&properties, to_srgb);
    let alpha_track = track(data, b"auxv")?;
    let mut colour = Stream::open(cap)?;
    let mut alpha = alpha_track
        .as_ref()
        .map(|_| Stream::open(cap))
        .transpose()?;
    let mut alpha_samples = match &alpha_track {
        Some(track) => Some(track.samples(data)?),
        None => None,
    };
    let mut out: Vec<Frame> = Vec::new();
    for sample in colour_track.samples(data)? {
        let (obus, delay_ms) = sample?;
        let picture = colour.decode(obus)?;
        let alpha_picture = match (alpha.as_mut(), alpha_samples.as_mut()) {
            (Some(stream), Some(samples)) => match samples.next() {
                Some(sample) => Some(stream.decode(sample?.0)?),
                None => None,
            },
            _ => None,
        };
        let rgba = composite(
            &picture,
            alpha_picture.as_ref(),
            avif.premultiplied_alpha,
            frames,
            cap,
        )?;
        let pixels = upright(rgba, &picture, profile.as_ref(), &properties, auto_orient);
        if let Some(first) = out.first()
            && (pixels.width, pixels.height) != (first.pixels.width, first.pixels.height)
        {
            return Err(parse(
                "a frame of the sequence is not the size of the first",
            ));
        }
        out.push(Frame { pixels, delay_ms });
    }
    if out.is_empty() {
        return Err(parse("the image sequence has no frame"));
    }
    Ok(out)
}

/// The primary item's RGB ICC profile, when `to_srgb` is set and the profile parses.
fn profile(properties: &Properties<'_>, to_srgb: bool) -> Option<ColorProfile> {
    if !to_srgb {
        return None;
    }
    properties
        .icc
        .and_then(|raw| ColorProfile::new_from_slice(raw).ok())
        .filter(|profile| profile.color_space == DataColorSpace::Rgb)
}

/// `color` as RGBA8, its alpha the luma of `alpha` when there is one, divided back to straight
/// alpha when `premultiplied`. The frame is held to the cap as one of `frames`.
fn composite(
    color: &Picture,
    alpha: Option<&Picture>,
    premultiplied: bool,
    frames: u64,
    cap: u64,
) -> Result<Vec<u8>, Error> {
    let (width, height) = color.size();
    check(u64::from(width), u64::from(height), frames, cap)?;
    if alpha.is_some_and(|alpha| alpha.size() != (width, height)) {
        return Err(parse("the alpha item is not the size of the image"));
    }
    let mut rgba = color.to_rgba();
    if let Some(alpha) = alpha {
        let values = alpha.luma();
        for (pixel, a) in rgba.chunks_exact_mut(4).zip(values) {
            pixel[3] = a;
            if premultiplied {
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
    Ok(rgba)
}

/// The RGBA8 `rgba` of `color`'s size converted from `profile` to sRGB when there is one, then
/// turned by the primary item's `irot` and `imir` when `auto_orient` is set.
fn upright(
    mut rgba: Vec<u8>,
    color: &Picture,
    profile: Option<&ColorProfile>,
    properties: &Properties<'_>,
    auto_orient: bool,
) -> Pixels {
    let (width, height) = color.size();
    if let Some(converted) = profile.and_then(|profile| srgb(profile, &rgba)) {
        rgba = converted;
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
    Pixels {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
        exif: None,
    }
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
    match track(data, b"pict")? {
        Some(track) => Ok(track.count()?.max(1)),
        None => Ok(1),
    }
}

/// One track of an image sequence: its `mdhd` and `stbl` box bodies, borrowed from the file.
struct Track<'a> {
    mdhd: &'a [u8],
    stbl: &'a [u8],
}

/// The first track of the `moov` box whose handler type is `handler`, `pict` for the colour and
/// `auxv` for the alpha.
fn track<'a>(data: &'a [u8], handler: &[u8; 4]) -> Result<Option<Track<'a>>, Error> {
    let Some(moov) = find(data, b"moov")? else {
        return Ok(None);
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
        let found = find(mdia, b"hdlr")?.and_then(|hdlr| hdlr.get(8..12));
        if found != Some(handler) {
            continue;
        }
        let Some(stbl) = find(mdia, b"minf")?
            .map(|minf| find(minf, b"stbl"))
            .transpose()?
            .flatten()
        else {
            continue;
        };
        let mdhd = find(mdia, b"mdhd")?.ok_or_else(|| parse("a sequence track has no `mdhd`"))?;
        return Ok(Some(Track { mdhd, stbl }));
    }
    Ok(None)
}

impl<'a> Track<'a> {
    /// The `stsz` or `stz2` box's body after its version and flags.
    fn sizes(&self) -> Result<(bool, &'a [u8]), Error> {
        match find(self.stbl, b"stsz")? {
            Some(stsz) => Ok((false, full_box(stsz)?.1)),
            None => Ok((
                true,
                full_box(
                    find(self.stbl, b"stz2")?
                        .ok_or_else(|| parse("a sequence track has no sample size box"))?,
                )?
                .1,
            )),
        }
    }

    /// The sample count. Both size boxes give it after one 32-bit field.
    fn count(&self) -> Result<u64, Error> {
        Ok(u64::from(be_u32(self.sizes()?.1, 4)?))
    }

    /// Each sample's bytes and its delay in milliseconds, in order. The tables are read as the
    /// walk goes, so it allocates nothing, and a sample outside the file returns `Parse`.
    fn samples(&self, data: &'a [u8]) -> Result<Samples<'a>, Error> {
        let (compact, sizes) = self.sizes()?;
        let (version, mdhd) = full_box(self.mdhd)?;
        let timescale = be_u32(mdhd, if version == 1 { 16 } else { 8 })?;
        let (wide, chunks) = match find(self.stbl, b"stco")? {
            Some(stco) => (false, full_box(stco)?.1),
            None => (
                true,
                full_box(
                    find(self.stbl, b"co64")?
                        .ok_or_else(|| parse("a sequence track has no chunk offset box"))?,
                )?
                .1,
            ),
        };
        let stsc = full_box(
            find(self.stbl, b"stsc")?
                .ok_or_else(|| parse("a sequence track has no sample-to-chunk box"))?,
        )?
        .1;
        let stts = full_box(
            find(self.stbl, b"stts")?
                .ok_or_else(|| parse("a sequence track has no time-to-sample box"))?,
        )?
        .1;
        Ok(Samples {
            data,
            compact,
            sizes,
            count: be_u32(sizes, 4)?,
            wide,
            chunks,
            stsc,
            stts,
            timescale,
            sample: 0,
            chunk: 0,
            left_in_chunk: 0,
            entry: 0,
            per_chunk: None,
            offset: 0,
            run: 0,
            left_in_run: 0,
            delta: 0,
        })
    }
}

/// The walk over a track's samples that [`Track::samples`] starts.
struct Samples<'a> {
    data: &'a [u8],
    /// Whether `sizes` is an `stz2` body, whose entries are 4, 8 or 16 bits wide.
    compact: bool,
    sizes: &'a [u8],
    count: u32,
    /// Whether `chunks` is a `co64` body, whose offsets are 64 bits wide.
    wide: bool,
    chunks: &'a [u8],
    stsc: &'a [u8],
    stts: &'a [u8],
    timescale: u32,
    /// The index of the next sample, and of the chunk it is in, counted from 0.
    sample: u32,
    chunk: u32,
    left_in_chunk: u32,
    /// The next `stsc` entry, and the sample count of the one before it.
    entry: u32,
    per_chunk: Option<u32>,
    /// Where the next sample starts in the file.
    offset: u64,
    /// The next `stts` entry, and what is left of the one being read.
    run: u32,
    left_in_run: u32,
    delta: u32,
}

impl<'a> Samples<'a> {
    fn size(&self, sample: u32) -> Result<u64, Error> {
        let sample = sample as usize;
        if self.compact {
            return match u8_at(self.sizes, 3)? {
                4 => {
                    let byte = u8_at(self.sizes, 8 + sample / 2)?;
                    Ok(u64::from(if sample.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 15
                    }))
                }
                8 => Ok(u64::from(u8_at(self.sizes, 8 + sample)?)),
                16 => Ok(u64::from(be_u16(self.sizes, 8 + 2 * sample)?)),
                _ => Err(parse(
                    "the compact sample size box has a field size it may not",
                )),
            };
        }
        match be_u32(self.sizes, 0)? {
            0 => Ok(u64::from(be_u32(self.sizes, 8 + 4 * sample)?)),
            fixed => Ok(u64::from(fixed)),
        }
    }

    /// The number of samples in the chunk counted from 0 as `chunk`, read off `stsc`'s last
    /// entry whose first chunk, counted from 1, is at most `chunk + 1`. Chunks are asked for in
    /// order, so the entry only moves forward.
    fn samples_in(&mut self, chunk: u32) -> Result<u32, Error> {
        let entries = be_u32(self.stsc, 0)?;
        while self.entry < entries
            && be_u32(self.stsc, 4 + 12 * self.entry as usize)? <= chunk.saturating_add(1)
        {
            self.per_chunk = Some(be_u32(self.stsc, 8 + 12 * self.entry as usize)?);
            self.entry += 1;
        }
        self.per_chunk
            .ok_or_else(|| parse("the sample-to-chunk box does not cover every chunk"))
    }

    fn next_sample(&mut self) -> Result<(&'a [u8], u32), Error> {
        while self.left_in_chunk == 0 {
            if self.chunk >= be_u32(self.chunks, 0)? {
                return Err(parse(
                    "the sequence's chunks hold fewer samples than it counts",
                ));
            }
            let at = self.chunk as usize;
            self.offset = if self.wide {
                u64::from(be_u32(self.chunks, 4 + 8 * at)?) << 32
                    | u64::from(be_u32(self.chunks, 8 + 8 * at)?)
            } else {
                u64::from(be_u32(self.chunks, 4 + 4 * at)?)
            };
            self.left_in_chunk = self.samples_in(self.chunk)?;
            self.chunk += 1;
        }
        let size = self.size(self.sample)?;
        let end = self.offset.saturating_add(size);
        let bytes = usize::try_from(self.offset)
            .ok()
            .zip(usize::try_from(end).ok())
            .and_then(|(start, end)| self.data.get(start..end))
            .ok_or_else(|| parse("a sample of the sequence is past the end of the file"))?;
        self.offset = end;
        self.left_in_chunk -= 1;
        self.sample += 1;
        let runs = be_u32(self.stts, 0)?;
        while self.left_in_run == 0 && self.run < runs {
            let at = 4 + 8 * self.run as usize;
            self.left_in_run = be_u32(self.stts, at)?;
            self.delta = be_u32(self.stts, at + 4)?;
            self.run += 1;
        }
        if self.left_in_run == 0 {
            // A sample past `stts`'s entries shows for no time.
            self.delta = 0;
        }
        self.left_in_run = self.left_in_run.saturating_sub(1);
        let delay_ms = (u64::from(self.delta) * 1000)
            .checked_div(u64::from(self.timescale))
            .map_or(0, |ms| u32::try_from(ms).unwrap_or(u32::MAX));
        Ok((bytes, delay_ms))
    }
}

impl<'a> Iterator for Samples<'a> {
    type Item = Result<(&'a [u8], u32), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample >= self.count {
            return None;
        }
        let sample = self.next_sample();
        if sample.is_err() {
            self.count = 0;
        }
        Some(sample)
    }
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

/// An open decoder that a sequence's samples go through in order, so a frame that predicts from
/// an earlier one finds it.
struct Stream {
    // The guard is kept for its drop, which closes `context`.
    _decoder: Decoder,
    context: Dav1dContext,
}

impl Stream {
    /// Opens a decoder under a frame size limit of `cap`.
    fn open(cap: u64) -> Result<Stream, Error> {
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
        Ok(Stream {
            _decoder: decoder,
            context,
        })
    }

    /// Decodes the next frame of the stream from the AV1 bitstream `obus`.
    fn decode(&mut self, obus: &[u8]) -> Result<Picture, Error> {
        let context = self.context;
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
        Ok(picture)
    }
}

impl Picture {
    /// Decodes the first frame of the AV1 bitstream `obus`, under a frame size limit of `cap`.
    fn decode(obus: &[u8], cap: u64) -> Result<Picture, Error> {
        Stream::open(cap)?.decode(obus)
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
