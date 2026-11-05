//! The pictures a document draws: an `<img>`, an inline `<svg>` and a background image, each
//! decoded far enough to know its size and placed as a box's background layer or as a replaced
//! box.
//!
//! An `<img>` or an inline `<svg>` whose `display` is `inline` or `inline-block` is an inline
//! box in its line, as an `inline-block` is ([`super::Atom`]): its `vertical-align` places it on
//! the line, and its margin, border and padding are drawn around it. A floated one is a float,
//! and any other `display` makes it a block of its own. Its size comes from `width` and `height`,
//! then the `width` and `height` attributes, then the picture, and `min-width`, `max-width`,
//! `min-height` and `max-height` limit it as CSS 2.1 §10.4 says, keeping its aspect ratio when no
//! size is given. A percent height counts against the page's content height, the only height a
//! document has. A picture wider than its line or taller than the page shrinks to fit, keeping
//! its aspect ratio. A JPEG's EXIF orientation turns and mirrors it ([`Turn`]).
//!
//! Cost: one inline box in its line for each inline picture. Memory: nothing more per request.
//!
//! A PDF asset is one of its pages, the first unless the reference ends in `#page=N`, drawn as
//! the page's own vector content at the page's size in points. Only its appearance is drawn: its
//! links, form fields and bookmarks are not. It opens as `read`'s module doc says, so an
//! encrypted PDF returns `Parse`. Every pass runs under the options' `read::Bound`, so a placed
//! page's streams decode under it here and when the writer copies them (`crate::render`).
//!
//! Cost: an `<img>` or a background image reference is resolved and decoded once per render,
//! and each later use of it draws the same picture ([`super::Work::image_decodes`]). An inline
//! `<svg>` is parsed at each use. Memory: one reference and one index for each distinct
//! picture, until the document's blocks are collected.

use std::sync::{Arc, Mutex};

use krilla::Data;
use krilla::image::Image;
use krilla::pdf::PdfDocument;

use super::block::sides;
use super::inline::Builder;
use super::inline::vertical_align;
use super::{Block, Frame, Images, Role, boxes, keyword, length};
use crate::Error;
use crate::assets::Assets;
use crate::css::value::{self, Comp};
use crate::css::{Style, Value};
use crate::parse::{Dom, Kind, Space};

/// A picture an [`super::Item::Image`] or a background draws.
pub enum Picture {
    /// A PNG, JPEG, GIF or WebP image, decoded far enough to know its size, and how a JPEG's
    /// EXIF orientation turns it.
    Raster(Image, Turn),
    /// A parsed SVG document, drawn as vector paths.
    Vector(Box<usvg::Tree>),
    /// One page of an existing PDF, drawn as its own vector content.
    Page(Box<PdfPage>),
}

/// A page of an existing PDF: the document, the page's index from 0, and its size in points
/// with its rotation applied.
pub struct PdfPage {
    pub document: PdfDocument,
    pub index: usize,
    pub size: (f32, f32),
}

impl Picture {
    /// The natural size in points: a pixel, or an SVG user unit, is three quarters of a point,
    /// and a PDF page is its own size.
    pub fn size(&self) -> (f32, f32) {
        match self {
            Picture::Raster(image, turn) => {
                let (w, h) = image.size();
                let (w, h) = (w as f32 * 0.75, h as f32 * 0.75);
                if turn.0 >= 5 { (h, w) } else { (w, h) }
            }
            Picture::Vector(tree) => (tree.size().width() * 0.75, tree.size().height() * 0.75),
            Picture::Page(page) => page.size,
        }
    }
}

/// A JPEG's EXIF orientation, from 1 to 8: 1 is upright, 2 to 4 mirror or turn it half a turn,
/// and 5 to 8 also swap its width and height. Every other picture is 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turn(pub u8);

impl Turn {
    pub const UPRIGHT: Turn = Turn(1);

    /// The matrix `[a, b, c, d, e, f]` that draws the stored picture, `w` by `h` as it is
    /// stored, into the box `width` by `height` it is shown in, y downwards.
    pub fn matrix(self, width: f32, height: f32) -> [f32; 6] {
        match self.0 {
            2 => [-1.0, 0.0, 0.0, 1.0, width, 0.0],
            3 => [-1.0, 0.0, 0.0, -1.0, width, height],
            4 => [1.0, 0.0, 0.0, -1.0, 0.0, height],
            5 => [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
            6 => [0.0, 1.0, -1.0, 0.0, width, 0.0],
            7 => [0.0, -1.0, -1.0, 0.0, width, height],
            8 => [0.0, -1.0, 1.0, 0.0, 0.0, height],
            _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        }
    }

    /// The orientation a JPEG's first APP1 segment gives in its EXIF data, or upright when it
    /// has none or the segment does not read. Only the segments before the image data are read.
    pub fn of_jpeg(data: &[u8]) -> Turn {
        let mut at = 2;
        while let [0xff, marker, hi, lo, ..] = data.get(at..).unwrap_or(&[]) {
            let length = usize::from(*hi) << 8 | usize::from(*lo);
            let Some(segment) = data.get(at + 4..at + 2 + length) else {
                break;
            };
            match marker {
                0xe1 if segment.starts_with(b"Exif\0\0") => {
                    return exif_orientation(&segment[6..]).map_or(Turn::UPRIGHT, Turn);
                }
                // The start of the scan, the image data: no EXIF segment comes after it.
                0xda | 0xd9 => break,
                _ => at += 2 + length,
            }
        }
        Turn::UPRIGHT
    }
}

/// The orientation tag, 0x0112, of the TIFF data `tiff` in its first image directory, when it
/// is from 1 to 8.
fn exif_orientation(tiff: &[u8]) -> Option<u8> {
    let big = match tiff.get(..4)? {
        [b'M', b'M', 0, 42] => true,
        [b'I', b'I', 42, 0] => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let b: [u8; 2] = tiff.get(at..at + 2)?.try_into().ok()?;
        Some(if big {
            u16::from_be_bytes(b)
        } else {
            u16::from_le_bytes(b)
        })
    };
    let u32_at = |at: usize| -> Option<usize> {
        let b: [u8; 4] = tiff.get(at..at + 4)?.try_into().ok()?;
        let v = if big {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        };
        usize::try_from(v).ok()
    };
    let directory = u32_at(4)?;
    let count = usize::from(u16_at(directory)?);
    (0..count).find_map(|i| {
        let entry = directory.checked_add(2 + 12 * i)?;
        // A SHORT, type 3, with one value, which sits in the entry's first two value bytes.
        (u16_at(entry)? == 0x0112 && u16_at(entry + 2)? == 3)
            .then(|| u16_at(entry + 8))
            .flatten()
            .and_then(|v| u8::try_from(v).ok())
            .filter(|v| (1..=8).contains(v))
    })
}

impl Builder<'_, '_> {
    /// An `<img>`, or an inline `<svg>` when `svg` is true, at `at` with `style` and its
    /// `display`, in `frame`'s content box: an inline box in the text, a float, or a block, which
    /// in a flex container (`item`) is an anonymous item of its own. An `<img>` is tagged as a
    /// figure with its alt text, and one with no `src` draws nothing.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn replaced(
        &mut self,
        dom: &Dom,
        at: usize,
        style: &Style,
        display: &str,
        frame: &Frame,
        item: bool,
        svg: bool,
    ) -> Result<(), Error> {
        let node = &dom.nodes[at];
        let image = if svg {
            let name = "<svg>";
            let picture = self::svg(self.assets, svg_markup(dom, at).as_bytes(), name)?;
            self.images.push((picture, name.to_owned()));
            self.images.len() - 1
        } else {
            let Some(reference) = node.attr("src") else {
                return Ok(());
            };
            self.load(reference)?
        };
        let figure = (!svg).then(|| Role::Figure(node.attr("alt").map(str::to_owned)));
        let out = matches!(keyword(style, "position"), Some("absolute" | "fixed"));
        let side = sides(style, "float");
        let floated = !item && !out && side != [false, false];
        if floated || (!item && !out && matches!(display, "inline" | "inline-block")) {
            let mut inner = frame.clone();
            self.open_atom(style, frame, &mut inner)?;
            let tagged = figure.is_some();
            if tagged {
                self.blocks.push(Block::Tag(figure));
            }
            let (width, height) = self.place(image, dom, at, style, &inner, frame.width);
            if tagged {
                self.blocks.push(Block::Tag(None));
            }
            // The box is as wide and as tall as its picture, with its border and padding.
            if let Some(Block::Open(g)) = self.blocks.first_mut() {
                g.width = width + g.border[1] + g.border[3] + g.padding[1] + g.padding[3];
                g.min_content = height;
                g.fill = false;
            }
            if floated {
                self.close_float(side[1], sides(style, "clear"), frame);
            } else {
                self.close_atom(vertical_align(style, None), frame);
            }
            return Ok(());
        }
        let inner = if item {
            self.open_anonymous(frame)
        } else {
            frame.clone()
        };
        self.flush();
        let tagged = figure.is_some();
        if tagged {
            self.blocks.push(Block::Tag(figure));
        }
        self.place(image, dom, at, style, &inner, inner.width);
        if tagged {
            self.blocks.push(Block::Tag(None));
        }
        if item {
            self.close_anonymous();
        }
        Ok(())
    }

    /// The index in `images` of the picture `reference` names. A reference this render loaded
    /// before is not resolved or decoded again, so a picture drawn many times is in the file once.
    fn load(&mut self, reference: &str) -> Result<usize, Error> {
        if let Some(&image) = self.loaded.get(reference) {
            return Ok(image);
        }
        let (image, _) = load_into(self.assets, &mut self.images, reference)?;
        self.work.image_resolves += 1;
        self.work.image_decodes += 1;
        self.loaded.insert(reference.to_owned(), image);
        Ok(image)
    }

    /// The picture at `image` in `images` as a block for the element at `at`, at the size its
    /// style and its `width` and `height` attributes give, scaled down to fit `frame`'s content
    /// box and the page. A percent width counts against `basis`. Returns the size it is drawn at.
    fn place(
        &mut self,
        image: usize,
        dom: &Dom,
        at: usize,
        style: &Style,
        frame: &Frame,
        basis: f32,
    ) -> (f32, f32) {
        let (natural_w, natural_h) = self.images[image].0.size();
        let attr = |name: &str| {
            dom.nodes[at]
                .attr(name)
                .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
                .filter(|px| px.is_finite() && *px >= 0.0)
                .map(|px| px * 0.75)
        };
        let page = self.page_height;
        let across = |name: &str| length(style, name, basis).filter(|v| *v >= 0.0);
        let down = |name: &str| length(style, name, page).filter(|v| *v >= 0.0);
        let css_w = across("width").or_else(|| attr("width"));
        let css_h = down("height").or_else(|| attr("height"));
        let (width, height) = match (css_w, css_h) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, w * natural_h / natural_w.max(f32::EPSILON)),
            (None, Some(h)) => (h * natural_w / natural_h.max(f32::EPSILON), h),
            (None, None) => (natural_w, natural_h),
        };
        let wide = [across("min-width"), across("max-width")];
        let tall = [down("min-height"), down("max-height")];
        let (mut width, mut height) = if css_w.is_some() && css_h.is_some() {
            (bound(width, wide), bound(height, tall))
        } else {
            constrain(width, height, wide, tall)
        };
        let scale = (frame.width / width.max(f32::EPSILON))
            .min(page / height.max(f32::EPSILON))
            .min(1.0);
        width *= scale;
        height *= scale;
        let (natural_w, natural_h) = (natural_w * scale, natural_h * scale);
        let ratio = |pick: fn(f32, f32) -> f32| {
            pick(
                width / natural_w.max(f32::EPSILON),
                height / natural_h.max(f32::EPSILON),
            )
        };
        let by = match keyword(style, "object-fit") {
            Some("contain") => Some(ratio(f32::min)),
            Some("cover") => Some(ratio(f32::max)),
            Some("none") => Some(1.0),
            Some("scale-down") => Some(ratio(f32::min).min(1.0)),
            _ => None,
        };
        let draw = match by {
            Some(by) => {
                let (w, h) = (natural_w * by, natural_h * by);
                let [x, y] = position(style, "object-position", self.rem).unwrap_or([CENTRE; 2]);
                [x.0 * (width - w) + x.1, y.0 * (height - h) + y.1, w, h]
            }
            None => [0.0, 0.0, width, height],
        };
        // A hidden image keeps its space and draws nothing.
        let draw = match keyword(style, "visibility") {
            Some("hidden" | "collapse") => [0.0; 4],
            _ => draw,
        };
        self.blocks.push(Block::Image {
            image,
            x: frame.x,
            width,
            height,
            draw,
        });
        (width, height)
    }

    /// The background image of `style`: a gradient, or an image `url()` loaded from the asset map
    /// with its natural size, its `background-size` and its `background-position`.
    pub(super) fn layer(&mut self, style: &Style, rem: f32) -> Result<Option<boxes::Layer>, Error> {
        layer(style, rem, &mut |reference| {
            let image = self.load(reference)?;
            Ok((image, self.images[image].0.size()))
        })
    }
}

/// `value` between a least and a greatest value, where a missing bound does not limit it and
/// the least wins over the greatest.
fn bound(value: f32, [min, max]: [Option<f32>; 2]) -> f32 {
    let value = max.map_or(value, |max| value.min(max));
    min.map_or(value, |min| value.max(min))
}

/// A picture `w` by `h`, with an aspect ratio to keep, limited by its least and greatest width
/// and height, by the table in CSS 2.1 §10.4.
fn constrain(w: f32, h: f32, wide: [Option<f32>; 2], tall: [Option<f32>; 2]) -> (f32, f32) {
    let (given_w, given_h) = (w, h);
    let (w, h) = (w.max(f32::EPSILON), h.max(f32::EPSILON));
    let min_w = wide[0].unwrap_or(0.0);
    let max_w = wide[1].unwrap_or(f32::INFINITY).max(min_w);
    let min_h = tall[0].unwrap_or(0.0);
    let max_h = tall[1].unwrap_or(f32::INFINITY).max(min_h);
    match (w > max_w, w < min_w, h > max_h, h < min_h) {
        (true, _, true, _) if max_w / w <= max_h / h => (max_w, (max_w * h / w).max(min_h)),
        (true, _, true, _) => ((max_h * w / h).max(min_w), max_h),
        (_, true, _, true) if min_w / w <= min_h / h => ((min_h * w / h).min(max_w), min_h),
        (_, true, _, true) => (min_w, (min_w * h / w).min(max_h)),
        (true, _, _, true) => (max_w, min_h),
        (_, true, true, _) => (min_w, max_h),
        (true, ..) => (max_w, (max_w * h / w).max(min_h)),
        (_, true, ..) => (min_w, (min_w * h / w).min(max_h)),
        (_, _, true, _) => ((max_h * w / h).max(min_w), max_h),
        (_, _, _, true) => ((min_h * w / h).min(max_w), min_h),
        _ => (given_w, given_h),
    }
}

/// A picture in `images`: its index there and its natural size in points.
pub(super) type Loaded = (usize, (f32, f32));

/// The picture `reference` names, decoded and added to `images`.
pub(super) fn load_into(
    assets: &Assets<'_>,
    images: &mut Images,
    reference: &str,
) -> Result<Loaded, Error> {
    let (picture, name) = decode(assets, reference)?;
    let size = picture.size();
    images.push((picture, name));
    Ok((images.len() - 1, size))
}

/// The background image of `style`, as [`Builder::layer`] says. `load` gives the index and the
/// size of the picture a reference names.
pub(super) fn layer(
    style: &Style,
    rem: f32,
    load: &mut dyn FnMut(&str) -> Result<Loaded, Error>,
) -> Result<Option<boxes::Layer>, Error> {
    let Some(reference) = boxes::image_reference(style) else {
        return Ok(boxes::gradient(style, rem));
    };
    let (image, (width, height)) = load(&reference)?;
    let (repeat_x, repeat_y) = boxes::repeat(style);
    Ok(Some(boxes::Layer::Image {
        image,
        width,
        height,
        repeat_x,
        repeat_y,
        size: boxes::background_size(style, rem),
        position: position(style, "background-position", rem).unwrap_or([(0.0, 0.0); 2]),
    }))
}

/// The picture `reference` names, decoded far enough to know its size, and the name an error
/// about it gives. A raster image asks no viewer to smooth it, because PDF/A forbids that
/// flag and a viewer smooths by its own setting either way.
fn decode(assets: &Assets<'_>, reference: &str) -> Result<(Picture, String), Error> {
    let bytes = assets.resolve(reference)?.into_owned();
    let name = if reference.len() > 64 {
        format!("{}…", reference.chars().take(64).collect::<String>())
    } else {
        reference.to_owned()
    };
    let turn = match bytes.as_slice() {
        [0xff, 0xd8, 0xff, ..] => Turn::of_jpeg(&bytes),
        _ => Turn::UPRIGHT,
    };
    let decoded = match bytes.as_slice() {
        [0x89, b'P', b'N', b'G', ..] => Image::from_png(Data::from(bytes), false),
        [0xff, 0xd8, 0xff, ..] => Image::from_jpeg(Data::from(bytes), false),
        [b'G', b'I', b'F', b'8', ..] => Image::from_gif(Data::from(bytes), false),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Image::from_webp(Data::from(bytes), false),
        _ if is_svg(&bytes) => return Ok((svg(assets, &bytes, &name)?, name)),
        _ if is_pdf(&bytes) => {
            let page = crate::assets::page_fragment(reference.trim()).1;
            return Ok((pdf_page(bytes, page.unwrap_or(1), &name)?, name));
        }
        _ => {
            return Err(Error::Parse(format!(
                "`{name}` is not a PNG, JPEG, GIF, WebP, SVG or PDF file"
            )));
        }
    };
    let image = decoded
        .map_err(|why| Error::Parse(format!("`{name}` does not decode as an image: {why}")))?;
    Ok((Picture::Raster(image, turn), name))
}

/// The SVG document `data` parsed. An `<image>` in it loads through the asset map, and one
/// the map does not have returns the error the map gives. A document that does not parse
/// returns `Parse` naming `name`.
fn svg(assets: &Assets<'_>, data: &[u8], name: &str) -> Result<Picture, Error> {
    let refused: Mutex<Option<Error>> = Mutex::new(None);
    let options = usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|href, options| match assets.resolve(href) {
                Ok(bytes) => (usvg::ImageHrefResolver::default_data_resolver())(
                    "text/plain",
                    Arc::new(bytes.into_owned()),
                    options,
                ),
                Err(err) => {
                    let mut first = refused.lock().unwrap_or_else(|p| p.into_inner());
                    first.get_or_insert(err);
                    None
                }
            }),
        },
        fontdb: crate::fonts::svg_database(),
        font_resolver: crate::fonts::svg_font_resolver(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(data, &options)
        .map_err(|why| Error::Parse(format!("`{name}` does not parse as SVG: {why}")))?;
    drop(options);
    if let Some(err) = refused.into_inner().unwrap_or_else(|p| p.into_inner()) {
        return Err(err);
    }
    Ok(Picture::Vector(Box::new(tree)))
}

/// The centre of a position's axis: half the room, and no length added.
const CENTRE: (f32, f32) = (0.5, 0.0);

/// `style`'s `name`, an `object-position` or a `background-position`, across and down, each as a
/// fraction of the room the box has beside the picture and a length in points added to it. A side
/// the value does not give is centred, and a value that is missing or does not read is `None`.
fn position(style: &Style, name: &str, rem: f32) -> Option<[(f32, f32); 2]> {
    let Some(Value::Text(text)) = style.get(name) else {
        return None;
    };
    let comps = value::components(text)?;
    let em = style.font_size();
    let side = |c: &Comp| match c {
        Comp::Ident(word) => match word.as_str() {
            "left" | "top" => Some((0.0, 0.0)),
            "center" => Some(CENTRE),
            "right" | "bottom" => Some((1.0, 0.0)),
            _ => None,
        },
        c => value::length(c).map(|sum| {
            let l = sum.compute(em, rem);
            (l.percent / 100.0, l.pt)
        }),
    };
    let down = |c: &Comp| matches!(c, Comp::Ident(w) if w == "top" || w == "bottom");
    let across = |c: &Comp| matches!(c, Comp::Ident(w) if w == "left" || w == "right");
    match comps.as_slice() {
        [one] => side(one).map(|at| {
            if down(one) {
                [CENTRE, at]
            } else {
                [at, CENTRE]
            }
        }),
        [a, b] => match (side(a), side(b)) {
            (Some(x), Some(y)) if down(a) || across(b) => Some([y, x]),
            (Some(x), Some(y)) => Some([x, y]),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `data` reads as a PDF document: `%PDF-` in its first 1024 bytes, where readers look.
fn is_pdf(data: &[u8]) -> bool {
    data[..data.len().min(1024)]
        .windows(5)
        .any(|window| window == b"%PDF-")
}

/// Page `page`, counted from 1, of the PDF `data`, which `name` names. An encrypted document
/// and one that does not parse return `Parse`, and a page outside it returns `Invalid` naming
/// the count, each naming `name` (`read`'s module doc).
fn pdf_page(data: Vec<u8>, page: u64, name: &str) -> Result<Picture, Error> {
    let named = |err| match err {
        Error::Parse(why) => Error::Parse(format!("`{name}`: {why}")),
        Error::Invalid(why) => Error::Invalid(format!("`{name}`: {why}")),
        other => other,
    };
    let pdf = crate::read::open(data).map_err(named)?;
    let index = crate::read::index(page, pdf.pages().len()).map_err(named)?;
    let size = pdf.pages()[index].render_dimensions();
    Ok(Picture::Page(Box::new(PdfPage {
        document: PdfDocument::new(Arc::new(pdf)),
        index,
        size,
    })))
}

/// Whether `data` reads as an SVG document: an optional byte-order mark and whitespace, then an
/// XML declaration, a comment, a doctype or the `<svg` tag, with `<svg` in the first 4 KiB.
fn is_svg(data: &[u8]) -> bool {
    let data = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);
    let start = data
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(data.len());
    let head = &data[start..data.len().min(start + 4096)];
    let opens = [&b"<svg"[..], b"<?xml", b"<!--", b"<!DOCTYPE"]
        .iter()
        .any(|prefix| head.starts_with(prefix));
    opens && head.windows(4).any(|window| window == b"<svg")
}

/// The `<svg>` element at `at` and everything in it written out as an SVG document. An element
/// outside the SVG namespace is left out with its content, and so is an `xmlns` declaration,
/// which the root's own replace.
fn svg_markup(dom: &Dom, at: usize) -> String {
    fn escape(out: &mut String, text: &str) {
        for c in text.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                _ => out.push(c),
            }
        }
    }
    enum Visit {
        Open(usize),
        Close(usize),
    }
    let mut out = String::new();
    let mut work = vec![Visit::Open(at)];
    while let Some(visit) = work.pop() {
        match visit {
            Visit::Close(node) => {
                if let Some(name) = dom.nodes[node].element(Space::Svg) {
                    out.push_str("</");
                    out.push_str(name);
                    out.push('>');
                }
            }
            Visit::Open(node) => match &dom.nodes[node].kind {
                Kind::Text(text) => escape(&mut out, text),
                Kind::Element {
                    name,
                    space: Space::Svg,
                    attrs,
                } => {
                    out.push('<');
                    out.push_str(name);
                    if node == at {
                        out.push_str(
                            " xmlns=\"http://www.w3.org/2000/svg\" \
                             xmlns:xlink=\"http://www.w3.org/1999/xlink\"",
                        );
                    }
                    for attr in attrs {
                        if !attr.xlink && (attr.name == "xmlns" || attr.name == "xlink") {
                            continue;
                        }
                        out.push(' ');
                        if attr.xlink {
                            out.push_str("xlink:");
                        }
                        out.push_str(&attr.name);
                        out.push_str("=\"");
                        escape(&mut out, &attr.value);
                        out.push('"');
                    }
                    out.push('>');
                    work.push(Visit::Close(node));
                    work.extend(
                        dom.nodes[node]
                            .children
                            .iter()
                            .rev()
                            .map(|&c| Visit::Open(c)),
                    );
                }
                _ => {}
            },
        }
    }
    out
}
