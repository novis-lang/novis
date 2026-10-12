//! The layout: the parsed document and its computed styles as pages of positioned glyph runs,
//! rectangles and images (`rule:core-classes/pdf-css-subset`).
//!
//! A page is A4 with a 2 cm margin on every side, and every length here is in PDF points (1/72
//! inch, three quarters of a CSS pixel). An element's `display` comes from the cascade, whose
//! user-agent sheet gives HTML's defaults: `none` draws nothing, `block`, `list-item` and the
//! display types later stages lay out (`flex`, `grid`, `table` and `inline-block`) start a block
//! box, and anything else is inline.
//!
//! **Blocks** are laid top to bottom. A block box takes its margins, borders, padding, `width`,
//! `min-width`, `max-width`, `height`, `min-height` and `box-sizing` from its style, with a
//! percentage of the containing block's width, and `margin: auto` on both sides centres a box
//! that has a width. Vertical margins collapse to the larger of two that meet, through a box
//! with no border or padding on that side, and are dropped at the top of a page after the first.
//! A box's background colour and its solid borders are drawn under its content; a box that
//! goes on past a page is drawn in a slice on each page, closed at its first and last edge.
//!
//! **Text** is the inline content of a block, shaped and broken into lines by `parley` over
//! `harfrust` and `skrifa`. Each run takes its font chain (`fonts`' module doc), `font-size`,
//! `font-weight`, `font-style`, `color`, `letter-spacing`, `word-spacing`,
//! `font-feature-settings`, `text-transform` and the underline or line-through it is inside;
//! each block takes `text-align`, `line-height`, `white-space`, `word-break` and
//! `overflow-wrap`. A line that does not fit under the last on the page starts the next page. A
//! character no font in its chain has is drawn as the first font's empty box and listed in
//! [`Laid::missing`] with the chain and the page.
//!
//! A `list-item` starts with its marker, from `list-style-type` and its number among the list's
//! items, `<ol start>` and `<li value>` included. An `<img>` whose bytes are PNG, JPEG, GIF or
//! WebP is a block at its CSS `width` and `height`, else its `width` and `height` attributes,
//! else its natural size, a pixel being a CSS pixel, scaled down to fit the content box. Nothing
//! outside the HTML namespace is drawn yet.
//!
//! Memory: one shaped `parley` layout at a time, and the positioned items of every page until
//! the writer has drawn them.

use std::borrow::Cow;
use std::ops::Range;

use krilla::Data;
use krilla::image::Image;
use parley::{
    Alignment, AlignmentOptions, FontFamily, FontFamilyName, FontFeatures, FontStyle, FontWeight,
    LayoutContext, LineHeight, OverflowWrap, StyleProperty, TextWrapMode, WordBreak,
};

use crate::Error;
use crate::assets::Assets;
use crate::css::{Color, Style, Styles, Value};
use crate::fonts::{FaceData, Fonts};
use crate::parse::{Dom, Kind, Space};

/// A4's width and height in points.
pub const PAGE_WIDTH: f32 = 595.275_6;
pub const PAGE_HEIGHT: f32 = 841.889_8;
/// 2 cm in points.
pub const MARGIN: f32 = 56.692_9;

const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN;
const CONTENT_HEIGHT: f32 = PAGE_HEIGHT - 2.0 * MARGIN;

const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 1.0,
};

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
    /// A run of glyphs in one face, size and colour, its pen starting at `x` on the baseline
    /// `y`. `face` indexes [`Laid::faces`].
    Text {
        face: usize,
        size: f32,
        x: f32,
        y: f32,
        color: Color,
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
    /// A filled rectangle: a background, a border side or a text decoration.
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub items: Vec<Item>,
}

/// Text no font in its chain has: the characters, the chain by family name, and the page, from 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    pub text: String,
    pub fonts: Vec<String>,
    pub page: u64,
}

/// The laid-out document: at least one page, the images and faces its pages draw, and the text
/// it could not draw.
pub struct Laid {
    pub pages: Vec<Page>,
    pub images: Images,
    pub faces: Vec<FaceData>,
    pub missing: Vec<Missing>,
}

/// Each image a document draws, with the reference it was loaded from.
pub type Images = Vec<(Image, String)>;

/// What one run of text is drawn with.
#[derive(Debug, Clone, PartialEq)]
struct Span {
    chain: Vec<String>,
    size: f32,
    weight: f32,
    italic: bool,
    color: Color,
    letter_spacing: f32,
    word_spacing: f32,
    features: Option<String>,
    underline: Option<Color>,
    strike: Option<Color>,
}

/// What a block's lines are broken and aligned with.
#[derive(Debug, Clone, Copy)]
struct Para {
    align: Alignment,
    line_height: LineHeight,
    wrap: TextWrapMode,
    word_break: WordBreak,
    overflow_wrap: OverflowWrap,
}

/// How a text node's whitespace is kept.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Spaces {
    Collapse,
    KeepBreaks,
    Keep,
}

/// A block box's geometry: its border box's `x` and `width`, and the rest top, right, bottom,
/// left.
#[derive(Debug, Clone)]
struct BoxGeometry {
    x: f32,
    width: f32,
    margin: [f32; 4],
    border: [f32; 4],
    border_color: [Color; 4],
    padding: [f32; 4],
    background: Option<Color>,
    /// The content box's least height, from `height` and `min-height`.
    min_content: f32,
}

enum Block {
    Open(BoxGeometry),
    Close,
    Text {
        text: String,
        spans: Vec<(Range<usize>, usize)>,
        para: Para,
        x: f32,
        width: f32,
    },
    Image {
        image: usize,
        x: f32,
        width: f32,
        height: f32,
    },
}

/// What every element on the stack passes to its content.
#[derive(Debug, Clone)]
struct Frame {
    /// The content box's left edge and width.
    x: f32,
    width: f32,
    /// The index in [`Builder::spans`] of this element's text style.
    span: usize,
    spaces: Spaces,
    para: Para,
    transform: Option<&'static str>,
    /// The last list item number among this element's children.
    items: i64,
}

fn keyword<'s>(style: &'s Style, name: &str) -> Option<&'s str> {
    match style.get(name) {
        Some(Value::Keyword(word)) => Some(word),
        _ => None,
    }
}

/// A length against `basis`, or `None` for a keyword or no value.
fn length(style: &Style, name: &str, basis: f32) -> Option<f32> {
    match style.get(name) {
        Some(Value::Length(l)) => Some(l.pt + l.percent / 100.0 * basis),
        _ => None,
    }
}

fn color(style: &Style, name: &str) -> Option<Color> {
    match style.get(name) {
        Some(Value::Color(c)) if c.a > 0.0 => Some(*c),
        _ => None,
    }
}

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

fn border_width(style: &Style, side: &str) -> f32 {
    match keyword(style, &format!("border-{side}-style")) {
        None | Some("none") | Some("hidden") => return 0.0,
        Some(_) => {}
    }
    let name = format!("border-{side}-width");
    match style.get(&name) {
        Some(Value::Length(l)) => l.pt,
        Some(Value::Keyword(w)) if w == "thin" => 0.75,
        Some(Value::Keyword(w)) if w == "thick" => 3.75,
        _ => 2.25,
    }
}

fn geometry(style: &Style, x: f32, containing: f32) -> BoxGeometry {
    let side = |prefix: &str, suffix: &str| {
        SIDES.map(|s| length(style, &format!("{prefix}-{s}{suffix}"), containing).unwrap_or(0.0))
    };
    let mut margin = side("margin", "");
    let padding = side("padding", "");
    let border = SIDES.map(|s| border_width(style, s));
    let text_color = color(style, "color").unwrap_or(BLACK);
    let border_color =
        SIDES.map(|s| color(style, &format!("border-{s}-color")).unwrap_or(text_color));
    let insets = border[1] + border[3] + padding[1] + padding[3];
    let border_box = keyword(style, "box-sizing") == Some("border-box");
    let to_border = |w: f32| if border_box { w } else { w + insets };
    let mut width = match length(style, "width", containing) {
        Some(w) => to_border(w),
        None => containing - margin[1] - margin[3],
    };
    if let Some(max) = length(style, "max-width", containing) {
        width = width.min(to_border(max));
    }
    if let Some(min) = length(style, "min-width", containing) {
        width = width.max(to_border(min));
    }
    width = width.max(insets);
    let auto = |s: &str| keyword(style, &format!("margin-{s}")) == Some("auto");
    if style.get("width").is_some() || style.get("max-width").is_some() {
        let free = (containing - width - margin[1] - margin[3]).max(0.0);
        match (auto("left"), auto("right")) {
            (true, true) => margin[3] += free / 2.0,
            (true, false) => margin[3] += free,
            _ => {}
        }
    }
    let vertical = border[0] + border[2] + padding[0] + padding[2];
    let to_content = |h: f32| {
        if border_box {
            (h - vertical).max(0.0)
        } else {
            h
        }
    };
    // A percentage height counts against a height the containing block does not have.
    let fixed = |name: &str| match style.get(name) {
        Some(Value::Length(l)) if l.percent == 0.0 => Some(to_content(l.pt)),
        _ => None,
    };
    let min_content = fixed("height")
        .unwrap_or(0.0)
        .max(fixed("min-height").unwrap_or(0.0));
    BoxGeometry {
        x: x + margin[3],
        width,
        margin,
        border,
        border_color,
        padding,
        background: color(style, "background-color"),
        min_content,
    }
}

fn span_of(style: &Style, fonts: &Fonts, parent: Option<&Span>) -> Span {
    let size = style.font_size();
    let families = match style.get("font-family") {
        Some(Value::Families(f)) => f.clone(),
        _ => Vec::new(),
    };
    let decoration = match style.get("text-decoration-line") {
        Some(Value::Keywords(words)) => words.clone(),
        _ => Vec::new(),
    };
    let color_of_text = color(style, "color").unwrap_or(BLACK);
    let line = color(style, "text-decoration-color").unwrap_or(color_of_text);
    let has = |w: &str| decoration.iter().any(|d| d == w).then_some(line);
    let spacing = |name: &str| length(style, name, size).unwrap_or(0.0);
    Span {
        chain: fonts.chain(&families),
        size,
        weight: match style.get("font-weight") {
            Some(Value::Number(n)) => *n,
            _ => 400.0,
        },
        italic: matches!(keyword(style, "font-style"), Some("italic" | "oblique")),
        color: color_of_text,
        letter_spacing: spacing("letter-spacing"),
        word_spacing: spacing("word-spacing"),
        features: match style.get("font-feature-settings") {
            Some(Value::Text(t)) if t != "normal" => Some(t.clone()),
            _ => None,
        },
        underline: has("underline").or(parent.and_then(|p| p.underline)),
        strike: has("line-through").or(parent.and_then(|p| p.strike)),
    }
}

fn para_of(style: &Style) -> Para {
    let size = style.font_size();
    let white = keyword(style, "white-space");
    Para {
        align: match keyword(style, "text-align") {
            Some("left") => Alignment::Left,
            Some("right") => Alignment::Right,
            Some("center") => Alignment::Center,
            Some("justify") => Alignment::Justify,
            Some("end") => Alignment::End,
            _ => Alignment::Start,
        },
        line_height: match style.get("line-height") {
            Some(Value::Number(n)) => LineHeight::FontSizeRelative(*n),
            Some(Value::Length(l)) => LineHeight::Absolute(l.pt + l.percent / 100.0 * size),
            _ => LineHeight::MetricsRelative(1.0),
        },
        wrap: if matches!(white, Some("nowrap" | "pre")) {
            TextWrapMode::NoWrap
        } else {
            TextWrapMode::Wrap
        },
        word_break: match keyword(style, "word-break") {
            Some("break-all") => WordBreak::BreakAll,
            Some("keep-all") => WordBreak::KeepAll,
            _ => WordBreak::Normal,
        },
        overflow_wrap: match keyword(style, "overflow-wrap") {
            Some("anywhere") => OverflowWrap::Anywhere,
            Some("break-word") => OverflowWrap::BreakWord,
            _ if keyword(style, "word-break") == Some("break-word") => OverflowWrap::BreakWord,
            _ => OverflowWrap::Normal,
        },
    }
}

fn spaces_of(style: &Style) -> Spaces {
    match keyword(style, "white-space") {
        Some("pre" | "pre-wrap" | "break-spaces") => Spaces::Keep,
        Some("pre-line") => Spaces::KeepBreaks,
        _ => Spaces::Collapse,
    }
}

/// `n` as `list-style-type` writes it, without the suffix.
fn marker(kind: &str, n: i64) -> Option<String> {
    let roman = |n: i64| {
        let mut n = n;
        let mut out = String::new();
        for (value, digits) in [
            (1000, "m"),
            (900, "cm"),
            (500, "d"),
            (400, "cd"),
            (100, "c"),
            (90, "xc"),
            (50, "l"),
            (40, "xl"),
            (10, "x"),
            (9, "ix"),
            (5, "v"),
            (4, "iv"),
            (1, "i"),
        ] {
            while n >= value {
                out.push_str(digits);
                n -= value;
            }
        }
        out
    };
    let alpha = |n: i64, first: u8, count: i64| {
        let mut n = n;
        let mut out = Vec::new();
        while n > 0 {
            n -= 1;
            out.push(first + (n % count) as u8);
            n /= count;
        }
        out.reverse();
        String::from_utf8(out).unwrap_or_default()
    };
    Some(match kind {
        "none" => return None,
        "disc" => "\u{2022}".to_owned(),
        "circle" => "\u{25e6}".to_owned(),
        "square" => "\u{25aa}".to_owned(),
        "decimal-leading-zero" => format!("{n:02}."),
        "lower-roman" if (1..4000).contains(&n) => format!("{}.", roman(n)),
        "upper-roman" if (1..4000).contains(&n) => format!("{}.", roman(n).to_uppercase()),
        "lower-alpha" | "lower-latin" if n > 0 => format!("{}.", alpha(n, b'a', 26)),
        "upper-alpha" | "upper-latin" if n > 0 => format!("{}.", alpha(n, b'A', 26)),
        "lower-greek" if (1..=24).contains(&n) => {
            format!(
                "{}.",
                char::from_u32(0x3b0 + n as u32 + u32::from(n > 17)).unwrap_or('?')
            )
        }
        _ => format!("{n}."),
    })
}

/// Collects a document's blocks in order.
struct Builder<'a, 'd> {
    assets: &'a Assets<'d>,
    blocks: Vec<Block>,
    images: Images,
    spans: Vec<Span>,
    text: String,
    runs: Vec<(Range<usize>, usize)>,
    /// The block the text being collected is laid out in: its paragraph style and content box.
    owner: Option<(Para, f32, f32)>,
    /// Whether the text so far ends in a collapsible space, or is empty.
    after_space: bool,
}

impl Builder<'_, '_> {
    fn push_text(&mut self, text: &str, frame: &Frame) {
        let start = self.text.len();
        let text: Cow<'_, str> = match frame.transform {
            Some("uppercase") => text.to_uppercase().into(),
            Some("lowercase") => text.to_lowercase().into(),
            Some("capitalize") => {
                let mut out = String::with_capacity(text.len());
                let mut word_start = self.after_space || self.text.is_empty();
                for c in text.chars() {
                    if word_start && c.is_alphabetic() {
                        out.extend(c.to_uppercase());
                    } else {
                        out.push(c);
                    }
                    word_start = c.is_whitespace();
                }
                out.into()
            }
            _ => text.into(),
        };
        for c in text.chars() {
            let space = matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r');
            match frame.spaces {
                Spaces::Keep => self.text.push(c),
                Spaces::KeepBreaks if c == '\n' => {
                    while self.text.ends_with(' ') {
                        self.text.pop();
                    }
                    self.text.push('\n');
                    self.after_space = true;
                    continue;
                }
                _ if space => {
                    if !self.after_space {
                        self.text.push(' ');
                        self.after_space = true;
                    }
                    continue;
                }
                _ => self.text.push(c),
            }
            self.after_space = false;
        }
        if self.text.len() > start {
            if self.owner.is_none() {
                self.owner = Some((frame.para, frame.x, frame.width));
            }
            match self.runs.last_mut() {
                Some((range, span)) if *span == frame.span && range.end == start => {
                    range.end = self.text.len();
                }
                _ => self.runs.push((start..self.text.len(), frame.span)),
            }
        }
    }

    /// Ends the text block being collected, if it has anything to draw.
    fn flush(&mut self) {
        let trimmed = self.text.trim_end_matches([' ', '\n']).len();
        self.text.truncate(trimmed);
        self.runs.retain_mut(|(range, _)| {
            range.end = range.end.min(trimmed);
            range.start < range.end
        });
        if let Some((para, x, width)) = self.owner.take()
            && !self.runs.is_empty()
        {
            self.blocks.push(Block::Text {
                text: std::mem::take(&mut self.text),
                spans: std::mem::take(&mut self.runs),
                para,
                x,
                width,
            });
        }
        self.text.clear();
        self.runs.clear();
        self.after_space = true;
    }

    /// An `<img>` at `at`, laid out in `frame`'s content box.
    fn image(&mut self, dom: &Dom, at: usize, style: &Style, frame: &Frame) -> Result<(), Error> {
        let Some(reference) = dom.nodes[at].attr("src") else {
            return Ok(());
        };
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
        let (w, h) = image.size();
        let (natural_w, natural_h) = (w as f32 * 0.75, h as f32 * 0.75);
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
        self.images.push((image, name));
        self.blocks.push(Block::Image {
            image: self.images.len() - 1,
            x: frame.x,
            width,
            height,
        });
        Ok(())
    }
}

/// One step of the walk in [`blocks`].
enum Step {
    Enter(usize),
    /// Leaves the element whose frame is on top, closing its box when it is a block.
    Exit {
        block: bool,
    },
}

/// The blocks of `dom`, in document order, and the text styles they name.
fn blocks(
    dom: &Dom,
    styles: &Styles,
    fonts: &Fonts,
    assets: &Assets<'_>,
) -> Result<(Vec<Block>, Images, Vec<Span>), Error> {
    let empty = Style::default();
    let mut builder = Builder {
        assets,
        blocks: Vec::new(),
        images: Vec::new(),
        spans: vec![span_of(&empty, fonts, None)],
        text: String::new(),
        runs: Vec::new(),
        owner: None,
        after_space: true,
    };
    let root = Frame {
        x: MARGIN,
        width: CONTENT_WIDTH,
        span: 0,
        spaces: Spaces::Collapse,
        para: para_of(&empty),
        transform: None,
        items: 0,
    };
    // An explicit stack, so a deep tree costs heap and never the call stack.
    let mut frames = vec![root];
    let mut work = vec![Step::Enter(0)];
    while let Some(step) = work.pop() {
        let at = match step {
            Step::Enter(at) => at,
            Step::Exit { block } => {
                frames.pop();
                if block {
                    builder.flush();
                    builder.blocks.push(Block::Close);
                }
                continue;
            }
        };
        let node = &dom.nodes[at];
        let frame = frames.last().expect("the root frame stays").clone();
        match &node.kind {
            Kind::Document => {
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
            Kind::Text(text) => builder.push_text(text, &frame),
            Kind::Ignored => {}
            Kind::Element { name, space, .. } => {
                if *space != Space::Html {
                    continue;
                }
                let Some(style) = styles.of(at) else {
                    continue;
                };
                let display = keyword(style, "display").unwrap_or("inline");
                if display == "none" {
                    continue;
                }
                let name = name.as_str();
                if name == "img" {
                    builder.image(dom, at, style, &frame)?;
                    continue;
                }
                if name == "br" {
                    builder.text.push('\n');
                    builder.after_space = true;
                    continue;
                }
                let parent_span = &builder.spans[frame.span];
                let span = span_of(style, fonts, Some(parent_span));
                let span = if span == *parent_span {
                    frame.span
                } else {
                    builder.spans.push(span);
                    builder.spans.len() - 1
                };
                let mut inner = Frame {
                    span,
                    spaces: spaces_of(style),
                    transform: match keyword(style, "text-transform") {
                        Some("uppercase") => Some("uppercase"),
                        Some("lowercase") => Some("lowercase"),
                        Some("capitalize") => Some("capitalize"),
                        _ => None,
                    },
                    items: if name == "ol" {
                        node.attr("start")
                            .and_then(|s| s.trim().parse::<i64>().ok())
                            .unwrap_or(1)
                            - 1
                    } else {
                        0
                    },
                    ..frame.clone()
                };
                let block = display != "inline";
                if block {
                    builder.flush();
                    let geometry = geometry(style, frame.x, frame.width);
                    inner.x = geometry.x + geometry.border[3] + geometry.padding[3];
                    inner.width = (geometry.width
                        - geometry.border[1]
                        - geometry.border[3]
                        - geometry.padding[1]
                        - geometry.padding[3])
                        .max(0.0);
                    inner.para = para_of(style);
                    builder.blocks.push(Block::Open(geometry));
                }
                if display == "list-item" {
                    let parent = frames.last_mut().expect("the root frame stays");
                    parent.items = node
                        .attr("value")
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(parent.items + 1);
                    let kind = keyword(style, "list-style-type").unwrap_or("disc");
                    if let Some(text) = marker(kind, parent.items) {
                        let marked = Frame {
                            spaces: Spaces::Keep,
                            transform: None,
                            ..inner.clone()
                        };
                        builder.push_text(&format!("{text} "), &marked);
                        builder.after_space = true;
                    }
                }
                frames.push(inner);
                work.push(Step::Exit { block });
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
        }
    }
    builder.flush();
    Ok((builder.blocks, builder.images, builder.spans))
}

/// A block box whose bottom has not been reached.
struct Open {
    geometry: BoxGeometry,
    /// Where its border box starts: the page, the index its items are drawn under, and the top.
    /// `None` until something inside it, or its own top border or padding, is placed.
    top: Option<(usize, usize, f32)>,
}

/// The vertical flow: the pages so far and where the next block goes.
struct Flow {
    pages: Vec<Page>,
    cursor: f32,
    pending: f32,
    open: Vec<Open>,
}

impl Flow {
    /// Moves to a new page when `height` does not fit under the last block, unless the page is
    /// empty, and returns the top of the box `height` is placed in.
    fn place(&mut self, height: f32) -> f32 {
        let empty = self.pages.last().is_some_and(|p| p.items.is_empty());
        let margin = if empty && self.pages.len() > 1 {
            0.0
        } else {
            self.pending
        };
        if !empty && self.cursor + margin + height > CONTENT_HEIGHT {
            self.pages.push(Page::default());
            self.cursor = 0.0;
        } else {
            self.cursor += margin;
        }
        self.pending = 0.0;
        let top = self.cursor;
        let page = self.pages.len() - 1;
        let item = self.pages[page].items.len();
        for open in &mut self.open {
            open.top.get_or_insert((page, item, top));
        }
        self.cursor += height;
        top
    }

    fn page(&mut self) -> &mut Page {
        self.pages.last_mut().expect("there is always a page")
    }

    fn open(&mut self, geometry: BoxGeometry) {
        self.pending = self.pending.max(geometry.margin[0]);
        let inset = geometry.border[0] + geometry.padding[0];
        self.open.push(Open {
            geometry,
            top: None,
        });
        if inset > 0.0 {
            self.place(inset);
        }
    }

    fn close(&mut self) {
        let Some(open) = self.open.pop() else {
            return;
        };
        let g = &open.geometry;
        let inset = g.border[2] + g.padding[2];
        let mut top = open.top;
        if top.is_none() && (inset > 0.0 || g.min_content > 0.0) {
            self.open.push(Open {
                geometry: g.clone(),
                top: None,
            });
            self.place(0.0);
            top = self.open.pop().and_then(|o| o.top);
        }
        if let Some((page, _, at)) = top
            && page == self.pages.len() - 1
        {
            let want = at + g.border[0] + g.padding[0] + g.min_content;
            if want > self.cursor {
                self.cursor += self.pending.min(want - self.cursor);
                self.pending = 0.0;
                if want > self.cursor {
                    let more = want - self.cursor;
                    self.place(more);
                }
            }
        }
        if inset > 0.0 {
            self.place(inset);
        }
        if let Some((first, item, at)) = top {
            let last = self.pages.len() - 1;
            for page in first..=last {
                let top = if page == first { at } else { 0.0 };
                let bottom = if page == last {
                    self.cursor
                } else {
                    CONTENT_HEIGHT
                };
                let mut items = Vec::new();
                let (x, y, w, h) = (g.x, MARGIN + top, g.width, bottom - top);
                let mut rect = |x: f32, y: f32, width: f32, height: f32, color: Color| {
                    if width > 0.0 && height > 0.0 {
                        items.push(Item::Rect {
                            x,
                            y,
                            width,
                            height,
                            color,
                        });
                    }
                };
                if let Some(background) = g.background {
                    rect(x, y, w, h, background);
                }
                if page == first {
                    rect(x, y, w, g.border[0], g.border_color[0]);
                }
                if page == last {
                    rect(x, y + h - g.border[2], w, g.border[2], g.border_color[2]);
                }
                rect(x + w - g.border[1], y, g.border[1], h, g.border_color[1]);
                rect(x, y, g.border[3], h, g.border_color[3]);
                let under = if page == first { item } else { 0 };
                let page = &mut self.pages[page].items;
                page.splice(under..under, items);
            }
        }
        self.pending = self.pending.max(open.geometry.margin[2]);
    }
}

/// Lays `dom` out in pages with its computed `styles` and `fonts`.
pub fn layout(
    dom: &Dom,
    styles: &Styles,
    assets: &Assets<'_>,
    mut fonts: Fonts,
) -> Result<Laid, Error> {
    let (blocks, images, spans) = blocks(dom, styles, &fonts, assets)?;
    let mut lcx: LayoutContext<usize> = LayoutContext::new();
    let mut flow = Flow {
        pages: vec![Page::default()],
        cursor: 0.0,
        pending: 0.0,
        open: Vec::new(),
    };
    let mut missing: Vec<Missing> = Vec::new();

    for block in blocks {
        match block {
            Block::Open(geometry) => flow.open(geometry),
            Block::Close => flow.close(),
            Block::Image {
                image,
                x,
                width,
                height,
            } => {
                let top = flow.place(height);
                flow.page().items.push(Item::Image {
                    image,
                    x,
                    y: MARGIN + top,
                    width,
                    height,
                });
            }
            Block::Text {
                text,
                spans: runs,
                para,
                x: left,
                width,
            } => {
                let mut builder = lcx.ranged_builder(&mut fonts.context, &text, 1.0, false);
                builder.push_default(para.line_height);
                builder.push_default(StyleProperty::WordBreak(para.word_break));
                builder.push_default(StyleProperty::OverflowWrap(para.overflow_wrap));
                builder.push_default(StyleProperty::TextWrapMode(para.wrap));
                for (range, index) in &runs {
                    let span = &spans[*index];
                    let range = range.clone();
                    let chain: Vec<FontFamilyName<'_>> = span
                        .chain
                        .iter()
                        .map(|name| FontFamilyName::Named(Cow::Borrowed(name.as_str())))
                        .collect();
                    builder.push(
                        StyleProperty::FontFamily(FontFamily::List(Cow::Owned(chain))),
                        range.clone(),
                    );
                    builder.push(StyleProperty::FontSize(span.size), range.clone());
                    builder.push(
                        StyleProperty::FontWeight(FontWeight::new(span.weight)),
                        range.clone(),
                    );
                    if span.italic {
                        builder.push(StyleProperty::FontStyle(FontStyle::Italic), range.clone());
                    }
                    if span.letter_spacing != 0.0 {
                        builder.push(
                            StyleProperty::LetterSpacing(span.letter_spacing),
                            range.clone(),
                        );
                    }
                    if span.word_spacing != 0.0 {
                        builder.push(StyleProperty::WordSpacing(span.word_spacing), range.clone());
                    }
                    if let Some(features) = &span.features {
                        builder.push(
                            StyleProperty::FontFeatures(FontFeatures::Source(Cow::Borrowed(
                                features.as_str(),
                            ))),
                            range.clone(),
                        );
                    }
                    builder.push(StyleProperty::Brush(*index), range);
                }
                let mut shaped = builder.build(&text);
                shaped.break_all_lines(Some(width));
                shaped.align(para.align, AlignmentOptions::default());
                for line in shaped.lines() {
                    let metrics = line.metrics();
                    let height = metrics.block_max_coord - metrics.block_min_coord;
                    let top = flow.place(height);
                    let page_number = flow.pages.len() as u64;
                    let baseline = MARGIN + top + (metrics.baseline - metrics.block_min_coord);
                    let line_range = line.text_range();
                    let line_text = &text[line_range.clone()];
                    let mut x = left + metrics.inline_min_coord + metrics.offset;
                    let mut items = Vec::new();
                    for run in line.runs() {
                        let face = fonts
                            .face_index(run.font().data.data())
                            .expect("the font context holds only the render's faces");
                        let font_metrics = run.font_metrics();
                        // One item per span the run's clusters change between: the span, the
                        // pen's start, the glyphs and their advance.
                        let mut current: Option<(usize, f32, Vec<Glyph>, f32)> = None;
                        let mut gap: Option<(String, usize)> = None;
                        let finish = |current: Option<(usize, f32, Vec<Glyph>, f32)>,
                                      items: &mut Vec<Item>| {
                            let Some((index, start, glyphs, advance)) = current else {
                                return;
                            };
                            let span = &spans[index];
                            for (color, offset, size) in [
                                (
                                    span.underline,
                                    font_metrics.underline_offset,
                                    font_metrics.underline_size,
                                ),
                                (
                                    span.strike,
                                    font_metrics.strikethrough_offset,
                                    font_metrics.strikethrough_size,
                                ),
                            ] {
                                if let Some(color) = color {
                                    items.push(Item::Rect {
                                        x: start,
                                        y: baseline - offset - size / 2.0,
                                        width: advance,
                                        height: size.max(0.5),
                                        color,
                                    });
                                }
                            }
                            if !glyphs.is_empty() {
                                items.push(Item::Text {
                                    face,
                                    size: run.font_size(),
                                    x: start,
                                    y: baseline,
                                    color: span.color,
                                    glyphs,
                                    text: line_text.to_owned(),
                                });
                            }
                        };
                        for cluster in run.visual_clusters() {
                            let index = cluster.style().brush;
                            if current.as_ref().is_some_and(|c| c.0 != index) {
                                finish(current.take(), &mut items);
                            }
                            let c = current.get_or_insert_with(|| (index, x, Vec::new(), 0.0));
                            let range = cluster.text_range();
                            let cluster_text = &text[range.clone()];
                            x += cluster.advance();
                            if cluster_text.chars().all(char::is_control) {
                                match c.2.last_mut() {
                                    Some(last) => {
                                        last.advance += cluster.advance();
                                        c.3 += cluster.advance();
                                    }
                                    None => c.1 += cluster.advance(),
                                }
                                continue;
                            }
                            c.3 += cluster.advance();
                            let glyphs = &mut c.2;
                            let start = glyphs.len();
                            let local =
                                range.start - line_range.start..range.end - line_range.start;
                            let mut drawn = 0.0;
                            for g in cluster.glyphs() {
                                drawn += g.advance;
                                glyphs.push(Glyph {
                                    id: g.id,
                                    text: local.clone(),
                                    advance: g.advance,
                                    x: g.x,
                                    y: g.y,
                                });
                            }
                            // Justification and spacing widen the cluster, not its glyphs.
                            if glyphs.len() > start
                                && let Some(last) = glyphs.last_mut()
                            {
                                last.advance += cluster.advance() - drawn;
                            }
                            let lost = glyphs[start..].iter().any(|g| g.id == 0)
                                && !cluster_text.chars().all(char::is_whitespace);
                            match (&mut gap, lost) {
                                (Some((missed, at)), true) if *at == index => {
                                    missed.push_str(cluster_text)
                                }
                                (_, true) => {
                                    if let Some((missed, at)) = gap.take() {
                                        report(&mut missing, missed, &spans[at], page_number);
                                    }
                                    gap = Some((cluster_text.to_owned(), index));
                                }
                                (_, false) => {
                                    if let Some((missed, at)) = gap.take() {
                                        report(&mut missing, missed, &spans[at], page_number);
                                    }
                                }
                            }
                        }
                        finish(current.take(), &mut items);
                        if let Some((missed, at)) = gap.take() {
                            report(&mut missing, missed, &spans[at], page_number);
                        }
                    }
                    flow.page().items.extend(items);
                }
            }
        }
    }
    while !flow.open.is_empty() {
        flow.close();
    }
    Ok(Laid {
        pages: flow.pages,
        images,
        faces: fonts.faces,
        missing,
    })
}

/// Adds `text` to the missing list, once per chain and page.
fn report(missing: &mut Vec<Missing>, text: String, span: &Span, page: u64) {
    let entry = Missing {
        text,
        fonts: span.chain.clone(),
        page,
    };
    if !missing.contains(&entry) {
        missing.push(entry);
    }
}
