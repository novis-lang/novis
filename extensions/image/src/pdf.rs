//! Rasterising one page of a PDF document through `hayro`
//! (`rule:core-classes/pdf-page-is-an-image-source`, `rule:core-classes/pdf-decode-refusals`).
//!
//! **One page per call, priced before it is drawn.** The page's box, in points and turned by its
//! rotation, is scaled to the requested `dpi`, each side rounded up to a whole pixel, and held to
//! the cap before the render context exists. A page past the end returns `Runtime` naming the
//! count. The page is drawn on white, so every pixel is opaque.
//!
//! **Only the bytes given are read.** `hayro` reads no file and no address. Each of the 14
//! standard fonts a document may name without embedding is one of the Liberation fonts under
//! `extensions/fonts/`: Helvetica is Sans, Times is Serif and Courier is Mono, each in its four
//! styles. Symbol is Serif and ZapfDingbats is Sans, which have no glyph for most of their
//! symbols. A font
//! the document names and does not embed is the standard font closest to its flags. No predefined
//! CMap is carried, so text in a CJK encoding that names one is not drawn.
//!
//! **An encrypted document returns `Parse`**, whatever its password: it is opened with a password
//! nobody sets, and a document that opens with it is not encrypted. A font or an image the
//! interpreter meets and cannot decode returns `Runtime` naming it, so a page is drawn whole or
//! not at all. `hayro` stops interpreting form XObjects and patterns nested deeper than 50, and
//! the guest's memory cap and deadline bound the rest of a hostile document.
//!
//! The same bytes and options give the same pixels: the interpreter reads no clock and runs on
//! one thread.

use std::sync::{Arc, Mutex};

use hayro::hayro_interpret::font::{FontData, FontQuery, StandardFont};
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{InterpreterSettings, InterpreterWarning};
use hayro::hayro_syntax::{LoadPdfError, Pdf};
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::peniko::ImageAlphaType;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{RenderCache, RenderSettings, render_into};

use crate::decode::{Pixels, check};
use crate::{Error, Format};

/// The password an encrypted document is opened with to find out that it is encrypted.
const NO_PASSWORD: &str = "\u{0}novis: a PDF is never decrypted\u{0}";

/// Whether `data` reads as a PDF document: `%PDF-` in its first 1024 bytes, where readers look.
pub(crate) fn sniff(data: &[u8]) -> bool {
    data[..data.len().min(1024)]
        .windows(5)
        .any(|window| window == b"%PDF-")
}

/// The parsed document. An encrypted one and one that does not parse return `Parse`.
fn open(data: &[u8]) -> Result<Pdf, Error> {
    Pdf::new_with_password(data.to_vec(), NO_PASSWORD).map_err(|err| match err {
        LoadPdfError::Decryption(_) => Error::Parse(
            "the PDF is encrypted. Only a PDF without encryption can be opened as an image"
                .to_owned(),
        ),
        LoadPdfError::Invalid => Error::Parse("the PDF document does not parse".to_owned()),
    })
}

/// A page's box in points, with its rotation applied.
fn points(pdf: &Pdf, page: u64) -> Result<(f32, f32), Error> {
    let pages = pdf.pages();
    let count = pages.len() as u64;
    let found = page
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| pages.get(index))
        .ok_or_else(|| crate::past_end(page, count))?;
    Ok(found.render_dimensions())
}

/// The pixel size of a box of `width` by `height` points at `dpi`, each side rounded up.
fn scaled((width, height): (f32, f32), dpi: f64) -> (u64, u64) {
    let side = |points: f32| (f64::from(points) * dpi / 72.0).ceil().max(1.0) as u64;
    (side(width), side(height))
}

/// The pixel size of `page` of the document `data` at `dpi`, read without drawing it.
pub(crate) fn size(data: &[u8], page: u64, dpi: f64) -> Result<(u64, u64), Error> {
    Ok(scaled(points(&open(data)?, page)?, dpi))
}

/// The page count and the first page's box in whole points, read from the page tree alone.
pub(crate) fn info(data: &[u8]) -> Result<crate::info::Info, Error> {
    let pdf = open(data)?;
    let frames = pdf.pages().len() as u64;
    let (width, height) = match frames {
        0 => (0, 0),
        _ => scaled(points(&pdf, 1)?, 72.0),
    };
    Ok(crate::info::Info {
        format: Format::Pdf,
        width,
        height,
        has_alpha: false,
        frames,
        orientation: 1,
        exif: None,
        has_icc: false,
    })
}

/// The Liberation font that replaces `font`.
fn substitute(font: &StandardFont) -> &'static [u8] {
    macro_rules! liberation {
        ($name:literal) => {
            include_bytes!(concat!("../../fonts/Liberation", $name, ".ttf")).as_slice()
        };
    }
    match font {
        StandardFont::Helvetica | StandardFont::ZapfDingBats => liberation!("Sans-Regular"),
        StandardFont::HelveticaBold => liberation!("Sans-Bold"),
        StandardFont::HelveticaOblique => liberation!("Sans-Italic"),
        StandardFont::HelveticaBoldOblique => liberation!("Sans-BoldItalic"),
        StandardFont::TimesRoman | StandardFont::Symbol => liberation!("Serif-Regular"),
        StandardFont::TimesBold => liberation!("Serif-Bold"),
        StandardFont::TimesItalic => liberation!("Serif-Italic"),
        StandardFont::TimesBoldItalic => liberation!("Serif-BoldItalic"),
        StandardFont::Courier => liberation!("Mono-Regular"),
        StandardFont::CourierBold => liberation!("Mono-Bold"),
        StandardFont::CourierOblique => liberation!("Mono-Italic"),
        StandardFont::CourierBoldOblique => liberation!("Mono-BoldItalic"),
    }
}

/// The twelve Liberation faces: Sans, Serif and Mono, each regular, bold, italic and bold italic.
pub(crate) fn liberation() -> [&'static [u8]; 12] {
    use StandardFont::*;
    [
        Helvetica,
        HelveticaBold,
        HelveticaOblique,
        HelveticaBoldOblique,
        TimesRoman,
        TimesBold,
        TimesItalic,
        TimesBoldItalic,
        Courier,
        CourierBold,
        CourierOblique,
        CourierBoldOblique,
    ]
    .map(|font| substitute(&font))
}

/// Liberation Sans Regular, the font `barcode` prints a code's text in.
pub(crate) fn sans() -> &'static [u8] {
    substitute(&StandardFont::Helvetica)
}

/// The interpreter's settings: the Liberation fonts, no CMap, and every warning kept in `seen`.
fn settings(seen: &Arc<Mutex<Option<InterpreterWarning>>>) -> InterpreterSettings {
    let sink = Arc::clone(seen);
    InterpreterSettings {
        font_resolver: Arc::new(|query| {
            let font = match query {
                FontQuery::Standard(font) => *font,
                FontQuery::Fallback(query) => query.pick_standard_font(),
            };
            let data: FontData = Arc::new(substitute(&font));
            Some((data, 0))
        }),
        cmap_resolver: Arc::new(|_| None),
        warning_sink: Arc::new(move |warning| {
            let mut first = sink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            first.get_or_insert(warning);
        }),
        render_annotations: true,
    }
}

/// Draws `page` of the document `data` at `dpi` on white, refusing a page over `cap` pixels
/// before the render context is allocated.
pub(crate) fn decode(data: &[u8], cap: u64, page: u64, dpi: f64) -> Result<Pixels, Error> {
    let pdf = open(data)?;
    let (width, height) = scaled(points(&pdf, page)?, dpi);
    check(width, height, 1, cap)?;
    let too_big = || {
        Error::Runtime(format!(
            "page {page} at {dpi} dpi is {width}x{height} pixels. A side may be at most 65535 pixels"
        ))
    };
    let w = u16::try_from(width).map_err(|_| too_big())?;
    let h = u16::try_from(height).map_err(|_| too_big())?;
    let page_ref = &pdf.pages()[page as usize - 1];
    let seen = Arc::default();
    let cache = RenderCache::new();
    let mut ctx = RenderContext::new(w, h);
    let scale = dpi / 72.0;
    render_into(
        page_ref,
        &cache,
        &settings(&seen),
        &RenderSettings::default(),
        &mut ctx,
        Affine::scale(scale) * page_ref.initial_transform(true).to_kurbo(),
    );
    let warning = seen
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    match warning {
        Some(InterpreterWarning::UnsupportedFont) => {
            return Err(Error::Runtime(format!(
                "page {page} uses a font in an encoding the image component cannot draw"
            )));
        }
        Some(InterpreterWarning::ImageDecodeFailure) => {
            return Err(Error::Runtime(format!(
                "page {page} contains an image that does not decode"
            )));
        }
        _ => {}
    }
    ctx.flush();
    let mut pixmap = Pixmap::new(w, h);
    ctx.render_with(
        &mut pixmap,
        &mut Resources::default(),
        RasterizerSettings {
            target_init: TargetInit::Clear(WHITE),
            ..Default::default()
        },
    );
    Ok(Pixels {
        width: u32::from(w),
        height: u32::from(h),
        rgba: pixmap.take_rgba8(ImageAlphaType::AlphaPremultiplied),
        exif: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document of `pages` pages of 200 by 100 points, each filled red.
    fn red_pages(pages: usize) -> Vec<u8> {
        let content = "1 0 0 rg 0 0 200 100 re f";
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {pages} >>",
                (0..pages)
                    .map(|at| format!("{} 0 R", 3 + at))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        let stream = 3 + pages;
        for _ in 0..pages {
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents {stream} 0 R >>"
            ));
        }
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (at, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{body}\nendobj\n", at + 1).bytes());
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
        for offset in offsets {
            out.extend(format!("{offset:010} 00000 n \n").bytes());
        }
        out.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .bytes(),
        );
        out
    }

    #[test]
    fn a_page_is_drawn_at_its_dpi() {
        let data = red_pages(3);
        assert!(sniff(&data));
        let page = decode(&data, 1 << 20, 2, 144.0).unwrap();
        assert_eq!((page.width, page.height), (400, 200));
        assert_eq!(&page.rgba[..4], &[255, 0, 0, 255]);
        assert_eq!(size(&data, 1, 100.0).unwrap(), (278, 139));
        assert_eq!(
            decode(&data, 1 << 20, 4, 72.0).unwrap_err(),
            crate::past_end(4, 3)
        );
    }

    #[test]
    fn info_counts_the_pages() {
        let got = info(&red_pages(5)).unwrap();
        assert_eq!((got.format, got.frames), (Format::Pdf, 5));
        assert_eq!((got.width, got.height), (200, 100));
    }

    #[test]
    fn an_encrypted_document_is_refused_though_it_has_no_user_password() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/encrypted.pdf");
        let data = std::fs::read(path).unwrap();
        // `hayro` alone opens it, because its user password is empty.
        assert!(Pdf::new(data.clone()).is_ok());
        assert!(matches!(open(&data), Err(Error::Parse(_))));
    }
}
