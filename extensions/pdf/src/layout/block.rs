//! Block boxes: a box's geometry from its style, and the vertical flow that places boxes and
//! lines down the pages and paints each box under its content when it closes.

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
    pub(super) decor: Decor,
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
        }
    }

    /// Moves to a new page when `height` does not fit under the last block, unless the page is
    /// empty, and returns the top of the box `height` is placed in.
    pub(super) fn place(&mut self, height: f32) -> f32 {
        let empty = self.pages.last().is_some_and(|p| p.items.is_empty());
        let margin = if empty && self.pages.len() > 1 {
            0.0
        } else {
            self.pending
        };
        if !empty && self.cursor + margin + height > self.limit {
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

    pub(super) fn page(&mut self) -> &mut Page {
        self.pages.last_mut().expect("there is always a page")
    }

    pub(super) fn open(&mut self, geometry: BoxGeometry) {
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
