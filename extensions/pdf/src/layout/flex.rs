//! Flex and grid containers, laid out by `taffy`.
//!
//! A flex or grid container is a block box whose content is one [`Flex`] block, and everything
//! here holds for both: only the `taffy` style differs, and `grid`'s module doc says how a
//! grid's is read. Each child element is an
//! item, whatever its `display`, and so are the container's `::before` and `::after` and a
//! `<br>`, which is an empty item. Each run of text between them that is not only spaces is an
//! anonymous item. An item's blocks are collected the way an `inline-block`'s are, against a
//! containing block at `x` 0 as wide as the container's content box.
//!
//! `taffy` sees each item as one leaf the size of its margin box. A leaf's min-content and
//! max-content widths are its blocks' narrowest and widest lines, and its height at a width is
//! its blocks laid out at that width; a box with a `width` of its own keeps it. Once `taffy` has
//! placed them, each item is laid out at its final width, as tall as its line when it stretches,
//! and drawn where `taffy` put it. A `flex-basis` length counts its margins in, `order` sorts the
//! items before `taffy` sees them, and a percentage height is not resolved. An `auto` margin is
//! the one margin `taffy` sees, so it takes the free space. A leaf's first baseline is its first
//! line's, measured only when an item aligns on `baseline`. Under `direction: rtl` a row and a
//! grid's columns run from the right, as `taffy` places them.
//!
//! **Breaks.** The items are placed in bands: each band is the items whose boxes overlap
//! vertically, so a flex line in a row is one band and each item of a column is its own. A band
//! goes on the page as one height, so a page breaks only between bands. A band taller than a
//! page starts where it is, and each of its items is laid out again down pages of its own, so it
//! breaks between its lines and is not stretched to the band's height. The space above a band
//! after the first, a gap, is placed as a margin, and is dropped at the top of a page.
//!
//! Cost of a band taller than a page: each of its items is laid out once more, and its pages
//! are kept until the band is placed.
//!
//! Cost: each item is laid out once per width `taffy` measures it at, and once more at its final
//! size. The heights measured are kept for the container's layout, and freed with it.

use taffy::{
    AlignContent, AlignItems, AvailableSpace, Dimension, FlexDirection, FlexWrap, LayoutOutput,
    LengthPercentage, LengthPercentageAuto, Rect, RunMode, Size, TaffyTree, compute_leaf_layout,
};

use super::block::{BoxGeometry, Flow, SIDES, geometry};
use super::grid;
use super::text::{Cx, Placed, carry, narrow, shift};
use super::{Block, MARGIN, keyword, length};
use crate::css::{Style, Value};

/// A flex container's content box, its `taffy` style, and its items.
#[derive(Clone)]
pub(super) struct Flex {
    /// The content box's left edge and width.
    pub(super) x: f32,
    pub(super) width: f32,
    pub(super) style: taffy::Style,
    /// Each item's index in the list of atoms and its `taffy` style, in `order`.
    pub(super) items: Vec<(usize, taffy::Style)>,
}

/// The `taffy` style of a flex or grid container with `style`, whose content box is `width` wide
/// and at least `height` tall, with `rem` for a grid's track lengths.
pub(super) fn container(style: &Style, width: f32, height: f32, rem: f32) -> taffy::Style {
    let gap = |name: &str| LengthPercentage::length(length(style, name, width).unwrap_or(0.0));
    let grid = matches!(keyword(style, "display"), Some("grid" | "inline-grid"));
    let mut out = taffy::Style {
        display: if grid {
            taffy::Display::Grid
        } else {
            taffy::Display::Flex
        },
        flex_direction: match keyword(style, "flex-direction") {
            Some("row-reverse") => FlexDirection::RowReverse,
            Some("column") => FlexDirection::Column,
            Some("column-reverse") => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row,
        },
        flex_wrap: match keyword(style, "flex-wrap") {
            Some("wrap") => FlexWrap::Wrap,
            Some("wrap-reverse") => FlexWrap::WrapReverse,
            _ => FlexWrap::NoWrap,
        },
        justify_content: keyword(style, "justify-content")
            .and_then(distribute)
            .unwrap_or_default(),
        align_content: keyword(style, "align-content")
            .and_then(distribute)
            .unwrap_or_default(),
        align_items: keyword(style, "align-items")
            .and_then(align)
            .unwrap_or_default(),
        justify_items: keyword(style, "justify-items")
            .and_then(align)
            .unwrap_or_default(),
        gap: Size {
            width: gap("column-gap"),
            height: gap("row-gap"),
        },
        size: Size {
            width: Dimension::length(width),
            height: Dimension::auto(),
        },
        min_size: Size {
            width: taffy::LengthPercentageAuto::auto(),
            height: taffy::LengthPercentageAuto::length(height),
        },
        // A row and a grid's columns run from the right under `direction: rtl`.
        direction: if keyword(style, "direction") == Some("rtl") {
            taffy::Direction::Rtl
        } else {
            taffy::Direction::Ltr
        },
        ..taffy::Style::default()
    };
    if grid {
        grid::container(style, rem, &mut out);
    }
    out
}

/// The `order` and the `taffy` style of an item with `style` in a container whose content box
/// is `width` wide and whose main axis is a row when `row` is true.
pub(super) fn item(style: &Style, width: f32, row: bool) -> (i64, taffy::Style) {
    let mut g = geometry(style, 0.0, width);
    let auto = unauto(style, &mut g);
    let side = |auto: bool| {
        if auto {
            LengthPercentageAuto::auto()
        } else {
            LengthPercentageAuto::length(0.0)
        }
    };
    let horizontal = g.margin[1] + g.margin[3];
    let vertical = g.margin[0] + g.margin[2];
    let insets = g.border[0] + g.border[2] + g.padding[0] + g.padding[2];
    let number = |name: &str, default: f32| match style.get(name) {
        Some(Value::Number(n)) => *n,
        _ => default,
    };
    let border_box = keyword(style, "box-sizing") == Some("border-box");
    let basis = match length(style, "flex-basis", width) {
        Some(b) if row => {
            let inset = g.border[1] + g.border[3] + g.padding[1] + g.padding[3];
            Dimension::length(if border_box { b } else { b + inset } + horizontal)
        }
        Some(b) => Dimension::length(if border_box { b } else { b + insets } + vertical),
        None => Dimension::auto(),
    };
    let order = match style.get("order") {
        Some(Value::Integer(n)) => *n,
        _ => 0,
    };
    let fixed_height = matches!(style.get("height"), Some(Value::Length(l)) if l.percent == 0.0);
    let mut taffy_style = taffy::Style {
        flex_grow: number("flex-grow", 0.0),
        flex_shrink: number("flex-shrink", 1.0),
        flex_basis: basis,
        align_self: keyword(style, "align-self").and_then(align),
        justify_self: keyword(style, "justify-self").and_then(align),
        // The leaf is the item's margin box, so only an `auto` margin is a margin to `taffy`.
        margin: Rect {
            top: side(auto[0]),
            right: side(auto[1]),
            bottom: side(auto[2]),
            left: side(auto[3]),
        },
        size: Size {
            width: if g.fill {
                Dimension::auto()
            } else {
                Dimension::length(g.width + horizontal)
            },
            height: if fixed_height {
                Dimension::length(g.min_content + insets + vertical)
            } else {
                Dimension::auto()
            },
        },
        ..taffy::Style::default()
    };
    grid::item(style, &mut taffy_style);
    (order, taffy_style)
}

/// Takes the free space back from each `auto` margin of an item's box `g`, because its container
/// shares the free space out among them, and returns those sides: top, right, bottom and left.
pub(super) fn unauto(style: &Style, g: &mut BoxGeometry) -> [bool; 4] {
    let auto = SIDES.map(|s| keyword(style, &format!("margin-{s}")) == Some("auto"));
    if auto[3] {
        g.x -= g.margin[3];
    }
    for (margin, auto) in g.margin.iter_mut().zip(auto) {
        if auto {
            *margin = 0.0;
        }
    }
    auto
}

/// An anonymous item's `taffy` style: it grows by nothing and shrinks as an item does.
pub(super) fn anonymous() -> taffy::Style {
    taffy::Style::default()
}

fn align(word: &str) -> Option<AlignItems> {
    Some(match word {
        "stretch" => AlignItems::STRETCH,
        "start" | "left" => AlignItems::START,
        "end" | "right" => AlignItems::END,
        "flex-start" => AlignItems::FLEX_START,
        "flex-end" => AlignItems::FLEX_END,
        "center" => AlignItems::CENTER,
        "baseline" => AlignItems::BASELINE,
        _ => return None,
    })
}

fn distribute(word: &str) -> Option<AlignContent> {
    Some(match word {
        "stretch" => AlignContent::STRETCH,
        "start" | "left" => AlignContent::START,
        "end" | "right" => AlignContent::END,
        "flex-start" => AlignContent::FLEX_START,
        "flex-end" => AlignContent::FLEX_END,
        "center" => AlignContent::CENTER,
        "space-between" => AlignContent::SPACE_BETWEEN,
        "space-around" => AlignContent::SPACE_AROUND,
        "space-evenly" => AlignContent::SPACE_EVENLY,
        _ => return None,
    })
}

/// The height and first baseline an item was measured at, by margin-box width.
type Measured = Vec<(f32, f32, Option<f32>)>;

/// The height and first baseline of an item's `blocks` laid out with a margin box `width`
/// wide, measured once per width.
fn measure(
    cx: &mut Cx<'_>,
    blocks: &[Block],
    measured: &mut Measured,
    width: f32,
) -> (f32, Option<f32>) {
    if let Some((_, h, first)) = measured.iter().find(|(w, ..)| (w - width).abs() < 0.01) {
        return (*h, *first);
    }
    let laid = cx.item_at(blocks, width, None);
    measured.push((width, laid.height, laid.first));
    (laid.height, laid.first)
}

impl Cx<'_> {
    /// Lays the items of `flex` out with `taffy` and places them down `flow` in bands.
    pub(super) fn flex(&mut self, flex: Flex, flow: &mut Flow) {
        let blocks: Vec<Vec<Block>> = flex
            .items
            .iter()
            .map(|(atom, _)| self.waiting[*atom].take().unwrap_or_default())
            .collect();
        let widths: Vec<[f32; 2]> = blocks
            .iter()
            .map(|b| [self.content_width(b, false), self.content_width(b, true)])
            .collect();
        let mut measured: Vec<Measured> = vec![Vec::new(); blocks.len()];
        let mut tree: TaffyTree<usize> = TaffyTree::new();
        tree.disable_rounding();
        let leaves: Vec<_> = flex
            .items
            .iter()
            .enumerate()
            .map(|(i, (_, style))| tree.new_leaf_with_context(style.clone(), i))
            .collect::<Result<_, _>>()
            .expect("a new taffy leaf has no parent to fail on");
        // The container may have narrowed since its style was taken, inside an inline-block.
        let mut style = flex.style.clone();
        style.size.width = Dimension::length(flex.width);
        let root = tree
            .new_with_children(style, &leaves)
            .expect("the leaves are new");
        // A baseline is measured only when an item aligns on one.
        let baseline = flex.style.align_items == AlignItems::BASELINE
            || flex
                .items
                .iter()
                .any(|(_, s)| s.align_self == Some(AlignItems::BASELINE));
        let available = Size {
            width: AvailableSpace::Definite(flex.width),
            height: AvailableSpace::MaxContent,
        };
        tree.compute_layout_with_measure(root, available, |input, _, context, style| {
            let Some(&mut i) = context else {
                return LayoutOutput::HIDDEN;
            };
            let mut output = compute_leaf_layout(
                input,
                style,
                |_, _| 0.0,
                |known, available| {
                    let [min, max] = widths[i];
                    let width = known.width.unwrap_or(match available.width {
                        AvailableSpace::MinContent => min,
                        AvailableSpace::MaxContent => max,
                        AvailableSpace::Definite(room) => room.min(max).max(min),
                    });
                    let height = known
                        .height
                        .unwrap_or_else(|| measure(self, &blocks[i], &mut measured[i], width).0);
                    Size { width, height }
                },
            );
            // An item's first baseline is its first line's, which `align-items: baseline` reads.
            if baseline && input.run_mode == RunMode::PerformLayout {
                let width = output.size.width;
                output.baselines.first = measure(self, &blocks[i], &mut measured[i], width).1;
            }
            output
        })
        .expect("measuring an item does not fail");
        let mut placed: Vec<(f32, f32, Placed)> = Vec::with_capacity(blocks.len());
        for (leaf, item) in leaves.iter().zip(&blocks) {
            let at = tree.layout(*leaf).expect("the leaf is in the tree");
            let laid = self.item_at(item, at.size.width, Some(at.size.height));
            placed.push((at.location.x, at.location.y, laid));
        }
        let total = tree
            .layout(root)
            .expect("the root is in the tree")
            .size
            .height;
        // Bands of items whose boxes overlap vertically, top to bottom.
        let mut order: Vec<usize> = (0..placed.len()).collect();
        order.sort_by(|a, b| placed[*a].1.total_cmp(&placed[*b].1));
        let mut bands: Vec<(f32, Vec<usize>)> = Vec::new();
        for i in order {
            let (_, top, ref laid) = placed[i];
            let bottom = top + laid.height;
            match bands.last_mut() {
                Some((end, members)) if top < *end - 0.01 => {
                    *end = end.max(bottom);
                    members.push(i);
                }
                _ => bands.push((bottom, vec![i])),
            }
        }
        let mut above = 0.0;
        for (k, (bottom, members)) in bands.into_iter().enumerate() {
            // The space between two bands is a gap, and is dropped at the top of a page as a
            // margin is.
            let from = if k == 0 {
                above
            } else {
                let lead = (placed[members[0]].1 - above).max(0.0);
                flow.pending = flow.pending.max(lead);
                above + lead
            };
            if flow.paginates() && bottom - from > flow.next_limit() + 0.01 {
                let widths: Vec<f32> = members
                    .iter()
                    .map(|&i| tree.layout(leaves[i]).map_or(0.0, |at| at.size.width))
                    .collect();
                self.split_band(&members, &widths, &blocks, &placed, from, flex.x, flow);
                above = bottom;
                continue;
            }
            let top = flow.place(bottom - from);
            let page = flow.pages.len() as u64;
            for i in members {
                let (x, y, laid) = &mut placed[i];
                let (dx, dy) = (flex.x + *x, MARGIN + top + *y - from);
                let items: Vec<_> = std::mem::take(&mut laid.items)
                    .into_iter()
                    .map(|item| shift(item, dx, dy))
                    .collect();
                flow.page().items.extend(items);
                carry(&mut self.missing, std::mem::take(&mut laid.missing), page);
            }
            above = bottom;
        }
        if total > above + 0.01 {
            flow.place(total - above);
        }
    }

    /// Places a band taller than a page, whose items `members` are `widths` wide and placed at
    /// `placed`, with its top at `from` in the container. Each item is laid out again down
    /// pages of its own, so it breaks between its lines. The first page is as tall as the room
    /// left and each later one as a whole page, and the last part is as tall as the item that
    /// reaches lowest. An item is not stretched to the band's height.
    #[allow(clippy::too_many_arguments)]
    fn split_band(
        &mut self,
        members: &[usize],
        widths: &[f32],
        blocks: &[Vec<Block>],
        placed: &[(f32, f32, Placed)],
        from: f32,
        left: f32,
        flow: &mut Flow,
    ) {
        let room = |flow: &mut Flow| match flow.room() {
            Some((top, limit, _)) => limit - top - 0.01,
            None => f32::INFINITY,
        };
        if room(flow) <= 0.0 {
            flow.break_page();
        }
        let now = room(flow);
        let later = flow.next_limit();
        // Each item's left edge, its top in the band and its pages.
        let mut parts: Vec<(f32, f32, Flow)> = Vec::with_capacity(members.len());
        let (mut count, mut tail) = (1, 0.0f32);
        let mut lost = Vec::new();
        for (&i, &width) in members.iter().zip(widths) {
            let (x, y, _) = &placed[i];
            let dy = y - from;
            let rooms = [now - dy, later];
            let pages = Flow::paged(Box::new(move |page, _| rooms[usize::from(page > 0)]), None);
            let (pages, missing) = self.down(fit(&blocks[i], width, None), pages);
            let n = pages.pages.len();
            let bottom = pages.cursor + pages.pending + if n == 1 { dy } else { 0.0 };
            if n > count {
                (count, tail) = (n, bottom);
            } else if n == count {
                tail = tail.max(bottom);
            }
            parts.push((left + x, dy, pages));
            lost.push(missing);
        }
        let page = flow.pages.len() as u64;
        for part in 0..count {
            let height = match part {
                _ if part + 1 == count => tail,
                0 => now,
                _ => later,
            };
            let top = flow.place(height);
            for (x, dy, pages) in &mut parts {
                let Some(on) = pages.pages.get_mut(part) else {
                    continue;
                };
                let dy = top + if part == 0 { *dy } else { 0.0 };
                let items: Vec<_> = std::mem::take(&mut on.items)
                    .into_iter()
                    .map(|item| shift(item, *x, dy))
                    .collect();
                flow.page().items.extend(items);
            }
        }
        for missing in lost {
            carry(&mut self.missing, missing, page);
        }
    }

    /// The margin-box width of an item's `blocks` with its lines unbroken when `max` is true,
    /// broken at every chance when it is false, or its own `width`.
    pub(super) fn content_width(&mut self, blocks: &[Block], max: bool) -> f32 {
        let Some(Block::Open(g)) = blocks.first() else {
            return 0.0;
        };
        let outside = g.margin[1] + g.margin[3];
        if !g.fill {
            return g.width + outside;
        }
        let inset = g.border[1] + g.border[3] + g.padding[1] + g.padding[3];
        let left = g.x + g.border[3] + g.padding[3];
        let room = g.width - inset;
        self.widest(blocks, left, room, max) + inset + outside
    }

    /// An item's `blocks` laid out with a margin box `width` wide, and at least `height` tall.
    pub(super) fn item_at(&mut self, blocks: &[Block], width: f32, height: Option<f32>) -> Placed {
        self.lay(fit(blocks, width, height), parley::VerticalAlign::BASELINE)
    }
}

/// An item's `blocks` with a margin box `width` wide, and at least `height` tall.
pub(super) fn fit(blocks: &[Block], width: f32, height: Option<f32>) -> Vec<Block> {
    let mut blocks = blocks.to_vec();
    let mut by = 0.0;
    if let Some(Block::Open(g)) = blocks.first_mut() {
        if let Some(height) = height {
            let edges =
                g.margin[0] + g.margin[2] + g.border[0] + g.border[2] + g.padding[0] + g.padding[2];
            g.min_content = g.min_content.max(height - edges);
        }
        if g.fill {
            let inset = g.border[1] + g.border[3] + g.padding[1] + g.padding[3];
            let least = g.margin[1] + g.margin[3] + inset;
            by = g.margin[1] + g.width + g.margin[3] - width.max(least);
        }
    }
    if by != 0.0 {
        narrow(&mut blocks, by);
    }
    blocks
}
