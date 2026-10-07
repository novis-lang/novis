//! The two arms of each job `image_guest` measures (`rule:packaging/the-boundary-is-the-cost`): the
//! image component called through the host as `Novis\Image` calls it, and the same codec core built
//! for the host and called directly, on the same input and the same plan. The test holds the two
//! outputs equal byte for byte and the bench times them, so the bench compares the same work.
//!
//! The input is a generated photo-like frame, a gradient with a seeded noise over it, so it carries
//! no personal data and a JPEG of it has detail for the decoder to work on. The JPEG is written by
//! the native arm once and decoded by both.

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

/// The size of the generated frame. Its RGBA8 rows cross the boundary one `Val` per byte, and
/// wasmtime's hostcall fuel caps what a guest returns at 128 MiB of `Val`s, so a frame much past
/// 800x600 traps on the way out (`data/gaps/nvs-ext/bytes-cross-one-val-per-byte.json`).
pub(crate) const WIDTH: u32 = 800;
pub(crate) const HEIGHT: u32 = 600;
/// The width the resize job scales the frame to. The height follows the aspect ratio.
pub(crate) const RESIZED_WIDTH: u64 = 200;

/// A job both arms run: one resize of decoded pixels, and one JPEG decode to RGBA8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Job {
    Resize,
    JpegDecode,
}

/// Every job, in the order the figures file lists them.
pub(crate) const JOBS: [Job; 2] = [Job::Resize, Job::JpegDecode];

impl Job {
    /// The job's key in `benches/results/image-guest.json`.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Job::Resize => "resize",
            Job::JpegDecode => "jpeg_decode",
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
        }
    }
}

/// The inputs every job reads: the frame's RGBA8 rows and a JPEG of them.
#[derive(Debug)]
pub(crate) struct Inputs {
    pub(crate) rgba: Vec<u8>,
    pub(crate) jpeg: Vec<u8>,
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
        Inputs { rgba, jpeg }
    }
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
