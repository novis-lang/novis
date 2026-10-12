//! Block boxes: a box's geometry from its style, and the vertical flow that places boxes and
//! lines down the pages and paints each box under its content when it closes.
//!
//! The flow holds the floats of its last page, and a new page starts with none: a float does
//! not go on past its page. A float's top is the top the next box would have, or the top of the
//! float before it if that is lower, and it goes down past the floats beside it until it fits
//! between them. A line is as wide as the room the floats leave it ([`Flow::band`]), and a box
//! with `clear` starts below the floats it clears. An `inline-block` box or a flex item holds
//! its floats: its own flow ends below them ([`Flow::contain`]).

use super::boxes::{self, Decor};
use super::{Item, MARGIN, Page, keyword, length};
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
    pub(super) decor: Decor,
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
    BoxGeometry {
        x: x + margin[3],
        width,
        margin,
        border,
        padding,
        min_content,
        fill,
        clear: sides(style, "clear"),
        decor: boxes::decor(style),
    }
}

/// A block box whose bottom has not been reached.
pub(super) struct Open {
    geometry: BoxGeometry,
    /// Where its border box starts: the page, the index its items are drawn under, and the top.
    /// `None` until something inside it, or its own top border or padding, is placed.
    top: Option<(usize, usize, f32)>,
}

/// The vertical flow: the pages so far and where the next block goes.
pub(super) struct Flow {
    pub(super) pages: Vec<Page>,
    pub(super) cursor: f32,
    pub(super) pending: f32,
    pub(super) open: Vec<Open>,
    /// How tall a page's content is: the page's, or unbounded for an inline-block's one page.
    limit: f32,
    /// The baseline of the last line placed, from the top of its page's content.
    pub(super) baseline: Option<f32>,
    /// The floats on the last page, in the order they were placed.
    floats: Vec<Floated>,
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
            limit,
            baseline: None,
            floats: Vec::new(),
        }
    }

    /// Whether nothing is drawn on the last page and nothing has taken space on it.
    fn empty(&self) -> bool {
        self.cursor <= 0.0 && self.pages.last().is_some_and(|p| p.items.is_empty())
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

    fn new_page(&mut self) {
        self.pages.push(Page::default());
        self.cursor = 0.0;
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
        if !self.empty() && top + height > self.limit {
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
        let empty = self.empty();
        let margin = self.margin();
        if !empty && self.cursor + margin + height > self.limit {
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
        self.cursor += height;
        top
    }

    pub(super) fn page(&mut self) -> &mut Page {
        self.pages.last_mut().expect("there is always a page")
    }

    pub(super) fn open(&mut self, geometry: BoxGeometry) {
        self.pending = self.pending.max(geometry.margin[0]);
        // A box that clears a float starts with its border edge below it.
        let below = self.cleared(geometry.clear);
        if self.next_top() < below {
            self.cursor = below;
            self.pending = 0.0;
        }
        let inset = geometry.border[0] + geometry.padding[0];
        self.open.push(Open {
            geometry,
            top: None,
        });
        if inset > 0.0 {
            self.place(inset);
        }
    }

    pub(super) fn close(&mut self) {
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
                    self.limit
                };
                let mut items = boxes::paint(
                    g,
                    MARGIN + top,
                    bottom - top,
                    [page == first, true, page == last, true],
                );
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
