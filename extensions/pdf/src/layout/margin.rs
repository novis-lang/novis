//! The margin boxes of each page, laid out once every page of the document is, so
//! `counter(pages)` is known (`paginate`'s module doc says where each box is).
//!
//! A margin box is made when its `content` is not `none` or `normal`. Its text is laid out as a
//! block of its own, with the box's own margins, borders, padding and background, and with the
//! text properties the box gives or else the root element's. A box along the top or the bottom is
//! aligned to its own end of the side, `center` in the middle, and every other box is centred,
//! unless it gives a `text-align`. It sits at the top, middle or bottom of its area by its
//! `vertical-align`: in the middle unless it is at an end of the left or right side. Text taller
//! than its area is drawn past it.
//!
//! **Running elements.** An element under `position: running(name)` is laid out as an
//! inline-block is, at the width of the first page's content box, and takes no room in the flow.
//! It is on the page the next box placed after it starts on. A box whose `content` is
//! `element(name)` draws a copy of it, aligned in the box's area as the box's text would be; the
//! box's own margins, borders, padding and background are not drawn. A running element inside an
//! inline-block, a float or a flex item counts as on the first page.
//!
//! **Named strings.** An element's `string-set` is kept with the page it is set on, and a box's
//! `string()` reads it by `paginate`'s module doc. A `string-set` inside an inline-block, a float
//! or a flex item counts as on the first page.
//!
//! Cost: one layout of a short paragraph for each margin box with text on each page, and a copy
//! of a running element's items for each page that draws it. Memory: its items on its page, one
//! text style for each, and each running element's items and each named string's value until the
//! render ends.

use parley::VerticalAlign;

use super::block::geometry;
use super::inline::{counter_text, para_of, span_of};
use super::text::{Cx, carry, shift};
use super::{Block, Item, Missing, Page, Span, keyword};
use crate::css::{MARGIN_BOXES, Style, Value};
use crate::fonts::Fonts;
use crate::paginate::{self, Sheet, Which};

/// A running element laid out: its name, the index of the page it is on, its items with its
/// margin box's top-left corner at 0, 0, the margin box's width and height, and the text no font
/// has.
pub(super) struct Running {
    pub(super) name: String,
    pub(super) page: usize,
    pub(super) items: Vec<Item>,
    pub(super) size: [f32; 2],
    pub(super) missing: Vec<Missing>,
}

/// A named string an element's `string-set` sets: its lower-cased name, its value, the index of
/// the page it is set on, and whether it is set at that page's top.
pub(super) struct Set {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) page: usize,
    pub(super) top: bool,
}

/// The value of the named string `name` that `which` takes on the page at `page`.
fn string(strings: &[Set], name: &str, page: usize, which: Which) -> Option<String> {
    let mut on = strings.iter().filter(|s| s.name == name && s.page == page);
    let entry = || strings.iter().rfind(|s| s.name == name && s.page < page);
    let one = match which {
        Which::First => on.next().or_else(entry),
        Which::Last => on.next_back().or_else(entry),
        Which::Start => on.next().filter(|s| s.top).or_else(entry),
        Which::FirstExcept => match on.next() {
            Some(_) => None,
            None => entry(),
        },
    };
    one.map(|s| s.value.clone())
}

/// What a margin box draws: its blocks, or a copy of a running element's items with their
/// size and the `text-align` that places them in the area's width.
enum Content {
    Blocks(Vec<Block>),
    Element {
        items: Vec<Item>,
        size: [f32; 2],
        align: &'static str,
        missing: Vec<Missing>,
    },
}

/// A margin box ready to lay out: its page, its area's left edge, top, width and height, where it
/// sits in the area's height, and what it draws.
pub(super) struct MarginBox {
    page: usize,
    rect: [f32; 4],
    vertical: &'static str,
    content: Content,
}

/// What fills a margin box before it is placed: its text, or the running element it draws.
enum Fill<'r> {
    Text(String),
    Element(&'r Running),
}

/// The running element named `name` that the page at `page` draws: the first or `last` one on
/// it, or else the last one on an earlier page.
fn pick<'r>(running: &'r [Running], name: &str, page: usize, last: bool) -> Option<&'r Running> {
    let mut on = running.iter().filter(|r| r.name == name && r.page == page);
    let here = if last { on.next_back() } else { on.next() };
    here.or_else(|| running.iter().rfind(|r| r.name == name && r.page < page))
}

/// Every margin box of `pages`, with the text style of each pushed onto `spans`.
pub(super) fn collect(
    pages: &[Page],
    sheet: &Sheet,
    root: &Style,
    fonts: &Fonts,
    (running, strings): (&[Running], &[Set]),
    spans: &mut Vec<Span>,
) -> Vec<MarginBox> {
    let count = pages.len() as i64;
    let mut out = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let matched = sheet.matched(index, page.blank, page.name.as_deref());
        let named = |name: &str, which| string(strings, name, index, which);
        let fills: Vec<(&str, Fill)> = MARGIN_BOXES
            .iter()
            .filter_map(|name| {
                let values = matched.boxes.get(*name)?;
                let content = values.get("content").copied();
                if let Some((element, last)) = paginate::element(content) {
                    let one = pick(running, &element, index, last)?;
                    return Some((*name, Fill::Element(one)));
                }
                let number = index as i64 + 1;
                let text = paginate::content(content, number, count, &counter_text, &named)?;
                Some((*name, Fill::Text(text)))
            })
            .collect();
        let present = |name: &str| fills.iter().any(|(n, _)| *n == name);
        for (name, fill) in &fills {
            let Some(rect) = paginate::rect(name, &page.geometry, &present) else {
                continue;
            };
            let mut own = Style::default();
            for (property, value) in &matched.boxes[name] {
                own.values.insert(property, (*value).clone());
            }
            let (horizontal, vertical) = paginate::alignment(name);
            let vertical = match keyword(&own, "vertical-align") {
                Some("top") => "top",
                Some("bottom") => "bottom",
                Some("middle") => "middle",
                _ => vertical,
            };
            let text = match fill {
                Fill::Text(text) => text,
                Fill::Element(one) => {
                    let align = match keyword(&own, "text-align") {
                        Some("left" | "start") => "left",
                        Some("right" | "end") => "right",
                        Some("center") => "center",
                        _ => horizontal,
                    };
                    out.push(MarginBox {
                        page: index,
                        rect,
                        vertical,
                        content: Content::Element {
                            items: one.items.clone(),
                            size: one.size,
                            align,
                            missing: one.missing.clone(),
                        },
                    });
                    continue;
                }
            };
            let mut style = root.clone();
            style
                .values
                .insert("text-align", Value::Keyword(horizontal.to_owned()));
            style.values.extend(own.values.clone());
            let g = geometry(&own, 0.0, rect[2]);
            let x = g.x + g.border[3] + g.padding[3];
            let width = g.width - g.border[1] - g.border[3] - g.padding[1] - g.padding[3];
            spans.push(span_of(&style, fonts, None));
            let span = spans.len() - 1;
            out.push(MarginBox {
                page: index,
                rect,
                vertical,
                content: Content::Blocks(
                    // An empty `string()` leaves the box with no text to lay out.
                    [
                        Some(Block::Open(g)),
                        (!text.is_empty()).then(|| Block::Text {
                            text: text.clone(),
                            spans: vec![(0..text.len(), span)],
                            para: para_of(&style, width, span),
                            x,
                            width,
                            marker: false,
                            inline: Vec::new(),
                            atoms: Vec::new(),
                        }),
                        Some(Block::Close),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                ),
            });
        }
    }
    out
}

/// Lays each of `boxes` out and draws it on its page, at its area's left edge and at the top,
/// middle or bottom of the area's height. A running element is placed in the area's width by
/// its box's `text-align`.
pub(super) fn draw(cx: &mut Cx<'_>, boxes: Vec<MarginBox>, pages: &mut [Page]) {
    for one in boxes {
        let [x, y, width, height] = one.rect;
        let (items, size, dx, lost) = match one.content {
            Content::Blocks(blocks) => {
                let mut placed = cx.lay(blocks, VerticalAlign::BASELINE);
                let lost = std::mem::take(&mut placed.missing);
                (placed.items, [0.0, placed.height], 0.0, lost)
            }
            Content::Element {
                items,
                size,
                align,
                missing,
            } => {
                let dx = match align {
                    "left" => 0.0,
                    "right" => width - size[0],
                    _ => (width - size[0]) / 2.0,
                };
                (items, size, dx, missing)
            }
        };
        let dy = match one.vertical {
            "top" => 0.0,
            "bottom" => height - size[1],
            _ => (height - size[1]) / 2.0,
        };
        carry(&mut cx.missing, lost, one.page as u64 + 1);
        pages[one.page]
            .items
            .extend(items.into_iter().map(|item| shift(item, x + dx, y + dy)));
    }
}
