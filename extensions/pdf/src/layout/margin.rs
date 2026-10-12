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
//! Cost: one layout of a short paragraph for each margin box on each page. Memory: its items on
//! its page, and one text style for each.

use parley::VerticalAlign;

use super::block::geometry;
use super::inline::{counter_text, para_of, span_of};
use super::text::{Cx, carry, shift};
use super::{Block, Page, Span, keyword};
use crate::css::{MARGIN_BOXES, Style, Value};
use crate::fonts::Fonts;
use crate::paginate::{self, Sheet};

/// A margin box ready to lay out: its page, its area's left edge, top, width and height, where it
/// sits in the area's height, and its blocks.
pub(super) struct MarginBox {
    page: usize,
    rect: [f32; 4],
    vertical: &'static str,
    blocks: Vec<Block>,
}

/// Every margin box of `pages`, with the text style of each pushed onto `spans`.
pub(super) fn collect(
    pages: &[Page],
    sheet: &Sheet,
    root: &Style,
    fonts: &Fonts,
    spans: &mut Vec<Span>,
) -> Vec<MarginBox> {
    let count = pages.len() as i64;
    let mut out = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let matched = sheet.matched(index, page.blank);
        let texts: Vec<(&str, String)> = MARGIN_BOXES
            .iter()
            .filter_map(|name| {
                let values = matched.boxes.get(*name)?;
                let content = values.get("content").copied();
                let text = paginate::content(content, index as i64 + 1, count, &counter_text)?;
                Some((*name, text))
            })
            .collect();
        let present = |name: &str| texts.iter().any(|(n, _)| *n == name);
        for (name, text) in &texts {
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
                blocks: vec![
                    Block::Open(g),
                    Block::Text {
                        text: text.clone(),
                        spans: vec![(0..text.len(), span)],
                        para: para_of(&style, width, span),
                        x,
                        width,
                        marker: false,
                        inline: Vec::new(),
                        atoms: Vec::new(),
                    },
                    Block::Close,
                ],
            });
        }
    }
    out
}

/// Lays each of `boxes` out and draws it on its page, at its area's left edge and at the top,
/// middle or bottom of the area's height.
pub(super) fn draw(cx: &mut Cx<'_>, boxes: Vec<MarginBox>, pages: &mut [Page]) {
    for one in boxes {
        let [x, y, _, height] = one.rect;
        let mut placed = cx.lay(one.blocks, VerticalAlign::BASELINE);
        let dy = match one.vertical {
            "top" => 0.0,
            "bottom" => height - placed.height,
            _ => (height - placed.height) / 2.0,
        };
        let lost = std::mem::take(&mut placed.missing);
        carry(&mut cx.missing, lost, one.page as u64 + 1);
        pages[one.page]
            .items
            .extend(placed.items.into_iter().map(|item| shift(item, x, y + dy)));
    }
}
