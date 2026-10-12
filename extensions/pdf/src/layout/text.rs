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
use super::inline::{blocks, is_bidi_control};
use super::{
    Atom, Block, CONTENT_HEIGHT, Glyph, Inline, Item, Laid, MARGIN, Missing, Paint, Para, Span,
    boxes,
};
use crate::Error;
use crate::assets::Assets;
use crate::css::{Color, Styles};
use crate::fonts::Fonts;
use crate::parse::Dom;

/// Lays `dom` out in pages with its computed `styles` and `fonts`.
pub fn layout(
    dom: &Dom,
    styles: &Styles,
    assets: &Assets<'_>,
    fonts: Fonts,
) -> Result<Laid, Error> {
    let (blocks, atoms, images, spans) = blocks(dom, styles, &fonts, assets)?;
    let mut cx = Cx {
        lcx: LayoutContext::new(),
        fonts,
        spans: &spans,
        missing: Vec::new(),
        placed: Vec::with_capacity(atoms.len()),
    };
    // An inner inline-block comes before the one around it, so each one's inner boxes are
    // placed before it is laid out.
    for atom in atoms {
        let placed = cx.atom(atom);
        cx.placed.push(placed);
    }
    let mut flow = Flow::new(CONTENT_HEIGHT);
    cx.run(blocks, &mut flow);
    while !flow.open.is_empty() {
        flow.close();
    }
    Ok(Laid {
        pages: flow.pages,
        images,
        faces: cx.fonts.faces,
        missing: cx.missing,
    })
}

/// What laying out every block shares: the shaping context, the fonts, the text styles, the
/// text no font has, and each `inline-block` box laid out so far.
struct Cx<'s> {
    lcx: LayoutContext<usize>,
    fonts: Fonts,
    spans: &'s [Span],
    missing: Vec<Missing>,
    placed: Vec<Placed>,
}

/// An `inline-block` box laid out: its items with its margin box's top-left corner at 0, 0, the
/// margin box's width and height, its baseline from its top, `None` when it has no line, its
/// `vertical-align`, and the text no font has, whose page is set when the box is drawn.
struct Placed {
    items: Vec<Item>,
    width: f32,
    height: f32,
    baseline: Option<f32>,
    align: VerticalAlign,
    missing: Vec<Missing>,
}

impl Cx<'_> {
    /// Lays `atom`'s blocks out on a page of their own with no height limit. A box with no
    /// `width` first narrows to its widest line laid out unbroken.
    fn atom(&mut self, atom: Atom) -> Placed {
        let Atom { mut blocks, align } = atom;
        if let Some(Block::Open(g)) = blocks.first()
            && g.fill
        {
            let left = g.x + g.border[3] + g.padding[3];
            let room = g.width - g.border[1] - g.border[3] - g.padding[1] - g.padding[3];
            let by = room - self.widest(&blocks, left, room).min(room);
            if by > 0.0 {
                narrow(&mut blocks, by);
            }
        }
        let width = match blocks.first() {
            Some(Block::Open(g)) => g.margin[3] + g.width + g.margin[1],
            _ => 0.0,
        };
        let outside = std::mem::take(&mut self.missing);
        let mut flow = Flow::new(f32::INFINITY);
        // The box's own close is the last block. Its edges are placed before its content and
        // after it, so no margin inside collapses through them.
        blocks.pop();
        let mut blocks = blocks.into_iter();
        if let Some(Block::Open(g)) = blocks.next() {
            flow.open(g);
            flow.place(0.0);
        }
        self.run(blocks, &mut flow);
        flow.place(0.0);
        while !flow.open.is_empty() {
            flow.close();
        }
        let missing = std::mem::replace(&mut self.missing, outside);
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

    /// The width `blocks` need with no line broken but at a forced break, from the content
    /// box's left edge `left`. A text block `room` wide is as wide as that content box.
    fn widest(&mut self, blocks: &[Block], left: f32, room: f32) -> f32 {
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
                    let shaped = self.shape(text, runs, para, inline, atoms);
                    need = need.max(shaped.calculate_content_widths().max + room - width);
                }
                Block::Image { x, width, .. } => need = need.max(x + width - left),
                Block::Open(g) if !g.fill => need = need.max(g.x + g.width + g.margin[1] - left),
                _ => {}
            }
        }
        need
    }

    /// A text block's text shaped and not yet broken into lines. Each side of an inline box
    /// is an empty `parley` inline box as wide as its margin, border and padding, and each
    /// `inline-block` box one as big as its margin box.
    fn shape(
        &mut self,
        text: &str,
        runs: &[(Range<usize>, usize)],
        para: &Para,
        inline: &[Inline],
        atoms: &[(usize, usize)],
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
        let mut shaped = builder.build(text);
        shaped.set_text_indent(para.indent, IndentOptions::default());
        shaped
    }

    /// Lays `blocks` out down `flow`.
    fn run(&mut self, blocks: impl IntoIterator<Item = Block>, flow: &mut Flow) {
        // The items of a list marker waiting for the line it is drawn beside.
        let mut marker: Option<Vec<Item>> = None;
        for block in blocks {
            match block {
                Block::Open(geometry) => flow.open(geometry),
                Block::Close => flow.close(),
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
                } => {
                    let spans = self.spans;
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
                    let mut shaped = self.shape(&text, &runs, &para, &inline, &atoms);
                    let Cx {
                        fonts,
                        missing,
                        placed,
                        ..
                    } = &mut *self;
                    let widest = hyphens.iter().map(|(_, h)| h.advance).fold(0.0, f32::max);
                    break_lines(&mut shaped, &text, width, widest);
                    shaped.align(para.align, AlignmentOptions::default());
                    let rtl = shaped.is_rtl();
                    for line in shaped.lines() {
                        let metrics = line.metrics();
                        let height = metrics.block_max_coord - metrics.block_min_coord;
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
                        let line_text = &text[line_range.clone()];
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
                                            x: start,
                                            y: baseline,
                                            color: span.color,
                                            glyphs,
                                            text: line_text.to_owned(),
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
                            for lost in std::mem::take(&mut atom.missing) {
                                let entry = Missing {
                                    page: page_number,
                                    ..lost
                                };
                                if !missing.contains(&entry) {
                                    missing.push(entry);
                                }
                            }
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

/// Narrows by `by` an `inline-block` box's own box and each block in it whose width fills its
/// containing block, from the outside in, so a box with a `width` of its own keeps it and
/// everything in it.
fn narrow(blocks: &mut [Block], by: f32) {
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
            _ => {}
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

/// One character drawn after a line's text: the hyphen of a line broken at a soft hyphen, or
/// the ellipsis of a line cut at its box. Its text, face, size, glyph and advance.
struct Mark {
    text: &'static str,
    face: usize,
    size: f32,
    id: u32,
    advance: f32,
}

impl Mark {
    /// The mark drawn with its pen at `x` on the baseline `y`.
    fn item(&self, x: f32, y: f32, color: Color) -> Item {
        Item::Text {
            face: self.face,
            size: self.size,
            x,
            y,
            color,
            glyphs: vec![Glyph {
                id: self.id,
                text: 0..self.text.len(),
                advance: self.advance,
                x: 0.0,
                y: 0.0,
            }],
            text: self.text.to_owned(),
        }
    }
}

/// `text`, one character, in `span` shaped through its font chain.
fn mark(
    lcx: &mut LayoutContext<usize>,
    fonts: &mut Fonts,
    span: &Span,
    index: usize,
    text: &'static str,
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
        text,
        face,
        size: run.font_size(),
        id: glyph.id,
        advance: glyph.advance,
    })
}

/// Breaks `shaped` into lines `width` wide. A line that ends at a soft hyphen is broken again
/// `hyphen` narrower, and is aligned in that width, so the hyphen drawn after it stays in the
/// box. The lines before the one narrowed do not change, so each pass narrows a later line.
fn break_lines(shaped: &mut Layout<usize>, text: &str, width: f32, hyphen: f32) {
    let mut narrow: Vec<usize> = Vec::new();
    loop {
        let mut breaker = shaped.break_lines();
        breaker.state_mut().set_layout_max_advance(width);
        let mut line = 0;
        loop {
            let max = if narrow.contains(&line) {
                width - hyphen
            } else {
                width
            };
            breaker.state_mut().set_line_max_advance(max);
            match breaker.break_next() {
                None => break,
                Some(YieldData::LineBreak(_)) => line += 1,
                Some(_) => {}
            }
        }
        breaker.finish();
        let next = shaped
            .lines()
            .enumerate()
            .find(|(i, l)| !narrow.contains(i) && text[l.text_range()].ends_with(SOFT_HYPHEN));
        match next {
            Some((i, _)) if hyphen > 0.0 => narrow.push(i),
            _ => return,
        }
    }
}

/// `item` moved right by `dx` and down by `dy`.
fn shift(mut item: Item, dx: f32, dy: f32) -> Item {
    match &mut item {
        Item::Text { x, y, .. } | Item::Image { x, y, .. } => {
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
        Item::Begin { clip, .. } => {
            if let Some(clip) = clip {
                boxes::shift(clip, dx, dy);
            }
        }
        Item::End => {}
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
