//! The `text` step and the `text` source: Latin text set in a font given as bytes
//! (`rule:core-classes/image-one-entry-point-per-job`, ADR 0120 § 9).
//!
//! The font is a TrueType or OpenType file, parsed by `ab_glyph` when the plan is checked, so
//! bytes it cannot read return `parse` before anything is decoded. There is no system font. Each
//! character is the glyph the font's `cmap` maps it to, and a character the font has no glyph
//! for draws the font's missing glyph; control characters other than a line feed draw nothing.
//! Glyphs follow one another by their advance widths, adjusted for each pair by the pair
//! adjustments of the `kern` feature in the font's `GPOS` table, read through `ttf-parser`, or by
//! its `kern` table when it has no `GPOS`. Every other shaping rule, ligatures and complex
//! scripts included, is not applied, so text is Latin until shaping lands.
//!
//! `size` is the font's em in pixels, as a CSS `font-size` is. A line is the font's ascent plus
//! its descent high, and lines are a line height plus the font's line gap apart. A line feed
//! starts a new line. With `max_width`, a line breaks at the last space that keeps it within
//! `max_width` pixels, the space is not drawn, and a word wider than `max_width` alone is a line
//! of its own and is not broken. `align` (default `Left`) places each line within the widest.
//!
//! The text's box is the widest line's advance by the lines' height, each rounded up to a whole
//! pixel; ink a glyph draws outside its advance is clipped to the box. Text with nothing to draw
//! in it returns `invalid`, and a box over the pixel cap returns `runtime` before it is
//! allocated. The `text` source is the box, transparent, with the text in opaque black. The
//! `text` step draws the text in its `color` over the frame as `composite` draws an overlay: at
//! `x` and `y`, the box's top-left corner, or at `gravity` (default `Center`) on an axis with
//! neither.
//!
//! Memory: the layout holds one glyph per character and the font's pair subtables, and the step
//! holds one RGBA8 frame of the box beside the frame it draws on. All are freed when the step
//! returns.

use ab_glyph::{Font as _, FontRef, GlyphId, PxScale, PxScaleFont, ScaleFont as _, point};
use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};

use crate::decode::check as within_cap;
use crate::ops::{self, Blend, Composite, Gravity};
use crate::{Error, Pixels};

/// Where a line goes within the widest, the WIT `align`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// The text and how it is set: what the `text` source and the `text` step share.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout<'a> {
    pub text: &'a str,
    /// A TrueType or OpenType file.
    pub font: &'a [u8],
    /// The em in pixels.
    pub size: f64,
    pub max_width: Option<u64>,
    pub align: Align,
}

/// A `text` step: the layout, its colour as straight-alpha RGBA8, and where the box goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Text<'a> {
    pub layout: Layout<'a>,
    pub color: [u8; 4],
    pub gravity: Gravity,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

/// One laid-out line: its glyphs with their pen positions from the line's left edge, and its
/// advance width.
struct Line {
    glyphs: Vec<(GlyphId, f32)>,
    width: f32,
}

/// The lines of a layout and the vertical metrics that place them.
struct Set {
    lines: Vec<Line>,
    ascent: f32,
    /// From one baseline to the next.
    step: f32,
    width: f32,
    height: f32,
}

/// The font at a scale, and the pair subtables of each `GPOS` lookup its `kern` feature names,
/// in lookup order.
struct Pen<'f> {
    scaled: PxScaleFont<&'f FontRef<'f>>,
    pairs: Vec<Vec<PairAdjustment<'f>>>,
}

impl<'f> Pen<'f> {
    fn new(font: &'f FontRef<'f>, data: &'f [u8], scale: PxScale) -> Self {
        Self {
            scaled: font.as_scaled(scale),
            pairs: pairs(data),
        }
    }

    /// The pen moves by this between `first` and `second`, in pixels. Each lookup adds the
    /// adjustment of its first subtable that covers `first`.
    fn kern(&self, first: GlyphId, second: GlyphId) -> f32 {
        if self.pairs.is_empty() {
            return self.scaled.kern(first, second);
        }
        let (a, b) = (ttf_parser::GlyphId(first.0), ttf_parser::GlyphId(second.0));
        let units: i32 = self
            .pairs
            .iter()
            .filter_map(|lookup| lookup.iter().find_map(|pair| adjustment(pair, a, b)))
            .map(i32::from)
            .sum();
        units as f32 * self.scaled.h_scale_factor()
    }
}

/// The pair subtables of the `kern` feature's lookups in `data`'s `GPOS`, or none.
fn pairs(data: &[u8]) -> Vec<Vec<PairAdjustment<'_>>> {
    let Some(gpos) = ttf_parser::Face::parse(data, 0)
        .ok()
        .and_then(|face| face.tables().gpos)
    else {
        return Vec::new();
    };
    let kern = ttf_parser::Tag::from_bytes(b"kern");
    let mut lookups: Vec<u16> = gpos
        .features
        .into_iter()
        .filter(|feature| feature.tag == kern)
        .flat_map(|feature| feature.lookup_indices)
        .collect();
    lookups.sort_unstable();
    lookups.dedup();
    lookups
        .into_iter()
        .filter_map(|index| gpos.lookups.get(index))
        .map(|lookup| {
            lookup
                .subtables
                .into_iter::<PositioningSubtable<'_>>()
                .filter_map(|subtable| match subtable {
                    PositioningSubtable::Pair(pair) => Some(pair),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .filter(|lookup| !lookup.is_empty())
        .collect()
}

/// The advance adjustment `pair` gives `first` before `second`, in font units, when it covers
/// `first`.
fn adjustment(
    pair: &PairAdjustment<'_>,
    first: ttf_parser::GlyphId,
    second: ttf_parser::GlyphId,
) -> Option<i16> {
    match pair {
        PairAdjustment::Format1 { coverage, sets } => {
            let set = sets.get(coverage.get(first)?)?;
            Some(set.get(second).map_or(0, |(value, _)| value.x_advance))
        }
        PairAdjustment::Format2 {
            coverage,
            classes,
            matrix,
        } => {
            if !coverage.contains(first) {
                return None;
            }
            let classes = (classes.0.get(first), classes.1.get(second));
            Some(matrix.get(classes).map_or(0, |(value, _)| value.x_advance))
        }
    }
}

/// `data` as a font, or `Parse` when it is not one `ab_glyph` reads.
fn font(data: &[u8]) -> Result<FontRef<'_>, Error> {
    FontRef::try_from_slice(data).map_err(|err| {
        Error::Parse(format!(
            "{} bytes are not a TrueType or OpenType font: {err}",
            data.len()
        ))
    })
}

/// `Invalid` when `layout`'s options are out of range, and `Parse` when its font does not parse.
pub fn check(layout: &Layout<'_>) -> Result<(), Error> {
    if !(layout.size.is_finite() && layout.size > 0.0) {
        return Err(Error::Invalid(format!(
            "`text` needs a finite `size` above 0, and was given {}",
            layout.size
        )));
    }
    if layout.max_width == Some(0) {
        return Err(Error::Invalid(
            "`text` needs a `maxWidth` above 0".to_owned(),
        ));
    }
    font(layout.font).map(|_| ())
}

/// The pen advance after `s` is set from `width`, with `last` the glyph before it: each glyph's
/// advance plus the `kern` adjustment from the one before. Returns the new width and last glyph.
fn extend(
    pen: &Pen<'_>,
    mut width: f32,
    mut last: Option<GlyphId>,
    s: &str,
    mut out: Option<&mut Vec<(GlyphId, f32)>>,
) -> (f32, Option<GlyphId>) {
    for c in s.chars().filter(|c| !c.is_control()) {
        let id = pen.scaled.glyph_id(c);
        if let Some(before) = last {
            width += pen.kern(before, id);
        }
        if let Some(out) = out.as_deref_mut() {
            out.push((id, width));
        }
        width += pen.scaled.h_advance(id);
        last = Some(id);
    }
    (width, last)
}

/// `paragraph` broken into lines of at most `max` pixels at its spaces, each line's byte range.
fn wrap(scaled: &Pen<'_>, paragraph: &str, max: f32) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut line: Option<(usize, usize, f32, Option<GlyphId>)> = None;
    let mut at = 0;
    for word in paragraph.split(' ') {
        let (start, end) = (at, at + word.len());
        at = end + 1;
        line = Some(match line {
            None => {
                let (width, last) = extend(scaled, 0.0, None, word, None);
                (start, end, width, last)
            }
            Some((from, to, width, last)) => {
                let (spaced, after) = extend(scaled, width, last, " ", None);
                let (joined, after) = extend(scaled, spaced, after, word, None);
                if joined <= max {
                    (from, end, joined, after)
                } else {
                    lines.push((from, to));
                    let (width, last) = extend(scaled, 0.0, None, word, None);
                    (start, end, width, last)
                }
            }
        });
    }
    if let Some((from, to, ..)) = line {
        lines.push((from, to));
    }
    lines
}

/// `layout` set in `font`, and the pen that set it.
fn set<'f>(font: &'f FontRef<'f>, layout: &Layout<'f>) -> (Pen<'f>, Set) {
    let em = font.units_per_em().unwrap_or(1000.0);
    let scale = PxScale::from(layout.size as f32 * font.height_unscaled() / em);
    let pen = Pen::new(font, layout.font, scale);
    let scaled = &pen;
    let mut lines = Vec::new();
    for paragraph in layout.text.split('\n') {
        let paragraph = paragraph.strip_suffix('\r').unwrap_or(paragraph);
        let ranges = match layout.max_width {
            Some(max) => wrap(scaled, paragraph, max as f32),
            None => vec![(0, paragraph.len())],
        };
        for (from, to) in ranges {
            let mut glyphs = Vec::new();
            let (width, _) = extend(scaled, 0.0, None, &paragraph[from..to], Some(&mut glyphs));
            lines.push(Line { glyphs, width });
        }
    }
    let ascent = pen.scaled.ascent();
    let tall = ascent - pen.scaled.descent();
    let step = tall + pen.scaled.line_gap();
    let width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
    let height = tall + step * lines.len().saturating_sub(1) as f32;
    let set = Set {
        lines,
        ascent,
        step,
        width,
        height,
    };
    (pen, set)
}

/// The box of `set` in whole pixels, or `Invalid` when it has no pixel.
fn size(set: &Set, text: &str) -> Result<(u64, u64), Error> {
    let (width, height) = (set.width.ceil() as u64, set.height.ceil() as u64);
    if width == 0 || height == 0 {
        return Err(Error::Invalid(format!(
            "the text {text:?} has nothing to draw"
        )));
    }
    Ok((width, height))
}

/// The width and height of `layout`'s box, as `render` would make it.
pub fn measure(layout: &Layout<'_>) -> Result<(u64, u64), Error> {
    check(layout)?;
    let font = font(layout.font)?;
    let (_, set) = set(&font, layout);
    size(&set, layout.text)
}

/// `layout`'s box, transparent, with the text drawn in `color`. A box over `cap` pixels returns
/// `Runtime` before it is allocated.
pub fn render(layout: &Layout<'_>, color: [u8; 4], cap: u64) -> Result<Pixels, Error> {
    check(layout)?;
    let font = font(layout.font)?;
    let (pen, set) = set(&font, layout);
    let (width, height) = size(&set, layout.text)?;
    within_cap(width, height, cap)?;
    let (w, h) = (width as usize, height as usize);
    let mut rgba = [color[0], color[1], color[2], 0].repeat(w * h);
    let share = match layout.align {
        Align::Left => 0.0,
        Align::Center => 0.5,
        Align::Right => 1.0,
    };
    for (row, line) in set.lines.iter().enumerate() {
        let left = (set.width - line.width) * share;
        let baseline = set.ascent + set.step * row as f32;
        for &(id, at) in &line.glyphs {
            let glyph = id.with_scale_and_position(pen.scaled.scale, point(left + at, baseline));
            let Some(outlined) = font.outline_glyph(glyph) else {
                continue;
            };
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let x = bounds.min.x as i64 + i64::from(gx);
                let y = bounds.min.y as i64 + i64::from(gy);
                if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
                    return;
                }
                let alpha = &mut rgba[(y as usize * w + x as usize) * 4 + 3];
                let ink = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
                *alpha = alpha.saturating_add(ink);
            });
        }
    }
    if color[3] != 255 {
        for px in rgba.chunks_exact_mut(4) {
            px[3] = ((u16::from(px[3]) * u16::from(color[3]) + 127) / 255) as u8;
        }
    }
    Ok(Pixels {
        width: u32::try_from(width).expect("the pixel cap holds the box to a u32"),
        height: u32::try_from(height).expect("the pixel cap holds the box to a u32"),
        rgba,
        exif: None,
    })
}

/// Draws `text` over `pixels` as the module doc says. The box is held to `cap`.
pub fn draw(pixels: &mut Pixels, text: Text<'_>, cap: u64) -> Result<(), Error> {
    let layer = render(&text.layout, text.color, cap)?;
    ops::composite(
        pixels,
        &layer,
        Composite {
            overlay: 0,
            gravity: text.gravity,
            x: text.x,
            y: text.y,
            opacity: 1.0,
            blend: Blend::Normal,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../fixtures/DancingScript-Regular.ttf");

    fn layout(text: &str) -> Layout<'_> {
        Layout {
            text,
            font: FONT,
            size: 32.0,
            max_width: None,
            align: Align::Left,
        }
    }

    fn inked(frame: &Pixels) -> usize {
        frame.rgba.chunks_exact(4).filter(|px| px[3] > 0).count()
    }

    #[test]
    fn the_source_box_is_the_measured_size_and_has_ink() {
        let shop = layout("Shop");
        let (width, height) = measure(&shop).unwrap();
        let frame = render(&shop, [0, 0, 0, 255], 1 << 20).unwrap();
        assert_eq!(
            (u64::from(frame.width), u64::from(frame.height)),
            (width, height)
        );
        assert!(width > 32 && height >= 32);
        assert!(inked(&frame) > 50);
    }

    #[test]
    fn gpos_kerning_moves_a_pair_closer() {
        let font = font(FONT).unwrap();
        let pen = Pen::new(&font, FONT, PxScale::from(32.0));
        assert!(!pen.pairs.is_empty());
        let letters = ('A'..='Z').chain('a'..='z');
        let (first, second) = letters
            .clone()
            .flat_map(|a| letters.clone().map(move |b| (a, b)))
            .find(|&(a, b)| pen.kern(pen.scaled.glyph_id(a), pen.scaled.glyph_id(b)) < 0.0)
            .expect("the font kerns some pair of Latin letters closer");
        let (a, b) = (pen.scaled.glyph_id(first), pen.scaled.glyph_id(second));
        let (pair, _) = extend(&pen, 0.0, None, &format!("{first}{second}"), None);
        assert!(pair < pen.scaled.h_advance(a) + pen.scaled.h_advance(b));
    }

    #[test]
    fn max_width_breaks_at_a_space_and_never_inside_a_word() {
        let one = measure(&layout("Shop")).unwrap();
        let wrapped = Layout {
            max_width: Some(one.0 + 4),
            ..layout("Shop Shop Shop")
        };
        let (width, height) = measure(&wrapped).unwrap();
        assert_eq!(width, one.0);
        assert!(height > one.1 * 2);
        let narrow = Layout {
            max_width: Some(1),
            ..layout("Shop")
        };
        assert_eq!(measure(&narrow).unwrap(), one);
    }

    #[test]
    fn a_line_feed_starts_a_line() {
        let one = measure(&layout("Blog")).unwrap();
        let two = measure(&layout("Blog\nBlog")).unwrap();
        assert_eq!(two.0, one.0);
        assert!(two.1 > one.1);
    }

    #[test]
    fn right_align_moves_a_short_line_to_the_right_edge() {
        let right = Layout {
            align: Align::Right,
            ..layout("Shop and Blog\ni")
        };
        let frame = render(&right, [0, 0, 0, 255], 1 << 20).unwrap();
        let width = frame.width as usize;
        let last = frame.height as usize - 1;
        // The bottom row is the descent of the second line, so its "i" draws nothing there; the
        // row above the descent is inked only near the right edge.
        let row = last - (last / 4);
        let inked: Vec<usize> = (0..width)
            .filter(|&x| frame.rgba[(row * width + x) * 4 + 3] > 0)
            .collect();
        assert!(!inked.is_empty() && inked[0] > width / 2);
    }

    #[test]
    fn the_step_draws_its_colour_at_x_and_y() {
        let mut canvas = Pixels {
            width: 200,
            height: 80,
            rgba: [255, 255, 255, 255].repeat(200 * 80),
            exif: None,
        };
        let text = Text {
            layout: layout("Shop"),
            color: [255, 0, 0, 255],
            gravity: Gravity::Center,
            x: Some(0),
            y: Some(0),
        };
        draw(&mut canvas, text, 1 << 20).unwrap();
        let red = canvas
            .rgba
            .chunks_exact(4)
            .filter(|px| px[0] == 255 && px[1] == 0 && px[2] == 0)
            .count();
        assert!(red > 20);
        let (width, _) = measure(&text.layout).unwrap();
        let right = (width as usize + 2)..200;
        assert!((0..80).all(|y| {
            right
                .clone()
                .all(|x| canvas.rgba[(y * 200 + x) * 4 + 1] == 255)
        }));
    }

    #[test]
    fn bytes_that_are_no_font_return_parse_and_bad_options_invalid() {
        let junk = Layout {
            font: b"\x00\x01\x00\x00not a font at all",
            ..layout("Shop")
        };
        assert!(matches!(check(&junk), Err(Error::Parse(_))));
        let zero = Layout {
            size: 0.0,
            ..layout("Shop")
        };
        assert!(matches!(check(&zero), Err(Error::Invalid(_))));
        assert!(matches!(measure(&layout("")), Err(Error::Invalid(_))));
    }

    #[test]
    fn a_box_over_the_cap_returns_runtime_before_it_is_allocated() {
        let huge = Layout {
            size: 100_000.0,
            ..layout("Shop")
        };
        assert!(matches!(
            render(&huge, [0; 4], 1 << 20),
            Err(Error::Runtime(_))
        ));
    }
}
