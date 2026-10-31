//! The lines of each text block: shaped and broken by `parley`, aligned, hyphenated at a soft
//! hyphen, placed in the flow, and written out as glyph runs and decorations.

use std::borrow::Cow;
use std::ops::Range;

use parley::{
    AlignmentOptions, FontFamily, FontFamilyName, FontFeatures, FontStyle, FontWeight,
    IndentOptions, InlineBox, InlineBoxKind, Layout, LayoutContext, PositionedLayoutItem,
    RangedBuilder, StyleProperty, VerticalAlign, YieldData,
};

use super::block::Flow;
use super::inline::{Targets, blocks, is_bidi_control};
use super::margin;
use super::{Atom, Block, Glyph, Inline, Item, Laid, MARGIN, Missing, Paint, Para, Span, boxes};
use crate::Error;
use crate::assets::Assets;
use crate::css::{Color, Styles};
use crate::fonts::Fonts;
use crate::paginate::Sheet;
use crate::parse::Dom;

/// How many times a document whose `target-counter()` reads a page is laid out at most
/// (`rule:core-classes/pdf-roster`).
const PASSES: usize = 5;

/// How many boxes laid out on their own may be inside each other: table cells and flex and grid
/// items (`rule:core-classes/pdf-one-engine`). Nested inline-blocks are laid out innermost first,
/// so they do not count. Each level is one [`Cx::down`] on the component's stack, about 21 KiB
/// for a flex or grid container in a release build, so 32 levels leave more than half of the
/// 2 MiB stack
/// `extensions/pdf/.cargo/config.toml` gives the component. A document nested deeper returns
/// `Invalid` naming the bound.
pub const MAX_DEPTH: usize = 32;

/// Lays `dom` out in pages with its computed `styles` and `fonts`.
///
/// A document whose generated content reads a page with `target-counter()` is laid out again
/// with the pages each `id` landed on in the pass before, until no page changes. When the pages
/// still change after [`PASSES`] passes, this returns `Runtime`. Each pass costs one layout of
/// the whole document, and only one pass is kept.
pub fn layout(
    dom: &Dom,
    styles: &Styles,
    assets: &Assets<'_>,
    fonts: Fonts,
) -> Result<Laid, Error> {
    layout_keeping(dom, styles, assets, fonts).map(|(laid, _)| laid)
}

/// [`layout`], which also returns `fonts`, so another document can be laid out with the same
/// faces at the same indices.
pub fn layout_keeping(
    dom: &Dom,
    styles: &Styles,
    assets: &Assets<'_>,
    fonts: Fonts,
) -> Result<(Laid, Fonts), Error> {
    let mut fonts = fonts;
    let mut targets = Targets::new();
    for _ in 0..PASSES {
        let (laid, rest, landed, targeted) = pass(dom, styles, assets, fonts, &targets)?;
        if !targeted || landed == targets {
            return Ok((laid, rest));
        }
        fonts = rest;
        targets = landed;
    }
    Err(Error::Runtime(format!(
        "the page numbers target-counter() prints still changed after {PASSES} layout passes"
    )))
}

/// What one layout pass returns: the pages, the fonts to lay out the next pass with, the page
/// each `id` landed on, and whether generated content read a page from `targets`.
type Pass = (Laid, Fonts, Targets, bool);

/// One layout pass of `dom`, whose `target-counter()` prints the pages in `targets`.
fn pass(
    dom: &Dom,
    styles: &Styles,
    assets: &Assets<'_>,
    fonts: Fonts,
    targets: &Targets,
) -> Result<Pass, Error> {
    let sheet = Sheet::new(&styles.pages);
    let first = sheet.page(0, false, None);
    let (blocks, atoms, mut images, spans, targeted) = blocks(
        dom,
        styles,
        &fonts,
        assets,
        [first.content_width(), first.content_height()],
        &|name| sheet.page(0, false, Some(name)).content_width(),
        targets,
    )?;
    let mut cx = Cx {
        lcx: LayoutContext::new(),
        fonts,
        spans: &spans,
        missing: Vec::new(),
        placed: Vec::with_capacity(atoms.len()),
        waiting: Vec::with_capacity(atoms.len()),
        running: Vec::new(),
        strings: Vec::new(),
        depth: 0,
        deep: false,
    };
    // An inner inline-block comes before the one around it, so each one's inner boxes are
    // placed before it is laid out. A flex item waits for its container, which sizes it.
    for atom in atoms {
        if atom.item {
            cx.placed.push(Placed::empty());
            cx.waiting.push(Some(atom.blocks));
        } else {
            let placed = cx.atom(atom);
            cx.placed.push(placed);
            cx.waiting.push(None);
        }
    }
    let heights = sheet.clone();
    let mut flow = Flow::paged(Box::new(move |page, name| {
        heights.page(page, false, name).content_height()
    }));
    cx.run(blocks, &mut flow);
    if cx.deep {
        return Err(too_deep());
    }
    while !flow.open.is_empty() {
        flow.close();
    }
    flow.land();
    let landed = std::mem::take(&mut flow.landed);
    let tops = std::mem::take(&mut flow.tops);
    let mut bookmarks = std::mem::take(&mut flow.bookmarks);
    flow.finish([MARGIN, first.content_width(), 0.0], None);
    // Each page was laid out with its content box at `MARGIN`, `MARGIN`, and is moved to its own.
    for (index, page) in flow.pages.iter_mut().enumerate() {
        let geometry = sheet.page(index, page.blank, page.name.as_deref());
        let (dx, dy) = (geometry.margin[3] - MARGIN, geometry.margin[0] - MARGIN);
        if dx != 0.0 || dy != 0.0 {
            page.items = std::mem::take(&mut page.items)
                .into_iter()
                .map(|item| shift(item, dx, dy))
                .collect();
            for (_, area) in &mut page.slots {
                area[0] += dx;
                area[1] += dy;
            }
        }
        page.geometry = geometry;
    }
    // An `id` or a heading at the very end lands on the last page.
    let last = flow.pages.len() - 1;
    let on_page = |page: usize, top: f32| {
        let page = page.min(last);
        (page, flow.pages[page].geometry.margin[0] + top)
    };
    let anchors = tops
        .into_iter()
        .map(|(id, (page, top))| (id, on_page(page, top)))
        .collect();
    for bookmark in &mut bookmarks {
        (bookmark.page, bookmark.y) = on_page(bookmark.page, bookmark.y);
    }
    // The margin boxes come last, when the page count is known, each with a text style of its
    // own after the document's.
    let root = styles
        .elements
        .iter()
        .flatten()
        .next()
        .cloned()
        .unwrap_or_default();
    margin::backgrounds(
        &mut flow.pages,
        &sheet,
        assets,
        &mut images,
        root.font_size(),
    )?;
    let Cx {
        lcx,
        fonts,
        missing,
        running,
        strings,
        ..
    } = cx;
    let mut spans = spans;
    let boxes = margin::collect(
        &flow.pages,
        &sheet,
        &root,
        &fonts,
        (&running, &strings),
        &mut spans,
    );
    let mut cx = Cx {
        lcx,
        fonts,
        spans: &spans,
        missing,
        placed: Vec::new(),
        waiting: Vec::new(),
        running: Vec::new(),
        strings: Vec::new(),
        depth: 0,
        deep: false,
    };
    margin::draw(&mut cx, boxes, &mut flow.pages);
    if cx.deep {
        return Err(too_deep());
    }
    margin::facts(&mut flow.pages, &strings);
    let laid = Laid {
        pages: flow.pages,
        images,
        faces: cx.fonts.faces.clone(),
        missing: cx.missing,
        anchors,
        bookmarks,
    };
    Ok((laid, cx.fonts, landed, targeted))
}

/// What laying out every block shares: the shaping context, the fonts, the text styles, the
/// text no font has, each `inline-block` box laid out so far, and the blocks of each flex item
/// whose container is not laid out yet, both by their index in the list of atoms, and the
/// running elements and named strings met so far in document order, and how many boxes are
/// being laid out inside each other now, and whether one was past [`MAX_DEPTH`].
pub(super) struct Cx<'s> {
    lcx: LayoutContext<usize>,
    fonts: Fonts,
    spans: &'s [Span],
    pub(super) missing: Vec<Missing>,
    placed: Vec<Placed>,
    pub(super) waiting: Vec<Option<Vec<Block>>>,
    running: Vec<margin::Running>,
    pub(super) strings: Vec<margin::Set>,
    depth: usize,
    deep: bool,
}

/// The error of a document nested past [`MAX_DEPTH`].
fn too_deep() -> Error {
    Error::Invalid(format!(
        "the document has tables, flex containers or grid containers nested more than \
         {MAX_DEPTH} levels deep. A PDF lays out at most {MAX_DEPTH} levels."
    ))
}

/// An `inline-block` box laid out: its items with its margin box's top-left corner at 0, 0, the
/// margin box's width and height, its baseline from its top, `None` when it has no line, its
/// `vertical-align`, and the text no font has, whose page is set when the box is drawn.
#[derive(Clone)]
pub(super) struct Placed {
    pub(super) items: Vec<Item>,
    width: f32,
    pub(super) height: f32,
    baseline: Option<f32>,
    align: VerticalAlign,
    pub(super) missing: Vec<Missing>,
}

impl Placed {
    /// The place of a flex item in the list of laid-out `inline-block` boxes.
    pub(super) fn empty() -> Placed {
        Placed {
            items: Vec::new(),
            width: 0.0,
            height: 0.0,
            baseline: None,
            align: VerticalAlign::BASELINE,
            missing: Vec::new(),
        }
    }
}

impl Cx<'_> {
    /// Lays `atom`'s blocks out on a page of their own with no height limit. A box with no
    /// `width` first narrows to its widest line laid out unbroken.
    fn atom(&mut self, atom: Atom) -> Placed {
        let Atom {
            mut blocks, align, ..
        } = atom;
        if let Some(Block::Open(g)) = blocks.first()
            && g.fill
        {
            let left = g.x + g.border[3] + g.padding[3];
            let room = g.width - g.border[1] - g.border[3] - g.padding[1] - g.padding[3];
            let by = room - self.widest(&blocks, left, room, true).min(room);
            if by > 0.0 {
                narrow(&mut blocks, by);
            }
        }
        self.lay(blocks, align)
    }

    /// Lays the blocks of one box, from its opening to its close, out on a page of their own
    /// with no height limit, its margin box's top-left corner at 0, 0.
    pub(super) fn lay(&mut self, blocks: Vec<Block>, align: VerticalAlign) -> Placed {
        let width = match blocks.first() {
            Some(Block::Open(g)) => g.margin[3] + g.width + g.margin[1],
            _ => 0.0,
        };
        let (flow, missing) = self.down(blocks, Flow::new(f32::INFINITY));
        Placed {
            items: flow
                .pages
                .into_iter()
                .flat_map(|page| page.items)
                .map(|item| shift(item, 0.0, -MARGIN))
                .collect(),
            width,
            height: flow.cursor + flow.pending,
            baseline: flow.baseline,
            align,
            missing,
        }
    }

    /// Lays the blocks of one box, from its opening to its close, down `flow`, its margin box's
    /// top-left corner at the top-left corner of the first page's content. Returns the flow
    /// and the text no font has.
    pub(super) fn down(&mut self, mut blocks: Vec<Block>, mut flow: Flow) -> (Flow, Vec<Missing>) {
        let outside = std::mem::take(&mut self.missing);
        // The box's own close is the last block. Its edges are placed before its content and
        // after it, so no margin inside collapses through them.
        blocks.pop();
        let mut blocks = blocks.into_iter();
        // The box's padding box: its left edge, width, top and bottom border width.
        let mut padding_box = [0.0; 4];
        if let Some(Block::Open(g)) = blocks.next() {
            padding_box = [
                g.x + g.border[3],
                g.width - g.border[1] - g.border[3],
                g.margin[0] + g.border[0],
                g.border[2],
            ];
            flow.open(g);
            flow.place(0.0);
        }
        // Past the bound the box is laid out empty, and the pass returns the error.
        if self.depth < MAX_DEPTH {
            self.depth += 1;
            self.run(blocks, &mut flow);
            self.depth -= 1;
        } else {
            self.deep = true;
        }
        flow.contain();
        flow.place(0.0);
        while !flow.open.is_empty() {
            flow.close();
        }
        let [left, room, top, bottom] = padding_box;
        let bottom = flow.cursor - bottom;
        flow.finish([left, room, top], Some(bottom));
        let missing = std::mem::replace(&mut self.missing, outside);
        (flow, missing)
    }

    /// The width `blocks` need from the content box's left edge `left`: with no line broken but
    /// at a forced break when `max` is true, and with every line broken where it can be when it
    /// is false. A text block `room` wide is as wide as that content box.
    pub(super) fn widest(&mut self, blocks: &[Block], left: f32, room: f32, max: bool) -> f32 {
        let mut need: f32 = 0.0;
        for block in blocks {
            match block {
                Block::Text {
                    text,
                    spans: runs,
                    para,
                    width,
                    marker: false,
                    inline,
                    atoms,
                    ..
                } => {
                    let shaped = self.shape(text, runs, para, inline, atoms, &[]);
                    let widths = shaped.calculate_content_widths();
                    let content = if max { widths.max } else { widths.min };
                    need = need.max(content + room - width);
                }
                Block::Image { x, width, .. } => need = need.max(x + width - left),
                Block::Flex(flex) => need = need.max(flex.x + flex.width - left),
                Block::Table(table) => need = need.max(table.x + table.width - left),
                Block::Open(g) if !g.fill => need = need.max(g.x + g.width + g.margin[1] - left),
                _ => {}
            }
        }
        need
    }

    /// A text block's text shaped and not yet broken into lines. Each side of an inline box
    /// is an empty `parley` inline box as wide as its margin, border and padding, and each
    /// `inline-block` box one as big as its margin box. Each `leader()` is one as wide as
    /// `leaders` gives at its byte offset, after the boxes of the `inline-block`s.
    fn shape(
        &mut self,
        text: &str,
        runs: &[(Range<usize>, usize)],
        para: &Para,
        inline: &[Inline],
        atoms: &[(usize, usize)],
        leaders: &[(usize, f32)],
    ) -> Layout<usize> {
        let spans = self.spans;
        let mut builder = self
            .lcx
            .ranged_builder(&mut self.fonts.context, text, 1.0, false);
        builder.set_base_direction(para.direction);
        builder.push_default(para.line_height);
        builder.push_default(StyleProperty::WordBreak(para.word_break));
        builder.push_default(StyleProperty::OverflowWrap(para.overflow_wrap));
        builder.push_default(StyleProperty::TextWrapMode(para.wrap));
        // The root style is the block's own, which `middle`, `text-top` and `text-bottom`
        // measure against. A block's `vertical-align` moves nothing.
        for property in properties(&spans[para.span], para.span) {
            if !matches!(property, StyleProperty::VerticalAlign(_)) {
                builder.push_default(property);
            }
        }
        for (range, index) in runs {
            push_span(&mut builder, &spans[*index], *index, range.clone());
        }
        for (i, b) in inline.iter().enumerate() {
            let (m, g) = (&b.geometry.margin, &b.geometry);
            let sides = [
                (b.range.start, m[3] + g.border[3] + g.padding[3]),
                (b.range.end, g.padding[1] + g.border[1] + m[1]),
            ];
            for (side, (index, width)) in sides.into_iter().enumerate() {
                if width > 0.0 {
                    builder.push_inline_box(InlineBox {
                        id: (2 * i + side) as u64,
                        kind: InlineBoxKind::InFlow,
                        index,
                        width,
                        height: 0.0,
                        baseline: None,
                        vertical_align: VerticalAlign::BASELINE,
                    });
                }
            }
        }
        for (k, (index, atom)) in atoms.iter().enumerate() {
            let placed = &self.placed[*atom];
            builder.push_inline_box(InlineBox {
                id: (2 * inline.len() + k) as u64,
                kind: InlineBoxKind::InFlow,
                index: *index,
                width: placed.width,
                height: placed.height,
                baseline: placed.baseline,
                vertical_align: placed.align,
            });
        }
        for (k, (index, width)) in leaders.iter().enumerate() {
            builder.push_inline_box(InlineBox {
                id: (2 * inline.len() + atoms.len() + k) as u64,
                kind: InlineBoxKind::InFlow,
                index: *index,
                width: *width,
                height: 0.0,
                baseline: None,
                vertical_align: VerticalAlign::BASELINE,
            });
        }
        let mut shaped = builder.build(text);
        shaped.set_text_indent(para.indent, IndentOptions::default());
        shaped
    }

    /// How tall the box whose blocks are `blocks`, from its opening to its close, is from its
    /// top border edge to its bottom one, laid out on a page of its own. Every `inline-block`
    /// box, running element and named string it takes is put back, so laying it out again
    /// draws and sets them.
    fn measure(&mut self, blocks: Vec<Block>) -> f32 {
        let mut atoms = Vec::new();
        self.atoms_in(&blocks, &mut atoms);
        let saved: Vec<(usize, Placed, Option<Vec<Block>>)> = atoms
            .into_iter()
            .map(|a| (a, self.placed[a].clone(), self.waiting[a].clone()))
            .collect();
        let (running, strings) = (self.running.len(), self.strings.len());
        let margin = match blocks.first() {
            Some(Block::Open(g)) => g.margin[0] + g.margin[2],
            _ => 0.0,
        };
        let height = self.lay(blocks, VerticalAlign::BASELINE).height;
        self.running.truncate(running);
        self.strings.truncate(strings);
        for (a, placed, waiting) in saved {
            self.placed[a] = placed;
            self.waiting[a] = waiting;
        }
        height - margin
    }

    /// Pushes onto `out` the index of every atom `blocks` lay out, a flex item's own included.
    fn atoms_in(&self, blocks: &[Block], out: &mut Vec<usize>) {
        for block in blocks {
            match block {
                Block::Float { atom, .. }
                | Block::Absolute { atom, .. }
                | Block::Running { atom, .. } => out.push(*atom),
                Block::Text { atoms, .. } => out.extend(atoms.iter().map(|(_, a)| *a)),
                Block::Flex(flex) => {
                    for (atom, _) in &flex.items {
                        out.push(*atom);
                        if let Some(inner) = &self.waiting[*atom] {
                            self.atoms_in(inner, out);
                        }
                    }
                }
                Block::Table(table) => {
                    let cells = table
                        .rows
                        .iter()
                        .flat_map(|r| r.cells.iter().map(|c| c.atom));
                    for atom in cells.chain(table.captions.iter().map(|(a, _)| *a)) {
                        out.push(atom);
                        if let Some(inner) = &self.waiting[atom] {
                            self.atoms_in(inner, out);
                        }
                    }
                }
                Block::Open(_)
                | Block::Close
                | Block::Image { .. }
                | Block::String { .. }
                | Block::Target(_)
                | Block::Tag(_) => {}
            }
        }
    }

    /// Lays `blocks` out down `flow`.
    fn run(&mut self, blocks: impl IntoIterator<Item = Block>, flow: &mut Flow) {
        // The items of a list marker waiting for the line it is drawn beside.
        let mut marker: Option<Vec<Item>> = None;
        let mut blocks = blocks.into_iter();
        // The blocks of a box with `break-inside: avoid`, measured and waiting to be laid out,
        // last first.
        let mut ahead: Vec<Block> = Vec::new();
        while let Some(block) = ahead.pop().or_else(|| blocks.next()) {
            match block {
                Block::Open(geometry)
                    if geometry.avoid && geometry.breaks[0].is_none() && flow.paginates() =>
                {
                    let mut inner = vec![Block::Open(geometry.clone())];
                    let mut depth = 0;
                    while let Some(block) = ahead.pop().or_else(|| blocks.next()) {
                        let end = matches!(block, Block::Close) && depth == 0;
                        match block {
                            Block::Open(_) => depth += 1,
                            Block::Close => depth -= 1,
                            _ => {}
                        }
                        inner.push(block);
                        if end {
                            break;
                        }
                    }
                    let height = self.measure(inner.clone());
                    flow.keep(geometry.margin[0], height);
                    flow.open(geometry);
                    ahead.extend(inner.into_iter().skip(1).rev());
                }
                Block::Open(geometry) => flow.open(geometry),
                Block::Close => flow.close(),
                Block::Flex(flex) => self.flex(flex, flow),
                Block::Table(table) => self.table(table, flow),
                Block::Float {
                    atom,
                    x,
                    width,
                    right,
                    clear,
                } => {
                    let placed = &mut self.placed[atom];
                    let size = (placed.width, placed.height);
                    let items = std::mem::take(&mut placed.items);
                    let lost = std::mem::take(&mut placed.missing);
                    let (left, top) = flow.float(size, (x, width), right, clear);
                    let page_number = flow.pages.len() as u64;
                    flow.page().items.extend(
                        items
                            .into_iter()
                            .map(|item| shift(item, left, MARGIN + top)),
                    );
                    carry(&mut self.missing, lost, page_number);
                }
                Block::Absolute {
                    atom,
                    x,
                    insets,
                    z,
                    fixed,
                } => {
                    let placed = &mut self.placed[atom];
                    let size = (placed.width, placed.height);
                    let items = std::mem::take(&mut placed.items);
                    let lost = std::mem::take(&mut placed.missing);
                    flow.absolute(items, size, x, insets, z, fixed);
                    carry(&mut self.missing, lost, flow.pages.len() as u64);
                }
                Block::Running { atom, name } => {
                    let placed = &mut self.placed[atom];
                    self.running.push(margin::Running {
                        name,
                        page: flow.landing(),
                        items: std::mem::take(&mut placed.items),
                        size: [placed.width, placed.height],
                        missing: std::mem::take(&mut placed.missing),
                    });
                }
                Block::String { name, value, mark } => self.strings.push(margin::Set {
                    name,
                    value,
                    page: flow.landing(),
                    top: flow.at_top(),
                    mark,
                }),
                // An `id` in a box laid out on a page of its own is not on any page yet.
                Block::Target(anchor) => {
                    if flow.paginates() {
                        flow.target(anchor);
                    }
                }
                Block::Tag(role) => flow.page().items.push(match role {
                    Some(role) => Item::Tag(role),
                    None => Item::Untag,
                }),
                Block::Image {
                    image,
                    x,
                    width,
                    height,
                    draw: [dx, dy, w, h],
                } => {
                    let y = MARGIN + flow.place(height);
                    // A picture past any edge of its box is drawn in a group clipped to the box.
                    let outside =
                        dx < -0.01 || dy < -0.01 || dx + w > width + 0.01 || dy + h > height + 0.01;
                    let items = &mut flow.page().items;
                    if outside {
                        items.push(Item::Begin {
                            opacity: 1.0,
                            clip: Some(boxes::rect(x, y, width, height)),
                            transform: None,
                        });
                    }
                    items.push(Item::Image {
                        image,
                        x: x + dx,
                        y: y + dy,
                        width: w,
                        height: h,
                    });
                    if outside {
                        items.push(Item::End);
                    }
                }
                Block::Text {
                    text,
                    spans: runs,
                    para,
                    x: left,
                    width,
                    marker: is_marker,
                    inline,
                    atoms,
                    leaders,
                } => {
                    let spans = self.spans;
                    // The character each leader repeats, with its span's index.
                    let fills: Vec<Option<(Mark, usize)>> = leaders
                        .iter()
                        .map(|(_, index, fill)| {
                            mark(&mut self.lcx, &mut self.fonts, &spans[*index], *index, fill)
                                .map(|m| (m, *index))
                        })
                        .collect();
                    // The hyphen of each span a soft hyphen is in, by the span's index.
                    let mut hyphens: Vec<(usize, Mark)> = Vec::new();
                    for (range, index) in &runs {
                        if text[range.clone()].contains(SOFT_HYPHEN)
                            && !hyphens.iter().any(|(at, _)| at == index)
                            && let Some(hyphen) =
                                mark(&mut self.lcx, &mut self.fonts, &spans[*index], *index, "-")
                        {
                            hyphens.push((*index, hyphen));
                        }
                    }
                    // The ellipsis of a cut line, in the first run's style, with that run's index.
                    let ellipsis = runs
                        .first()
                        .filter(|_| para.ellipsis && !is_marker)
                        .and_then(|(_, index)| {
                            mark(
                                &mut self.lcx,
                                &mut self.fonts,
                                &spans[*index],
                                *index,
                                ELLIPSIS,
                            )
                            .map(|e| (e, *index))
                        });
                    let mut room: Vec<(usize, f32)> =
                        leaders.iter().map(|(at, ..)| (*at, 0.0)).collect();
                    let mut shaped = self.shape(&text, &runs, &para, &inline, &atoms, &room);
                    let widest = hyphens.iter().map(|(_, h)| h.advance).fold(0.0, f32::max);
                    // The left edge and width of each line beside a float. A line's room
                    // depends on its height and the lines above it, so the text is broken
                    // again until the lines keep the room they were broken in.
                    let mut bands: Vec<(f32, f32)> = Vec::new();
                    for pass in 0..4 {
                        break_lines(&mut shaped, &text, width, &bands, widest);
                        if is_marker || !flow.has_floats() || pass == 3 {
                            break;
                        }
                        let mut top = flow.next_top();
                        let next: Vec<(f32, f32)> = shaped
                            .lines()
                            .map(|line| {
                                let m = line.metrics();
                                let bottom = top + m.block_max_coord - m.block_min_coord;
                                let band = flow.band(top, bottom, left, width);
                                top = bottom;
                                band
                            })
                            .collect();
                        if next == bands {
                            break;
                        }
                        bands = next;
                    }
                    // Each leader takes the room left on its line, shared with the other
                    // leaders there, and the text is shaped and broken again with it.
                    if !leaders.is_empty() {
                        let first = 2 * inline.len() + atoms.len();
                        for (i, line) in shaped.lines().enumerate() {
                            let ids: Vec<usize> = line
                                .items()
                                .filter_map(|item| match item {
                                    PositionedLayoutItem::InlineBox(b) => {
                                        (b.id as usize).checked_sub(first)
                                    }
                                    PositionedLayoutItem::GlyphRun(_) => None,
                                })
                                .collect();
                            let m = line.metrics();
                            let line_width = bands.get(i).map_or(width, |b| b.1);
                            let free = line_width - (m.indent + m.advance - m.hanging_advance);
                            for k in &ids {
                                if let Some(r) = room.get_mut(*k) {
                                    r.1 = (free / ids.len() as f32 - 0.01).max(0.0);
                                }
                            }
                        }
                        shaped = self.shape(&text, &runs, &para, &inline, &atoms, &room);
                        break_lines(&mut shaped, &text, width, &bands, widest);
                    }
                    let Cx {
                        fonts,
                        missing,
                        placed,
                        ..
                    } = &mut *self;
                    shaped.align(para.align, AlignmentOptions::default());
                    let rtl = shaped.is_rtl();
                    let heights: Vec<f32> = shaped
                        .lines()
                        .map(|line| {
                            let m = line.metrics();
                            m.block_max_coord - m.block_min_coord
                        })
                        .collect();
                    // The line the next page starts with, chosen at the paragraph's first line
                    // and again at each break.
                    let mut next_break = None;
                    let mut plan = !is_marker;
                    for (i, line) in shaped.lines().enumerate() {
                        while plan || next_break == Some(i) {
                            if next_break == Some(i) {
                                flow.break_page();
                            }
                            next_break = flow
                                .room()
                                .and_then(|room| kept(&heights[i..], room, &para))
                                .map(|at| i + at);
                            plan = false;
                        }
                        let (left, width) = bands.get(i).copied().unwrap_or((left, width));
                        let metrics = line.metrics();
                        let height = heights[i];
                        // A marker is drawn on baseline 0 and moved down to the next line's.
                        let top = if is_marker { 0.0 } else { flow.place(height) };
                        let page_number = flow.pages.len() as u64;
                        let baseline = if is_marker {
                            0.0
                        } else {
                            MARGIN + top + (metrics.baseline - metrics.block_min_coord)
                        };
                        if !is_marker {
                            flow.baseline = Some(baseline - MARGIN);
                        }
                        // Each span's baseline on the page, which its `vertical-align` moves.
                        let shifted: Vec<(usize, f32)> = line
                            .items()
                            .filter_map(|item| match item {
                                PositionedLayoutItem::GlyphRun(run) => {
                                    Some((run.style().brush, run.baseline()))
                                }
                                PositionedLayoutItem::InlineBox(_) => None,
                            })
                            .collect();
                        let baseline_of = |index: usize| {
                            shifted
                                .iter()
                                .find(|(at, _)| *at == index)
                                .map_or(baseline, |(_, y)| baseline + y - metrics.baseline)
                        };
                        let line_range = line.text_range();
                        // A line of inline boxes alone has a range outside `text`, and no text.
                        let line_text = text.get(line_range.clone()).unwrap_or("");
                        // An outside marker of a right-to-left item starts at the content box's
                        // right edge.
                        let mut x = match (is_marker, rtl) {
                            (true, true) => left + width,
                            (true, false) => left - metrics.advance,
                            (false, _) => left + metrics.inline_min_coord + metrics.offset,
                        };
                        // The window of a line wider than its box, which leaves room for the
                        // ellipsis at the line's end: the right in a left-to-right paragraph.
                        let wide = metrics.indent + metrics.advance - metrics.hanging_advance;
                        let cut = (ellipsis.is_some() && wide > width + 0.01).then(|| {
                            let room = ellipsis.as_ref().map_or(0.0, |(e, _)| e.advance);
                            if rtl {
                                (left + room, left + width)
                            } else {
                                (left, left + width - room)
                            }
                        });
                        // The left and right of the clusters a cut line draws, and whether a cluster
                        // was left out in a left-to-right one.
                        let mut kept: Option<(f32, f32)> = None;
                        let mut past = false;
                        let mut items = Vec::new();
                        // The left edge and width of each inline box's space on the line, left to
                        // right, and how many of them the pen has passed.
                        let spaces: Vec<(f32, f32)> = line
                            .items()
                            .filter_map(|item| match item {
                                PositionedLayoutItem::InlineBox(b) => Some((left + b.x, b.width)),
                                PositionedLayoutItem::GlyphRun(_) => None,
                            })
                            .collect();
                        let mut passed = 0;
                        // The left, top, right and bottom of each inline box's text on the line.
                        let mut around: Vec<Option<[f32; 4]>> = vec![None; inline.len()];
                        for run in line.runs() {
                            let face = fonts
                                .face_index(run.font().data.data())
                                .expect("the font context holds only the render's faces");
                            let font_metrics = run.font_metrics();
                            // One item per span the run's clusters change between: the span, the
                            // pen's start, the glyphs and their advance.
                            let mut current: Option<(usize, f32, Vec<Glyph>, f32)> = None;
                            let mut gap: Option<(String, usize)> = None;
                            let finish =
                                |current: Option<(usize, f32, Vec<Glyph>, f32)>,
                                 items: &mut Vec<Item>| {
                                    let Some((index, start, glyphs, advance)) = current else {
                                        return;
                                    };
                                    let span = &spans[index];
                                    let baseline = baseline_of(index);
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
                                            descent: font_metrics.descent,
                                            x: start,
                                            y: baseline,
                                            color: span.color,
                                            glyphs,
                                            text: line_text.to_owned(),
                                        });
                                    }
                                    if let Some(href) = &span.link
                                        && advance > 0.0
                                    {
                                        items.push(Item::Link {
                                            href: href.clone(),
                                            x: start,
                                            y: baseline - font_metrics.ascent,
                                            width: advance,
                                            height: font_metrics.ascent + font_metrics.descent,
                                        });
                                    }
                                };
                            for cluster in run.visual_clusters() {
                                let index = cluster.style().brush;
                                if current.as_ref().is_some_and(|c| c.0 != index) {
                                    finish(current.take(), &mut items);
                                }
                                while let Some(&(at, room)) = spaces.get(passed)
                                    && at <= x + 0.01
                                {
                                    if current.is_some() {
                                        finish(current.take(), &mut items);
                                    }
                                    x += room;
                                    passed += 1;
                                }
                                let start_x = x;
                                x += cluster.advance();
                                // A cut line draws the clusters inside its window and no others.
                                if let Some((lo, hi)) = cut {
                                    let inside = if rtl {
                                        kept.is_some() || start_x >= lo - 0.01
                                    } else {
                                        !past && x <= hi + 0.01
                                    };
                                    if !inside {
                                        past = true;
                                        finish(current.take(), &mut items);
                                        continue;
                                    }
                                    kept = Some((kept.map_or(start_x, |k| k.0), x));
                                }
                                let at = cluster.text_range().start;
                                let base = baseline_of(index);
                                for (edges, b) in around.iter_mut().zip(&inline) {
                                    if b.range.contains(&at) {
                                        let [l, t, r, bottom] =
                                            edges.get_or_insert([start_x, f32::MAX, x, f32::MIN]);
                                        *l = l.min(start_x);
                                        *r = r.max(x);
                                        *t = t.min(base - font_metrics.ascent);
                                        *bottom = bottom.max(base + font_metrics.descent);
                                    }
                                }
                                let c = current
                                    .get_or_insert_with(|| (index, start_x, Vec::new(), 0.0));
                                let range = cluster.text_range();
                                let cluster_text = &text[range.clone()];
                                if cluster_text.chars().all(|c| {
                                    c.is_control() || is_bidi_control(c) || c == SOFT_HYPHEN
                                }) {
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
                                            report(missing, missed, &spans[at], page_number);
                                        }
                                        gap = Some((cluster_text.to_owned(), index));
                                    }
                                    (_, false) => {
                                        if let Some((missed, at)) = gap.take() {
                                            report(missing, missed, &spans[at], page_number);
                                        }
                                    }
                                }
                            }
                            finish(current.take(), &mut items);
                            if let Some((missed, at)) = gap.take() {
                                report(missing, missed, &spans[at], page_number);
                            }
                        }
                        x += spaces[passed.min(spaces.len())..]
                            .iter()
                            .map(|(_, room)| room)
                            .sum::<f32>();
                        // Each inline box is drawn under the line's text, around the part of its
                        // text on this line. Its left side is on the line its text starts on, and
                        // its right side on the line its text ends on.
                        let mut under = Vec::new();
                        for (edges, b) in around.iter().zip(&inline) {
                            let Some([l, t, r, bottom]) = *edges else {
                                continue;
                            };
                            let g = &b.geometry;
                            let first = line_range.contains(&b.range.start);
                            let last = b.range.end <= line_range.end;
                            let left = if first {
                                l - g.padding[3] - g.border[3]
                            } else {
                                l
                            };
                            let right = if last {
                                r + g.padding[1] + g.border[1]
                            } else {
                                r
                            };
                            let top = t - g.padding[0] - g.border[0];
                            let height = bottom + g.padding[2] + g.border[2] - top;
                            let mut slice = g.clone();
                            slice.x = left;
                            slice.width = right - left;
                            under.extend(boxes::paint(
                                &slice,
                                top,
                                height,
                                [true, last, true, first],
                            ));
                        }
                        items.splice(0..0, under);
                        // Each inline-block box on the line is drawn at its place there, and the
                        // text in it no font has is reported on this page.
                        for item in line.items() {
                            let PositionedLayoutItem::InlineBox(b) = item else {
                                continue;
                            };
                            // A leader's characters line up from the content box's left edge,
                            // so the leaders of lines one above the other line up too.
                            if let Some(Some((fill, index))) = (b.id as usize)
                                .checked_sub(2 * inline.len() + atoms.len())
                                .and_then(|k| fills.get(k))
                                && fill.advance > 0.0
                            {
                                let start = left + (b.x / fill.advance).ceil() * fill.advance;
                                let count =
                                    ((left + b.x + b.width - start) / fill.advance + 0.001).floor();
                                if count >= 1.0 {
                                    items.push(fill.repeated(
                                        start,
                                        baseline_of(*index),
                                        spans[*index].color,
                                        count as usize,
                                    ));
                                }
                                continue;
                            }
                            let Some(&(_, atom)) = (b.id as usize)
                                .checked_sub(2 * inline.len())
                                .and_then(|k| atoms.get(k))
                            else {
                                continue;
                            };
                            let atom = &mut placed[atom];
                            let (dx, dy) =
                                (left + b.x, MARGIN + top + b.y - metrics.block_min_coord);
                            items.extend(
                                std::mem::take(&mut atom.items)
                                    .into_iter()
                                    .map(|item| shift(item, dx, dy)),
                            );
                            carry(missing, std::mem::take(&mut atom.missing), page_number);
                        }
                        let last = line_range.end.saturating_sub(SOFT_HYPHEN.len_utf8());
                        if cut.is_none()
                            && line_text.ends_with(SOFT_HYPHEN)
                            && let Some((_, index)) = runs.iter().find(|(r, _)| r.contains(&last))
                            && let Some((_, h)) = hyphens.iter().find(|(at, _)| at == index)
                        {
                            items.push(h.item(x, baseline_of(*index), spans[*index].color));
                        }
                        if let (Some((lo, hi)), Some((e, index))) = (cut, &ellipsis) {
                            let at = if rtl {
                                kept.map_or(hi, |(start, _)| start) - e.advance
                            } else {
                                kept.map_or(lo, |(_, end)| end)
                            };
                            items.push(e.item(at, baseline_of(*index), spans[*index].color));
                        }
                        if is_marker {
                            marker = Some(items);
                            break;
                        }
                        if let Some(drawn) = marker.take() {
                            items.extend(drawn.into_iter().map(|item| shift(item, 0.0, baseline)));
                        }
                        flow.page().items.extend(items);
                    }
                }
            }
        }
    }
}

/// How many of the lines `heights` tall go on the last page before a break, or `None` when
/// they all fit. `room` is where the first line starts, how tall the page's content is and
/// whether the page is empty. A break leaves at least `orphans` lines before it and `widows`
/// after it. When it cannot, a paragraph on a page that is not empty starts on the next one,
/// and one at the top of a page breaks where the page ends.
fn kept(heights: &[f32], room: (f32, f32, bool), para: &Para) -> Option<usize> {
    let (mut y, limit, empty) = room;
    let mut fit = 0;
    // The same test as `Flow::place`, so the break falls where placing the lines puts it.
    while let Some(&height) = heights.get(fit) {
        if !(fit == 0 && empty) && y + height > limit {
            break;
        }
        y += height;
        fit += 1;
    }
    if fit == heights.len() {
        return None;
    }
    let at = fit.min(heights.len().saturating_sub(para.widows));
    Some(if at >= para.orphans.max(1) {
        at
    } else if empty {
        fit.max(1)
    } else {
        0
    })
}

/// Narrows by `by` an `inline-block` box's own box and each block in it whose width fills its
/// containing block, from the outside in, so a box with a `width` of its own keeps it and
/// everything in it.
pub(super) fn narrow(blocks: &mut [Block], by: f32) {
    // Whether each open box narrowed, innermost last.
    let mut narrowed: Vec<bool> = Vec::new();
    for block in blocks {
        let outer = narrowed.last().copied().unwrap_or(true);
        match block {
            Block::Open(g) => {
                let narrows = outer && g.fill;
                if narrows {
                    g.width -= by;
                }
                narrowed.push(narrows);
            }
            Block::Close => {
                narrowed.pop();
            }
            Block::Text {
                width,
                marker: false,
                ..
            } if outer => *width -= by,
            Block::Flex(flex) if outer => flex.width -= by,
            Block::Table(table) if outer => table.width -= by,
            _ => {}
        }
    }
}

/// Adds each of `lost` to `missing` on `page`, once.
pub(super) fn carry(missing: &mut Vec<Missing>, lost: Vec<Missing>, page: u64) {
    for lost in lost {
        let entry = Missing { page, ..lost };
        if !missing.contains(&entry) {
            missing.push(entry);
        }
    }
}

const SOFT_HYPHEN: char = '\u{ad}';

/// `span`'s text style as `parley` properties, with `index` as its brush.
fn properties(span: &Span, index: usize) -> Vec<StyleProperty<'_, usize>> {
    let chain: Vec<FontFamilyName<'_>> = span
        .chain
        .iter()
        .map(|name| FontFamilyName::Named(Cow::Borrowed(name.as_str())))
        .collect();
    let mut out = vec![
        StyleProperty::FontFamily(FontFamily::List(Cow::Owned(chain))),
        StyleProperty::FontSize(span.size),
        StyleProperty::FontWeight(FontWeight::new(span.weight)),
    ];
    if span.italic {
        out.push(StyleProperty::FontStyle(FontStyle::Italic));
    }
    if span.letter_spacing != 0.0 {
        out.push(StyleProperty::LetterSpacing(span.letter_spacing));
    }
    if span.word_spacing != 0.0 {
        out.push(StyleProperty::WordSpacing(span.word_spacing));
    }
    if let Some(features) = &span.features {
        out.push(StyleProperty::FontFeatures(FontFeatures::Source(
            Cow::Borrowed(features.as_str()),
        )));
    }
    if span.align != VerticalAlign::BASELINE {
        out.push(StyleProperty::VerticalAlign(span.align));
    }
    out.push(StyleProperty::Brush(index));
    out
}

/// Pushes `span`'s text style onto `range` of the text, with `index` as its brush.
fn push_span(
    builder: &mut RangedBuilder<'_, usize>,
    span: &Span,
    index: usize,
    range: Range<usize>,
) {
    for property in properties(span, index) {
        builder.push(property, range.clone());
    }
}

const ELLIPSIS: &str = "\u{2026}";

/// One character drawn beside a line's text: the hyphen of a line broken at a soft hyphen, the
/// ellipsis of a line cut at its box, or the character a `leader()` repeats. Its text, face,
/// size, descent, glyph and advance.
struct Mark {
    text: String,
    face: usize,
    size: f32,
    descent: f32,
    id: u32,
    advance: f32,
}

impl Mark {
    /// The mark drawn with its pen at `x` on the baseline `y`.
    fn item(&self, x: f32, y: f32, color: Color) -> Item {
        self.repeated(x, y, color, 1)
    }

    /// The mark drawn `count` times one after another, the first with its pen at `x` on the
    /// baseline `y`.
    fn repeated(&self, x: f32, y: f32, color: Color, count: usize) -> Item {
        let len = self.text.len();
        Item::Text {
            face: self.face,
            size: self.size,
            descent: self.descent,
            x,
            y,
            color,
            glyphs: (0..count)
                .map(|i| Glyph {
                    id: self.id,
                    text: i * len..(i + 1) * len,
                    advance: self.advance,
                    x: 0.0,
                    y: 0.0,
                })
                .collect(),
            text: self.text.repeat(count),
        }
    }
}

/// `text`, one character, in `span` shaped through its font chain.
fn mark(
    lcx: &mut LayoutContext<usize>,
    fonts: &mut Fonts,
    span: &Span,
    index: usize,
    text: &str,
) -> Option<Mark> {
    let mut builder = lcx.ranged_builder(&mut fonts.context, text, 1.0, false);
    push_span(&mut builder, span, index, 0..text.len());
    let mut shaped = builder.build(text);
    shaped.break_all_lines(None);
    let line = shaped.lines().next()?;
    let run = line.runs().next()?;
    let face = fonts.face_index(run.font().data.data())?;
    let glyph = run.visual_clusters().next()?.glyphs().next()?;
    Some(Mark {
        text: text.to_owned(),
        face,
        size: run.font_size(),
        descent: run.font_metrics().descent,
        id: glyph.id,
        advance: glyph.advance,
    })
}

/// Breaks `shaped` into lines `width` wide, or as wide as `bands` gives a line beside a float.
/// A line that ends at a soft hyphen is broken again `hyphen` narrower, and is aligned in that
/// width, so the hyphen drawn after it stays in the box. The lines before the one narrowed do
/// not change, so each pass narrows a later line.
fn break_lines(
    shaped: &mut Layout<usize>,
    text: &str,
    width: f32,
    bands: &[(f32, f32)],
    hyphen: f32,
) {
    let mut narrow: Vec<usize> = Vec::new();
    loop {
        let mut breaker = shaped.break_lines();
        breaker.state_mut().set_layout_max_advance(width);
        let mut line = 0;
        loop {
            let room = bands.get(line).map_or(width, |b| b.1);
            let max = if narrow.contains(&line) {
                room - hyphen
            } else {
                room
            };
            breaker.state_mut().set_line_max_advance(max);
            match breaker.break_next() {
                None => break,
                Some(YieldData::LineBreak(_)) => line += 1,
                Some(_) => {}
            }
        }
        breaker.finish();
        let next = shaped.lines().enumerate().find(|(i, l)| {
            // A line of inline boxes alone has a text range outside `text`.
            !narrow.contains(i)
                && text
                    .get(l.text_range())
                    .is_some_and(|t| t.ends_with(SOFT_HYPHEN))
        });
        match next {
            Some((i, _)) if hyphen > 0.0 => narrow.push(i),
            _ => return,
        }
    }
}

/// `item` moved right by `dx` and down by `dy`.
pub(super) fn shift(mut item: Item, dx: f32, dy: f32) -> Item {
    match &mut item {
        Item::Text { x, y, .. } | Item::Image { x, y, .. } | Item::Link { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        Item::Path { path, paint, .. } => {
            boxes::shift(path, dx, dy);
            match paint {
                Paint::Color(_) => {}
                Paint::Linear { x1, y1, x2, y2, .. } => {
                    *x1 += dx;
                    *y1 += dy;
                    *x2 += dx;
                    *y2 += dy;
                }
                Paint::Radial { cx: x, cy: y, .. } | Paint::Tiles { x, y, .. } => {
                    *x += dx;
                    *y += dy;
                }
            }
        }
        Item::Begin {
            clip, transform, ..
        } => {
            if let Some(clip) = clip {
                boxes::shift(clip, dx, dy);
            }
            if let Some(m) = transform {
                *m = super::transform::shift(*m, dx, dy);
            }
        }
        Item::End | Item::Tag(_) | Item::Untag => {}
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
