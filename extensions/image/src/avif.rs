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

use std::io::Cursor;
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

use crate::Error;
use crate::decode::{Pixels, check};

fn parse(what: impl std::fmt::Display) -> Error {
    Error::Parse(format!("the AVIF file does not parse: {what}"))
}

/// Decodes the AVIF file `data` to RGBA8, refusing one over `cap` pixels before a buffer exists.
pub(crate) fn decode(data: &[u8], cap: u64) -> Result<Pixels, Error> {
    let avif = avif_parse::read_avif(&mut Cursor::new(data)).map_err(|err| parse(format!("{err:?}")))?;
    let meta = avif
        .primary_item_metadata()
        .map_err(|err| parse(format!("{err:?}")))?;
    check(
        u64::from(meta.max_frame_width.get()),
        u64::from(meta.max_frame_height.get()),
        cap,
    )?;
    let color = Picture::decode(&avif.primary_item, cap)?;
    let (width, height) = color.size();
    check(u64::from(width), u64::from(height), cap)?;
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
    Ok(Pixels {
        width,
        height,
        rgba,
        exif: None,
    })
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
                return Err(Error::Runtime(
                    "the AVIF decoder did not start".to_string(),
                ));
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
                let sent = unsafe { dav1d_send_data(Some(context), Some(NonNull::from(&mut data.0))) };
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
        if self.0.p.layout == DAV1D_PIXEL_LAYOUT_I400 { 1 } else { 3 }
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
        let header = self.0.seq_hdr.expect("checked when the picture was decoded");
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
