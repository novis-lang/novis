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
//! A box's background, borders and outline are drawn under its content as paths (`boxes`'
//! module doc), and a box under `opacity` is drawn with its content in one group. A box that
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
//! A block's base direction is its `direction`, or the first strong character of its text under
//! `unicode-bidi: plaintext`, and `parley` runs the Unicode bidi algorithm over each paragraph.
//! An inline box's `unicode-bidi` is written into its paragraph's text as the bidi control
//! characters that open and close it, and a block's override is written around its text; a
//! cluster of control characters draws nothing.
//!
//! **Hyphenation**: under `hyphens: auto`, each word of a left-to-right paragraph whose `lang`
//! `hypher` knows gets a soft hyphen at each of its break points, from that language's patterns.
//! A soft hyphen draws nothing. A line that ends at one, written by `hypher` or by the author, is
//! broken and aligned narrower by the width of a hyphen, and a hyphen is drawn at its end. Each
//! such line costs one more line-breaking pass over its paragraph, and the patterns of every
//! language are in the component.
//!
//! **Counters** follow CSS: `counter-reset` makes a counter that the element's following
//! siblings and every descendant see, and `counter-increment` changes the innermost one of that
//! name. A `list-item` increments `list-item`, which `<ul>` and `<ol>` reset and `<ol start>`
//! and `<li value>` set. Its marker is that number in its `list-style-type`, drawn outside the
//! content box level with the item's first line, or as its first text under
//! `list-style-position: inside`. A `::before` or `::after` box is inline text at the start or
//! end of its element, from the strings, `counter()`, `counters()`, `attr()` and quotes of its
//! `content`, after its own counter properties apply.
//!
//! An `<img>` whose bytes are PNG, JPEG, GIF or
//! WebP is a block at its CSS `width` and `height`, else its `width` and `height` attributes,
//! else its natural size, a pixel being a CSS pixel, scaled down to fit the content box. Nothing
//! outside the HTML namespace is drawn yet.
//!
//! Memory: one shaped `parley` layout at a time, and the positioned items of every page until
//! the writer has drawn them.

use std::borrow::Cow;
use std::ops::Range;

use hypher::Lang;
use krilla::Data;
use krilla::image::Image;
use parley::{
    Alignment, AlignmentOptions, BaseDirection, FontFamily, FontFamilyName, FontFeatures,
    FontStyle, FontWeight, Layout, LayoutContext, LineHeight, OverflowWrap, RangedBuilder,
    StyleProperty, TextWrapMode, WordBreak, YieldData,
};

use cssparser::{Parser, Token};

mod boxes;

pub use boxes::{Paint, Seg, Stop, Stroke, bounds};

use boxes::Decor;

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
    /// A shape: a background, a border, an outline or a text decoration. It is stroked when it
    /// has a `stroke`, and filled with the even-odd rule when it has none.
    Path {
        path: Vec<Seg>,
        paint: Paint,
        stroke: Option<Stroke>,
    },
    /// Starts a group that ends at the matching [`Item::End`]: its items are drawn at `opacity`,
    /// and only inside `clip` when it has one.
    Begin {
        opacity: f32,
        clip: Option<Vec<Seg>>,
    },
    End,
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
    direction: BaseDirection,
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
    padding: [f32; 4],
    /// The content box's least height, from `height` and `min-height`.
    min_content: f32,
    decor: Decor,
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
        /// A list item's marker, drawn on one line that ends at `x` and level with the first
        /// line laid out after it.
        marker: bool,
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
    /// The language of the element's `lang`, if `hypher` has its patterns.
    lang: Option<Lang>,
    /// The language this element's words are hyphenated in, under `hyphens: auto`.
    hyphenate: Option<Lang>,
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
        padding,
        min_content,
        decor: boxes::decor(style),
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
        direction: match (keyword(style, "unicode-bidi"), keyword(style, "direction")) {
            (Some("plaintext"), _) => BaseDirection::Auto,
            (_, Some("rtl")) => BaseDirection::Rtl,
            _ => BaseDirection::Ltr,
        },
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

/// The bidi control characters that open and close a box's `unicode-bidi` in its paragraph's
/// text, by its `direction`. A block is its own paragraph, so only an override changes it.
fn bidi_controls(style: &Style, block: bool) -> Option<(&'static str, &'static str)> {
    let rtl = keyword(style, "direction") == Some("rtl");
    let pick = |ltr: &'static str, rtl_: &'static str| if rtl { rtl_ } else { ltr };
    match keyword(style, "unicode-bidi") {
        Some("embed") if !block => Some((pick("\u{202a}", "\u{202b}"), "\u{202c}")),
        Some("isolate") if !block => Some((pick("\u{2066}", "\u{2067}"), "\u{2069}")),
        Some("plaintext") if !block => Some(("\u{2068}", "\u{2069}")),
        Some("isolate-override") if !block => Some((
            pick("\u{2066}\u{202d}", "\u{2067}\u{202e}"),
            "\u{202c}\u{2069}",
        )),
        Some("bidi-override" | "isolate-override") => {
            Some((pick("\u{202d}", "\u{202e}"), "\u{202c}"))
        }
        _ => None,
    }
}

/// Whether `c` is one of the characters [`bidi_controls`] writes.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn spaces_of(style: &Style) -> Spaces {
    match keyword(style, "white-space") {
        Some("pre" | "pre-wrap" | "break-spaces") => Spaces::Keep,
        Some("pre-line") => Spaces::KeepBreaks,
        _ => Spaces::Collapse,
    }
}

/// A list item's marker: `n` as `list-style-type` writes it, a number followed by a full stop.
fn marker(kind: &str, n: i64) -> Option<String> {
    let text = counter_text(kind, n)?;
    Some(if matches!(kind, "disc" | "circle" | "square") {
        text
    } else {
        format!("{text}.")
    })
}

/// `n` as the counter style `kind` writes it, without a suffix. A style with no form for `n`
/// writes it in decimal.
fn counter_text(kind: &str, n: i64) -> Option<String> {
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
        "decimal-leading-zero" => format!("{n:02}"),
        "lower-roman" if (1..4000).contains(&n) => roman(n),
        "upper-roman" if (1..4000).contains(&n) => roman(n).to_uppercase(),
        "lower-alpha" | "lower-latin" if n > 0 => alpha(n, b'a', 26),
        "upper-alpha" | "upper-latin" if n > 0 => alpha(n, b'A', 26),
        "lower-greek" if (1..=24).contains(&n) => {
            char::from_u32(0x3b0 + n as u32 + u32::from(n > 17))
                .unwrap_or('?')
                .to_string()
        }
        _ => n.to_string(),
    })
}

/// A counter of CSS's `counter-reset` and `counter-increment`. `scope` is the depth of the
/// frame whose element's children see it: the parent of the element that created it.
struct Counter {
    name: String,
    value: i64,
    scope: usize,
}

/// The counters in scope, outermost first.
#[derive(Default)]
struct Counters(Vec<Counter>);

impl Counters {
    /// `counter-reset`: a new counter, in place of one a sibling made.
    fn reset(&mut self, name: &str, value: i64, scope: usize) {
        match self
            .0
            .iter_mut()
            .rfind(|c| c.name == name && c.scope == scope)
        {
            Some(c) => c.value = value,
            None => self.0.push(Counter {
                name: name.to_owned(),
                value,
                scope,
            }),
        }
    }

    /// `counter-increment` by `by`, or `counter-set` to `value` when `set`. A name no counter has
    /// starts one at 0 first.
    fn change(&mut self, name: &str, by: i64, set: bool, scope: usize) {
        match self.0.iter_mut().rfind(|c| c.name == name) {
            Some(c) if set => c.value = by,
            Some(c) => c.value = c.value.saturating_add(by),
            None => self.reset(name, by, scope),
        }
    }

    /// The innermost counter's value, and every counter's from the outermost.
    fn get(&self, name: &str) -> i64 {
        self.0
            .iter()
            .rfind(|c| c.name == name)
            .map_or(0, |c| c.value)
    }

    fn all(&self, name: &str) -> Vec<i64> {
        let all: Vec<i64> = self
            .0
            .iter()
            .filter(|c| c.name == name)
            .map(|c| c.value)
            .collect();
        if all.is_empty() { vec![0] } else { all }
    }

    /// Ends the counters of a frame deeper than `depth`.
    fn leave(&mut self, depth: usize) {
        self.0.retain(|c| c.scope <= depth);
    }

    /// Applies `style`'s `counter-reset` and `counter-increment` for an element whose parent is
    /// the frame at `scope`.
    fn apply(&mut self, style: &Style, scope: usize) {
        for (name, value) in counter_list(style, "counter-reset", 0) {
            self.reset(&name, value, scope);
        }
        for (name, by) in counter_list(style, "counter-increment", 1) {
            self.change(&name, by, false, scope);
        }
    }
}

/// The names and numbers of `counter-reset` or `counter-increment`, `default` where a name has
/// no number.
fn counter_list(style: &Style, property: &str, default: i64) -> Vec<(String, i64)> {
    let Some(Value::Text(text)) = style.get(property) else {
        return Vec::new();
    };
    let mut parser = Parser::new(text);
    let mut out: Vec<(String, i64)> = Vec::new();
    while let Ok(token) = parser.next() {
        match token {
            Token::Ident(name) if !name.eq_ignore_ascii_case("none") => {
                out.push((name.to_string(), default));
            }
            Token::Number {
                int_value: Some(n), ..
            } => {
                if let Some(last) = out.last_mut() {
                    last.1 = i64::from(*n);
                }
            }
            _ => {}
        }
    }
    out
}

/// The text of a `::before` or `::after` box's `content`, `None` for `none`, `normal` or no
/// value: its strings, `counter()`, `counters()`, `attr()` and quotes, in order.
fn content_text(style: &Style, dom: &Dom, at: usize, counters: &Counters) -> Option<String> {
    let Some(Value::Text(text)) = style.get("content") else {
        return None;
    };
    let mut parser = Parser::new(text);
    let mut out = String::new();
    while let Ok(token) = parser.next() {
        match token.clone() {
            Token::QuotedString(s) => out.push_str(&s),
            Token::Ident(name) => match &*name.to_ascii_lowercase() {
                "none" | "normal" => return None,
                "open-quote" => out.push('\u{201c}'),
                "close-quote" => out.push('\u{201d}'),
                _ => {}
            },
            Token::Function(name) => {
                let name = name.to_ascii_lowercase();
                let _ = parser.parse_nested_block(|p| -> Result<(), cssparser::ParseError<()>> {
                    let mut idents: Vec<String> = Vec::new();
                    let mut separator = String::new();
                    while let Ok(token) = p.next() {
                        match token {
                            Token::Ident(i) => idents.push(i.to_string()),
                            Token::QuotedString(s) => separator = s.to_string(),
                            _ => {}
                        }
                    }
                    let Some(first) = idents.first() else {
                        return Ok(());
                    };
                    let kind = idents.get(1).map_or("decimal", String::as_str);
                    match name.as_str() {
                        "counter" => {
                            out.push_str(
                                &counter_text(kind, counters.get(first)).unwrap_or_default(),
                            );
                        }
                        "counters" => {
                            let parts: Vec<String> = counters
                                .all(first)
                                .into_iter()
                                .filter_map(|n| counter_text(kind, n))
                                .collect();
                            out.push_str(&parts.join(&separator));
                        }
                        "attr" => out.push_str(dom.nodes[at].attr(first).unwrap_or_default()),
                        _ => {}
                    }
                    Ok(())
                });
            }
            _ => {}
        }
    }
    Some(out)
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
        let text = match frame.hyphenate {
            Some(lang) if frame.para.direction != BaseDirection::Rtl => {
                hyphenated(&text, lang).into()
            }
            _ => text,
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

    /// The index in [`Builder::spans`] of `style`'s text style inside the one at `parent`.
    fn span(&mut self, style: &Style, parent: usize, fonts: &Fonts) -> usize {
        let span = span_of(style, fonts, Some(&self.spans[parent]));
        if span == self.spans[parent] {
            parent
        } else {
            self.spans.push(span);
            self.spans.len() - 1
        }
    }

    /// The inline text of the `::before` or `::after` box of the element at `at`, whose own
    /// content box is `frame`.
    fn generated(
        &mut self,
        style: &Style,
        dom: &Dom,
        at: usize,
        counters: &Counters,
        frame: &Frame,
        fonts: &Fonts,
    ) {
        if keyword(style, "display") == Some("none") {
            return;
        }
        let Some(text) = content_text(style, dom, at, counters) else {
            return;
        };
        let inner = Frame {
            span: self.span(style, frame.span, fonts),
            spaces: spaces_of(style),
            ..frame.clone()
        };
        self.push_text(&text, &inner);
    }

    /// Adds bidi control characters in `frame`'s text style, leaving the collapsing of spaces as
    /// it was.
    fn push_control(&mut self, controls: &str, frame: &Frame) {
        let start = self.text.len();
        self.text.push_str(controls);
        if self.owner.is_none() {
            self.owner = Some((frame.para, frame.x, frame.width));
        }
        self.runs.push((start..self.text.len(), frame.span));
    }

    /// Ends the text block being collected, if it has anything to draw. A control character
    /// that closes the paragraph is trimmed with its trailing spaces, because the paragraph's end
    /// closes it too.
    fn flush(&mut self) {
        let trimmed = self
            .text
            .trim_end_matches(|c| c == ' ' || c == '\n' || is_bidi_control(c))
            .len();
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
                marker: false,
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
        let (image, name) = self.decode(reference)?;
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

    /// The background image of `style`: a gradient, or an image `url()` loaded from the asset map
    /// at its natural size.
    fn layer(&mut self, style: &Style, rem: f32) -> Result<Option<boxes::Layer>, Error> {
        let Some(reference) = boxes::image_reference(style) else {
            return Ok(boxes::gradient(style, rem));
        };
        let (image, name) = self.decode(&reference)?;
        let (w, h) = image.size();
        let (repeat_x, repeat_y) = boxes::repeat(style);
        self.images.push((image, name));
        Ok(Some(boxes::Layer::Image {
            image: self.images.len() - 1,
            width: w as f32 * 0.75,
            height: h as f32 * 0.75,
            repeat_x,
            repeat_y,
        }))
    }

    /// The image `reference` names, decoded far enough to know its size, and the name an error
    /// about it gives.
    fn decode(&self, reference: &str) -> Result<(Image, String), Error> {
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
        Ok((image, name))
    }
}

/// One step of the walk in [`blocks`].
enum Step {
    Enter(usize),
    /// Leaves the element whose frame is on top, writing the bidi controls that close it and
    /// closing its box when it is a block.
    Exit {
        at: usize,
        block: bool,
        close: Option<&'static str>,
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
        lang: None,
        hyphenate: None,
    };
    // The root element is the first in document order.
    let rem = styles
        .elements
        .iter()
        .flatten()
        .next()
        .map_or(empty.font_size(), Style::font_size);
    let mut counters = Counters::default();
    // An explicit stack, so a deep tree costs heap and never the call stack.
    let mut frames = vec![root];
    let mut work = vec![Step::Enter(0)];
    while let Some(step) = work.pop() {
        let at = match step {
            Step::Enter(at) => at,
            Step::Exit { at, block, close } => {
                let frame = frames.pop().expect("an exit follows its enter");
                let depth = frames.len();
                if let Some(after) = styles.after.get(&at) {
                    counters.apply(after, depth + 1);
                    builder.generated(after, dom, at, &counters, &frame, fonts);
                }
                if let Some(close) = close {
                    builder.push_control(close, &frame);
                }
                counters.leave(depth);
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
                let span = builder.span(style, frame.span, fonts);
                let mut inner = Frame {
                    span,
                    spaces: spaces_of(style),
                    transform: match keyword(style, "text-transform") {
                        Some("uppercase") => Some("uppercase"),
                        Some("lowercase") => Some("lowercase"),
                        Some("capitalize") => Some("capitalize"),
                        _ => None,
                    },
                    ..frame.clone()
                };
                if let Some(lang) = node.attr("lang") {
                    inner.lang = lang_of(lang);
                }
                inner.hyphenate = if keyword(style, "hyphens") == Some("auto") {
                    inner.lang
                } else {
                    None
                };
                let block = display != "inline";
                if block {
                    builder.flush();
                    let mut geometry = geometry(style, frame.x, frame.width);
                    geometry.decor.layer = builder.layer(style, rem)?;
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
                let depth = frames.len();
                counters.apply(style, depth);
                if name == "ol"
                    && let Some(start) = node
                        .attr("start")
                        .and_then(|s| s.trim().parse::<i64>().ok())
                {
                    counters.reset("list-item", start.saturating_sub(1), depth);
                }
                if display == "list-item" {
                    if !counter_list(style, "counter-increment", 1)
                        .iter()
                        .any(|(name, _)| name == "list-item")
                    {
                        counters.change("list-item", 1, false, depth);
                    }
                    if let Some(value) = node.attr("value").and_then(|v| v.trim().parse().ok()) {
                        counters.change("list-item", value, true, depth);
                    }
                    let kind = keyword(style, "list-style-type").unwrap_or("disc");
                    if let Some(text) = marker(kind, counters.get("list-item")) {
                        let text = format!("{text} ");
                        if keyword(style, "list-style-position") == Some("inside") {
                            let marked = Frame {
                                spaces: Spaces::Keep,
                                transform: None,
                                ..inner.clone()
                            };
                            builder.push_text(&text, &marked);
                            builder.after_space = true;
                        } else {
                            builder.blocks.push(Block::Text {
                                spans: vec![(0..text.len(), inner.span)],
                                text,
                                para: Para {
                                    direction: BaseDirection::Ltr,
                                    align: Alignment::Left,
                                    wrap: TextWrapMode::NoWrap,
                                    ..inner.para
                                },
                                x: inner.x,
                                width: inner.width,
                                marker: true,
                            });
                        }
                    }
                }
                let controls = bidi_controls(style, block);
                if let Some((open, _)) = controls {
                    builder.push_control(open, &inner);
                }
                if let Some(before) = styles.before.get(&at) {
                    counters.apply(before, depth + 1);
                    builder.generated(before, dom, at, &counters, &inner, fonts);
                }
                frames.push(inner);
                work.push(Step::Exit {
                    at,
                    block,
                    close: controls.map(|(_, close)| close),
                });
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
                let mut items =
                    boxes::paint(g, MARGIN + top, bottom - top, page == first, page == last);
                let opacity = g.decor.opacity < 1.0;
                if opacity {
                    items.insert(
                        0,
                        Item::Begin {
                            opacity: g.decor.opacity,
                            clip: None,
                        },
                    );
                }
                let under = if page == first { item } else { 0 };
                let page = &mut self.pages[page].items;
                page.splice(under..under, items);
                if opacity {
                    page.push(Item::End);
                }
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
    // The items of a list marker waiting for the line it is drawn beside.
    let mut marker: Option<Vec<Item>> = None;

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
                marker: is_marker,
            } => {
                // The hyphen of each span a soft hyphen is in, by the span's index.
                let mut hyphens: Vec<(usize, Hyphen)> = Vec::new();
                for (range, index) in &runs {
                    if text[range.clone()].contains(SOFT_HYPHEN)
                        && !hyphens.iter().any(|(at, _)| at == index)
                        && let Some(hyphen) = hyphen(&mut lcx, &mut fonts, &spans[*index], *index)
                    {
                        hyphens.push((*index, hyphen));
                    }
                }
                let mut builder = lcx.ranged_builder(&mut fonts.context, &text, 1.0, false);
                builder.set_base_direction(para.direction);
                builder.push_default(para.line_height);
                builder.push_default(StyleProperty::WordBreak(para.word_break));
                builder.push_default(StyleProperty::OverflowWrap(para.overflow_wrap));
                builder.push_default(StyleProperty::TextWrapMode(para.wrap));
                for (range, index) in &runs {
                    push_span(&mut builder, &spans[*index], *index, range.clone());
                }
                let mut shaped = builder.build(&text);
                let widest = hyphens.iter().map(|(_, h)| h.advance).fold(0.0, f32::max);
                break_lines(&mut shaped, &text, width, widest);
                shaped.align(para.align, AlignmentOptions::default());
                for line in shaped.lines() {
                    let metrics = line.metrics();
                    let height = metrics.block_max_coord - metrics.block_min_coord;
                    // A marker is drawn on baseline 0 and moved down to the next line's.
                    let top = if is_marker { 0.0 } else { flow.place(height) };
                    let page_number = flow.pages.len() as u64;
                    let baseline = if is_marker {
                        0.0
                    } else {
                        MARGIN + top + (metrics.baseline - metrics.block_min_coord)
                    };
                    let line_range = line.text_range();
                    let line_text = &text[line_range.clone()];
                    let mut x = if is_marker {
                        left - metrics.advance
                    } else {
                        left + metrics.inline_min_coord + metrics.offset
                    };
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
                                    items.push(boxes::fill(
                                        boxes::rect(
                                            start,
                                            baseline - offset - size / 2.0,
                                            advance,
                                            size.max(0.5),
                                        ),
                                        color,
                                    ));
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
                            if cluster_text
                                .chars()
                                .all(|c| c.is_control() || is_bidi_control(c) || c == SOFT_HYPHEN)
                            {
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
                    let last = line_range.end.saturating_sub(SOFT_HYPHEN.len_utf8());
                    if line_text.ends_with(SOFT_HYPHEN)
                        && let Some((_, index)) = runs.iter().find(|(r, _)| r.contains(&last))
                        && let Some((_, h)) = hyphens.iter().find(|(at, _)| at == index)
                    {
                        items.push(Item::Text {
                            face: h.face,
                            size: h.size,
                            x,
                            y: baseline,
                            color: spans[*index].color,
                            glyphs: vec![Glyph {
                                id: h.id,
                                text: 0..1,
                                advance: h.advance,
                                x: 0.0,
                                y: 0.0,
                            }],
                            text: "-".to_owned(),
                        });
                    }
                    if is_marker {
                        marker = Some(items);
                        break;
                    }
                    if let Some(drawn) = marker.take() {
                        items.extend(drawn.into_iter().map(|item| lower(item, baseline)));
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

const SOFT_HYPHEN: char = '\u{ad}';

/// The language of a `lang` attribute, by its primary subtag, if `hypher` has its patterns.
fn lang_of(tag: &str) -> Option<Lang> {
    let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();
    let code: [u8; 2] = primary.as_bytes().try_into().ok()?;
    Lang::from_iso(code)
}

/// `text` with a soft hyphen at each point `hypher` breaks its words at in `lang`.
fn hyphenated(text: &str, lang: Lang) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(char::is_alphabetic) {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest
            .find(|c: char| !c.is_alphabetic())
            .unwrap_or(rest.len());
        out.push_str(&hypher::hyphenate(&rest[..end], lang).join("\u{ad}"));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Pushes `span`'s text style onto `range` of the text, with `index` as its brush.
fn push_span(
    builder: &mut RangedBuilder<'_, usize>,
    span: &Span,
    index: usize,
    range: Range<usize>,
) {
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
            StyleProperty::FontFeatures(FontFeatures::Source(Cow::Borrowed(features.as_str()))),
            range.clone(),
        );
    }
    builder.push(StyleProperty::Brush(index), range);
}

/// The hyphen drawn at the end of a line broken at a soft hyphen: its face, size, glyph and
/// advance.
struct Hyphen {
    face: usize,
    size: f32,
    id: u32,
    advance: f32,
}

/// The hyphen of `span`, shaped through its font chain.
fn hyphen(
    lcx: &mut LayoutContext<usize>,
    fonts: &mut Fonts,
    span: &Span,
    index: usize,
) -> Option<Hyphen> {
    let mut builder = lcx.ranged_builder(&mut fonts.context, "-", 1.0, false);
    push_span(&mut builder, span, index, 0..1);
    let mut shaped = builder.build("-");
    shaped.break_all_lines(None);
    let line = shaped.lines().next()?;
    let run = line.runs().next()?;
    let face = fonts.face_index(run.font().data.data())?;
    let glyph = run.visual_clusters().next()?.glyphs().next()?;
    Some(Hyphen {
        face,
        size: run.font_size(),
        id: glyph.id,
        advance: glyph.advance,
    })
}

/// Breaks `shaped` into lines `width` wide. A line that ends at a soft hyphen is broken again
/// `hyphen` narrower, and is aligned in that width, so the hyphen drawn after it stays in the
/// box. The lines before the one narrowed do not change, so each pass narrows a later line.
fn break_lines(shaped: &mut Layout<usize>, text: &str, width: f32, hyphen: f32) {
    let mut narrow: Vec<usize> = Vec::new();
    loop {
        let mut breaker = shaped.break_lines();
        breaker.state_mut().set_layout_max_advance(width);
        let mut line = 0;
        loop {
            let max = if narrow.contains(&line) {
                width - hyphen
            } else {
                width
            };
            breaker.state_mut().set_line_max_advance(max);
            match breaker.break_next() {
                None => break,
                Some(YieldData::LineBreak(_)) => line += 1,
                Some(_) => {}
            }
        }
        breaker.finish();
        let next = shaped
            .lines()
            .enumerate()
            .find(|(i, l)| !narrow.contains(i) && text[l.text_range()].ends_with(SOFT_HYPHEN));
        match next {
            Some((i, _)) if hyphen > 0.0 => narrow.push(i),
            _ => return,
        }
    }
}

/// `item` moved down by `by`.
fn lower(mut item: Item, by: f32) -> Item {
    match &mut item {
        Item::Text { y, .. } | Item::Image { y, .. } => *y += by,
        Item::Path { path, paint, .. } => {
            boxes::lower(path, by);
            match paint {
                Paint::Color(_) => {}
                Paint::Linear { y1, y2, .. } => {
                    *y1 += by;
                    *y2 += by;
                }
                Paint::Radial { cy: y, .. } | Paint::Tiles { y, .. } => *y += by,
            }
        }
        Item::Begin { clip, .. } => {
            if let Some(clip) = clip {
                boxes::lower(clip, by);
            }
        }
        Item::End => {}
    }
    item
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
