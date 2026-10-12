//! The writer: laid-out pages as a PDF file, by `krilla` (`rule:core-classes/pdf-output-is-inert`).
//!
//! Each page is A4, and each glyph run is drawn in its embedded face, which `krilla` subsets to
//! the glyphs the document draws. Each image is drawn at its box.
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

use krilla::Document;
use krilla::SerializeSettings;
use krilla::geom::{Point, Size, Transform};
use krilla::metadata::{DateTime, Metadata};
use krilla::page::PageSettings;
use krilla::text::{Font, GlyphId, KrillaGlyph};

use crate::layout::{self, Item, Laid};
use crate::{Error, Instant, fonts};

/// The PDF file of `laid`, created at `created` when there is one.
pub fn write(laid: &Laid, created: Option<Instant>) -> Result<Vec<u8>, Error> {
    let mut document = Document::new_with(SerializeSettings::default());
    if let Some(created) = created {
        document.set_metadata(Metadata::new().creation_date(date(created)?));
    }
    let mut faces: Vec<Option<Font>> = vec![None; fonts::FACES.len()];
    let size = PageSettings::from_wh(layout::PAGE_WIDTH, layout::PAGE_HEIGHT)
        .expect("A4 is a valid page size");
    for laid_page in &laid.pages {
        let mut page = document.start_page_with(size.clone());
        let mut surface = page.surface();
        for item in &laid_page.items {
            match item {
                Item::Text {
                    face,
                    size,
                    x,
                    y,
                    glyphs,
                    text,
                } => {
                    let font = match &faces[*face] {
                        Some(font) => font.clone(),
                        None => {
                            let font = Font::new(fonts::FACES[*face].into(), 0)
                                .expect("the embedded faces parse");
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
                    surface.draw_glyphs(Point::from_xy(*x, *y), &glyphs, font, text, *size, false);
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
