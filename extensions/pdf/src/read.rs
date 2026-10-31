//! Reading an existing PDF: its page count, metadata, attachments and text
//! (`rule:core-classes/pdf-reader-and-merge`), through `hayro-syntax` and `hayro-interpret`.
//!
//! [`open`] parses the bytes. **An encrypted document returns `Parse`**, whatever its password: it
//! is opened with a password nobody sets, and a document that opens with it is not encrypted, as
//! the image component's decoder does. A document that does not parse returns `Parse` too. `merge`
//! and placing a page open their sources here, so all three jobs refuse the same files.
//!
//! [`read`] returns the page count, the six metadata strings of the document's information
//! dictionary, and every file of the catalog's `EmbeddedFiles` name tree, under its `/UF` name or
//! else its `/F` one. A text string in UTF-16 or UTF-8 with its byte-order mark is decoded as such,
//! and any other is read as Latin-1, which differs from PDFDocEncoding only in a few punctuation
//! marks. The invoice XML is the attachment named `factur-x.xml`, `zugferd-invoice.xml` or
//! `xrechnung.xml`, in any case. One that is not UTF-8 returns `Parse`, and so does one that
//! parses and nests deeper than `factur_x`'s bound (`factur_x`'s module doc). The name tree is walked
//! at most [`MAX_DEPTH`] levels deep, so a cycle in it ends.
//!
//! [`read`] also returns the parsed document as a [`Reading`], which the guest returns as the WIT
//! `reading` resource. [`text`] reads that, so a reader's file crosses into the component once,
//! and is parsed once, however many pages it reads.
//!
//! **Every job decodes under a [`Bound`]** (`rule:core-classes/decompression-bound`). [`under`]
//! sets it on the thread for the job, and the repository's patched `hayro-syntax` stops each
//! stream's decode once its output passes `min(compressed size × ratio, ceiling)`. That covers
//! the cross-reference and object streams [`open`] reads, the content and font streams [`text`]
//! interprets, and each attachment [`read`] returns. A job that passed the bound returns `Parse`
//! naming the half that stopped it, even where the interpreter went on without that stream. The
//! attachments [`read`] returns are held together, so their total counts against the ceiling too.
//! An image's pixels are not counted, because its own dimensions bound them.
//!
//! [`text`] interprets each page asked for with a device that keeps only glyphs, and writes each
//! glyph's Unicode value in the order the content stream draws it. A glyph whose baseline moves by
//! more than half its size, or that starts more than half its size before the last glyph ended,
//! starts a new line. A gap wider than a fifth of its size is a space.
//! A glyph with no Unicode value is skipped. Nothing rebuilds a table or a column.
//!
//! Each of the 14 standard fonts a document names without embedding is read through one of the
//! Liberation faces, so its glyphs reach the device. No predefined CMap is carried, so text in a
//! CJK encoding that names one has no Unicode value. The interpreter reads no clock and no file.

use std::sync::Arc;

use hayro_interpret::font::{FontData, FontQuery, Glyph, StandardFont};
use hayro_interpret::hayro_syntax::bound::Breach;
use hayro_interpret::hayro_syntax::object::{Array, Dict, Object, Stream};
use hayro_interpret::hayro_syntax::{LoadPdfError, Pdf};
use hayro_interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache,
    InterpreterSettings, Paint, PathDrawMode, SoftMask, interpret_page,
};
use kurbo::{Affine, BezPath, Point, Rect};

use crate::fonts::{self, Family};
use crate::{Error, factur_x};

/// The password an encrypted document is opened with to find out that it is encrypted.
const NO_PASSWORD: &str = "\u{0}novis: a PDF is never decrypted\u{0}";

/// How many levels of `/Kids` the `EmbeddedFiles` name tree is followed.
pub const MAX_DEPTH: usize = 32;

/// The attachment names a received e-invoice carries its XML under: Factur-X, ZUGFeRD 2 and
/// XRechnung.
const INVOICE_NAMES: [&str; 3] = ["factur-x.xml", "zugferd-invoice.xml", "xrechnung.xml"];

/// The WIT `metadata`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
}

/// The WIT `pdf-info`: the page count, the metadata, each attachment's name and bytes, and the
/// invoice XML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    pub page_count: u64,
    pub metadata: Metadata,
    pub attachments: Vec<(String, Vec<u8>)>,
    pub invoice_xml: Option<String>,
}

/// The WIT `decompression-bound` after the host's clamp: the most bytes one stream decodes to,
/// and the most bytes of output per byte of input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bound {
    pub max_bytes: u64,
    pub max_ratio: u64,
}

impl Default for Bound {
    /// The shipped `[limits] max_decompressed` and `max_decompression_ratio`.
    fn default() -> Self {
        Bound {
            max_bytes: 64 << 20,
            max_ratio: 1000,
        }
    }
}

/// `job` with every decode on this thread under `bound`. A job that passed the bound returns
/// `Parse` naming the half that stopped it, whatever the job returned.
pub fn under<T>(bound: Bound, job: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    hayro_interpret::hayro_syntax::bound::set(bound.max_bytes, bound.max_ratio);
    let result = job();
    match hayro_interpret::hayro_syntax::bound::take() {
        None => result,
        Some(Breach::Ratio) => Err(over_ratio(bound.max_ratio)),
        Some(Breach::Ceiling) => Err(over_bytes(bound.max_bytes)),
    }
}

fn over_ratio(ratio: u64) -> Error {
    Error::Parse(format!(
        "A stream in the PDF decompresses to more than {ratio} times its compressed size. \
         To allow a larger ratio, raise `[limits] max_decompression_ratio` in the server's \
         configuration."
    ))
}

fn over_bytes(bytes: u64) -> Error {
    Error::Parse(format!(
        "The PDF decompresses to more than {bytes} bytes. To allow more, raise \
         `[limits] max_decompressed` in the server's configuration."
    ))
}

/// The parsed document. An encrypted one and one that does not parse return `Parse`.
pub fn open(data: Vec<u8>) -> Result<Pdf, Error> {
    Pdf::new_with_password(data, NO_PASSWORD).map_err(|err| match err {
        LoadPdfError::Decryption(_) => Error::Parse(
            "the PDF is encrypted. Only a PDF without encryption can be read, merged or placed"
                .to_owned(),
        ),
        LoadPdfError::Invalid => Error::Parse("the PDF document does not parse".to_owned()),
    })
}

/// The index of `page`, counted from 1, in a document of `count` pages; `Invalid` naming the
/// count when it is outside.
pub fn index(page: u64, count: usize) -> Result<usize, Error> {
    page.checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .filter(|&index| index < count)
        .ok_or_else(|| {
            Error::Invalid(format!(
                "page {page} is outside the PDF, which has {count} page{}",
                if count == 1 { "" } else { "s" }
            ))
        })
}

/// The WIT `reading`: a document [`read`] opened, parsed once and kept for each [`text`] call.
/// Memory: the file and its cross-reference table, per open reader.
pub struct Reading(Pdf);

impl std::fmt::Debug for Reading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reading")
            .field("pages", &self.0.pages().len())
            .finish_non_exhaustive()
    }
}

/// The document `data` opened for [`text`], with its page count, metadata and attachments,
/// decoded under `bound`.
pub fn read(data: Vec<u8>, bound: Bound) -> Result<(Reading, Info), Error> {
    under(bound, || {
        let pdf = open(data)?;
        let info = read_info(&pdf, bound)?;
        Ok((Reading(pdf), info))
    })
}

fn read_info(pdf: &Pdf, bound: Bound) -> Result<Info, Error> {
    let meta = pdf.metadata();
    let decode = |field: &Option<Vec<u8>>| field.as_deref().map(text_string);
    let metadata = Metadata {
        title: decode(&meta.title),
        author: decode(&meta.author),
        subject: decode(&meta.subject),
        keywords: decode(&meta.keywords),
        creator: decode(&meta.creator),
        producer: decode(&meta.producer),
    };
    let mut attachments = Vec::new();
    let xref = pdf.xref();
    let tree = xref
        .get::<Dict<'_>>(xref.root_id())
        .and_then(|catalog| catalog.get::<Dict<'_>>(b"Names"))
        .and_then(|names| names.get::<Dict<'_>>(b"EmbeddedFiles"));
    if let Some(tree) = tree {
        let mut room = bound.max_bytes;
        walk(&tree, 0, &mut attachments, &mut room).ok_or_else(|| over_bytes(bound.max_bytes))?;
    }
    let invoice_xml = attachments
        .iter()
        .find(|(name, _)| {
            INVOICE_NAMES
                .iter()
                .any(|invoice| name.eq_ignore_ascii_case(invoice))
        })
        .map(|(name, data)| {
            let xml = String::from_utf8(data.clone())
                .map_err(|_| Error::Parse(format!("the attachment `{name}` is not UTF-8")))?;
            match factur_x::depth_error(&xml) {
                Some(err) => Err(err),
                None => Ok(xml),
            }
        })
        .transpose()?;
    Ok(Info {
        page_count: pdf.pages().len() as u64,
        metadata,
        attachments,
        invoice_xml,
    })
}

/// Adds each file of the name tree node `node` to `out`, in the tree's order, and takes its
/// size from `room`. `None` once the files together are larger than `room` was.
fn walk(
    node: &Dict<'_>,
    depth: usize,
    out: &mut Vec<(String, Vec<u8>)>,
    room: &mut u64,
) -> Option<()> {
    if depth >= MAX_DEPTH {
        return Some(());
    }
    if let Some(kids) = node.get::<Array<'_>>(b"Kids") {
        for kid in kids.iter::<Dict<'_>>() {
            walk(&kid, depth + 1, out, room)?;
        }
    }
    let Some(names) = node.get::<Array<'_>>(b"Names") else {
        return Some(());
    };
    let mut entries = names.iter::<Object<'_>>();
    while let (Some(key), Some(spec)) = (entries.next(), entries.next()) {
        let Some(spec) = spec.into_dict() else {
            continue;
        };
        if let Some(file) = attachment(&spec, key) {
            *room = room.checked_sub(file.1.len() as u64)?;
            out.push(file);
        }
    }
    Some(())
}

/// The name and bytes of the file specification `spec`, whose key in the name tree is `key`.
fn attachment(spec: &Dict<'_>, key: Object<'_>) -> Option<(String, Vec<u8>)> {
    let name = [b"UF".as_slice(), b"F"]
        .iter()
        .find_map(|field| spec.get::<hayro_interpret::hayro_syntax::object::String<'_>>(field))
        .map(|name| text_string(name.as_bytes()))
        .or_else(|| key.into_string().map(|key| text_string(key.as_bytes())))?;
    let files = spec.get::<Dict<'_>>(b"EF")?;
    let stream = files
        .get::<Stream<'_>>(b"UF")
        .or_else(|| files.get::<Stream<'_>>(b"F"))?;
    let data = stream.decoded().ok()?.into_owned();
    Some((name, data))
}

/// A PDF text string: UTF-16BE or UTF-16LE, or UTF-8, after its byte-order mark, and Latin-1
/// without one.
fn text_string(bytes: &[u8]) -> String {
    let units = |rest: &[u8], big: bool| {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| {
                if big {
                    u16::from_be_bytes([pair[0], pair[1]])
                } else {
                    u16::from_le_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xfe, 0xff, rest @ ..] => units(rest, true),
        [0xff, 0xfe, rest @ ..] => units(rest, false),
        [0xef, 0xbb, 0xbf, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => bytes.iter().map(|&byte| char::from(byte)).collect(),
    }
}

/// The text of each page in `pages`, counted from 1, of the document `file` holds, decoded under
/// `bound`. A page outside the document returns `Invalid` naming the count, before any page is
/// read.
pub fn text(file: &Reading, pages: &[u64], bound: Bound) -> Result<Vec<String>, Error> {
    under(bound, || page_text(&file.0, pages))
}

fn page_text(pdf: &Pdf, pages: &[u64]) -> Result<Vec<String>, Error> {
    let all = pdf.pages();
    let indices = pages
        .iter()
        .map(|&page| index(page, all.len()))
        .collect::<Result<Vec<_>, _>>()?;
    let cache = InterpreterCache::new();
    Ok(indices
        .into_iter()
        .map(|index| {
            let page = &all[index];
            let (width, height) = page.render_dimensions();
            let bbox = Rect::new(0.0, 0.0, f64::from(width), f64::from(height));
            let mut context = Context::new(Affine::IDENTITY, bbox, &cache, pdf.xref(), settings());
            let mut device = TextDevice::default();
            interpret_page(page, &mut context, &mut device);
            device.out
        })
        .collect())
}

/// The interpreter's settings: the Liberation faces for the standard fonts, and no CMap.
pub fn settings() -> InterpreterSettings {
    InterpreterSettings {
        font_resolver: Arc::new(|query| {
            let font = match query {
                FontQuery::Standard(font) => *font,
                FontQuery::Fallback(query) => query.pick_standard_font(),
            };
            let data: FontData = Arc::new(substitute(font));
            Some((data, 0))
        }),
        cmap_resolver: Arc::new(|_| None),
        ..InterpreterSettings::default()
    }
}

/// The Liberation face that replaces `font`. Symbol is Serif and ZapfDingbats is Sans.
fn substitute(font: StandardFont) -> &'static [u8] {
    use StandardFont as S;
    let (family, bold, italic) = match font {
        S::Helvetica | S::ZapfDingBats => (Family::Sans, false, false),
        S::HelveticaBold => (Family::Sans, true, false),
        S::HelveticaOblique => (Family::Sans, false, true),
        S::HelveticaBoldOblique => (Family::Sans, true, true),
        S::TimesRoman | S::Symbol => (Family::Serif, false, false),
        S::TimesBold => (Family::Serif, true, false),
        S::TimesItalic => (Family::Serif, false, true),
        S::TimesBoldItalic => (Family::Serif, true, true),
        S::Courier => (Family::Mono, false, false),
        S::CourierBold => (Family::Mono, true, false),
        S::CourierOblique => (Family::Mono, false, true),
        S::CourierBoldOblique => (Family::Mono, true, true),
    };
    fonts::face(family, bold, italic)
}

/// A device that writes the Unicode value of each glyph it is given, and draws nothing.
#[derive(Default)]
struct TextDevice {
    out: String,
    /// The origin of the last glyph written, its size, and where it ends along its line.
    last: Option<(Point, f64, f64)>,
}

impl<'a> Device<'a> for TextDevice {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_image(&mut self, _: Image<'a, '_>, _: Affine) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph(
        &mut self,
        glyph: &Glyph<'a>,
        transform: Affine,
        glyph_transform: Affine,
        _: &Paint<'a>,
        _: &GlyphDrawMode,
    ) {
        let Some(value) = glyph.as_unicode() else {
            return;
        };
        let value = match value {
            hayro_interpret::hayro_cmap::BfString::Char(c) => c.to_string(),
            hayro_interpret::hayro_cmap::BfString::String(s) => s,
        };
        let whole = transform * glyph_transform;
        let origin = whole * Point::ZERO;
        let size = (whole * Point::new(0.0, 1000.0) - origin).hypot().max(0.1);
        let advance = match glyph {
            Glyph::Outline(outline) => outline.advance_width().map(f64::from),
            Glyph::Type3(_) => None,
        };
        let end = (whole * Point::new(advance.unwrap_or(0.0), 0.0)).x;
        if let Some((last, last_size, last_end)) = self.last {
            let ends_space = self.out.ends_with(char::is_whitespace);
            let half = last_size.min(size) / 2.0;
            if (origin.y - last.y).abs() > half || last_end - origin.x > half {
                if !self.out.ends_with('\n') {
                    self.out.push('\n');
                }
            } else if origin.x - last_end > size / 5.0 && !ends_space && !value.starts_with(' ') {
                self.out.push(' ');
            }
        }
        self.out.push_str(&value);
        self.last = Some((origin, size, end));
    }
}
