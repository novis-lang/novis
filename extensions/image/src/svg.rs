//! Decoding an SVG document to RGBA8 pixels through `usvg` and `resvg`
//! (`rule:core-classes/image-format-roster`).
//!
//! **Only the bytes given are read.** The parser's `<image>` resolver is replaced: a `data:` URI
//! decodes from the document itself, and any other `href` — a path or an address — is recorded and
//! returns `Runtime` naming it once the document is parsed. The resolver `usvg` offers by default
//! reads files from disk and is never built. A nested SVG inside a `data:` URI is parsed with every
//! external image ignored, which is `usvg`'s own rule for one.
//!
//! **The cap is read off the parsed tree.** The document's size, each side rounded up to a whole
//! pixel, is held to the cap before the pixmap exists. `usvg` refuses a document of more than a
//! million elements, and the guest's memory cap and deadline bound the rest.
//!
//! `<text>` is not drawn: the component is built without `usvg`'s text feature, which needs fonts.
//! The result is straight-alpha RGBA, with no EXIF block.

use std::sync::{Arc, Mutex};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{ImageHrefResolver, Options, Tree};

use crate::decode::{Pixels, check};
use crate::{Error, Format};

/// Whether `data` reads as an SVG document: optional byte-order mark and whitespace, then an XML
/// declaration, a comment, a doctype or the `<svg` tag, with `<svg` in the first 4 KiB.
pub(crate) fn sniff(data: &[u8]) -> bool {
    let data = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);
    let start = data
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(data.len());
    let head = &data[start..data.len().min(start + 4096)];
    let opens = [&b"<svg"[..], b"<?xml", b"<!--", b"<!DOCTYPE"]
        .iter()
        .any(|prefix| head.starts_with(prefix));
    opens && head.windows(4).any(|window| window == b"<svg")
}

/// The parsed document. An external `href` returns `Runtime` naming it, and a document that does
/// not parse returns `Parse`.
fn parse(data: &[u8]) -> Result<Tree, Error> {
    let external: Arc<Mutex<Option<String>>> = Arc::default();
    let seen = Arc::clone(&external);
    let options = Options {
        resources_dir: None,
        image_href_resolver: ImageHrefResolver {
            resolve_data: ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(move |href, _| {
                let mut first = seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                first.get_or_insert_with(|| href.to_owned());
                None
            }),
        },
        ..Options::default()
    };
    let tree = Tree::from_data(data, &options)
        .map_err(|err| Error::Parse(format!("the SVG document does not parse: {err}")))?;
    let href = external
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    if let Some(href) = href {
        return Err(Error::Runtime(format!(
            "the SVG links to `{href}`. An SVG can only use images it contains as `data:` URIs"
        )));
    }
    Ok(tree)
}

/// The document's size in whole pixels, each side rounded up.
fn size(tree: &Tree) -> (u64, u64) {
    let size = tree.size();
    (size.width().ceil() as u64, size.height().ceil() as u64)
}

/// The width and height of the SVG document `data`, read by parsing it and drawing nothing.
pub(crate) fn info(data: &[u8]) -> Result<crate::info::Info, Error> {
    let (width, height) = size(&parse(data)?);
    Ok(crate::info::Info {
        format: Format::Svg,
        width,
        height,
        has_alpha: true,
        frames: 1,
        orientation: 1,
        exif: None,
        has_icc: false,
    })
}

/// Draws the SVG document `data` at its own size, refusing one over `cap` pixels before the
/// pixmap is allocated.
pub(crate) fn decode(data: &[u8], cap: u64) -> Result<Pixels, Error> {
    let tree = parse(data)?;
    let (width, height) = size(&tree);
    check(width, height, cap)?;
    let too_big = || Error::Runtime(format!("an SVG of {width}x{height} is too big"));
    let width = u32::try_from(width).map_err(|_| too_big())?;
    let height = u32::try_from(height).map_err(|_| too_big())?;
    let mut pixmap = Pixmap::new(width, height).ok_or_else(too_big)?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    let mut rgba = pixmap.take();
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    Ok(Pixels {
        width,
        height,
        rgba,
        exif: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="3"><rect width="4" height="3" fill="#ff0000"/></svg>"##;

    #[test]
    fn an_svg_is_sniffed_after_a_declaration_and_whitespace() {
        assert!(sniff(SQUARE.as_bytes()));
        assert!(sniff(
            format!("\u{feff}  <?xml version=\"1.0\"?>\n{SQUARE}").as_bytes()
        ));
        assert!(!sniff(b"<html><body></body></html>"));
        assert!(!sniff(b"not an svg"));
    }

    #[test]
    fn an_svg_draws_at_its_size() {
        let got = decode(SQUARE.as_bytes(), 100).unwrap();
        assert_eq!((got.width, got.height), (4, 3));
        assert_eq!(&got.rgba[..4], &[255, 0, 0, 255]);
        assert!(matches!(
            decode(SQUARE.as_bytes(), 11),
            Err(Error::Runtime(_))
        ));
    }

    #[test]
    fn an_external_href_throws_naming_it() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><image href="/etc/passwd" width="4" height="4"/></svg>"#;
        assert_eq!(
            decode(svg.as_bytes(), 100).unwrap_err(),
            Error::Runtime(
                "the SVG links to `/etc/passwd`. An SVG can only use images it contains as `data:` URIs"
                    .to_owned()
            )
        );
    }
}
