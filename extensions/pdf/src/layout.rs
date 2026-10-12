//! The layout: the parsed document as pages of positioned glyph runs and images.
//!
//! This is the engine's first layout, and it reads only the elements' names, never a
//! stylesheet. A page is A4 with a 2 cm margin on every side, and every length here is in PDF
//! points (1/72 inch, three quarters of a CSS pixel). The document's blocks are laid out top to
//! bottom in the content box: `p`, `div`, `h1` to `h6`, `li` and the other block elements of
//! HTML's default stylesheet each start a new block, with HTML's default margins between them,
//! collapsed to the larger of two that meet and dropped at the top of a page after the first.
//! Text is Liberation Sans at 12 points; `h1` to `h6` are bold at HTML's default sizes, `b` and
//! `strong` are bold, `i` and `em` italic, and `pre`, `code`, `kbd`, `samp` and `tt` are
//! Liberation Mono. Whitespace collapses to one space except in `pre`, and `br` breaks the line.
//! A list item starts with a bullet in `ul` and its number in `ol`, and a list is indented by
//! 30 points.
//!
//! Text is shaped and broken into lines by `parley`, over `harfrust` and `skrifa`, at word
//! boundaries greedily to the content width; a word wider than the line overflows it. A line
//! that does not fit under the last on the page starts the next page. An
//! `<img>` whose bytes are PNG, JPEG, GIF or WebP is a block of its natural size, a pixel being
//! a CSS pixel, scaled down to fit the content box, and starts the next page when it does not
//! fit under the last block. Nothing in `head`, `script`, `style`, `template`, `noscript`, an
//! inline `<svg>` or a form control's content is drawn: a script never runs.
//!
//! Memory: one shaped `parley` layout at a time, and the positioned glyphs of every page until
//! the writer has drawn them.

use std::ops::Range;

use krilla::Data;
use krilla::image::Image;
use parley::{
    Alignment, AlignmentOptions, FontFamily, FontStyle, FontWeight, LayoutContext, LineHeight,
    StyleProperty,
};

use crate::Error;
use crate::assets::Assets;
use crate::fonts::{self, Family};
use crate::parse::{Dom, Kind, Space};

/// A4's width and height in points.
pub const PAGE_WIDTH: f32 = 595.275_6;
pub const PAGE_HEIGHT: f32 = 841.889_8;
/// 2 cm in points.
pub const MARGIN: f32 = 56.692_9;
/// The default font size in points.
pub const FONT_SIZE: f32 = 12.0;
/// How far a list's items are indented, in points.
const LIST_INDENT: f32 = 30.0;

const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN;
const CONTENT_HEIGHT: f32 = PAGE_HEIGHT - 2.0 * MARGIN;

/// One glyph of a run, in points at the run's size: its advance and its offset from where the
/// previous glyph's advance left the pen, `y` downward.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub id: u32,
    /// The bytes of the run's text the glyph's cluster covers.
    pub text: Range<usize>,
    pub advance: f32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub enum Item {
    /// A run of glyphs in one face and size, its pen starting at `x` on the baseline `y`.
    Text {
        face: usize,
        size: f32,
        x: f32,
        y: f32,
        glyphs: Vec<Glyph>,
        text: String,
    },
    /// The image at `image` in [`Laid::images`], its top-left corner at `x`, `y`.
    Image {
        image: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub items: Vec<Item>,
}

/// The laid-out document: at least one page, and the images its pages draw with the reference
/// each came from.
pub struct Laid {
    pub pages: Vec<Page>,
    pub images: Images,
}

/// Each image a document draws, with the reference it was loaded from.
pub type Images = Vec<(Image, String)>;

/// One styled range of a text block's text.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Style {
    bold: bool,
    italic: bool,
    size: f32,
    family: Family,
}

/// What every element on the stack passes to its content.
#[derive(Debug, Clone, Copy)]
struct Frame {
    style: Style,
    preformatted: bool,
    indent: f32,
    /// The next item number when this is an `ol`, and `None` in any other element.
    counter: Option<u64>,
}

enum Block {
    Text {
        text: String,
        spans: Vec<(Range<usize>, Style)>,
        indent: f32,
    },
    Image {
        image: usize,
        indent: f32,
    },
    /// Vertical space before the next block, collapsed with any beside it.
    Margin(f32),
}

/// The elements whose content is never drawn.
fn is_hidden(name: &str) -> bool {
    matches!(
        name,
        "head"
            | "script"
            | "style"
            | "title"
            | "meta"
            | "link"
            | "template"
            | "noscript"
            | "iframe"
            | "object"
            | "embed"
            | "video"
            | "audio"
            | "canvas"
            | "select"
            | "textarea"
            | "button"
            | "input"
            | "math"
    )
}

/// The block elements of HTML's default stylesheet, with their top and bottom margin in ems.
fn block_margin(name: &str) -> Option<f32> {
    Some(match name {
        "p" | "ul" | "ol" | "dl" | "pre" | "blockquote" | "figure" | "hr" | "table" => 1.0,
        "h1" => 0.67,
        "h2" => 0.83,
        "h3" => 1.0,
        "h4" => 1.33,
        "h5" => 1.67,
        "h6" => 2.33,
        "html" | "body" | "div" | "li" | "section" | "article" | "header" | "footer" | "main"
        | "nav" | "aside" | "address" | "dt" | "dd" | "figcaption" | "form" | "fieldset"
        | "details" | "summary" | "tr" | "thead" | "tbody" | "tfoot" | "caption" | "td" | "th"
        | "center" | "legend" => 0.0,
        _ => return None,
    })
}

/// A heading's size as a multiple of the default, from HTML's default stylesheet.
fn heading_scale(name: &str) -> Option<f32> {
    Some(match name {
        "h1" => 2.0,
        "h2" => 1.5,
        "h3" => 1.17,
        "h4" => 1.0,
        "h5" => 0.83,
        "h6" => 0.67,
        _ => return None,
    })
}

/// Collects a document's blocks in order.
struct Builder<'a, 'd> {
    assets: &'a Assets<'d>,
    blocks: Vec<Block>,
    images: Images,
    text: String,
    spans: Vec<(Range<usize>, Style)>,
    indent: f32,
    /// Whether the text so far ends in a collapsible space, or is empty.
    after_space: bool,
}

impl Builder<'_, '_> {
    fn push_text(&mut self, text: &str, frame: &Frame) {
        let start = self.text.len();
        if frame.preformatted {
            self.text.push_str(text);
            self.after_space = false;
        } else {
            for c in text.chars() {
                if matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r') {
                    if !self.after_space {
                        self.text.push(' ');
                        self.after_space = true;
                    }
                } else {
                    self.text.push(c);
                    self.after_space = false;
                }
            }
        }
        if self.text.len() > start {
            if self.spans.is_empty() {
                self.indent = frame.indent;
            }
            match self.spans.last_mut() {
                Some((range, style)) if *style == frame.style && range.end == start => {
                    range.end = self.text.len();
                }
                _ => self.spans.push((start..self.text.len(), frame.style)),
            }
        }
    }

    /// Ends the text block being collected, if it has anything to draw.
    fn flush(&mut self) {
        let trimmed = self.text.trim_end_matches([' ', '\n']).len();
        self.text.truncate(trimmed);
        self.spans.retain_mut(|(range, _)| {
            range.end = range.end.min(trimmed);
            range.start < range.end
        });
        if !self.spans.is_empty() {
            self.blocks.push(Block::Text {
                text: std::mem::take(&mut self.text),
                spans: std::mem::take(&mut self.spans),
                indent: self.indent,
            });
        }
        self.text.clear();
        self.spans.clear();
        self.after_space = true;
    }

    fn image(&mut self, reference: &str, indent: f32) -> Result<(), Error> {
        self.flush();
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
            _ => {
                return Err(Error::Parse(format!(
                    "`{name}` is not a PNG, JPEG, GIF or WebP image"
                )));
            }
        };
        let image = decoded
            .map_err(|why| Error::Parse(format!("`{name}` does not decode as an image: {why}")))?;
        self.images.push((image, name));
        self.blocks.push(Block::Image {
            image: self.images.len() - 1,
            indent,
        });
        Ok(())
    }
}

/// The blocks of `dom`, in document order.
fn blocks(dom: &Dom, assets: &Assets<'_>) -> Result<(Vec<Block>, Images), Error> {
    let mut builder = Builder {
        assets,
        blocks: Vec::new(),
        images: Vec::new(),
        text: String::new(),
        spans: Vec::new(),
        indent: 0.0,
        after_space: true,
    };
    let root = Frame {
        style: Style {
            bold: false,
            italic: false,
            size: FONT_SIZE,
            family: Family::Sans,
        },
        preformatted: false,
        indent: 0.0,
        counter: None,
    };
    // An explicit stack, so a deep tree costs heap and never the call stack.
    let mut frames = vec![root];
    let mut work = vec![Step::Enter(0)];
    while let Some(step) = work.pop() {
        let at = match step {
            Step::Enter(at) => at,
            Step::Exit { margin } => {
                frames.pop();
                if let Some(margin) = margin {
                    builder.flush();
                    if margin > 0.0 {
                        builder.blocks.push(Block::Margin(margin));
                    }
                }
                continue;
            }
        };
        let node = &dom.nodes[at];
        let frame = *frames.last().expect("the root frame stays");
        match &node.kind {
            Kind::Document => {
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
            Kind::Text(text) => builder.push_text(text, &frame),
            Kind::Ignored => {}
            Kind::Element { name, space, .. } => {
                if *space != Space::Html || is_hidden(name) {
                    continue;
                }
                let name = name.as_str();
                if name == "img" {
                    if let Some(src) = node.attr("src") {
                        builder.image(src, frame.indent)?;
                    }
                    continue;
                }
                if name == "br" {
                    builder.text.push('\n');
                    builder.after_space = true;
                    continue;
                }
                let mut inner = frame;
                inner.counter = None;
                let margin = block_margin(name);
                if let Some(scale) = heading_scale(name) {
                    inner.style.size = FONT_SIZE * scale;
                    inner.style.bold = true;
                }
                match name {
                    "b" | "strong" | "th" => inner.style.bold = true,
                    "i" | "em" | "cite" | "var" | "dfn" | "address" => inner.style.italic = true,
                    "code" | "kbd" | "samp" | "tt" => inner.style.family = Family::Mono,
                    "pre" => {
                        inner.style.family = Family::Mono;
                        inner.preformatted = true;
                    }
                    "ul" => inner.indent += LIST_INDENT,
                    "ol" => {
                        inner.indent += LIST_INDENT;
                        inner.counter = Some(
                            node.attr("start")
                                .and_then(|s| s.trim().parse().ok())
                                .unwrap_or(1),
                        );
                    }
                    _ => {}
                }
                if let Some(em) = margin {
                    builder.flush();
                    if em > 0.0 {
                        builder.blocks.push(Block::Margin(em * inner.style.size));
                    }
                }
                if name == "li" {
                    let parent = frames.last_mut().expect("the root frame stays");
                    let marker = if let Some(n) = parent.counter.as_mut() {
                        *n += 1;
                        format!("{}. ", *n - 1)
                    } else {
                        "\u{2022} ".to_owned()
                    };
                    builder.push_text(&marker, &inner);
                }
                frames.push(inner);
                work.push(Step::Exit {
                    margin: margin.map(|em| em * inner.style.size),
                });
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
        }
    }
    builder.flush();
    Ok((builder.blocks, builder.images))
}

/// One step of the walk in [`blocks`].
enum Step {
    Enter(usize),
    /// Leaves the element whose frame is on top. A block element's exit ends its text and adds
    /// its bottom margin.
    Exit {
        margin: Option<f32>,
    },
}

/// Lays `dom` out in pages.
pub fn layout(dom: &Dom, assets: &Assets<'_>) -> Result<Laid, Error> {
    let (blocks, images) = blocks(dom, assets)?;
    let mut fcx = fonts::context();
    let mut lcx: LayoutContext<()> = LayoutContext::new();
    let mut pages = vec![Page::default()];
    let mut cursor = 0.0_f32;
    let mut pending = 0.0_f32;

    // Moves to a new page when `height` does not fit under the last block, unless the page is
    // empty, and returns the top of the box `height` is placed in.
    let place = |pages: &mut Vec<Page>, cursor: &mut f32, pending: &mut f32, height: f32| {
        let empty = pages.last().is_some_and(|p| p.items.is_empty());
        let margin = if empty && pages.len() > 1 {
            0.0
        } else {
            *pending
        };
        if !empty && *cursor + margin + height > CONTENT_HEIGHT {
            pages.push(Page::default());
            *cursor = 0.0;
        } else {
            *cursor += margin;
        }
        *pending = 0.0;
        let top = *cursor;
        *cursor += height;
        top
    };

    for block in blocks {
        match block {
            Block::Margin(margin) => pending = pending.max(margin),
            Block::Image { image, indent } => {
                let (w, h) = images[image].0.size();
                let (w, h) = (w as f32 * 0.75, h as f32 * 0.75);
                let width = CONTENT_WIDTH - indent;
                let scale = (width / w).min(CONTENT_HEIGHT / h).min(1.0);
                let (w, h) = (w * scale, h * scale);
                let top = place(&mut pages, &mut cursor, &mut pending, h);
                let page = pages.last_mut().expect("there is always a page");
                page.items.push(Item::Image {
                    image,
                    x: MARGIN + indent,
                    y: MARGIN + top,
                    width: w,
                    height: h,
                });
            }
            Block::Text {
                text,
                spans,
                indent,
            } => {
                let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);
                builder.push_default(StyleProperty::FontFamily(FontFamily::named(
                    Family::Sans.registered_name(),
                )));
                builder.push_default(StyleProperty::FontSize(FONT_SIZE));
                builder.push_default(LineHeight::MetricsRelative(1.0));
                for (range, style) in &spans {
                    let range = range.clone();
                    if style.family != Family::Sans {
                        builder.push(
                            StyleProperty::FontFamily(FontFamily::named(
                                style.family.registered_name(),
                            )),
                            range.clone(),
                        );
                    }
                    if style.size != FONT_SIZE {
                        builder.push(StyleProperty::FontSize(style.size), range.clone());
                    }
                    if style.bold {
                        builder.push(StyleProperty::FontWeight(FontWeight::BOLD), range.clone());
                    }
                    if style.italic {
                        builder.push(StyleProperty::FontStyle(FontStyle::Italic), range);
                    }
                }
                let mut shaped = builder.build(&text);
                shaped.break_all_lines(Some(CONTENT_WIDTH - indent));
                shaped.align(Alignment::Start, AlignmentOptions::default());
                for line in shaped.lines() {
                    let metrics = line.metrics();
                    let height = metrics.block_max_coord - metrics.block_min_coord;
                    let top = place(&mut pages, &mut cursor, &mut pending, height);
                    let baseline = MARGIN + top + (metrics.baseline - metrics.block_min_coord);
                    let line_range = line.text_range();
                    let line_text = &text[line_range.clone()];
                    let mut x = MARGIN + indent + metrics.inline_min_coord + metrics.offset;
                    let page = pages.last_mut().expect("there is always a page");
                    for run in line.runs() {
                        let font = run.font();
                        let face = fonts::face_index(font.data.data())
                            .expect("the font context holds only the embedded faces");
                        let mut glyphs = Vec::new();
                        let mut advance = 0.0;
                        for cluster in run.visual_clusters() {
                            advance += cluster.advance();
                            let range = cluster.text_range();
                            if text[range.clone()].chars().all(char::is_control) {
                                continue;
                            }
                            let range =
                                range.start - line_range.start..range.end - line_range.start;
                            glyphs.extend(cluster.glyphs().map(|g| Glyph {
                                id: g.id,
                                text: range.clone(),
                                advance: g.advance,
                                x: g.x,
                                y: g.y,
                            }));
                        }
                        if !glyphs.is_empty() {
                            page.items.push(Item::Text {
                                face,
                                size: run.font_size(),
                                x,
                                y: baseline,
                                glyphs,
                                text: line_text.to_owned(),
                            });
                        }
                        x += advance;
                    }
                }
            }
        }
    }
    Ok(Laid { pages, images })
}
