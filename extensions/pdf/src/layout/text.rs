//! The lines of each text block: shaped and broken by `parley`, aligned, hyphenated at a soft
//! hyphen, placed in the flow, and written out as glyph runs and decorations.

use std::borrow::Cow;
use std::ops::Range;

use parley::{
    AlignmentOptions, FontFamily, FontFamilyName, FontFeatures, FontStyle, FontWeight,
    IndentOptions, Layout, LayoutContext, PositionedLayoutItem, RangedBuilder, StyleProperty,
    VerticalAlign, YieldData,
};

use super::block::Flow;
use super::inline::{blocks, is_bidi_control};
use super::{Block, Glyph, Item, Laid, MARGIN, Missing, Page, Paint, Span, boxes};
use crate::Error;
use crate::assets::Assets;
use crate::css::Styles;
use crate::fonts::Fonts;
use crate::parse::Dom;

/// Lays `dom` out in pages with its computed `styles` and `fonts`.
pub fn layout(
    dom: &Dom,
    styles: &Styles,
    assets: &Assets<'_>,
    mut fonts: Fonts,
) -> Result<Laid, Error> {
    let (blocks, images, spans) = blocks(dom, styles, &fonts, assets)?;
    let mut lcx: LayoutContext<usize> = LayoutContext::new();
    let mut flow = Flow {
        pages: vec![Page::default()],
        cursor: 0.0,
        pending: 0.0,
        open: Vec::new(),
    };
    let mut missing: Vec<Missing> = Vec::new();
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
            } => {
                // The hyphen of each span a soft hyphen is in, by the span's index.
                let mut hyphens: Vec<(usize, Hyphen)> = Vec::new();
                for (range, index) in &runs {
                    if text[range.clone()].contains(SOFT_HYPHEN)
                        && !hyphens.iter().any(|(at, _)| at == index)
                        && let Some(hyphen) = hyphen(&mut lcx, &mut fonts, &spans[*index], *index)
                    {
                        hyphens.push((*index, hyphen));
                    }
                }
                let mut builder = lcx.ranged_builder(&mut fonts.context, &text, 1.0, false);
                builder.set_base_direction(para.direction);
                builder.push_default(para.line_height);
                builder.push_default(StyleProperty::WordBreak(para.word_break));
                builder.push_default(StyleProperty::OverflowWrap(para.overflow_wrap));
                builder.push_default(StyleProperty::TextWrapMode(para.wrap));
                for (range, index) in &runs {
                    push_span(&mut builder, &spans[*index], *index, range.clone());
                }
                let mut shaped = builder.build(&text);
                shaped.set_text_indent(para.indent, IndentOptions::default());
                let widest = hyphens.iter().map(|(_, h)| h.advance).fold(0.0, f32::max);
                break_lines(&mut shaped, &text, width, widest);
                shaped.align(para.align, AlignmentOptions::default());
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
                    let mut x = if is_marker {
                        left - metrics.advance
                    } else {
                        left + metrics.inline_min_coord + metrics.offset
                    };
                    let mut items = Vec::new();
                    for run in line.runs() {
                        let face = fonts
                            .face_index(run.font().data.data())
                            .expect("the font context holds only the render's faces");
                        let font_metrics = run.font_metrics();
                        // One item per span the run's clusters change between: the span, the
                        // pen's start, the glyphs and their advance.
                        let mut current: Option<(usize, f32, Vec<Glyph>, f32)> = None;
                        let mut gap: Option<(String, usize)> = None;
                        let finish = |current: Option<(usize, f32, Vec<Glyph>, f32)>,
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
                            let c = current.get_or_insert_with(|| (index, x, Vec::new(), 0.0));
                            let range = cluster.text_range();
                            let cluster_text = &text[range.clone()];
                            x += cluster.advance();
                            if cluster_text
                                .chars()
                                .all(|c| c.is_control() || is_bidi_control(c) || c == SOFT_HYPHEN)
                            {
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
                                        report(&mut missing, missed, &spans[at], page_number);
                                    }
                                    gap = Some((cluster_text.to_owned(), index));
                                }
                                (_, false) => {
                                    if let Some((missed, at)) = gap.take() {
                                        report(&mut missing, missed, &spans[at], page_number);
                                    }
                                }
                            }
                        }
                        finish(current.take(), &mut items);
                        if let Some((missed, at)) = gap.take() {
                            report(&mut missing, missed, &spans[at], page_number);
                        }
                    }
                    let last = line_range.end.saturating_sub(SOFT_HYPHEN.len_utf8());
                    if line_text.ends_with(SOFT_HYPHEN)
                        && let Some((_, index)) = runs.iter().find(|(r, _)| r.contains(&last))
                        && let Some((_, h)) = hyphens.iter().find(|(at, _)| at == index)
                    {
                        items.push(Item::Text {
                            face: h.face,
                            size: h.size,
                            x,
                            y: baseline_of(*index),
                            color: spans[*index].color,
                            glyphs: vec![Glyph {
                                id: h.id,
                                text: 0..1,
                                advance: h.advance,
                                x: 0.0,
                                y: 0.0,
                            }],
                            text: "-".to_owned(),
                        });
                    }
                    if is_marker {
                        marker = Some(items);
                        break;
                    }
                    if let Some(drawn) = marker.take() {
                        items.extend(drawn.into_iter().map(|item| lower(item, baseline)));
                    }
                    flow.page().items.extend(items);
                }
            }
        }
    }
    while !flow.open.is_empty() {
        flow.close();
    }
    Ok(Laid {
        pages: flow.pages,
        images,
        faces: fonts.faces,
        missing,
    })
}

const SOFT_HYPHEN: char = '\u{ad}';

/// Pushes `span`'s text style onto `range` of the text, with `index` as its brush.
fn push_span(
    builder: &mut RangedBuilder<'_, usize>,
    span: &Span,
    index: usize,
    range: Range<usize>,
) {
    let chain: Vec<FontFamilyName<'_>> = span
        .chain
        .iter()
        .map(|name| FontFamilyName::Named(Cow::Borrowed(name.as_str())))
        .collect();
    builder.push(
        StyleProperty::FontFamily(FontFamily::List(Cow::Owned(chain))),
        range.clone(),
    );
    builder.push(StyleProperty::FontSize(span.size), range.clone());
    builder.push(
        StyleProperty::FontWeight(FontWeight::new(span.weight)),
        range.clone(),
    );
    if span.italic {
        builder.push(StyleProperty::FontStyle(FontStyle::Italic), range.clone());
    }
    if span.letter_spacing != 0.0 {
        builder.push(
            StyleProperty::LetterSpacing(span.letter_spacing),
            range.clone(),
        );
    }
    if span.word_spacing != 0.0 {
        builder.push(StyleProperty::WordSpacing(span.word_spacing), range.clone());
    }
    if let Some(features) = &span.features {
        builder.push(
            StyleProperty::FontFeatures(FontFeatures::Source(Cow::Borrowed(features.as_str()))),
            range.clone(),
        );
    }
    if span.align != VerticalAlign::BASELINE {
        builder.push(StyleProperty::VerticalAlign(span.align), range.clone());
    }
    builder.push(StyleProperty::Brush(index), range);
}

/// The hyphen drawn at the end of a line broken at a soft hyphen: its face, size, glyph and
/// advance.
struct Hyphen {
    face: usize,
    size: f32,
    id: u32,
    advance: f32,
}

/// The hyphen of `span`, shaped through its font chain.
fn hyphen(
    lcx: &mut LayoutContext<usize>,
    fonts: &mut Fonts,
    span: &Span,
    index: usize,
) -> Option<Hyphen> {
    let mut builder = lcx.ranged_builder(&mut fonts.context, "-", 1.0, false);
    push_span(&mut builder, span, index, 0..1);
    let mut shaped = builder.build("-");
    shaped.break_all_lines(None);
    let line = shaped.lines().next()?;
    let run = line.runs().next()?;
    let face = fonts.face_index(run.font().data.data())?;
    let glyph = run.visual_clusters().next()?.glyphs().next()?;
    Some(Hyphen {
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

/// `item` moved down by `by`.
fn lower(mut item: Item, by: f32) -> Item {
    match &mut item {
        Item::Text { y, .. } | Item::Image { y, .. } => *y += by,
        Item::Path { path, paint, .. } => {
            boxes::lower(path, by);
            match paint {
                Paint::Color(_) => {}
                Paint::Linear { y1, y2, .. } => {
                    *y1 += by;
                    *y2 += by;
                }
                Paint::Radial { cy: y, .. } | Paint::Tiles { y, .. } => *y += by,
            }
        }
        Item::Begin { clip, .. } => {
            if let Some(clip) = clip {
                boxes::lower(clip, by);
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
