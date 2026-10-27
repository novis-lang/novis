//! The `text` step and the `text` source: text shaped in a font given as bytes
//! (`rule:core-classes/image-one-entry-point-per-job`).
//!
//! The font is a TrueType or OpenType file, parsed by `ab_glyph` and by `read-fonts` when the
//! plan is checked, so bytes either cannot read return `parse` before anything is decoded. There
//! is no system font. Control characters other than a line feed, and the paragraph separator
//! U+2029, draw nothing.
//!
//! Each line is split into runs of one direction by the Unicode bidirectional algorithm
//! (`unicode-bidi`), in the order they are drawn, and each run is shaped by `harfrust` in the
//! font's `GSUB` and `GPOS` tables: ligatures, the joined forms of Arabic, Devanagari's conjuncts
//! and the `kern` feature all apply. A line's direction is that of its first strong letter, so a
//! line of Arabic or Hebrew reads right to left. A run is shaped as the script of its first letter
//! that has one. A character the font has no glyph for draws the font's missing glyph.
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
//! Cost: wrapping shapes a line again for each word it tries, so a paragraph of `n` words under
//! `max_width` is shaped about `n` times its line length. Memory: the layout holds one glyph per
//! shaped glyph and the font's shaping tables, and the step holds one RGBA8 frame of the box
//! beside the frame it draws on. All are freed when the step returns.

use std::ops::Range;

use ab_glyph::{Font as _, FontRef, GlyphId, PxScale, ScaleFont as _, point};
use harfrust::{Direction, ShapeOptions, Shaper, ShaperData, UnicodeBuffer};
use unicode_bidi::BidiInfo;

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

/// One shaped glyph: its pen position from the line's left edge, and how far above the baseline
/// it is raised, both in pixels.
#[derive(Debug, Clone, Copy)]
struct Glyph {
    id: GlyphId,
    x: f32,
    rise: f32,
}

/// One laid-out line: its glyphs in the order they are drawn, and its advance width.
struct Line {
    glyphs: Vec<Glyph>,
    width: f32,
}

/// The lines of a layout and the vertical metrics that place them.
struct Set {
    lines: Vec<Line>,
    ascent: f32,
    scale: PxScale,
    /// From one baseline to the next.
    step: f32,
    width: f32,
    height: f32,
}

/// The shaper and the pixels one of its font units is.
struct Pen<'f> {
    shaper: Shaper<'f>,
    per_unit: f32,
}

impl Pen<'_> {
    /// `text[range]` shaped into one line, its runs in the order `bidi` draws them.
    fn line(&self, bidi: &BidiInfo<'_>, range: Range<usize>) -> Line {
        let mut glyphs = Vec::new();
        let mut pen = 0.0;
        if range.is_empty() {
            return Line { glyphs, width: pen };
        }
        let (levels, runs) = bidi.visual_runs(&bidi.paragraphs[0], range);
        for run in runs {
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(&bidi.text[run.clone()]);
            buffer.set_direction(if levels[run.start].is_rtl() {
                Direction::RightToLeft
            } else {
                Direction::LeftToRight
            });
            buffer.guess_segment_properties();
            let shaped = self.shaper.shape(buffer, ShapeOptions::new());
            for (info, at) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                glyphs.push(Glyph {
                    id: GlyphId(u16::try_from(info.glyph_id).unwrap_or(0)),
                    x: pen + at.x_offset as f32 * self.per_unit,
                    rise: at.y_offset as f32 * self.per_unit,
                });
                pen += at.x_advance as f32 * self.per_unit;
            }
        }
        Line { glyphs, width: pen }
    }
}

/// The error for `data` when a font reader rejects it.
fn not_a_font(data: &[u8], err: impl std::fmt::Display) -> Error {
    Error::Parse(format!(
        "{} bytes are not a TrueType or OpenType font: {err}",
        data.len()
    ))
}

/// `data` as a font, or `Parse` when it is not one `ab_glyph` and `read-fonts` both read.
fn font(data: &[u8]) -> Result<FontRef<'_>, Error> {
    harfrust::FontRef::new(data).map_err(|err| not_a_font(data, err))?;
    FontRef::try_from_slice(data).map_err(|err| not_a_font(data, err))
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

/// `paragraph`, the whole of `bidi`'s text, broken into lines of at most `max` pixels at its
/// spaces, each line's byte range.
fn wrap(pen: &Pen<'_>, bidi: &BidiInfo<'_>, max: f32) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut line: Option<Range<usize>> = None;
    let mut at = 0;
    for word in bidi.text.split(' ') {
        let (start, end) = (at, at + word.len());
        at = end + 1;
        line = Some(match line {
            Some(open) if pen.line(bidi, open.start..end).width <= max => open.start..end,
            Some(open) => {
                lines.push(open);
                start..end
            }
            None => start..end,
        });
    }
    lines.extend(line);
    lines
}

/// `layout` set in `font`, whose bytes are `layout.font`.
fn set(font: &FontRef<'_>, layout: &Layout<'_>) -> Result<Set, Error> {
    let shaping =
        harfrust::FontRef::new(layout.font).map_err(|err| not_a_font(layout.font, err))?;
    let data = ShaperData::new(&shaping);
    let shaper = data.shaper(&shaping).build();
    let per_unit = layout.size as f32 / shaper.units_per_em() as f32;
    let pen = Pen { shaper, per_unit };
    let em = font.units_per_em().unwrap_or(1000.0);
    let scale = PxScale::from(layout.size as f32 * font.height_unscaled() / em);
    let mut lines = Vec::new();
    for paragraph in layout.text.split('\n') {
        let clean: String = paragraph
            .chars()
            .filter(|&c| !c.is_control() && c != '\u{2029}')
            .collect();
        let bidi = BidiInfo::new(&clean, None);
        let ranges = match layout.max_width {
            Some(max) => wrap(&pen, &bidi, max as f32),
            None => vec![0..clean.len()],
        };
        lines.extend(ranges.into_iter().map(|range| pen.line(&bidi, range)));
    }
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let tall = ascent - scaled.descent();
    let step = tall + scaled.line_gap();
    let width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
    let height = tall + step * lines.len().saturating_sub(1) as f32;
    Ok(Set {
        lines,
        ascent,
        scale,
        step,
        width,
        height,
    })
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
    size(&set(&font, layout)?, layout.text)
}

/// `layout`'s box, transparent, with the text drawn in `color`. A box over `cap` pixels returns
/// `Runtime` before it is allocated.
pub fn render(layout: &Layout<'_>, color: [u8; 4], cap: u64) -> Result<Pixels, Error> {
    check(layout)?;
    let font = font(layout.font)?;
    let set = set(&font, layout)?;
    let (width, height) = size(&set, layout.text)?;
    within_cap(width, height, 1, cap)?;
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
        for glyph in &line.glyphs {
            let placed = glyph
                .id
                .with_scale_and_position(set.scale, point(left + glyph.x, baseline - glyph.rise));
            let Some(outlined) = font.outline_glyph(placed) else {
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

    /// The advance of `text` set alone in one line.
    fn advance(text: &str) -> f32 {
        let font = font(FONT).unwrap();
        let set = set(&font, &layout(text)).unwrap();
        set.width
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
    fn the_shaper_kerns_a_pair_closer() {
        let letters = ('A'..='Z').chain('a'..='z');
        let kerned = letters
            .clone()
            .flat_map(|a| letters.clone().map(move |b| (a, b)))
            .find(|&(a, b)| {
                advance(&format!("{a}{b}")) < advance(&a.to_string()) + advance(&b.to_string())
            });
        assert!(
            kerned.is_some(),
            "the font kerns some pair of Latin letters closer"
        );
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
