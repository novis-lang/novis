//! Lossy WebP through the linked libwebp (`rule:core-classes/image-format-roster`).
//!
//! `image` writes only lossless WebP, so a lossy one is libwebp's `WebPEncodeRGBA`, from the
//! static library `build.rs` links into the wasm build. A host build links no libwebp, and
//! `lossy` returns `Runtime` there.
//!
//! libwebp's simple interface writes no metadata. A kept EXIF block is added to its output here:
//! the file gets a `VP8X` header chunk with the EXIF flag set, and an `EXIF` chunk after the image
//! data, which is the order the WebP container gives.

use crate::Error;

/// `rgba`, `width` by `height` RGBA8 pixels, as a lossy WebP file at `quality` (1 to 100).
#[cfg(target_family = "wasm")]
pub fn lossy(rgba: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, Error> {
    use std::ffi::c_void;

    unsafe extern "C" {
        fn WebPEncodeRGBA(
            rgba: *const u8,
            width: i32,
            height: i32,
            stride: i32,
            quality_factor: f32,
            output: *mut *mut u8,
        ) -> usize;
        fn WebPFree(ptr: *mut c_void);
    }

    let too_large = || {
        Error::Runtime(format!(
            "a {width} by {height} image is too large for a WebP file"
        ))
    };
    let columns = i32::try_from(width).map_err(|_| too_large())?;
    let rows = i32::try_from(height).map_err(|_| too_large())?;
    let stride = columns.checked_mul(4).ok_or_else(too_large)?;
    if rgba.len() < (width as usize) * (height as usize) * 4 {
        return Err(Error::Runtime(
            "the pixel buffer is shorter than its width and height".to_string(),
        ));
    }
    let mut output: *mut u8 = std::ptr::null_mut();
    // SAFETY: `rgba` holds `rows` rows of `stride` bytes, checked above, and `output` is a live
    // pointer libwebp writes once. On failure libwebp returns 0 and allocates nothing.
    let length = unsafe {
        WebPEncodeRGBA(
            rgba.as_ptr(),
            columns,
            rows,
            stride,
            f32::from(quality),
            &raw mut output,
        )
    };
    if length == 0 || output.is_null() {
        return Err(Error::Runtime(format!(
            "the WebP encoder failed for a {width} by {height} image"
        )));
    }
    // SAFETY: libwebp allocated `length` bytes at `output`; they are copied out and then freed
    // with libwebp's own allocator, once.
    let file = unsafe {
        let file = std::slice::from_raw_parts(output, length).to_vec();
        WebPFree(output.cast());
        file
    };
    Ok(file)
}

/// A host build links no libwebp.
#[cfg(not(target_family = "wasm"))]
pub fn lossy(_rgba: &[u8], _width: u32, _height: u32, _quality: u8) -> Result<Vec<u8>, Error> {
    Err(Error::Runtime(
        "lossy WebP is encoded by libwebp, which only the wasm build links".to_string(),
    ))
}

/// The flag in a `VP8X` chunk's first byte that says the file has an `EXIF` chunk.
const EXIF_FLAG: u8 = 0x08;

/// The WebP `file`, `width` by `height`, with `exif` added as an `EXIF` chunk.
pub fn with_exif(file: &[u8], exif: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Error> {
    let malformed = || Error::Runtime("the WebP encoder wrote a malformed container".to_string());
    if file.len() < 20 || &file[..4] != b"RIFF" || &file[8..12] != b"WEBP" {
        return Err(malformed());
    }
    let chunks = &file[12..];
    let mut out = file[..12].to_vec();
    if &chunks[..4] == b"VP8X" {
        if chunks.len() < 18 {
            return Err(malformed());
        }
        out.extend_from_slice(chunks);
        out[12 + 8] |= EXIF_FLAG;
    } else {
        out.extend_from_slice(b"VP8X");
        out.extend_from_slice(&10u32.to_le_bytes());
        out.extend_from_slice(&[EXIF_FLAG, 0, 0, 0]);
        out.extend_from_slice(&width.saturating_sub(1).to_le_bytes()[..3]);
        out.extend_from_slice(&height.saturating_sub(1).to_le_bytes()[..3]);
        out.extend_from_slice(chunks);
    }
    let size = u32::try_from(exif.len()).map_err(|_| malformed())?;
    out.extend_from_slice(b"EXIF");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(exif);
    if exif.len() % 2 == 1 {
        out.push(0);
    }
    let riff = u32::try_from(out.len() - 8).map_err(|_| malformed())?;
    out[4..8].copy_from_slice(&riff.to_le_bytes());
    Ok(out)
}
