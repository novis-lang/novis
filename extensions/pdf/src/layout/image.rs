//! The pictures a document draws: an `<img>`, an inline `<svg>` and a background image, each
//! decoded far enough to know its size and placed as a block or a box's background layer.

use std::sync::{Arc, Mutex};

use krilla::Data;
use krilla::image::Image;

use super::inline::Builder;
use super::{Block, CONTENT_HEIGHT, Frame, boxes, keyword, length};
use crate::Error;
use crate::css::value::{self, Comp};
use crate::css::{Style, Value};
use crate::parse::{Dom, Kind, Space};

/// A picture an [`super::Item::Image`] or a background draws.
pub enum Picture {
    /// A PNG, JPEG, GIF or WebP image, decoded far enough to know its size.
    Raster(Image),
    /// A parsed SVG document, drawn as vector paths.
    Vector(Box<usvg::Tree>),
}

impl Picture {
    /// The natural size in points: a pixel, or an SVG user unit, is three quarters of a point.
    pub fn size(&self) -> (f32, f32) {
        match self {
            Picture::Raster(image) => {
                let (w, h) = image.size();
                (w as f32 * 0.75, h as f32 * 0.75)
            }
            Picture::Vector(tree) => (tree.size().width() * 0.75, tree.size().height() * 0.75),
        }
    }
}

impl Builder<'_, '_> {
    /// An `<img>` at `at`, laid out in `frame`'s content box.
    pub(super) fn image(
        &mut self,
        dom: &Dom,
        at: usize,
        style: &Style,
        frame: &Frame,
    ) -> Result<(), Error> {
        let Some(reference) = dom.nodes[at].attr("src") else {
            return Ok(());
        };
        self.flush();
        let (picture, name) = self.decode(reference)?;
        self.place(picture, name, dom, at, style, frame);
        Ok(())
    }

    /// An inline `<svg>` at `at`, laid out in `frame`'s content box like an `<img>`.
    pub(super) fn inline_svg(
        &mut self,
        dom: &Dom,
        at: usize,
        style: &Style,
        frame: &Frame,
    ) -> Result<(), Error> {
        self.flush();
        let name = "<svg>";
        let picture = self.svg(svg_markup(dom, at).as_bytes(), name)?;
        self.place(picture, name.to_owned(), dom, at, style, frame);
        Ok(())
    }

    /// `picture` as a block for the element at `at`, at the size its style and its `width` and
    /// `height` attributes give, scaled down to fit `frame`'s content box.
    fn place(
        &mut self,
        picture: Picture,
        name: String,
        dom: &Dom,
        at: usize,
        style: &Style,
        frame: &Frame,
    ) {
        let (natural_w, natural_h) = picture.size();
        let attr = |name: &str| {
            dom.nodes[at]
                .attr(name)
                .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
                .map(|px| px * 0.75)
        };
        let css_w = length(style, "width", frame.width).or_else(|| attr("width"));
        let css_h = match style.get("height") {
            Some(Value::Length(l)) if l.percent == 0.0 => Some(l.pt),
            _ => attr("height"),
        };
        let (mut width, mut height) = match (css_w, css_h) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, w * natural_h / natural_w.max(f32::EPSILON)),
            (None, Some(h)) => (h * natural_w / natural_h.max(f32::EPSILON), h),
            (None, None) => (natural_w, natural_h),
        };
        let scale = (frame.width / width.max(f32::EPSILON))
            .min(CONTENT_HEIGHT / height.max(f32::EPSILON))
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
                let [x, y] = object_position(style, self.rem);
                [x.0 * (width - w) + x.1, y.0 * (height - h) + y.1, w, h]
            }
            None => [0.0, 0.0, width, height],
        };
        self.images.push((picture, name));
        self.blocks.push(Block::Image {
            image: self.images.len() - 1,
            x: frame.x,
            width,
            height,
            draw,
        });
    }

    /// The background image of `style`: a gradient, or an image `url()` loaded from the asset map
    /// at its natural size.
    pub(super) fn layer(&mut self, style: &Style, rem: f32) -> Result<Option<boxes::Layer>, Error> {
        let Some(reference) = boxes::image_reference(style) else {
            return Ok(boxes::gradient(style, rem));
        };
        let (picture, name) = self.decode(&reference)?;
        let (width, height) = picture.size();
        let (repeat_x, repeat_y) = boxes::repeat(style);
        self.images.push((picture, name));
        Ok(Some(boxes::Layer::Image {
            image: self.images.len() - 1,
            width,
            height,
            repeat_x,
            repeat_y,
        }))
    }

    /// The picture `reference` names, decoded far enough to know its size, and the name an error
    /// about it gives.
    fn decode(&self, reference: &str) -> Result<(Picture, String), Error> {
        let bytes = self.assets.resolve(reference)?.into_owned();
        let name = if reference.len() > 64 {
            format!("{}…", reference.chars().take(64).collect::<String>())
        } else {
            reference.to_owned()
        };
        let decoded = match bytes.as_slice() {
            [0x89, b'P', b'N', b'G', ..] => Image::from_png(Data::from(bytes), true),
            [0xff, 0xd8, 0xff, ..] => Image::from_jpeg(Data::from(bytes), true),
            [b'G', b'I', b'F', b'8', ..] => Image::from_gif(Data::from(bytes), true),
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
            ] => Image::from_webp(Data::from(bytes), true),
            _ if is_svg(&bytes) => return Ok((self.svg(&bytes, &name)?, name)),
            _ => {
                return Err(Error::Parse(format!(
                    "`{name}` is not a PNG, JPEG, GIF, WebP or SVG image"
                )));
            }
        };
        let image = decoded
            .map_err(|why| Error::Parse(format!("`{name}` does not decode as an image: {why}")))?;
        Ok((Picture::Raster(image), name))
    }

    /// The SVG document `data` parsed. An `<image>` in it loads through the asset map, and one
    /// the map does not have returns the error the map gives. A document that does not parse
    /// returns `Parse` naming `name`.
    fn svg(&self, data: &[u8], name: &str) -> Result<Picture, Error> {
        let refused: Mutex<Option<Error>> = Mutex::new(None);
        let assets = self.assets;
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
}

/// `style`'s `object-position` across and down, each as a fraction of the room the box has
/// beside the picture and a length in points added to it. Each side is centred when the value
/// does not give it, and a value that does not read centres both.
fn object_position(style: &Style, rem: f32) -> [(f32, f32); 2] {
    const CENTRE: (f32, f32) = (0.5, 0.0);
    let comps = match style.get("object-position") {
        Some(Value::Text(text)) => value::components(text).unwrap_or_default(),
        _ => Vec::new(),
    };
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
        [one] => match side(one) {
            Some(at) if down(one) => [CENTRE, at],
            Some(at) => [at, CENTRE],
            None => [CENTRE; 2],
        },
        [a, b] => match (side(a), side(b)) {
            (Some(x), Some(y)) if down(a) || across(b) => [y, x],
            (Some(x), Some(y)) => [x, y],
            _ => [CENTRE; 2],
        },
        _ => [CENTRE; 2],
    }
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
