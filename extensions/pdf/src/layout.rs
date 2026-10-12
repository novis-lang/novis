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
//! **Images**: an `<img>` whose bytes are PNG, JPEG, GIF, WebP or an SVG document, and an inline
//! `<svg>`, is a block at its CSS `width` and `height`, else its `width` and `height` attributes,
//! else its natural size, a pixel being a CSS pixel, scaled down to fit the content box. The
//! picture fills that box, or under `object-fit` is fitted inside it, covers it or keeps its own
//! size, at the place `object-position` gives, and the box clips what is outside it. An SVG
//! is a [`Picture::Vector`]: `usvg` parses it, and the writer draws it as PDF paths through
//! `krilla-svg`. An inline `<svg>` is its subtree written out again as an SVG document
//! ([`image::svg_markup`]), with every element outside the SVG namespace left out. An `<image>`
//! in an SVG loads through the asset map, as an `<img>` does. Its `<text>` draws nothing, because
//! its font database is empty. Nothing else outside the HTML namespace is drawn.
//!
//! The submodules: `inline` walks the document into [`Block`]s, `image` loads the pictures they
//! draw, `block` is the vertical flow of boxes, `text` shapes and places each block's lines, and
//! `boxes` paints a box.
//!
//! Memory: one shaped `parley` layout at a time, and the positioned items of every page until
//! the writer has drawn them.

use std::ops::Range;

use hypher::Lang;
use parley::{Alignment, BaseDirection, LineHeight, OverflowWrap, TextWrapMode, WordBreak};

mod block;
mod boxes;
mod image;
mod inline;
mod text;

pub use boxes::{Paint, Seg, Stop, Stroke, bounds};
pub use image::Picture;
pub use text::layout;

use block::{BoxGeometry, SIDES};

use crate::css::{Color, Style, Value};
use crate::fonts::FaceData;

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

/// Each picture a document draws, with the reference it was loaded from.
pub type Images = Vec<(Picture, String)>;

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
        /// Where the picture is drawn in the box by its `object-fit` and `object-position`: its
        /// left and top from the box's, and its width and height. The box clips it.
        draw: [f32; 4],
    },
}

/// What every element on the stack passes to its content.
#[derive(Debug, Clone)]
struct Frame {
    /// The content box's left edge and width.
    x: f32,
    width: f32,
    /// The index in [`inline::Builder::spans`] of this element's text style.
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
