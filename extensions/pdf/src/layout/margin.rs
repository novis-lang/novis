//! The margin boxes of each page, laid out once every page of the document is, so
//! `counter(pages)` is known (`paginate`'s module doc says where each box is).
//!
//! A margin box is made when its `content` is not `none` or `normal`. Its text is laid out as a
//! block of its own, with the box's own margins, borders, padding and background, and with the
//! text properties the box gives or else the root element's. A box along the top or the bottom is
//! aligned to its own end of the side, `center` in the middle, a corner box toward the page's
//! content, and every other box is centred, unless it gives a `text-align` (css-page-3's table,
//! [`paginate::alignment`]). It sits at the top, middle or bottom of its area by its
//! `vertical-align`: in the middle unless it is at an end of the left or right side. A box, or
//! a running element, taller than its area is drawn past it and listed in the page's
//! [`Page::overflows`], unless markup over that margin replaces it ([`overlay`]).
//!
//! **Running elements.** An element under `position: running(name)` is laid out as an
//! inline-block is, at the width of the first page's content box, and takes no room in the flow.
//! It is on the page the next box placed after it starts on. A box whose `content` is
//! `element(name)` draws a copy of it, aligned in the box's area as the box's text would be; the
//! box's own margins, borders, padding and background are not drawn. A running element inside an
//! inline-block, a float, a flex item or a table cell is on the page that box draws it on.
//!
//! **Named strings.** An element's `string-set` is kept with the page it is set on, and a box's
//! `string()` reads it by `paginate`'s module doc. A `string-set` inside an inline-block, a float,
//! a flex item or a table cell is set on the page that box draws it on, and one inside a running
//! element on the running element's page.
//!
//! **Counters.** A box's `counter(name)` reads a counter other than `page` and `pages` as
//! `string(name)` reads a named string with `first`: its value after its first change on the
//! page, or else its value when the page starts, and 0 before its first change. The walk
//! records a change only for the names the `@page` margin boxes read, at the element that
//! resets or increments the counter, or where an inner counter of the name ends. A counter
//! keeps its last value when its last scope ends.
//!
//! `counter(page)` is the page's number: 1 on the first page and one more on each page after,
//! unless an element on the page resets it. The first `counter-reset: page N` of an element
//! that starts on a page sets that page's number to N, or to 1 when it gives no number, and the
//! pages after it count on from there. `counter(pages)` is always the number of pages in the
//! file. `target-counter(…, page)` counts every page from 1, with no reset.
//!
//! **Page facts.** Each page carries its named strings, each name's value as `string(name)`
//! reads it there, and the `data-mark` marks set on it, from the top of the page down. A mark is
//! set on a page as a named string is, and a table row's on the page the row is first drawn on.
//!
//! **Markup over a margin.** [`overlay`] draws a document laid out on its own, such as the markup
//! of a header function, over one page's margin: clipped to the margin's area, and in place of
//! the margin boxes along that side. The page's other items do not move.
//!
//! Cost: one layout of a short paragraph for each margin box with text on each page, and a copy
//! of a running element's items for each page that draws it. The named strings, counter changes
//! and running elements are indexed by name and page once ([`Index`]), so each lookup reads its
//! own page's entries and the last one before it. Memory: its items on its page, one
//! text style for each, and each running element's items, each named string's value and each
//! change of a counter a margin box reads until the render ends.

use parley::VerticalAlign;

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use super::block::geometry;
use super::boxes::{self, Layer, rect};
use super::inline::{counter_text, para_of, span_of};
use super::text::{Cx, carry, shift};
use super::{Block, Fact, Images, Item, Laid, Missing, Page, Span, image, keyword};
use crate::Error;
use crate::assets::Assets;
use crate::css::{MARGIN_BOXES, Style, Value};
use crate::fonts::Fonts;
use crate::paginate::{self, Sheet, Which};

/// A running element laid out: its name, the index of the page it is on and its top there,
/// whether it is at that page's top, its items with its margin box's top-left corner at 0, 0,
/// the margin box's width and height, and the text no font has.
pub(super) struct Running {
    pub(super) name: String,
    pub(super) page: usize,
    pub(super) y: f32,
    pub(super) top: bool,
    pub(super) items: Vec<Item>,
    pub(super) size: [f32; 2],
    pub(super) missing: Vec<Missing>,
}

/// A named string an element's `string-set` sets: its name as written, its value, the index of
/// the page it is set on and its top there, and whether it is set at that page's top. `fact`
/// says whether it is a named string, a `data-mark` mark or a counter's value.
pub(super) struct Set {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) page: usize,
    pub(super) y: f32,
    pub(super) top: bool,
    pub(super) fact: Fact,
}

/// The entries of one name in a list of named strings, counter changes or running elements, by
/// page: each page with an entry, from the first, with its entries' indices in the list in list
/// order, and the latest index in the list of an entry on that page or an earlier one.
struct ByPage(Vec<(usize, Vec<usize>, usize)>);

impl ByPage {
    /// The pages of `entries`, each an index in the list and the page of that entry.
    fn new(entries: impl Iterator<Item = (usize, usize)>) -> Self {
        let mut all: Vec<(usize, usize)> = entries.map(|(i, page)| (page, i)).collect();
        all.sort_unstable();
        let mut pages: Vec<(usize, Vec<usize>, usize)> = Vec::new();
        for (page, i) in all {
            match pages.last_mut() {
                Some(last) if last.0 == page => {
                    last.1.push(i);
                    last.2 = last.2.max(i);
                }
                last => {
                    let latest = last.map_or(i, |last| last.2.max(i));
                    pages.push((page, vec![i], latest));
                }
            }
        }
        Self(pages)
    }

    /// The entries on the page at `page`, in list order.
    fn on(&self, page: usize) -> &[usize] {
        match self.0.binary_search_by_key(&page, |p| p.0) {
            Ok(k) => &self.0[k].1,
            Err(_) => &[],
        }
    }

    /// The latest entry in the list on a page before `page`, or up to it when `with` is true.
    fn before(&self, page: usize, with: bool) -> Option<usize> {
        let k = self
            .0
            .partition_point(|p| p.0 < page || (with && p.0 == page));
        k.checked_sub(1).map(|k| self.0[k].2)
    }
}

/// The named strings, counter changes and running elements of a document, indexed by name and
/// page once every page is laid out, so a margin box reads the entries of its own page and the
/// last one before it, and never the whole list. `reads` counts each lookup and each entry
/// indexed ([`super::Work::margin_reads`]). Memory: three numbers for each entry, until the
/// margin boxes are drawn.
pub(super) struct Index<'a> {
    strings: &'a [Set],
    running: &'a [Running],
    named: BTreeMap<&'a str, ByPage>,
    counters: BTreeMap<&'a str, ByPage>,
    elements: BTreeMap<&'a str, ByPage>,
    pub(super) reads: Cell<usize>,
}

impl<'a> Index<'a> {
    pub(super) fn new(running: &'a [Running], strings: &'a [Set]) -> Self {
        let by = |fact: Fact| {
            let mut names: BTreeMap<&'a str, Vec<(usize, usize)>> = BTreeMap::new();
            for (i, set) in strings.iter().enumerate().filter(|(_, s)| s.fact == fact) {
                names.entry(&set.name).or_default().push((i, set.page));
            }
            names
                .into_iter()
                .map(|(name, entries)| (name, ByPage::new(entries.into_iter())))
                .collect()
        };
        let mut elements: BTreeMap<&'a str, Vec<(usize, usize)>> = BTreeMap::new();
        for (i, one) in running.iter().enumerate() {
            elements.entry(&one.name).or_default().push((i, one.page));
        }
        Self {
            strings,
            running,
            named: by(Fact::String),
            counters: by(Fact::Counter),
            elements: elements
                .into_iter()
                .map(|(name, entries)| (name, ByPage::new(entries.into_iter())))
                .collect(),
            reads: Cell::new(strings.len() + running.len()),
        }
    }

    /// The entry of `list` that `which` takes on the page at `page`, from the entries `by`
    /// indexes. `top` says whether an entry is at its page's top.
    fn take<T>(
        &self,
        list: &'a [T],
        by: Option<&ByPage>,
        page: usize,
        which: Which,
        top: fn(&T) -> bool,
    ) -> Option<&'a T> {
        self.reads.set(self.reads.get() + 1);
        let by = by?;
        let on = by.on(page);
        let entry = || by.before(page, false).map(|i| &list[i]);
        let one = match which {
            Which::First => on.first(),
            Which::Last => on.last(),
            Which::Start => on.first().filter(|i| top(&list[**i])),
            Which::FirstExcept => return if on.is_empty() { entry() } else { None },
        };
        one.map(|i| &list[*i]).or_else(entry)
    }

    /// The value of the named string `name` that `which` takes on the page at `page`.
    fn string(&self, name: &str, page: usize, which: Which) -> Option<String> {
        let by = self.named.get(name);
        let one = self.take(self.strings, by, page, which, |s| s.top)?;
        Some(one.value.clone())
    }

    /// The value of the counter `name` a margin box reads on the page at `page`: the value after
    /// its first change on the page, else the value in force when the page starts, else 0.
    fn counter(&self, name: &str, page: usize) -> i64 {
        self.reads.set(self.reads.get() + 1);
        let Some(by) = self.counters.get(name) else {
            return 0;
        };
        by.on(page)
            .first()
            .copied()
            .or_else(|| by.before(page, true))
            .and_then(|i| self.strings[i].value.parse().ok())
            .unwrap_or(0)
    }

    /// The number the first `counter-reset: page` on the page at `page` restarts it at.
    fn restart(&self, page: usize) -> Option<i64> {
        self.reads.set(self.reads.get() + 1);
        let first = *self.counters.get("page")?.on(page).first()?;
        self.strings[first].value.parse().ok()
    }

    /// The running element named `name` that `which` takes on the page at `page`, as
    /// [`Index::string`] takes a named string.
    fn pick(&self, name: &str, page: usize, which: Which) -> Option<&'a Running> {
        let by = self.elements.get(name);
        self.take(self.running, by, page, which, |r| r.top)
    }
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

/// A margin box ready to lay out: its name, its page, its area's left edge, top, width and
/// height, where it sits in the area's height, what it draws, and the text an overflow lists.
pub(super) struct MarginBox {
    name: &'static str,
    page: usize,
    rect: [f32; 4],
    vertical: &'static str,
    content: Content,
    text: String,
}

/// What fills a margin box before it is placed: its text, or the running element it draws.
enum Fill<'r> {
    Text(String),
    Element(&'r Running),
}

/// Every margin box of `pages`, with the text style of each pushed onto `spans`.
pub(super) fn collect<'a>(
    pages: &[Page],
    sheet: &Sheet,
    root: &Style,
    fonts: &Fonts,
    found: &Index<'a>,
    spans: &mut Vec<Span>,
) -> Vec<MarginBox> {
    let count = pages.len() as i64;
    let mut out = Vec::new();
    let mut number = 0i64;
    for (index, page) in pages.iter().enumerate() {
        number = found
            .restart(index)
            .unwrap_or_else(|| number.saturating_add(1));
        let matched = sheet.matched(index, page.blank, page.name.as_deref());
        let named = |name: &str, which| found.string(name, index, which);
        let counted = |name: &str| found.counter(name, index);
        let fills: Vec<(&'static str, Fill)> = MARGIN_BOXES
            .iter()
            .filter_map(|name| {
                let values = matched.boxes.get(*name)?;
                let content = values.get("content").copied();
                if let Some((element, which)) = paginate::element(content) {
                    let one = found.pick(&element, index, which)?;
                    return Some((*name, Fill::Element(one)));
                }
                let text =
                    paginate::content(content, number, count, &counter_text, &named, &counted)?;
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
                        name,
                        page: index,
                        rect,
                        vertical,
                        content: Content::Element {
                            items: one.items.clone(),
                            size: one.size,
                            align,
                            missing: one.missing.clone(),
                        },
                        text: format!("element({})", one.name),
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
                name,
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
                            leaders: Vec::new(),
                            floats: Vec::new(),
                        }),
                        Some(Block::Close),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                ),
                text: text.clone(),
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
        let page = &mut pages[one.page];
        if size[1] > height + 0.01 {
            page.overflows.push((one.name, one.text));
        }
        let start = page.items.len();
        page.items
            .extend(items.into_iter().map(|item| shift(item, x + dx, y + dy)));
        page.boxes.push((one.name, start..page.items.len()));
    }
}

/// Paints each of `pages`' `@page` background, its `background-*` values, across the whole page
/// and under everything on it. A background image is loaded into `images` once, for every page
/// whose background values are the same, so a PDF letterhead is one form XObject in the file.
pub(super) fn backgrounds(
    pages: &mut [Page],
    sheet: &Sheet,
    assets: &Assets<'_>,
    images: &mut Images,
    rem: f32,
) -> Result<(), Error> {
    let mut loaded: Vec<(String, Option<Layer>)> = Vec::new();
    for (index, page) in pages.iter_mut().enumerate() {
        let matched = sheet.matched(index, page.blank, page.name.as_deref());
        let mut style = Style::default();
        for (property, value) in &matched.values {
            if property.starts_with("background") {
                style.values.insert(property, (*value).clone());
            }
        }
        if style.values.is_empty() {
            continue;
        }
        let key = format!("{:?}", style.values);
        let layer = match loaded.iter().find(|(each, _)| *each == key) {
            Some((_, layer)) => layer.clone(),
            None => {
                let layer = image::layer(&style, rem, &mut |reference| {
                    image::load_into(assets, images, reference)
                })?;
                loaded.push((key, layer.clone()));
                layer
            }
        };
        let mut g = geometry(&style, 0.0, page.geometry.width);
        g.decor.layer = layer;
        let painted = boxes::paint(&g, 0.0, page.geometry.height, [true; 4]);
        page.items.splice(0..0, painted);
    }
    Ok(())
}

/// Sets each of `pages`' named strings and marks from `found`'s list, which holds both. A mark
/// set past the last page is on the last page.
pub(super) fn facts(pages: &mut [Page], found: &Index<'_>) {
    let mut names: Vec<&str> = Vec::new();
    let mut seen = BTreeSet::new();
    for set in found.strings.iter().filter(|s| s.fact == Fact::String) {
        if seen.insert(set.name.as_str()) {
            names.push(&set.name);
        }
    }
    let last = pages.len().saturating_sub(1);
    let mut marks: Vec<Vec<(String, String)>> = vec![Vec::new(); pages.len()];
    for set in found.strings.iter().filter(|s| s.fact == Fact::Mark) {
        if let Some(on) = marks.get_mut(set.page.min(last)) {
            on.push((set.name.clone(), set.value.clone()));
        }
    }
    for ((index, page), marks) in pages.iter_mut().enumerate().zip(marks) {
        page.strings = names
            .iter()
            .filter_map(|name| {
                let value = found.string(name, index, Which::First)?;
                Some(((*name).to_owned(), value))
            })
            .collect();
        page.marks = marks;
    }
}

/// Draws the first page of `markup`, a document laid out on a page the size of `area`, over the
/// page at `page` of `laid`: at the area's left edge and top, clipped to the area. Every margin
/// box of that page whose name starts with `side` is taken off first. Returns whether `markup`
/// is taller than the area: it went on past its first page, or `below` finds its first page
/// drawing past the area's bottom, where the clip cuts it off.
pub fn overlay(laid: &mut Laid, page: usize, side: &str, area: [f32; 4], markup: Laid) -> bool {
    clear(&mut laid.pages[page], side);
    let [x, y, width, height] = area;
    let offset = laid.images.len();
    laid.images.extend(markup.images);
    carry(&mut laid.missing, markup.missing, page as u64 + 1);
    let mut over = markup.pages.len() > 1;
    let Some(first) = markup.pages.into_iter().next() else {
        return over;
    };
    over |= below(&first.items, height);
    let items = &mut laid.pages[page].items;
    items.push(Item::Begin {
        opacity: 1.0,
        clip: Some(rect(x, y, width, height)),
        transform: None,
    });
    items.extend(first.items.into_iter().map(|item| {
        let mut item = shift(item, x, y);
        if let Item::Image { image, .. } = &mut item {
            *image += offset;
        }
        item
    }));
    items.push(Item::End);
    over
}

/// Whether a run of text in `items` reaches more than a quarter point below `height`, from its
/// baseline down by its face's descent, or an image's bottom edge does. A single line taller
/// than its page stays on that page, so this finds what going on to a second page does not.
/// Items inside a transformed group are not measured, because their coordinates are not the
/// page's.
fn below(items: &[Item], height: f32) -> bool {
    let limit = height + 0.25;
    let mut groups: Vec<bool> = Vec::new();
    for item in items {
        let bottom = match item {
            Item::Begin { transform, .. } => {
                groups.push(transform.is_some());
                continue;
            }
            Item::End => {
                groups.pop();
                continue;
            }
            _ if groups.contains(&true) => continue,
            Item::Text { y, descent, .. } => y + descent,
            Item::Image { y, height, .. } => y + height,
            _ => continue,
        };
        if bottom > limit {
            return true;
        }
    }
    false
}

/// Takes the margin boxes whose name starts with `side` off `page`.
fn clear(page: &mut Page, side: &str) {
    let gone: Vec<Range<usize>> = page
        .boxes
        .iter()
        .filter(|(name, _)| name.starts_with(side))
        .map(|(_, range)| range.clone())
        .collect();
    if gone.is_empty() {
        return;
    }
    let mut at = 0;
    page.items.retain(|_| {
        at += 1;
        !gone.iter().any(|range| range.contains(&(at - 1)))
    });
    page.boxes.retain(|(name, _)| !name.starts_with(side));
    page.overflows.retain(|(name, _)| !name.starts_with(side));
    for (_, range) in &mut page.boxes {
        let before: usize = gone
            .iter()
            .filter(|g| g.end <= range.start)
            .map(ExactSizeIterator::len)
            .sum();
        *range = range.start - before..range.end - before;
    }
}
