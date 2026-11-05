//! Edge cases of images and of the written file on the host: one given image side, CMYK,
//! 16-bit and interlaced pictures, `background-size`, a link over two lines, a tagged element
//! across pages, a merged rotated page, a password with an escaped title, and an invoice that
//! looks like XMP (`rule:core-classes/pdf-css-subset`, `rule:core-classes/pdf-reader-and-merge`,
//! `rule:core-classes/pdf-standards-and-passwords`,
//! `rule:core-classes/pdf-factur-x-is-the-one-attachment`).

use std::collections::BTreeSet;

use hayro_interpret::hayro_syntax::Pdf;
use hayro_interpret::hayro_syntax::object::{Array, Dict, Name, Object, Stream};
use hayro_interpret::hayro_syntax::page::Rotation;
use nvs_pdf::layout::{self, Item, Laid, bounds};
use nvs_pdf::merge::Part;
use nvs_pdf::read::Bound;
use nvs_pdf::{Asset, Document, Instant, Options, Standard, assets, css, fonts, parse, render};

fn lay_with(html: &str, list: &[Asset]) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(list);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
}

fn rendered(html: &str, assets: &[Asset], options: Options) -> Vec<u8> {
    render(&Document {
        html: html.to_owned(),
        assets: assets.to_vec(),
        options,
    })
    .unwrap()
    .data
}

fn asset(name: &str, data: Vec<u8>) -> Asset {
    Asset {
        name: name.to_owned(),
        data,
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

/// Each image on page `page`: its left, top, width and height.
fn images(laid: &Laid, page: usize) -> Vec<[f32; 4]> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Image {
                x,
                y,
                width,
                height,
                ..
            } => Some([*x, *y, *width, *height]),
            _ => None,
        })
        .collect()
}

/// A PNG `width` by `height` pixels in `color` at `depth`, every byte of its pixels `0x80`.
fn png(width: u32, height: u32, color: png::ColorType, depth: png::BitDepth) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        let mut writer = encoder.write_header().unwrap();
        let bytes_per_pixel = color.samples()
            * if depth == png::BitDepth::Sixteen {
                2
            } else {
                1
            };
        let size = width as usize * height as usize * bytes_per_pixel;
        writer.write_image_data(&vec![0x80; size]).unwrap();
    }
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &byte in bytes {
        c ^= u32::from(byte);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// `data` as a zlib stream of one stored block.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let len = u16::try_from(data.len()).unwrap();
    let mut out = vec![0x78, 0x01, 0x01];
    out.extend(len.to_le_bytes());
    out.extend((!len).to_le_bytes());
    out.extend(data);
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend(((b << 16) | a).to_be_bytes());
    out
}

/// A 2 by 2 RGB PNG, interlaced with Adam7. Of its seven passes, only pass 1 (the top-left
/// pixel), pass 6 (the top-right pixel) and pass 7 (the bottom row) have pixels.
fn interlaced_png() -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], body: &[u8]| {
        out.extend(u32::try_from(body.len()).unwrap().to_be_bytes());
        let mut typed = kind.to_vec();
        typed.extend(body);
        out.extend(&typed);
        out.extend(crc32(&typed).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend(2u32.to_be_bytes());
    header.extend(2u32.to_be_bytes());
    // Depth 8, colour type 2 (RGB), deflate, adaptive filtering, Adam7.
    header.extend([8, 2, 0, 0, 1]);
    chunk(b"IHDR", &header);
    let red = [200, 30, 30];
    let mut raw = Vec::new();
    for row in [&red[..], &red[..], &[red, red].concat()[..]] {
        raw.push(0);
        raw.extend(row);
    }
    chunk(b"IDAT", &zlib_stored(&raw));
    chunk(b"IEND", &[]);
    out
}

/// A CMYK JPEG `width` by `height` pixels, every pixel the same, with the Adobe segment that
/// names its four components CMYK. Each 8 by 8 block is coded as a zero DC difference and an end
/// of block, each a one-bit Huffman code of `0`.
fn cmyk_jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    let mut segment = |marker: u8, body: &[u8]| {
        out.extend([0xff, marker]);
        out.extend(u16::try_from(body.len() + 2).unwrap().to_be_bytes());
        out.extend(body);
    };
    // "Adobe", version 100, two flag words, and transform 0: the components are CMYK.
    segment(0xee, b"Adobe\x00\x64\x00\x00\x00\x00\x00");
    let mut dqt = vec![0];
    dqt.extend([1; 64]);
    segment(0xdb, &dqt);
    let mut sof = vec![8];
    sof.extend(height.to_be_bytes());
    sof.extend(width.to_be_bytes());
    sof.push(4);
    for id in 1..=4 {
        sof.extend([id, 0x11, 0]);
    }
    segment(0xc0, &sof);
    for class in [0x00, 0x10] {
        let mut dht = vec![class, 1];
        dht.extend([0; 15]);
        dht.push(0);
        segment(0xc4, &dht);
    }
    segment(0xda, &[4, 1, 0x00, 2, 0x00, 3, 0x00, 4, 0x00, 0, 63, 0]);
    let blocks = 4 * usize::from(width.div_ceil(8)) * usize::from(height.div_ceil(8));
    let bits = 2 * blocks;
    let mut data = vec![0u8; bits.div_ceil(8)];
    // The bits after the last block are padded with ones.
    if bits % 8 != 0 {
        *data.last_mut().unwrap() = 0xff >> (bits % 8);
    }
    out.extend(data);
    out.extend([0xff, 0xd9]);
    out
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

/// The catalog of `pdf`.
fn catalog(pdf: &Pdf) -> Dict<'_> {
    let xref = pdf.xref();
    xref.get::<Dict<'_>>(xref.root_id()).expect("a catalog")
}

/// A PDF text string: UTF-16BE or UTF-8 after its byte-order mark, and single bytes otherwise.
fn text_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let units: Vec<u16> = rest
            .chunks(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair.get(1).copied().unwrap_or(0)]))
            .collect();
        String::from_utf16(&units).unwrap()
    } else if let Some(rest) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        String::from_utf8(rest.to_vec()).unwrap()
    } else {
        bytes.iter().map(|&b| char::from(b)).collect()
    }
}

/// `<img width=…>` alone, or a CSS `height` alone, sets one side, and the other side follows the
/// picture's own width-to-height ratio.
#[test]
fn one_given_image_dimension_keeps_the_aspect_ratio() {
    // 4 by 2 pixels: twice as wide as it is tall.
    let assets = [asset(
        "wide.png",
        png(4, 2, png::ColorType::Rgb, png::BitDepth::Eight),
    )];
    let size = |html: &str| {
        let laid = lay_with(html, &assets);
        let [[_, _, width, height]] = images(&laid, 0)[..] else {
            panic!("{html}: {:?}", images(&laid, 0));
        };
        (width, height)
    };
    // 40px is 30pt.
    let (width, height) = size("<img src='wide.png' width='40'>");
    assert!(
        close(width, 30.0) && close(height, 15.0),
        "{width} {height}"
    );
    let (width, height) = size("<img src='wide.png' height='40'>");
    assert!(
        close(width, 60.0) && close(height, 30.0),
        "{width} {height}"
    );
    let (width, height) = size("<img src='wide.png' style='height: 10pt'>");
    assert!(
        close(width, 20.0) && close(height, 10.0),
        "{width} {height}"
    );
    let (width, height) = size("<img src='wide.png' style='width: 50pt'>");
    assert!(
        close(width, 50.0) && close(height, 25.0),
        "{width} {height}"
    );
}

/// A CMYK JPEG, a 16-bit PNG and an Adam7-interlaced PNG are each laid out at their size and
/// written as an image.
#[test]
fn cmyk_jpeg_sixteen_bit_and_interlaced_png_render() {
    let cases = [
        ("cmyk.jpg", cmyk_jpeg(16, 8), (12.0, 6.0)),
        (
            "deep.png",
            png(4, 2, png::ColorType::Rgb, png::BitDepth::Sixteen),
            (3.0, 1.5),
        ),
        (
            "deep-grey.png",
            png(4, 4, png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen),
            (3.0, 3.0),
        ),
        ("adam7.png", interlaced_png(), (1.5, 1.5)),
    ];
    for (name, data, (want_w, want_h)) in cases {
        let assets = [asset(name, data)];
        let html = format!("<p>Shop</p><img src='{name}'>");
        let laid = lay_with(&html, &assets);
        let [[_, _, width, height]] = images(&laid, 0)[..] else {
            panic!("{name}: {:?}", images(&laid, 0));
        };
        assert!(
            close(width, want_w) && close(height, want_h),
            "{name}: {width} {height}"
        );
        let result = render(&Document {
            html: html.clone(),
            assets: assets.to_vec(),
            ..Document::default()
        });
        let data = match result {
            Ok(rendered) => rendered.data,
            Err(err) => panic!("{name} did not render: {err:?}"),
        };
        let pdf = Pdf::new(data).unwrap();
        let mut found = Vec::new();
        if let Some(resources) = pdf.pages()[0].raw().get::<Dict<'_>>(b"Resources") {
            image_colors(&resources, &mut found, 0);
        }
        let want = if name == "cmyk.jpg" { 4 } else { 3 };
        let want = if name == "deep-grey.png" { 1 } else { want };
        assert!(
            found.contains(&Some(want)),
            "{name}: no image of {want} colour components in {found:?}"
        );
    }
}

/// The colour components of each image under `resources`, and under the resources of each form
/// in them, or `None` for a colour space this does not read.
fn image_colors(resources: &Dict<'_>, out: &mut Vec<Option<u32>>, depth: usize) {
    let Some(objects) = resources.get::<Dict<'_>>(b"XObject") else {
        return;
    };
    for key in objects.keys() {
        let Some(stream) = objects.get::<Stream<'_>>(key.as_str().as_bytes()) else {
            continue;
        };
        let dict = stream.dict();
        match dict.get::<Name<'_>>(b"Subtype").as_ref().map(Name::as_str) {
            Some("Image") => {
                let components = match dict.get::<Object<'_>>(b"ColorSpace") {
                    Some(Object::Name(name)) => match name.as_str() {
                        "DeviceGray" => Some(1),
                        "DeviceRGB" => Some(3),
                        "DeviceCMYK" => Some(4),
                        _ => None,
                    },
                    Some(Object::Array(array)) => array
                        .iter::<Object<'_>>()
                        .find_map(Object::into_stream)
                        .and_then(|icc| icc.dict().get::<u32>(b"N")),
                    _ => None,
                };
                out.push(components);
            }
            Some("Form") if depth < 8 => {
                if let Some(inner) = dict.get::<Dict<'_>>(b"Resources") {
                    image_colors(&inner, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// `background-size: cover` scales the picture until it covers the padding box, clipped to it,
/// and `contain` scales it until it fits inside. Both keep the picture's ratio.
#[test]
fn background_size_cover_and_contain_fill_the_box() {
    // 2 by 2 pixels: square, in a box twice as wide as it is tall.
    let assets = [asset(
        "tile.png",
        png(2, 2, png::ColorType::Rgb, png::BitDepth::Eight),
    )];
    // The clip and the image of each background image on page 0.
    let drawn = |size: &str| {
        let html = format!(
            "<div style='width: 100pt; height: 50pt; background-image: url(tile.png); \
             background-repeat: no-repeat; background-size: {size}'></div>"
        );
        let laid = lay_with(&html, &assets);
        let pairs: Vec<([f32; 4], [f32; 4])> = laid.pages[0]
            .items
            .windows(2)
            .filter_map(|pair| match pair {
                [
                    Item::Begin {
                        clip: Some(clip), ..
                    },
                    Item::Image {
                        x,
                        y,
                        width,
                        height,
                        ..
                    },
                ] => {
                    let (left, top, right, bottom) = bounds(clip);
                    Some(([left, top, right, bottom], [*x, *y, *width, *height]))
                }
                _ => None,
            })
            .collect();
        let [pair] = pairs[..] else {
            panic!("{size}: {pairs:?}");
        };
        pair
    };

    let ([left, top, right, bottom], [x, y, width, height]) = drawn("cover");
    assert!(
        close(right - left, 100.0) && close(bottom - top, 50.0),
        "cover clip {left} {top} {right} {bottom}"
    );
    assert!(
        close(width, 100.0) && close(height, 100.0),
        "cover {width} {height}"
    );
    assert!(
        x <= left + 0.05
            && y <= top + 0.05
            && x + width >= right - 0.05
            && y + height >= bottom - 0.05,
        "cover does not cover the box: {x} {y} {width} {height}"
    );

    let ([left, top, right, bottom], [x, y, width, height]) = drawn("contain");
    assert!(
        close(right - left, 100.0) && close(bottom - top, 50.0),
        "contain clip {left} {top} {right} {bottom}"
    );
    assert!(
        close(width, 50.0) && close(height, 50.0),
        "contain {width} {height}"
    );
    assert!(
        x >= left - 0.05
            && y >= top - 0.05
            && x + width <= right + 0.05
            && y + height <= bottom + 0.05,
        "contain is outside the box: {x} {y} {width} {height}"
    );
}

/// A link whose words wrap onto a second line gets one clickable rectangle on each line, and
/// the file has one link annotation for each.
#[test]
fn a_link_wrapped_over_two_lines_has_a_rectangle_on_each() {
    let html = "<p style='width: 40pt; font-size: 10pt'>\
                <a href='https://example.com/'>lorem ipsum</a></p>";
    let laid = lay_with(html, &[]);
    let mut links: Vec<[f32; 4]> = laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Link {
                x,
                y,
                width,
                height,
                ..
            } => Some([*x, *y, *width, *height]),
            _ => None,
        })
        .collect();
    links.sort_by(|a, b| a[1].total_cmp(&b[1]));
    let [first, second] = links[..] else {
        panic!("{links:?}");
    };
    assert!(first[2] > 0.0 && first[3] > 0.0, "{first:?}");
    assert!(second[2] > 0.0 && second[3] > 0.0, "{second:?}");
    // The second rectangle is below the first, on its own line.
    assert!(second[1] >= first[1] + first[3] - 0.05, "{links:?}");
    // Each rectangle is inside the paragraph's 40pt.
    assert!(first[2] <= 40.05 && second[2] <= 40.05, "{links:?}");

    let data = rendered(html, &[], Options::default());
    let pdf = Pdf::new(data).unwrap();
    let page = &pdf.pages()[0];
    let annots = page
        .raw()
        .get::<Array<'_>>(b"Annots")
        .expect("the page has annotations");
    let kinds: Vec<String> = annots
        .iter::<Dict<'_>>()
        .filter_map(|a| a.get::<Name<'_>>(b"Subtype").map(|n| n.as_str().to_owned()))
        .collect();
    assert_eq!(kinds, ["Link", "Link"]);
}

/// Each structure element under `obj`, in order: its type, and the pages its content is on.
fn structure(obj: Object<'_>, out: &mut Vec<(String, BTreeSet<i32>)>, current: Option<usize>) {
    match obj {
        Object::Array(array) => {
            for child in array.iter::<Object<'_>>() {
                structure(child, out, current);
            }
        }
        Object::Dict(dict) => {
            let page = dict.get_ref(b"Pg").map(|r| r.obj_number);
            if let Some(kind) = dict.get::<Name<'_>>(b"S") {
                out.push((kind.as_str().to_owned(), page.into_iter().collect()));
                let at = out.len() - 1;
                if let Some(kids) = dict.get::<Object<'_>>(b"K") {
                    structure(kids, out, Some(at));
                }
            } else if let (Some(page), Some(at)) = (page, current) {
                // A marked-content or object reference on a page of its own.
                out[at].1.insert(page);
            }
        }
        _ => {}
    }
}

/// A paragraph that continues on the next page is one `P` element in the structure tree, and its
/// content is on both pages.
#[test]
fn an_element_split_across_pages_is_one_structure_element() {
    let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod. ";
    let html = format!(
        "<html lang='en'><head><title>Shop</title></head><body>\
         <h1>Shop</h1><p>{}</p></body></html>",
        lorem.repeat(150)
    );
    let data = rendered(&html, &[], Options::default());
    let pdf = Pdf::new(data).unwrap();
    assert!(pdf.pages().len() >= 2, "{} page", pdf.pages().len());
    let root = catalog(&pdf)
        .get::<Dict<'_>>(b"StructTreeRoot")
        .expect("the file is tagged");
    let mut elements = Vec::new();
    structure(root.get::<Object<'_>>(b"K").unwrap(), &mut elements, None);
    let paragraphs: Vec<_> = elements.iter().filter(|(kind, _)| kind == "P").collect();
    let [(_, pages)] = paragraphs[..] else {
        panic!("{elements:?}");
    };
    assert!(pages.len() >= 2, "the paragraph is on {pages:?}");
    assert_eq!(
        elements.iter().filter(|(kind, _)| kind == "H1").count(),
        1,
        "{elements:?}"
    );
}

/// A one-page PDF, its page `/MediaBox` `0 0 width height` and turned by `rotate`.
fn rotated_pdf(width: u32, height: u32, rotate: u32) -> Vec<u8> {
    let content = b"0 0 1 rg 10 10 100 50 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Rotate {rotate} \
             /Contents 4 0 R /Resources << >> >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            std::str::from_utf8(content).unwrap()
        ),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// A landscape page turned a quarter turn keeps its `/MediaBox` and its `/Rotate` when it is
/// merged after a portrait page.
#[test]
fn a_merged_rotated_landscape_page_keeps_its_size_and_rotation() {
    let portrait = rendered("<p>Shop</p>", &[], Options::default());
    let source = rotated_pdf(842, 595, 90);
    // The source reads as written.
    let pdf = Pdf::new(source.clone()).unwrap();
    assert!(matches!(pdf.pages()[0].rotation(), Rotation::Horizontal));

    let merged = nvs_pdf::merge::merge(
        vec![
            Part {
                data: portrait,
                pages: None,
            },
            Part {
                data: source,
                pages: None,
            },
        ],
        Bound::default(),
    )
    .unwrap();
    let pdf = Pdf::new(merged).unwrap();
    assert_eq!(pdf.pages().len(), 2);
    let page = &pdf.pages()[1];
    let media = page.media_box();
    assert_eq!(
        (media.x0, media.y0, media.x1, media.y1),
        (0.0, 0.0, 842.0, 595.0)
    );
    assert!(
        matches!(page.rotation(), Rotation::Horizontal),
        "{:?}",
        page.rotation()
    );
    // Turned, it shows as a portrait page 595pt wide.
    let (width, height) = page.render_dimensions();
    assert!(
        close(width, 595.0) && close(height, 842.0),
        "{width} {height}"
    );
    // The portrait page before it is not turned.
    assert!(matches!(pdf.pages()[0].rotation(), Rotation::None));
}

/// A file with a password, whose title has parentheses, unbalanced ones among them, a backslash
/// and a percent sign, opens with that password and has that title.
#[test]
fn a_password_and_a_title_with_parentheses_and_percent_round_trip() {
    let title = "Shop (draft) 100% off \\ a) b( c";
    let html = format!(
        "<html lang='en'><head><title>{title}</title></head><body><p>Shop</p></body></html>"
    );
    let data = rendered(
        &html,
        &[],
        Options {
            password: Some("Shop-secret".to_owned()),
            ..Options::default()
        },
    );
    assert!(count(&data, b"/Encrypt") >= 1, "the file is not encrypted");
    assert!(
        Pdf::new(data.clone()).is_err(),
        "the file opens with no password"
    );
    let pdf = match Pdf::new_with_password(data, "Shop-secret") {
        Ok(pdf) => pdf,
        Err(err) => panic!("the file does not open with its password: {err:?}"),
    };
    let read = pdf
        .metadata()
        .title
        .as_deref()
        .map(text_string)
        .expect("the file has a title");
    assert_eq!(read, title);
    assert_eq!(pdf.pages().len(), 1);
}

/// An invoice whose XML has `<?xpacket …?>` instructions of its own is attached byte for byte,
/// and the file's XMP metadata still parses, with the Factur-X fields once.
#[test]
fn invoice_xml_with_xpacket_text_embeds_intact() {
    const INVOICE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<?xpacket begin=\"\" id=\"Shop\"?>\
<rsm:CrossIndustryInvoice \
xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\" \
xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\">\
<!-- <?xpacket begin=\"\" id=\"example.com\"?> --><?xpacket begin=\"\" id=\"Shop-2\"?>\
<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter>\
<ram:ID>urn:cen.eu:en16931:2017</ram:ID>\
</ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>\
<rsm:ExchangedDocument><ram:ID>Shop-1</ram:ID><ram:TypeCode>380</ram:TypeCode>\
</rsm:ExchangedDocument></rsm:CrossIndustryInvoice><?xpacket end=\"r\"?>";
    let data = rendered(
        "<html lang=\"en\"><head><title>Shop</title></head><body><p>Invoice</p></body></html>",
        &[],
        Options {
            standard: Some(Standard::PdfA3b),
            invoice_xml: Some(INVOICE.to_owned()),
            created: Some(Instant {
                seconds: 1_767_225_600,
                nanos: 0,
            }),
            ..Options::default()
        },
    );

    let (_, info) = nvs_pdf::read::read(data.clone(), Bound::default()).unwrap();
    assert_eq!(info.attachments.len(), 1);
    assert_eq!(info.attachments[0].1, INVOICE.as_bytes());
    assert_eq!(info.invoice_xml.as_deref(), Some(INVOICE));

    let pdf = Pdf::new(data).unwrap();
    let metadata = catalog(&pdf)
        .get::<Stream<'_>>(b"Metadata")
        .expect("the file has XMP metadata");
    let xmp = metadata.decoded().unwrap().into_owned();
    let xmp = String::from_utf8(xmp).expect("the XMP is UTF-8");
    assert_eq!(xmp.matches("<?xpacket begin=").count(), 1, "{xmp}");
    assert!(!xmp.contains("Shop-1"), "the invoice is in the XMP: {xmp}");
    let doc = match roxmltree::Document::parse(&xmp) {
        Ok(doc) => doc,
        Err(err) => panic!("the XMP is not well-formed: {err}\n{xmp}"),
    };
    let named = |name: &str| {
        doc.descendants()
            .filter(|n| n.tag_name().name() == name)
            .map(|n| n.text().unwrap_or_default().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(named("DocumentType"), ["INVOICE"]);
    assert_eq!(named("DocumentFileName"), ["factur-x.xml"]);
    assert_eq!(named("ConformanceLevel").len(), 1);
}
