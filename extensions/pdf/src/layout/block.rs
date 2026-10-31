//! Block boxes: a box's geometry from its style, and the vertical flow that places boxes and
//! lines down the pages and paints each box under its content when it closes.
//!
//! The flow holds the floats of its last page, and a new page starts with none: a float does
//! not go on past its page. A float's top is the top the next box would have, or the top of the
//! float before it if that is lower, and it goes down past the floats beside it until it fits
//! between them. A line is as wide as the room the floats leave it ([`Flow::band`]), and a box
//! with `clear` starts below the floats it clears. An `inline-block` box or a flex item holds
//! its floats: its own flow ends below them ([`Flow::contain`]).
//!
//! A box cut by a page break is painted on each page it is on. CSS's `slice` leaves its bottom
//! edge open before the break and its top edge open after it. A table and a table cell are
//! closed instead (`rule:core-classes/pdf-tables-across-pages`), unless their
//! `box-decoration-break` is `slice`: each page that cuts one ends it under the last block and
//! starts it again at the top, with its top and bottom border and padding on both sides of the
//! break. That room is taken from the page, so [`Flow::room`] and [`Flow::next_limit`] count it.
//!
//! A positioned box leaves its pages when it closes and becomes a layer on each page it is on
//! ([`Flow::lift`]), and [`Flow::finish`] draws the layers back by `z-index`, then in document
//! order: below 0 under the page's other items, and the rest over them. A box with a
//! `z-index` draws the layers inside it as part of its own, so it stacks them only among
//! themselves. An absolute box waits in the innermost positioned box open, its containing
//! block, until that box closes and its padding box is known, or in the flow when there is none,
//! where the page's content box is its containing block. A `fixed` box always waits in the
//! flow, and [`Flow::finish`] places a copy of it on every page against that page's content box.

use std::collections::BTreeMap;

use super::boxes::{self, Decor};
use super::inline::Targets;
use super::text::shift;
use super::{Anchor, Bookmark, Item, MARGIN, Page, keyword, length};
use crate::css::{Style, Value};

/// A block box's geometry: its border box's `x` and `width`, and the rest top, right, bottom,
/// left.
#[derive(Debug, Clone)]
pub(super) struct BoxGeometry {
    pub(super) x: f32,
    pub(super) width: f32,
    pub(super) margin: [f32; 4],
    pub(super) border: [f32; 4],
    pub(super) padding: [f32; 4],
    /// The content box's least height, from `height` and `min-height`.
    pub(super) min_content: f32,
    /// Whether its width is its containing block's less its margins, so it narrows when that
    /// block does.
    pub(super) fill: bool,
    /// Whether its `clear` moves it below the left floats and below the right ones.
    pub(super) clear: [bool; 2],
    pub(super) position: Position,
    /// Its forced `break-before` and `break-after`.
    pub(super) breaks: [Option<Break>; 2],
    /// Whether its `break-inside` is `avoid`, so it starts on a new page when it fits on one
    /// and not under the last block.
    pub(super) avoid: bool,
    /// Its `z-index` when it is positioned and the value is not `auto`.
    pub(super) z: Option<i64>,
    pub(super) decor: Decor,
    /// The name of the pages it is on, its own `page` or else its parent's.
    pub(super) page: Option<String>,
    /// Whether its top and bottom border and padding are drawn on both sides of every page
    /// break and take room there: a table's and a table cell's, unless its
    /// `box-decoration-break` is `slice`.
    pub(super) closed: bool,
}

/// A block box's `position`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Position {
    Static,
    /// Moved right and down by these offsets after it is laid out. The flow keeps its place.
    Relative([f32; 2]),
    /// Placed by its insets against its containing block, which for `fixed` is every page's
    /// content box. The box is an [`super::Atom`].
    Absolute,
}

/// A forced page break: to the next page, or to the next left or right one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Break {
    Page,
    Left,
    Right,
}

/// The forced break the property `name` gives, if it gives one.
fn forced(style: &Style, name: &str) -> Option<Break> {
    match keyword(style, name) {
        Some("page" | "always" | "all") => Some(Break::Page),
        Some("left" | "verso") => Some(Break::Left),
        Some("right" | "recto") => Some(Break::Right),
        _ => None,
    }
}

/// An absolute box's `top`, `right`, `bottom` and `left`, each as points and a percentage of
/// its containing block, or `None` for `auto`.
pub(super) type Insets = [Option<(f32, f32)>; 4];

pub(super) fn insets(style: &Style) -> Insets {
    SIDES.map(|s| match style.get(s) {
        Some(Value::Length(l)) => Some((l.pt, l.percent)),
        _ => None,
    })
}

pub(super) fn z_index(style: &Style) -> Option<i64> {
    match style.get("z-index") {
        Some(Value::Integer(z)) => Some(*z),
        _ => None,
    }
}

/// Which side a `float` or `clear` value names, `inline-start` and `inline-end` by the box's
/// `direction`: `[left, right]`.
pub(super) fn sides(style: &Style, name: &str) -> [bool; 2] {
    let rtl = keyword(style, "direction") == Some("rtl");
    match keyword(style, name) {
        Some("left") => [true, false],
        Some("right") => [false, true],
        Some("both") => [true, true],
        Some("inline-start") => [!rtl, rtl],
        Some("inline-end") => [rtl, !rtl],
        _ => [false, false],
    }
}

pub(super) const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

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

pub(super) fn geometry(style: &Style, x: f32, containing: f32) -> BoxGeometry {
    let side = |prefix: &str, suffix: &str| {
        SIDES.map(|s| length(style, &format!("{prefix}-{s}{suffix}"), containing).unwrap_or(0.0))
    };
    let mut margin = side("margin", "");
    let padding = side("padding", "");
    let border = SIDES.map(|s| border_width(style, s));
    let insets = border[1] + border[3] + padding[1] + padding[3];
    let border_box = keyword(style, "box-sizing") == Some("border-box");
    let to_border = |w: f32| if border_box { w } else { w + insets };
    let filled = containing - margin[1] - margin[3];
    let mut width = match length(style, "width", containing) {
        Some(w) => to_border(w),
        None => filled,
    };
    if let Some(max) = length(style, "max-width", containing) {
        width = width.min(to_border(max));
    }
    if let Some(min) = length(style, "min-width", containing) {
        width = width.max(to_border(min));
    }
    let fill = style.get("width").is_none() && width == filled;
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
    // A relative offset's percentage counts against a height the containing block does not
    // have, so only `left` and `right` read one.
    let offset = |start: &str, end: &str, basis: f32| {
        length(style, start, basis)
            .or_else(|| length(style, end, basis).map(|v| -v))
            .unwrap_or(0.0)
    };
    let position = match keyword(style, "position") {
        Some("relative") => Position::Relative([
            offset("left", "right", containing),
            offset("top", "bottom", 0.0),
        ]),
        Some("absolute" | "fixed") => Position::Absolute,
        _ => Position::Static,
    };
    BoxGeometry {
        x: x + margin[3],
        width,
        margin,
        border,
        padding,
        min_content,
        fill,
        clear: sides(style, "clear"),
        position,
        breaks: [forced(style, "break-before"), forced(style, "break-after")],
        avoid: matches!(keyword(style, "break-inside"), Some("avoid" | "avoid-page")),
        z: (position != Position::Static)
            .then(|| z_index(style))
            .flatten(),
        decor: boxes::decor(style),
        page: None,
        closed: matches!(
            keyword(style, "display"),
            Some("table" | "inline-table" | "table-cell")
        ) && keyword(style, "box-decoration-break") != Some("slice"),
    }
}

/// A block box whose bottom has not been reached.
pub(super) struct Open {
    geometry: BoxGeometry,
    /// Where its border box starts: the page, the index its items are drawn under, and the top.
    /// `None` until something inside it, or its own top border or padding, is placed.
    top: Option<(usize, usize, f32)>,
    /// What it carries until it closes, when it is positioned.
    stack: Option<Stack>,
    /// For a closed box, at each page break after its top: its bottom on the page before and
    /// its top on the page after, each from the top of its page's content.
    cuts: Vec<(f32, f32)>,
}

/// A positioned box's part in the stacking order: its `z-index`, its place in the document,
/// the number of layers before it opened, and the absolute boxes it is the containing block of.
struct Stack {
    z: Option<i64>,
    order: usize,
    layers: usize,
    absolute: Vec<Absolute>,
}

/// An absolute box waiting for its containing block to close: its items with its margin box's
/// top-left corner at 0, 0, its margin box's size, its insets, the page, left edge and top it
/// would have in the flow, its `z-index` and its place in the document.
#[derive(Clone)]
struct Absolute {
    items: Vec<Item>,
    size: (f32, f32),
    insets: Insets,
    page: usize,
    x: f32,
    y: f32,
    z: i64,
    order: usize,
}

/// The items of a positioned box on one page, drawn by `z`, then by `order`.
struct Layer {
    page: usize,
    z: i64,
    order: usize,
    items: Vec<Item>,
}

/// A containing block's padding box: its left edge and width, and its page and top, and its
/// page and bottom, each from the top of its page's content.
struct Area {
    left: f32,
    width: f32,
    top: (usize, f32),
    bottom: (usize, f32),
}

/// The content height of the page at an index, named by a page name or by none.
pub(super) type Heights = Box<dyn Fn(usize, Option<&str>) -> f32>;

/// The vertical flow: the pages so far and where the next block goes.
pub(super) struct Flow {
    pub(super) pages: Vec<Page>,
    pub(super) cursor: f32,
    pub(super) pending: f32,
    pub(super) open: Vec<Open>,
    /// How tall each page's content is, or unbounded for an inline-block's one page.
    limits: Vec<f32>,
    /// The content height of the page at an index with a page name, for a flow of the
    /// document's pages. A flow without it has one page, and reads no forced break.
    heights: Option<Heights>,
    /// A `break-after` that has not been taken, because nothing has been placed after it yet.
    forced: Option<Break>,
    /// Where the last page's content starts: under the top edges the closed boxes open across
    /// its break draw again.
    start: f32,
    /// The baseline of the last line placed, from the top of its page's content.
    pub(super) baseline: Option<f32>,
    /// The baseline of the first line placed, from the top of its page's content.
    pub(super) first: Option<f32>,
    /// The floats on the last page, in the order they were placed.
    floats: Vec<Floated>,
    /// The positioned boxes placed so far, which [`Flow::finish`] draws in order.
    layers: Vec<Layer>,
    /// The absolute boxes with no positioned box around them.
    loose: Vec<Absolute>,
    /// The `fixed` boxes, which [`Flow::finish`] places on every page.
    fixed: Vec<Absolute>,
    /// The number of positioned boxes met so far, which orders those of equal `z-index`.
    order: usize,
    /// The `id`s and headings met since the last box was placed. Each lands on the page the
    /// next box is placed on.
    waiting: Vec<Anchor>,
    /// The page, counting from 1, each `id` landed on, the first of each name.
    pub(super) landed: Targets,
    /// The page index and the top in the content box of each `id`, the first of each name.
    pub(super) tops: BTreeMap<String, (usize, f32)>,
    /// Each heading landed so far, its top in the content box.
    pub(super) bookmarks: Vec<Bookmark>,
}

/// A float's margin box on its page: its left and right edge, its top and bottom from the top
/// of the page's content, and whether it floats right.
struct Floated {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    on_right: bool,
}

impl Flow {
    pub(super) fn new(limit: f32) -> Flow {
        Flow {
            pages: vec![Page::default()],
            cursor: 0.0,
            pending: 0.0,
            open: Vec::new(),
            limits: vec![limit],
            heights: None,
            forced: None,
            start: 0.0,
            baseline: None,
            first: None,
            floats: Vec::new(),
            layers: Vec::new(),
            loose: Vec::new(),
            fixed: Vec::new(),
            order: 0,
            waiting: Vec::new(),
            landed: Targets::new(),
            tops: BTreeMap::new(),
            bookmarks: Vec::new(),
        }
    }

    /// Records that the element with an `id`, or the heading, starts at the next box placed.
    pub(super) fn target(&mut self, anchor: Anchor) {
        self.waiting.push(anchor);
    }

    /// Lands every `id` and heading still waiting on the page the next box would start on, at
    /// the end of the document: at the top of that page when it is a new one.
    pub(super) fn land(&mut self) {
        let page = self.landing();
        let top = if page == self.pages.len() - 1 {
            self.cursor
        } else {
            0.0
        };
        self.arrive(page, top);
    }

    /// Lands every `id` and heading waiting at `top` on the page at index `page`.
    fn arrive(&mut self, page: usize, top: f32) {
        for anchor in std::mem::take(&mut self.waiting) {
            self.put(anchor, page, top);
        }
    }

    /// Lands the `id` or heading `anchor` at `top` on the page at index `page`.
    pub(super) fn put(&mut self, anchor: Anchor, page: usize, top: f32) {
        match anchor {
            Anchor::Id(id) => {
                self.landed.entry(id.clone()).or_insert(page + 1);
                self.tops.entry(id).or_insert((page, top));
            }
            Anchor::Heading(level, title) => self.bookmarks.push(Bookmark {
                level,
                title,
                page,
                y: top,
            }),
        }
    }

    /// The flow of the document's pages, the page at each index and name `heights` tall.
    pub(super) fn paged(heights: Heights) -> Flow {
        let mut flow = Flow::new(heights(0, None));
        flow.heights = Some(heights);
        flow
    }

    /// How tall the last page's content is, less the bottom edges of the closed boxes open.
    fn limit(&self) -> f32 {
        self.limits.last().copied().unwrap_or(f32::INFINITY) - self.edges(2)
    }

    /// How tall the next page's content will be, named as the innermost open box's pages are,
    /// less the top and bottom edges of the closed boxes open.
    pub(super) fn next_limit(&self) -> f32 {
        let name = self.open.last().and_then(|o| o.geometry.page.as_deref());
        match &self.heights {
            Some(heights) => heights(self.pages.len(), name) - self.edges(0) - self.edges(2),
            None => self.limit(),
        }
    }

    /// The border and padding on the side `side` of the closed boxes open, added up.
    fn edges(&self, side: usize) -> f32 {
        self.open
            .iter()
            .filter(|o| o.geometry.closed)
            .map(|o| o.geometry.border[side] + o.geometry.padding[side])
            .sum()
    }

    /// Takes a forced break: a new page unless nothing is placed on the last one, and an empty
    /// blank page before it when it is on the wrong side. The first page is a right one.
    fn force(&mut self, side: Break) {
        if self.heights.is_none() {
            return;
        }
        if !self.empty() {
            self.new_page();
        }
        let right = self.pages.len() % 2 == 1;
        if matches!((side, right), (Break::Left, true) | (Break::Right, false)) {
            self.page().blank = true;
            self.new_page();
        }
    }

    /// The index of the page the next box placed starts on: the last page, or a later one when
    /// a `break-after` is waiting to be taken.
    pub(super) fn landing(&self) -> usize {
        let last = self.pages.len() - 1;
        let Some(side) = self.forced.filter(|_| self.heights.is_some()) else {
            return last;
        };
        let next = if self.empty() { last } else { last + 1 };
        // The page at an even index is a right one.
        match (side, next % 2 == 0) {
            (Break::Left, true) | (Break::Right, false) => next + 1,
            _ => next,
        }
    }

    /// Whether the next box placed starts at the top of its page: nothing is on the last page
    /// yet, or a `break-after` is waiting to be taken.
    pub(super) fn at_top(&self) -> bool {
        self.empty() || self.forced.is_some() && self.heights.is_some()
    }

    /// Whether the flow breaks into the document's pages.
    pub(super) fn paginates(&self) -> bool {
        self.heights.is_some()
    }

    /// Moves to a new page before a box with `break-inside: avoid`, `margin` below the last
    /// block and `height` tall from its top border edge to its bottom one, when it does not fit
    /// on the last page and fits on an empty one.
    pub(super) fn keep(&mut self, margin: f32, height: f32) {
        let Some(heights) = &self.heights else {
            return;
        };
        if self.forced.is_some() || self.empty() {
            return;
        }
        let top = self.cursor + self.pending.max(margin);
        let name = self.pages.last().and_then(|p| p.name.as_deref());
        if top + height > self.limit() && height <= heights(self.pages.len(), name) {
            self.new_page();
        }
    }

    /// Takes a waiting `break-after`, and returns where the next box placed starts, how tall
    /// the last page's content is and whether that page is empty, or `None` for a flow that
    /// does not break into pages.
    pub(super) fn room(&mut self) -> Option<(f32, f32, bool)> {
        self.heights.as_ref()?;
        if let Some(side) = self.forced.take() {
            self.force(side);
        }
        Some((self.cursor + self.margin(), self.limit(), self.empty()))
    }

    /// Moves to a new page unless the last one is empty.
    pub(super) fn break_page(&mut self) {
        if !self.empty() {
            self.new_page();
        }
    }

    /// Whether nothing is drawn on the last page and nothing has taken space on it. An
    /// [`Item::Tag`], [`Item::Untag`] or [`Item::Here`] draws nothing, so a page holding only
    /// those is empty.
    fn empty(&self) -> bool {
        self.cursor <= self.start
            && self.pages.last().is_some_and(|p| {
                p.items
                    .iter()
                    .all(|item| matches!(item, Item::Tag(_) | Item::Untag | Item::Here { .. }))
            })
    }

    /// The margin the next box is placed under: the pending one, which the top of a page after
    /// the first drops.
    fn margin(&self) -> f32 {
        if self.empty() && self.pages.len() > 1 {
            0.0
        } else {
            self.pending
        }
    }

    /// Starts a page, named as the innermost open box's pages are. Each closed box open across
    /// the break ends under the last block and the bottom edges of the closed boxes inside it,
    /// and starts again under the top edges of those around it.
    fn new_page(&mut self) {
        let mut below = self.cursor;
        for open in self.open.iter_mut().rev() {
            let g = &open.geometry;
            if g.closed && open.top.is_some() {
                below += g.border[2] + g.padding[2];
                open.cuts.push((below, 0.0));
            }
        }
        let mut above = 0.0;
        for open in &mut self.open {
            let g = &open.geometry;
            if g.closed && open.top.is_some() {
                if let Some(cut) = open.cuts.last_mut() {
                    cut.1 = above;
                }
                above += g.border[0] + g.padding[0];
            }
        }
        let name = self.open.last().and_then(|o| o.geometry.page.clone());
        let limit = match &self.heights {
            Some(heights) => heights(self.pages.len(), name.as_deref()),
            None => self.limit(),
        };
        self.pages.push(Page {
            name,
            ..Page::default()
        });
        self.limits.push(limit);
        self.cursor = above;
        self.start = above;
        self.floats.clear();
    }

    /// Where the next box's top would be, before it is placed.
    pub(super) fn next_top(&self) -> f32 {
        self.cursor + self.margin()
    }

    /// The lowest bottom of the floats on the sides `clear` names, or 0 when there is none.
    fn cleared(&self, clear: [bool; 2]) -> f32 {
        self.floats
            .iter()
            .filter(|f| clear[usize::from(f.on_right)])
            .map(|f| f.bottom)
            .fold(0.0, f32::max)
    }

    /// The part of the content box at `x`, `width` wide, that a line from `top` to `bottom`
    /// has beside the floats: its left edge and its width.
    pub(super) fn band(&self, top: f32, bottom: f32, x: f32, width: f32) -> (f32, f32) {
        let (mut lo, mut hi) = (x, x + width);
        for f in self
            .floats
            .iter()
            .filter(|f| f.top < bottom && f.bottom > top)
        {
            if f.on_right {
                hi = hi.min(f.left);
            } else {
                lo = lo.max(f.right);
            }
        }
        (lo, (hi - lo).max(0.0))
    }

    /// Whether any float is on the last page.
    pub(super) fn has_floats(&self) -> bool {
        !self.floats.is_empty()
    }

    /// Places a float `width` by `height` in the content box at `x`, `width` wide: at the
    /// top the next box would have, or lower where the floats placed before it leave no room,
    /// and on the next page when it does not fit under the last block. Returns its left edge
    /// and its top. The flow's cursor does not move.
    pub(super) fn float(
        &mut self,
        size: (f32, f32),
        containing: (f32, f32),
        on_right: bool,
        clear: [bool; 2],
    ) -> (f32, f32) {
        let (width, height) = size;
        let (x, room) = containing;
        let mut top = self
            .next_top()
            .max(self.cleared(clear))
            .max(self.floats.last().map_or(0.0, |f| f.top));
        if !self.empty() && top + height > self.limit() {
            self.new_page();
            top = 0.0;
        }
        let left = loop {
            let (lo, free) = self.band(top, top + height, x, room);
            let beside: Vec<f32> = self
                .floats
                .iter()
                .filter(|f| f.top < top + height && f.bottom > top)
                .map(|f| f.bottom)
                .collect();
            if free >= width - 0.01 || beside.is_empty() {
                break if on_right { lo + free - width } else { lo };
            }
            top = beside.into_iter().fold(f32::MAX, f32::min);
        };
        let page = self.pages.len() - 1;
        let item = self.pages[page].items.len();
        let at = self.next_top();
        for open in &mut self.open {
            open.top.get_or_insert((page, item, at));
        }
        self.floats.push(Floated {
            left,
            right: left + width,
            top,
            bottom: top + height,
            on_right,
        });
        (left, top)
    }

    /// Moves the cursor below every float, so the box being laid out holds them.
    pub(super) fn contain(&mut self) {
        let bottom = self.cleared([true, true]);
        if bottom > self.cursor {
            self.cursor = bottom;
            self.pending = 0.0;
        }
    }

    /// Moves to a new page when `height` does not fit under the last block, unless the page is
    /// empty, and returns the top of the box `height` is placed in. A page is empty while
    /// nothing is drawn on it and nothing has taken space on it, so a tall box that draws
    /// nothing still moves the next block down.
    pub(super) fn place(&mut self, height: f32) -> f32 {
        if let Some(side) = self.forced.take() {
            self.force(side);
        }
        let empty = self.empty();
        let margin = self.margin();
        if !empty && self.cursor + margin + height > self.limit() {
            self.new_page();
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
        self.arrive(page, top);
        self.cursor += height;
        top
    }

    pub(super) fn page(&mut self) -> &mut Page {
        self.pages.last_mut().expect("there is always a page")
    }

    /// Whether the innermost open box's width fills its containing block.
    pub(super) fn fills(&self) -> bool {
        self.open.last().is_some_and(|o| o.geometry.fill)
    }

    /// Narrows the innermost open box by `by` from its right edge. Its edges are drawn with
    /// the new width when it closes.
    pub(super) fn narrow(&mut self, by: f32) {
        if let Some(open) = self.open.last_mut() {
            open.geometry.width -= by;
        }
    }

    pub(super) fn open(&mut self, geometry: BoxGeometry) {
        let after = self.forced.take();
        if let Some(side) = geometry.breaks[0].or(after) {
            self.force(side);
        }
        // The first box placed on a page names it.
        let last = self.pages.len() - 1;
        if let Some(heights) = &self.heights
            && self.empty()
            && self.pages[last].name != geometry.page
        {
            let limit = heights(last, geometry.page.as_deref());
            *self.limits.last_mut().expect("a limit for each page") = limit;
            self.pages[last].name.clone_from(&geometry.page);
        }
        self.pending = self.pending.max(geometry.margin[0]);
        // A box that clears a float starts with its border edge below it.
        let below = self.cleared(geometry.clear);
        if self.next_top() < below {
            self.cursor = below;
            self.pending = 0.0;
        }
        let inset = geometry.border[0] + geometry.padding[0];
        let stack = (geometry.position != Position::Static).then(|| {
            self.order += 1;
            Stack {
                z: geometry.z,
                order: self.order,
                layers: self.layers.len(),
                absolute: Vec::new(),
            }
        });
        self.open.push(Open {
            geometry,
            top: None,
            stack,
            cuts: Vec::new(),
        });
        if inset > 0.0 {
            self.place(inset);
        }
    }

    pub(super) fn close(&mut self) {
        let Some(Open {
            geometry,
            mut top,
            stack,
            cuts,
        }) = self.open.pop()
        else {
            return;
        };
        // A break after a box inside this one is taken by what follows this box, not by its
        // bottom edge.
        let inner = self.forced.take();
        let g = &geometry;
        let inset = g.border[2] + g.padding[2];
        if top.is_none() && (inset > 0.0 || g.min_content > 0.0) {
            self.open.push(Open {
                geometry: g.clone(),
                top: None,
                stack: None,
                cuts: Vec::new(),
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
        // How many items the box's own paint is on each of its pages.
        let mut painted = Vec::new();
        if let Some((first, item, at)) = top {
            let last = self.pages.len() - 1;
            for page in first..=last {
                if self.pages[page].blank {
                    painted.push(0);
                    continue;
                }
                // A closed box draws its top and bottom edge on each page, where `cuts` says.
                let cut = |at: usize| cuts.get(at).filter(|_| g.closed);
                let top = match page.checked_sub(first + 1).and_then(cut) {
                    _ if page == first => at,
                    Some(&(_, top)) => top,
                    None => 0.0,
                };
                let bottom = match cut(page - first) {
                    _ if page == last => self.cursor,
                    Some(&(bottom, _)) => bottom,
                    None => self.limits[page],
                };
                let mut items = boxes::paint(
                    g,
                    MARGIN + top,
                    bottom - top,
                    [
                        page == first || g.closed,
                        true,
                        page == last || g.closed,
                        true,
                    ],
                );
                // A transform turns the slice of the box on this page about its own origin.
                let grouped = g.decor.grouped();
                if grouped {
                    let transform = g
                        .decor
                        .transform
                        .as_ref()
                        .map(|t| t.matrix(g.x, MARGIN + top, g.width, bottom - top));
                    items.insert(
                        0,
                        Item::Begin {
                            opacity: g.decor.opacity,
                            clip: None,
                            transform,
                        },
                    );
                }
                let under = if page == first { item } else { 0 };
                painted.push(items.len());
                let page = &mut self.pages[page].items;
                page.splice(under..under, items);
                if grouped {
                    page.push(Item::End);
                }
            }
        }
        if let Some(stack) = stack {
            self.lift(g, top, stack, &painted);
        }
        self.pending = self.pending.max(geometry.margin[2]);
        self.forced = geometry.breaks[1].or(inner);
    }

    /// Takes a positioned box that has just closed out of its pages into layers: places the
    /// absolute boxes it is the containing block of against its padding box, moves it and them
    /// by its relative offset, and, when it has a `z-index`, draws the layers inside it as part
    /// of its own: those below 0 after its background and the rest after its content.
    fn lift(
        &mut self,
        g: &BoxGeometry,
        top: Option<(usize, usize, f32)>,
        stack: Stack,
        painted: &[usize],
    ) {
        let last = self.pages.len() - 1;
        let (first, under, at) = top.unwrap_or((last, self.pages[last].items.len(), self.cursor));
        let area = Area {
            left: g.x + g.border[3],
            width: g.width - g.border[1] - g.border[3],
            top: (first, at + g.border[0]),
            bottom: (last, self.cursor - g.border[2]),
        };
        for absolute in stack.absolute {
            self.place_absolute(absolute, &area);
        }
        let [dx, dy] = match g.position {
            Position::Relative(offset) => offset,
            _ => [0.0, 0.0],
        };
        let mut inner = self.layers.split_off(stack.layers);
        for layer in &mut inner {
            layer.items = std::mem::take(&mut layer.items)
                .into_iter()
                .map(|item| shift(item, dx, dy))
                .collect();
        }
        inner.sort_by_key(|l| (l.z, l.order));
        let grouped = g.decor.grouped();
        for page in first..=last {
            let from = if page == first { under } else { 0 };
            let mut items: Vec<Item> = self.pages[page]
                .items
                .drain(from..)
                .map(|item| shift(item, dx, dy))
                .collect();
            if stack.z.is_some() {
                let mut layers = |below: bool| {
                    inner
                        .iter_mut()
                        .filter(move |l| l.page == page && (l.z < 0) == below)
                        .flat_map(|l| std::mem::take(&mut l.items))
                        .collect::<Vec<Item>>()
                };
                let above = layers(false);
                let end = items.len() - usize::from(grouped && top.is_some());
                items.splice(end..end, above);
                let paint = painted.get(page - first).copied().unwrap_or(0);
                let below = layers(true);
                items.splice(paint..paint, below);
            }
            self.layers.push(Layer {
                page,
                z: stack.z.unwrap_or(0),
                order: stack.order,
                items,
            });
        }
        // A box with no `z-index` stacks the positioned boxes inside it beside its own.
        if stack.z.is_none() {
            self.layers.extend(inner);
        }
    }

    /// Places an absolute box `size` big, whose items have its margin box's top-left corner at
    /// 0, 0, at the static place this flow has reached, to wait for its containing block: the
    /// innermost positioned box open, or the page. A `fixed` one waits for every page.
    pub(super) fn absolute(
        &mut self,
        items: Vec<Item>,
        size: (f32, f32),
        x: f32,
        insets: Insets,
        z: Option<i64>,
        fixed: bool,
    ) {
        self.order += 1;
        let absolute = Absolute {
            items,
            size,
            insets,
            page: self.pages.len() - 1,
            x,
            y: self.next_top(),
            z: z.unwrap_or(0),
            order: self.order,
        };
        if fixed {
            self.fixed.push(absolute);
            return;
        }
        match self.open.iter_mut().rev().find_map(|o| o.stack.as_mut()) {
            Some(stack) => stack.absolute.push(absolute),
            None => self.loose.push(absolute),
        }
    }

    /// Places `absolute` against `area` as a layer. `left` wins over `right` and `top` over
    /// `bottom`, and a side with neither keeps the place the flow gave it. A percentage of the
    /// height counts against the area's height when it is on one page.
    fn place_absolute(&mut self, absolute: Absolute, area: &Area) {
        let (width, height) = absolute.size;
        let inset = |side: usize, basis: f32| {
            absolute.insets[side].map(|(pt, percent)| pt + percent / 100.0 * basis)
        };
        let x = match (inset(3, area.width), inset(1, area.width)) {
            (Some(left), _) => area.left + left,
            (None, Some(right)) => area.left + area.width - right - width,
            (None, None) => absolute.x,
        };
        let tall = if area.top.0 == area.bottom.0 {
            area.bottom.1 - area.top.1
        } else {
            0.0
        };
        let (page, y) = match (inset(0, tall), inset(2, tall)) {
            (Some(top), _) => (area.top.0, area.top.1 + top),
            (None, Some(bottom)) => (area.bottom.0, area.bottom.1 - bottom - height),
            (None, None) => (absolute.page, absolute.y),
        };
        self.layers.push(Layer {
            page,
            z: absolute.z,
            order: absolute.order,
            items: absolute
                .items
                .into_iter()
                .map(|item| shift(item, x, MARGIN + y))
                .collect(),
        });
    }

    /// Places the absolute boxes with no positioned box around them against `area`, the
    /// content box's left edge, width and top on the page each one is on, and its `bottom`, or
    /// else the bottom of that page's content. Places a copy of each `fixed` box against it on
    /// every page, and draws every layer onto its page: those below 0 under the page's items and
    /// the rest over them.
    pub(super) fn finish(&mut self, area: [f32; 3], bottom: Option<f32>) {
        let [left, width, top] = area;
        let fixed = std::mem::take(&mut self.fixed);
        let copies = (0..self.pages.len())
            .flat_map(|page| fixed.iter().map(move |f| Absolute { page, ..f.clone() }));
        let loose: Vec<Absolute> = std::mem::take(&mut self.loose)
            .into_iter()
            .chain(copies)
            .collect();
        for absolute in loose {
            let page = absolute.page;
            let area = Area {
                left,
                width,
                top: (page, top),
                bottom: (page, bottom.unwrap_or(self.limits[page])),
            };
            self.place_absolute(absolute, &area);
        }
        let mut layers = std::mem::take(&mut self.layers);
        layers.sort_by_key(|l| (l.z, l.order));
        let mut below: Vec<Vec<Item>> = vec![Vec::new(); self.pages.len()];
        for layer in layers {
            if layer.z < 0 {
                below[layer.page].extend(layer.items);
            } else {
                self.pages[layer.page].items.extend(layer.items);
            }
        }
        for (page, items) in self.pages.iter_mut().zip(below) {
            page.items.splice(0..0, items);
        }
    }
}
