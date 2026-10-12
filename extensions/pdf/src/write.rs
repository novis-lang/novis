//! The writer: laid-out pages as a PDF file, by `krilla` (`rule:core-classes/pdf-output-is-inert`).
//!
//! Each page is A4, and each glyph run is drawn in its colour and its face, a default or an
//! `@font-face` font, which `krilla` subsets to the glyphs the document draws. Each path is
//! filled with the even-odd rule or stroked, in a colour, a gradient or an image's tiles, and each
//! image is drawn at its box. A group is a clip path, an opacity or both, pushed at its start and
//! popped at its end.
//!
//! The file is the same bytes for the same input. Nothing here reads a clock: the creation
//! time is written only when the program passes one, in UTC. The file's `/ID` and its XMP
//! document and instance ids are `krilla`'s defaults, a SipHash of the serialized file with
//! fixed keys, so they come from the content and nothing else. Every collection the writer
//! walks is a `Vec` in document order. `krilla` writes no JavaScript, launch action, open
//! action, form field or attachment unless asked, and this writer asks for none.
//!
//! Every document is written with `krilla`'s default settings. Tagging, the PDF/A and PDF/UA
//! standards, the password and the invoice attachment are Stage 12 and Stage 10's
//! (`render`'s doc in the crate root says what each option does until then).

use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point, Size, Transform};
use krilla::metadata::{DateTime, Metadata};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{
    Fill, FillRule, LineCap, LinearGradient, Pattern, RadialGradient, SpreadMethod, Stop, Stroke,
    StrokeDash,
};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use krilla::{Data, Document, SerializeSettings};

use crate::css::Color;
use crate::layout::{self, Item, Laid, Paint, Seg};
use crate::{Error, Instant};

fn fill(surface: &mut Surface<'_>, color: Color) {
    surface.set_fill(Some(Fill {
        paint: rgb_of(color).into(),
        opacity: normal(color.a),
        ..Fill::default()
    }));
}

fn rgb_of(color: Color) -> krilla::color::Color {
    rgb::Color::new(color.r, color.g, color.b).into()
}

fn normal(value: f32) -> NormalizedF32 {
    NormalizedF32::new(value.clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE)
}

/// `path` as a `krilla` path, or `None` when it has no area or no length.
fn shape(path: &[Seg]) -> Option<Path> {
    let mut builder = PathBuilder::new();
    for seg in path {
        match *seg {
            Seg::Move(x, y) => builder.move_to(x, y),
            Seg::Line(x, y) => builder.line_to(x, y),
            Seg::Cubic(x1, y1, x2, y2, x, y) => builder.cubic_to(x1, y1, x2, y2, x, y),
            Seg::Close => builder.close(),
        }
    }
    builder.finish()
}

/// `paint` as a `krilla` paint and the opacity it is drawn at. A gradient's stops carry their
/// own opacity, and an image's tile is drawn once into a pattern.
fn paint_of(
    surface: &mut Surface<'_>,
    paint: &Paint,
    laid: &Laid,
) -> Option<(krilla::paint::Paint, NormalizedF32)> {
    let stops = |stops: &[layout::Stop]| -> Vec<Stop> {
        stops
            .iter()
            .map(|s| Stop {
                offset: normal(s.offset),
                color: rgb_of(s.color),
                opacity: normal(s.color.a),
            })
            .collect()
    };
    Some(match paint {
        Paint::Color(color) => (rgb_of(*color).into(), normal(color.a)),
        Paint::Linear {
            x1,
            y1,
            x2,
            y2,
            stops: each,
        } => (
            LinearGradient {
                x1: *x1,
                y1: *y1,
                x2: *x2,
                y2: *y2,
                transform: Transform::identity(),
                spread_method: SpreadMethod::Pad,
                stops: stops(each),
                anti_alias: false,
            }
            .into(),
            NormalizedF32::ONE,
        ),
        Paint::Radial {
            cx,
            cy,
            r,
            scale_y,
            stops: each,
        } => (
            RadialGradient {
                fx: *cx,
                fy: *cy,
                fr: 0.0,
                cx: *cx,
                cy: *cy,
                cr: *r,
                // Stretches the circle down about its centre into the ellipse.
                transform: Transform::from_row(1.0, 0.0, 0.0, *scale_y, 0.0, cy * (1.0 - scale_y)),
                spread_method: SpreadMethod::Pad,
                stops: stops(each),
                anti_alias: false,
            }
            .into(),
            NormalizedF32::ONE,
        ),
        Paint::Tiles {
            image,
            x,
            y,
            width,
            height,
        } => {
            let size = Size::from_wh(*width, *height)?;
            let mut builder = surface.stream_builder();
            let mut tile = builder.surface();
            tile.draw_image(laid.images[*image].0.clone(), size);
            tile.finish();
            let stream = builder.finish();
            (
                Pattern {
                    stream,
                    transform: Transform::from_translate(*x, *y),
                    width: *width,
                    height: *height,
                }
                .into(),
                NormalizedF32::ONE,
            )
        }
    })
}

/// The PDF file of `laid`, created at `created` when there is one.
pub fn write(laid: &Laid, created: Option<Instant>) -> Result<Vec<u8>, Error> {
    let mut document = Document::new_with(SerializeSettings::default());
    if let Some(created) = created {
        document.set_metadata(Metadata::new().creation_date(date(created)?));
    }
    let mut faces: Vec<Option<Font>> = vec![None; laid.faces.len()];
    let size = PageSettings::from_wh(layout::PAGE_WIDTH, layout::PAGE_HEIGHT)
        .expect("A4 is a valid page size");
    for laid_page in &laid.pages {
        let mut page = document.start_page_with(size.clone());
        let mut surface = page.surface();
        // How many states each open group pushed, innermost last.
        let mut groups: Vec<usize> = Vec::new();
        for item in &laid_page.items {
            match item {
                Item::Text {
                    face,
                    size,
                    x,
                    y,
                    color,
                    glyphs,
                    text,
                } => {
                    let font = match &faces[*face] {
                        Some(font) => font.clone(),
                        None => {
                            let font = Font::new(Data::from(laid.faces[*face].clone()), 0)
                                .ok_or_else(|| {
                                    Error::Parse("a font of the document does not parse".to_owned())
                                })?;
                            faces[*face] = Some(font.clone());
                            font
                        }
                    };
                    let glyphs: Vec<KrillaGlyph> = glyphs
                        .iter()
                        .map(|g| KrillaGlyph {
                            glyph_id: GlyphId::new(g.id),
                            text_range: g.text.clone(),
                            x_advance: g.advance / size,
                            x_offset: g.x / size,
                            // `parley`'s offset points down the page and `krilla`'s up it.
                            y_offset: -g.y / size,
                            y_advance: 0.0,
                            location: None,
                        })
                        .collect();
                    fill(&mut surface, *color);
                    surface.draw_glyphs(Point::from_xy(*x, *y), &glyphs, font, text, *size, false);
                }
                Item::Path {
                    path,
                    paint,
                    stroke,
                } => {
                    let Some(path) = shape(path) else {
                        continue;
                    };
                    let (paint, opacity) = match paint_of(&mut surface, paint, laid) {
                        Some(paint) => paint,
                        None => continue,
                    };
                    match stroke {
                        None => surface.set_fill(Some(Fill {
                            paint,
                            opacity,
                            rule: FillRule::EvenOdd,
                        })),
                        Some(stroke) => {
                            surface.set_fill(None);
                            surface.set_stroke(Some(Stroke {
                                paint,
                                width: stroke.width,
                                line_cap: if stroke.round {
                                    LineCap::Round
                                } else {
                                    LineCap::Butt
                                },
                                opacity,
                                dash: (!stroke.dash.is_empty()).then(|| StrokeDash {
                                    array: stroke.dash.clone(),
                                    offset: 0.0,
                                }),
                                ..Stroke::default()
                            }));
                        }
                    }
                    surface.draw_path(&path);
                    surface.set_stroke(None);
                }
                Item::Begin { opacity, clip } => {
                    let mut pushed = 0;
                    if let Some(clip) = clip.as_deref().and_then(shape) {
                        surface.push_clip_path(&clip, &FillRule::EvenOdd);
                        pushed += 1;
                    }
                    if *opacity < 1.0 {
                        surface.push_opacity(normal(*opacity));
                        pushed += 1;
                    }
                    groups.push(pushed);
                }
                Item::End => {
                    for _ in 0..groups.pop().unwrap_or(0) {
                        surface.pop();
                    }
                }
                Item::Image {
                    image,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let Some(box_size) = Size::from_wh(*width, *height) else {
                        continue;
                    };
                    surface.push_transform(&Transform::from_translate(*x, *y));
                    surface.draw_image(laid.images[*image].0.clone(), box_size);
                    surface.pop();
                }
            }
        }
        for _ in 0..groups.into_iter().sum::<usize>() {
            surface.pop();
        }
        surface.finish();
        page.finish();
    }
    document.finish().map_err(|err| match err {
        krilla::error::KrillaError::Image(image, _, why) => {
            let name = laid
                .images
                .iter()
                .find(|(each, _)| *each == image)
                .map_or("an image", |(_, name)| name.as_str());
            Error::Parse(format!("`{name}` does not decode as an image: {why}"))
        }
        other => Error::Runtime(format!("the PDF could not be written: {other}")),
    })
}

/// `instant` as a UTC date and time, to the second. A year before 1 or after 9999 returns
/// `invalid`.
fn date(instant: Instant) -> Result<DateTime, Error> {
    let days = instant.seconds.div_euclid(86_400);
    let seconds = instant.seconds.rem_euclid(86_400);
    let (year, month, day) = civil(days);
    if !(1..=9999).contains(&year) {
        return Err(Error::Invalid(format!(
            "the creation time is in the year {year}, and a PDF date is between 1 and 9999"
        )));
    }
    Ok(DateTime::new(year as u16)
        .month(month)
        .day(day)
        .hour((seconds / 3600) as u8)
        .minute((seconds / 60 % 60) as u8)
        .second((seconds % 60) as u8)
        .utc_offset_hour(0)
        .utc_offset_minute(0))
}

/// The proleptic Gregorian year, month and day `days` after 1970-01-01.
fn civil(days: i64) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::civil;

    #[test]
    fn days_are_civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }
}
