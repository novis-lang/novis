//! The layout: the parsed document and its computed styles as pages of positioned glyph runs,
//! rectangles and images (`rule:core-classes/pdf-css-subset`).
//!
//! A page's size and margins are its `@page` rules' (`paginate`'s module doc), and every length
//! here is in PDF points (1/72 inch, three quarters of a CSS pixel). An element's `display` comes from the cascade, whose
//! user-agent sheet gives HTML's defaults: `none` draws nothing, `block` and `list-item` start a
//! block box, `flex` and `inline-flex` start a flex container (`flex`'s module doc), `grid` and
//! `inline-grid` a grid container (`grid`'s module doc), `table` and `inline-table` a table
//! (`table`'s module doc), `inline-block` is an atomic inline box, and anything else is inline.
//! A table part outside a table is a block box.
//! A child of a flex or grid container is an item, whatever its `display`. A `::before` or `::after` box whose `display` is `inline-block` is a
//! block box.
//!
//! **Blocks** are laid top to bottom. A block box takes its margins, borders, padding, `width`,
//! `min-width`, `max-width`, `height`, `min-height` and `box-sizing` from its style, with a
//! percentage of the containing block's width, and `margin: auto` on both sides centres a box
//! that has a width. Vertical margins collapse to the larger of two that meet, through a box
//! with no border or padding on that side, and are dropped at the top of a page after the first.
//! A box's background, borders and outline are drawn under its content as paths (`boxes`'
//! module doc), and a box under `opacity` is drawn with its content in one group. A box that
//! goes on past a page is drawn in a slice on each page, closed at its first and last edge.
//! A block box's `break-before` of `page`, `left` or `right` (`always`, `verso` and `recto` the
//! same) starts it on a new page, or on the next left or right one with an empty `:blank` page
//! before it when it must, and does nothing on a page nothing has been placed on yet. Its
//! `break-after` does the same for what is placed after it, so the last box's adds no page. A
//! blank page draws no slice of a box around it, and an image or an inline-block reads no break.
//!
//! **Inline boxes.** An inline element with a horizontal margin, a border, padding or a
//! background is an [`Inline`] of its text block. Its left margin, border and padding are an empty
//! `parley` inline box at the start of its text and its right ones another at the end, so the
//! line makes room for them. On each line its text reaches, it is painted under the line's
//! glyphs, from its text's left to right end and from the highest ascent to the lowest descent
//! of the runs in it, grown by its padding and border. Its left side is on the line its text
//! starts on and its right side on the line it ends on, as in a left-to-right line, whatever the
//! direction. Its vertical margins, and its `opacity`, change nothing. A block inside it splits
//! it, and the part after the block draws its left side again.
//!
//! **Inline-blocks.** An `inline-block` box is an [`Atom`]: its content is laid out as a block of
//! its own, then placed in its line as an in-flow `parley` inline box as wide and tall as its
//! margin box, its baseline on the line's at its last line's baseline, or at its bottom margin
//! edge when it has no line. A line grows to hold it, and it is never split across lines or
//! pages. Without a `width`, it is as wide as its widest line laid out unbroken, up to its
//! containing block: the box and every block in it whose width fills its own containing block
//! narrow by the room left over. Its vertical margins do not collapse with its content's. An
//! inner one is laid out before the one around it, each once, so nesting costs no recursion.
//! The inline box around it draws around its text and not around the inline-block.
//!
//! **Floats.** An element whose `float` is not `none`, and that is not a flex or grid item, is
//! laid out as an inline-block is, whatever its `display`, and narrows to its content the same
//! way. It is placed at the left or right of its containing block's content box, at the top of
//! the paragraph it is in or of the next box after it (`block`'s module doc). Each line beside
//! it is shortened by the room it takes, and is aligned in what is left. A box's background and
//! borders are not shortened, as in CSS. `clear` on a block box moves it below the floats it
//! names. A float in an `<img>` is not read yet, so the picture stays a block.
//!
//! **Positioning.** A block box under `position: relative` is laid out in its place and then
//! drawn moved by its `left` or else `right`, and its `top` or else `bottom`. A percentage of
//! `top` or `bottom` is 0, and an inline element's `position` does nothing. An element under
//! `position: absolute`, whatever its `display` and also in a flex or grid container, is laid
//! out as an inline-block is and takes no room. Its margin box is placed against the padding box
//! of the nearest positioned block box around it, or else the page's content box, by `left`
//! or else `right`, and `top` or else `bottom`, on the page that box starts or ends on. A side
//! with neither keeps the place the box would have in the flow. Without a `width`, it is as wide
//! as its content, up to its parent's content box, and `left` and `right` together do not
//! stretch it. Inside an inline-block, a float or a flex item, the item's own box is its
//! containing block when no box between them is positioned. An element under `position: fixed`
//! is laid out as an absolute one is, and its containing block is the page's content box whatever
//! is around it: a copy of it is drawn on every page, which is how a watermark is written. Inside
//! an inline-block, a float or a flex item it is placed once, against that item's box. Positioned
//! boxes are drawn over the others by `z-index` (`block`'s module doc). An element under
//! `position: running()` is drawn only in a margin box (`margin`'s module doc).
//!
//! **Transforms.** A block box under `transform` is laid out in its place and drawn with its
//! content in one group under the transform's matrix (`transform`'s module doc), so it takes the
//! room it would take untransformed. A box that goes on past a page turns each slice about the
//! slice's own origin. A `transform` on an inline element does nothing, and a transformed box is
//! not the containing block of an absolute or `fixed` box inside it.
//!
//! **Text** is the inline content of a block, shaped and broken into lines by `parley` over
//! `harfrust` and `skrifa`. Each run takes its font chain (`fonts`' module doc), `font-size`,
//! `font-weight`, `font-style`, `color`, `letter-spacing`, `word-spacing`,
//! `font-feature-settings`, `text-transform`, `vertical-align` and the underline or line-through
//! it is inside; each block takes `text-align`, `line-height`, `white-space`, `word-break`,
//! `overflow-wrap` and `text-indent`. The indent moves the first line laid out in a block, and
//! text after a child block in the same block is not indented again. A `vertical-align` shift is
//! measured from the paragraph's baseline, the shifts of nested spans adding up. `middle`,
//! `text-top` and `text-bottom` are measured against the block's own font, which is the root
//! style of `parley`'s flat builder, and not against an enclosing inline box. They align the
//! span's line box, its half-leading included. A raised or lowered run makes its line taller
//! where it needs to. Under
//! `text-overflow: ellipsis` and an `overflow-x` that is not `visible`, a line wider than its box
//! draws the clusters that fit with a `…` in the first run's style after them, at the right end
//! of a left-to-right paragraph and at the left end of a right-to-left one.
//! A line that does not fit under the last on the page starts the next page. A
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
//! content box level with the item's first line, on the right in a right-to-left item and
//! in the item's direction, or as its first text under
//! `list-style-position: inside`. A `::before` or `::after` box is inline text at the start or
//! end of its element, or a block box of its own when its `display` is not `inline`, from the
//! strings, `counter()`, `counters()`, `attr()` and quotes of its `content`, after its own
//! counter properties apply.
//!
//! **A table of contents.** A `target-counter()` in `content` prints the page, counting from 1,
//! of the first element whose `id` its `url()`, string or `attr()` names, and nothing for an
//! `id` no element has or a counter other than `page`. An element lands on the page its first
//! box placed after it starts on. An `id` inside an `inline-block`, float, flex item or table
//! cell lands on the page that box draws it on, and one inside a running element on the
//! running element's page. A `leader()` is an empty `parley` inline box that takes
//! the room left on its line, shared with the other leaders there; the text is shaped and broken
//! again with it, and its one character (`.` for `dotted`, `_` for `solid`, a space for
//! `space`, or the first of its string) is drawn across it, lined up from the content box's
//! left edge. A document that reads a page is laid out again with the pages the last pass gave,
//! until no page changes, and `text::layout` throws after five passes.
//!
//! **Links and bookmarks.** An `<a href>` to `http:`, `https:` or `mailto:`, or to `#` and an
//! `id`, gives its text style an [`Href`], and each glyph run drawn in that style is followed by
//! an [`Item::Link`] over the run's advance, from its ascent to its descent. Any other `href`
//! is no link. Each element with an `id` records the page and the top of the first box placed
//! after it in [`Laid::anchors`], and each `h1` to `h6` a [`Bookmark`] the same way, so a link
//! to an `id` no element has goes nowhere and the writer leaves it out.
//!
//! **Images**: an `<img>` whose bytes are PNG, JPEG, GIF, WebP or an SVG document, and an inline
//! `<svg>`, is a block at its CSS `width` and `height`, else its `width` and `height` attributes,
//! else its natural size, a pixel being a CSS pixel, scaled down to fit the content box. The
//! picture fills that box, or under `object-fit` is fitted inside it, covers it or keeps its own
//! size, at the place `object-position` gives, and the box clips what is outside it. An SVG
//! is a [`Picture::Vector`]: `usvg` parses it, and the writer draws it as PDF paths through
//! `krilla-svg`. An inline `<svg>` is its subtree written out again as an SVG document
//! ([`image::svg_markup`]), with every element outside the SVG namespace left out. An `<image>`
//! in an SVG loads through the asset map, as an `<img>` does. Its `<text>` is laid out by `usvg`
//! over the twelve default faces ([`crate::fonts::svg_database`]) and drawn as glyphs. Nothing
//! else outside the HTML namespace is drawn.
//!
//! The submodules: `inline` walks the document into [`Block`]s, `image` loads the pictures they
//! draw, `block` is the vertical flow of boxes, `text` shapes and places each block's lines,
//! `flex` lays out a flex or grid container's items, `grid` reads a grid's tracks and
//! placements, `table` lays out a table's rows across pages, `boxes` paints a box, and `margin` lays out each page's margin boxes.
//!
//! Memory: one shaped `parley` layout at a time, and the positioned items of every page until
//! the writer has drawn them.

use std::collections::BTreeMap;
use std::ops::Range;

use hypher::Lang;
use parley::{
    Alignment, BaseDirection, LineHeight, OverflowWrap, TextWrapMode, VerticalAlign, WordBreak,
};

mod block;
mod boxes;
mod flex;
mod grid;
mod image;
mod inline;
mod margin;
mod table;
mod text;
mod transform;

pub use boxes::{Paint, Seg, Stop, Stroke, bounds};
pub use image::Picture;
pub use margin::overlay;
pub use text::{layout, layout_keeping};
pub use transform::Matrix;

use block::{BoxGeometry, SIDES};

use crate::css::{Color, Style, Value};
use crate::fonts::FaceData;
use crate::paginate::PageBox;

/// A4's width and height in points, the page size when no `@page` rule gives one.
pub const PAGE_WIDTH: f32 = 595.275_6;
pub const PAGE_HEIGHT: f32 = 841.889_8;
/// 2 cm in points: a page's margin when no `@page` rule gives one, and the left and top edge of
/// the content box every page is laid out in before it is moved to its own.
pub const MARGIN: f32 = 56.692_9;

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
    /// `y`. `face` indexes [`Laid::faces`], and `descent` is how far the face reaches below the
    /// baseline at `size`.
    Text {
        face: usize,
        size: f32,
        descent: f32,
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
    /// under `transform` when it has one, and only inside `clip` when it has one.
    Begin {
        opacity: f32,
        clip: Option<Vec<Seg>>,
        transform: Option<Matrix>,
    },
    End,
    /// The box `x`, `y`, `width`, `height` follows `href` when it is clicked. It draws nothing.
    Link {
        href: Href,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Starts the content of an element in this role, which ends at the matching
    /// [`Item::Untag`]. It draws nothing: the writer tags the content between them with it.
    Tag(Role),
    Untag,
    /// Where a box laid out on a page of its own met an `id`, a heading, a named string, a
    /// mark or a running element: the index of what it met, and its top. It draws nothing, and
    /// the layout takes it off the page once it knows the page it is on.
    Here { at: usize, y: f32 },
}

/// What an element is in the document's structure: the tag the writer gives its content.
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// An `h1` to `h6`, by its level.
    Heading(u8),
    Paragraph,
    /// A `ul`, or an `ol` when `ordered`.
    List { ordered: bool },
    ListItem,
    Quote,
    /// A `figure`, or an `img` with its `alt`.
    Figure(Option<String>),
    Caption,
    Table,
    Row,
    HeaderCell,
    Cell,
    /// Content that is not part of the document's text: a repeated table row.
    Artifact,
}

impl Role {
    /// The role of a block-level HTML element `name`, or none when it groups nothing.
    pub fn of(name: &str) -> Option<Role> {
        Some(match name {
            "p" | "pre" | "address" | "dt" | "dd" => Role::Paragraph,
            "ul" | "menu" | "dl" => Role::List { ordered: false },
            "ol" => Role::List { ordered: true },
            "li" => Role::ListItem,
            "blockquote" => Role::Quote,
            "figure" => Role::Figure(None),
            "caption" | "figcaption" => Role::Caption,
            "table" => Role::Table,
            "th" => Role::HeaderCell,
            "td" => Role::Cell,
            _ => match name.as_bytes() {
                [b'h', level @ b'1'..=b'6'] => Role::Heading(level - b'0'),
                _ => return None,
            },
        })
    }
}

/// Where a link goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Href {
    /// An `http:`, `https:` or `mailto:` address.
    Uri(String),
    /// The element with this `id`.
    Anchor(String),
}

impl Href {
    /// The link an `href` attribute gives, or none when its scheme is not one a link may have.
    pub fn of(href: &str) -> Option<Href> {
        let href = href.trim();
        if let Some(id) = href.strip_prefix('#') {
            return (!id.is_empty()).then(|| Href::Anchor(id.to_owned()));
        }
        let (scheme, _) = href.split_once(':')?;
        ["http", "https", "mailto"]
            .iter()
            .any(|s| s.eq_ignore_ascii_case(scheme))
            .then(|| Href::Uri(href.to_owned()))
    }
}

/// An outline entry: a heading's level from 1, its text, its page index and its top.
#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub level: u8,
    pub title: String,
    pub page: usize,
    pub y: f32,
}

/// What a [`Block::Target`] records where it lands.
#[derive(Debug, Clone)]
enum Anchor {
    /// The element with this `id`.
    Id(String),
    /// A heading of this level and text.
    Heading(u8, String),
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub items: Vec<Item>,
    /// Whether a forced break to a left or right page left it empty.
    pub blank: bool,
    /// Its size and margins, from the `@page` rules that match it.
    pub geometry: PageBox,
    /// Its name, from the `page` of the first block box on it.
    pub name: Option<String>,
    /// Each named string's value on it by `string(name)`, the names in the order they are
    /// first set in the document.
    pub strings: Vec<(String, String)>,
    /// Each `data-mark` name and value set on it, in document order.
    pub marks: Vec<(String, String)>,
    /// Each margin box drawn on it: its name and its items' range in [`Page::items`].
    pub boxes: Vec<(&'static str, Range<usize>)>,
    /// Each `<slot name>` of a table row drawn on it: its name, and its cell's content box as
    /// its left edge, top, width and height on the page.
    pub slots: Vec<(String, [f32; 4])>,
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
    /// The page index and the top of each `id`, the first element of each name.
    pub anchors: BTreeMap<String, (usize, f32)>,
    /// Each heading in document order.
    pub bookmarks: Vec<Bookmark>,
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
    /// Its `vertical-align`, with the parent's shift added to its own because each span's
    /// shift is given against the paragraph's baseline.
    align: VerticalAlign,
    /// The link of the `<a href>` the text is in.
    link: Option<Href>,
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
    /// The first line's `text-indent` in points.
    indent: f32,
    /// Whether a line wider than the box ends in `…`: `text-overflow: ellipsis` under an
    /// `overflow-x` that is not `visible`.
    ellipsis: bool,
    /// The fewest lines left at the bottom of a page and carried to the top of the next one:
    /// its `orphans` and `widows`.
    orphans: usize,
    widows: usize,
    /// The index in [`inline::Builder::spans`] of the block's own text style.
    span: usize,
}

/// How a text node's whitespace is kept.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Spaces {
    Collapse,
    KeepBreaks,
    Keep,
}

/// An inline element's box, drawn around its text on each line the text reaches.
#[derive(Debug, Clone)]
struct Inline {
    /// The element's text, as bytes of its text block's.
    range: Range<usize>,
    /// Its margin, border, padding and paint. Each line sets `x` and `width` to its own slice.
    geometry: BoxGeometry,
}

#[derive(Clone)]
enum Block {
    Open(BoxGeometry),
    Close,
    /// A flex container's content.
    Flex(flex::Flex),
    /// A table's content.
    Table(table::Table),
    /// A float: its index in the list of [`Atom`]s, laid out as an `inline-block` box is, the
    /// left edge and width of its containing block's content box, whether it floats right, and
    /// the sides its own `clear` names.
    Float {
        atom: usize,
        x: f32,
        width: f32,
        right: bool,
        clear: [bool; 2],
    },
    /// An absolute box: its index in the list of [`Atom`]s, laid out as an `inline-block` box
    /// is, the left edge of the content box it would be in, its insets, its `z-index`, and
    /// whether it is `fixed` and drawn on every page.
    Absolute {
        atom: usize,
        x: f32,
        insets: block::Insets,
        z: Option<i64>,
        fixed: bool,
    },
    /// A running element: its index in the list of [`Atom`]s, laid out as an `inline-block`
    /// box is, and the name its `position: running()` gives. It takes no room in the flow.
    Running {
        atom: usize,
        name: String,
    },
    /// A named string an element's `string-set` sets, or a `data-mark` when `mark` is set, on
    /// the page the next box placed starts on. It takes no room in the flow.
    String {
        name: String,
        value: String,
        mark: bool,
    },
    Text {
        text: String,
        spans: Vec<(Range<usize>, usize)>,
        para: Para,
        x: f32,
        width: f32,
        /// A list item's marker, drawn on one line that ends at `x` and level with the first
        /// line laid out after it.
        marker: bool,
        /// The boxes of the inline elements in the text that draw one, outermost first.
        inline: Vec<Inline>,
        /// Each `inline-block` box in the text: its byte offset there and its index in the list
        /// of [`Atom`]s.
        atoms: Vec<(usize, usize)>,
        /// Each `leader()` in the text: its byte offset there, the index of its text style and
        /// the character it repeats. It fills the room left on its line.
        leaders: Vec<(usize, usize, String)>,
    },
    /// The element with an `id`, or the heading, starts on the page and at the top of the next
    /// box placed. It takes no room in the flow.
    Target(Anchor),
    /// An [`Item::Tag`] or, with no role, an [`Item::Untag`] on the page the flow is on. It
    /// takes no room in the flow.
    Tag(Option<Role>),
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

/// An `inline-block` box: laid out as a block of its own, then placed in its line as one piece.
struct Atom {
    /// Its blocks, from its own box's opening to its close, against a containing block at `x` 0.
    blocks: Vec<Block>,
    /// Its `vertical-align` in its line.
    align: VerticalAlign,
    /// Whether it is a flex item, which its container sizes and places, and not an
    /// `inline-block` box.
    item: bool,
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
    /// The name of the pages its block boxes are on.
    page: Option<String>,
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
