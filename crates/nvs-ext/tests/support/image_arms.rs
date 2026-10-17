//! The two arms of each job `image_guest` measures (`rule:packaging/the-boundary-is-the-cost`): the
//! image component called through the host as `Novis\Image` calls it, and the same codec core built
//! for the host and called directly, on the same input and the same plan. The test holds the two
//! outputs equal byte for byte and the bench times them, so the bench compares the same work.
//!
//! The input is a generated photo-like frame, a gradient with a seeded noise over it, so it carries
//! no personal data and a JPEG of it has detail for the decoder to work on. The JPEG is written by
//! the native arm once and decoded by both. The two PDF pages are written here object by object:
//! one of text in unembedded standard fonts, and one placing that JPEG several times.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::call::{Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Extension, Loader};
use nvs_image::ops::Resize;
use nvs_image::{Encoding, Format, Input, Op, Output, Pixels, Plan};

/// The size of the generated frame: a full HD frame, whose RGBA8 rows cross the boundary in one
/// copy each way because the image world is called typed.
pub(crate) const WIDTH: u32 = 1920;
pub(crate) const HEIGHT: u32 = 1080;
/// The width the resize job scales the frame to. The height follows the aspect ratio.
pub(crate) const RESIZED_WIDTH: u64 = 200;

/// The page both PDF jobs rasterise: A4, in points.
pub(crate) const PAGE_WIDTH: u32 = 595;
pub(crate) const PAGE_HEIGHT: u32 = 842;
/// The lines of text the text page carries.
const TEXT_LINES: usize = 64;
/// How many times the image page draws the frame's JPEG, in two columns.
const IMAGE_DRAWS: usize = 6;

/// A job both arms run: one resize of decoded pixels, one JPEG decode to RGBA8, and one PDF page
/// rasterised at the default dpi, either a page of text or a page of images
/// (`rule:core-classes/pdf-page-is-an-image-source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Job {
    Resize,
    JpegDecode,
    PdfTextPage,
    PdfImagePage,
}

/// Every job, in the order the figures file lists them.
pub(crate) const JOBS: [Job; 4] = [
    Job::Resize,
    Job::JpegDecode,
    Job::PdfTextPage,
    Job::PdfImagePage,
];

impl Job {
    /// The job's key in `benches/results/image-guest.json`.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Job::Resize => "resize",
            Job::JpegDecode => "jpeg_decode",
            Job::PdfTextPage => "pdf_text_page",
            Job::PdfImagePage => "pdf_image_page",
        }
    }

    /// What the job does, written beside its figures.
    pub(crate) fn what(self) -> String {
        match self {
            Job::Resize => format!(
                "{WIDTH}x{HEIGHT} RGBA8 pixels resized to {RESIZED_WIDTH} wide with Lanczos3, returned as raw pixels"
            ),
            Job::JpegDecode => format!(
                "one {WIDTH}x{HEIGHT} JPEG at the default quality decoded to RGBA8, returned as raw pixels"
            ),
            Job::PdfTextPage => format!(
                "one A4 page of {TEXT_LINES} lines of text in two unembedded standard fonts, rasterised at {} dpi and returned as raw pixels",
                nvs_image::DEFAULT_DPI
            ),
            Job::PdfImagePage => format!(
                "one A4 page drawing the {WIDTH}x{HEIGHT} JPEG {IMAGE_DRAWS} times, rasterised at {} dpi and returned as raw pixels",
                nvs_image::DEFAULT_DPI
            ),
        }
    }
}

/// The inputs every job reads: the frame's RGBA8 rows, a JPEG of them, and the two one-page PDFs.
#[derive(Debug)]
pub(crate) struct Inputs {
    pub(crate) rgba: Vec<u8>,
    pub(crate) jpeg: Vec<u8>,
    pub(crate) text_pdf: Vec<u8>,
    pub(crate) image_pdf: Vec<u8>,
}

impl Inputs {
    pub(crate) fn new() -> Self {
        let rgba = frame();
        let pixels = Pixels {
            width: WIDTH,
            height: HEIGHT,
            rgba: rgba.clone(),
            exif: None,
        };
        let jpeg = nvs_image::encode(pixels, Format::Jpeg, Encoding::default())
            .unwrap_or_else(|err| panic!("the frame does not encode: {err:?}"));
        let text_pdf = text_page();
        let image_pdf = image_page(&jpeg);
        Inputs {
            rgba,
            jpeg,
            text_pdf,
            image_pdf,
        }
    }

    /// The PDF `job` rasterises, if it rasterises one.
    fn pdf(&self, job: Job) -> Option<&[u8]> {
        match job {
            Job::PdfTextPage => Some(&self.text_pdf),
            Job::PdfImagePage => Some(&self.image_pdf),
            Job::Resize | Job::JpegDecode => None,
        }
    }
}

/// A PDF of `objects`, numbered from 1 with the catalog first, and a cross-reference table whose
/// offsets are the objects' own.
fn pdf_of(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (at, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", at + 1).bytes());
        out.extend(body);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    out
}

fn stream(dict: &str, content: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", content.len()).into_bytes();
    out.extend(content);
    out.extend(b"\nendstream");
    out
}

/// A one-page A4 document drawn by `content`, with `resources` and `extra` objects numbered
/// from 5.
fn one_page(content: &[u8], resources: &str, extra: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] /Resources << {resources} >> /Contents 4 0 R >>"
        )
        .into_bytes(),
        stream("", content),
    ];
    objects.extend(extra);
    pdf_of(&objects)
}

/// A page of text: a bold heading and `TEXT_LINES` lines of body text, in Helvetica and
/// Times-Roman, neither embedded, so both are drawn with their Liberation substitutes.
fn text_page() -> Vec<u8> {
    let font =
        |name: &str| format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} >>").into_bytes();
    let mut content = String::from("BT /Head 16 Tf 50 800 Td (Shop order summary) Tj ET\n");
    content.push_str("BT /Body 10 Tf 12 TL 50 776 Td\n");
    for line in 0..TEXT_LINES {
        content.push_str(&format!(
            "(Line {line}: a widget, a gadget and a sprocket were shipped to example.com in one parcel.) '\n"
        ));
    }
    content.push_str("ET");
    one_page(
        content.as_bytes(),
        "/Font << /Head 5 0 R /Body 6 0 R >>",
        vec![font("Helvetica-Bold"), font("Times-Roman")],
    )
}

/// A page of images: `jpeg` placed `IMAGE_DRAWS` times in two columns, each copy scaled to
/// 270 by 202 points.
fn image_page(jpeg: &[u8]) -> Vec<u8> {
    let image = stream(
        &format!(
            "/Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"
        ),
        jpeg,
    );
    let mut content = String::new();
    for at in 0..IMAGE_DRAWS {
        let x = 20 + (at % 2) * 285;
        let y = 620 - (at / 2) * 210;
        content.push_str(&format!("q 270 0 0 202 {x} {y} cm /Photo Do Q\n"));
    }
    one_page(
        content.as_bytes(),
        "/XObject << /Photo 5 0 R >>",
        vec![image],
    )
}

/// A gradient in each channel with a seeded noise added, opaque.
fn frame() -> Vec<u8> {
    let mut seed: u32 = 0x2545_f491;
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        u8::try_from(seed % 33).unwrap()
    };
    let mut rgba = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * 4);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let r = u8::try_from(x * 200 / WIDTH).unwrap();
            let g = u8::try_from(y * 200 / HEIGHT).unwrap();
            let b = u8::try_from((x + y) * 100 / (WIDTH + HEIGHT)).unwrap();
            rgba.extend_from_slice(&[r + noise(), g + noise(), b + noise(), 255]);
        }
    }
    rgba
}

/// `job` run by the codec core built for the host.
pub(crate) fn native(job: Job, inputs: &Inputs) -> Vec<u8> {
    let raw = |steps| Plan {
        steps,
        overlays: Vec::new(),
        encoding: Encoding::default(),
        output: Output::Raw,
    };
    let (input, plan) = match job {
        Job::Resize => (
            Input::Pixels {
                width: u64::from(WIDTH),
                height: u64::from(HEIGHT),
                rgba: &inputs.rgba,
            },
            raw(vec![Op::Resize(Resize {
                width: Some(RESIZED_WIDTH),
                ..Resize::default()
            })]),
        ),
        Job::JpegDecode => (
            Input::Encoded {
                data: &inputs.jpeg,
                auto_orient: true,
                to_srgb: true,
                page: 1,
                dpi: nvs_image::DEFAULT_DPI,
            },
            raw(Vec::new()),
        ),
        Job::PdfTextPage | Job::PdfImagePage => (
            Input::Encoded {
                data: inputs.pdf(job).expect("a PDF job has a PDF"),
                auto_orient: true,
                to_srgb: true,
                page: 1,
                dpi: nvs_image::DEFAULT_DPI,
            },
            raw(Vec::new()),
        ),
    };
    nvs_image::run(&input, nvs_image::DEFAULT_MAX_PIXELS, &plan)
        .unwrap_or_else(|err| panic!("the native {job:?} fails: {err:?}"))
}

/// The guest's half: a host, the image component compiled on its engine, and one request every
/// call runs in, so the instance is made once.
pub(crate) struct Guest {
    _host: Host,
    extension: Extension,
    request: Request,
}

impl Guest {
    pub(crate) fn new() -> Self {
        let host = Host::new(4, |_| Ok(())).expect("the host starts");
        let extension = Loader::new(host.engine())
            .builtins()
            .expect("the built-in components load")
            .into_iter()
            .find(|builtin| builtin.manifest().class == "Novis\\Image\\Codec")
            .expect("the image component is built in")
            .extension()
            .expect("the image component compiles")
            .clone();
        let request = host.request(Arc::new(Meter::new(Duration::from_secs(3600), None)));
        Guest {
            _host: host,
            extension,
            request,
        }
    }

    /// `job` run by the image component, one host-to-guest call, returning the RGBA8 pixels
    /// behind their size header.
    pub(crate) fn run(&self, job: Job, inputs: &Inputs) -> Vec<u8> {
        self.run_returning(job, inputs, "Raw")
    }

    /// `job` run as `run` does, returning the 16-byte size header alone, so no pixel crosses
    /// back to the host.
    pub(crate) fn run_size(&self, job: Job, inputs: &Inputs) -> Vec<u8> {
        self.run_returning(job, inputs, "Size")
    }

    fn run_returning(&self, job: Job, inputs: &Inputs, output: &str) -> Vec<u8> {
        let (source, steps) = match job {
            Job::Resize => (
                shape(vec![
                    ("width", Value::Uint(u64::from(WIDTH))),
                    ("height", Value::Uint(u64::from(HEIGHT))),
                    ("pixels", Value::Bytes(inputs.rgba.clone())),
                ]),
                vec![shape(vec![(
                    "resize",
                    shape(vec![("width", Value::Uint(RESIZED_WIDTH))]),
                )])],
            ),
            Job::JpegDecode => (
                shape(vec![
                    ("data", Value::Bytes(inputs.jpeg.clone())),
                    ("autoOrient", Value::Bool(true)),
                    ("toSrgb", Value::Bool(true)),
                ]),
                Vec::new(),
            ),
            Job::PdfTextPage | Job::PdfImagePage => (
                shape(vec![
                    (
                        "data",
                        Value::Bytes(inputs.pdf(job).expect("a PDF job has a PDF").to_vec()),
                    ),
                    ("autoOrient", Value::Bool(true)),
                    ("toSrgb", Value::Bool(true)),
                    ("page", Value::Uint(1)),
                    ("dpi", Value::Float(nvs_image::DEFAULT_DPI)),
                ]),
                Vec::new(),
            ),
        };
        let plan = shape(vec![
            (
                "steps",
                Value::Array(
                    steps
                        .into_iter()
                        .enumerate()
                        .map(|(at, step)| (Key::Int(i64::try_from(at).unwrap()), step))
                        .collect(),
                ),
            ),
            ("overlays", Value::Array(Vec::new())),
            ("output", Value::Case(output.to_owned())),
        ]);
        match block_on(
            self.request
                .call_values(&self.extension, "run", vec![source, plan]),
        ) {
            Ok(Some(Value::Bytes(bytes))) => bytes,
            other => panic!("the guest {job:?} returned {other:?}"),
        }
    }

    /// Ends the request the calls ran in.
    pub(crate) fn end(self) {
        block_on(self.request.end()).expect("the request ends");
    }
}

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(key, value)| (Key::String(key.to_owned()), value))
            .collect(),
    )
}

/// `future` polled on this thread until it finishes.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}
