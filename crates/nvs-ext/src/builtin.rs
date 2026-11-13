//! The built-in components: `.nvsx` bytes embedded in the binary
//! (`rule:packaging/the-first-party-components-are-built-in`).
//!
//! `build.rs` compiles each component's crate under `extensions/` for `wasm32-wasip2` and packs it
//! with [`crate::pack::pack`], compiled into the build script with `#[path]`, so the bytes here are
//! the ones `nvs ext build` would write from the same inputs. It also writes each one's SHA-256,
//! which is the component's pin: an `[[extension]]` entry carries a `sha256`, and a built-in
//! component has none because the binary is its pin.
//!
//! **The binary stores each component compressed with Brotli.** Its manifest and source sections
//! are stored plain beside it, so reading a manifest expands nothing. [`expanded`] is the one
//! reader of the wasm, and only a compile that misses the cache calls it. The idle process holds
//! no expanded bytes. The pin is the SHA-256 of the expanded `.nvsx`, so every cache key and
//! `env_hash` are those of the plain bytes.
//!
//! [`cap_pixels`] is the host's half of the image component's pixel cap, and
//! [`cap_decompression`] of the spreadsheet and PDF components' decompression bound: each clamps a call's
//! arguments to what the request's snapshot gives, before they cross.

#[cfg(debug_assertions)]
use std::cell::Cell;
use std::io::Read;

use sha2::{Digest, Sha256};

use crate::convert::{Key, Value};
use crate::{Malformed, malformed};

/// One built-in component as the binary stores it: the `.nvsx` compressed with Brotli, its two
/// sections plain, and the SHA-256 of the expanded `.nvsx`, taken when the binary was built.
#[derive(Debug, Clone, Copy)]
pub struct Stored {
    /// Its crate's directory under `extensions/`.
    pub name: &'static str,
    /// The class it declares.
    pub class: &'static str,
    /// The `.nvsx`, compressed with Brotli.
    pub compressed: &'static [u8],
    /// The payload of its `nvs.manifest` section.
    pub manifest: &'static [u8],
    /// The payload of its `nvs.source` section, empty when it has none.
    pub source: &'static [u8],
    /// The SHA-256 of the expanded `.nvsx`: its pin.
    pub sha256: [u8; 32],
}

macro_rules! stored {
    ($name:literal, $class:expr) => {
        Stored {
            name: $name,
            class: $class,
            compressed: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".nvsx.br")),
            manifest: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".manifest")),
            source: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".source")),
            sha256: *include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".sha256")),
        }
    };
}

/// The image component, `Novis\Image\Codec`.
pub static IMAGE: Stored = stored!("image", IMAGE_CLASS);

/// The SHA-256 of the expanded [`IMAGE`].
pub const IMAGE_SHA256: [u8; 32] = IMAGE.sha256;

/// The class the image component declares.
pub const IMAGE_CLASS: &str = r"Novis\Image\Codec";

/// The intl component, `Novis\Intl\Icu`.
pub static INTL: Stored = stored!("intl", INTL_CLASS);

/// The SHA-256 of the expanded [`INTL`].
pub const INTL_SHA256: [u8; 32] = INTL.sha256;

/// The class the intl component declares.
pub const INTL_CLASS: &str = r"Novis\Intl\Icu";

/// The PDF component, `Novis\Pdf\Engine`.
pub static PDF: Stored = stored!("pdf", PDF_CLASS);

/// The SHA-256 of the expanded [`PDF`].
pub const PDF_SHA256: [u8; 32] = PDF.sha256;

/// The class the PDF component declares.
pub const PDF_CLASS: &str = r"Novis\Pdf\Engine";

/// The spreadsheet component, `Novis\Spreadsheet\Engine`.
pub static SPREADSHEET: Stored = stored!("spreadsheet", SPREADSHEET_CLASS);

/// The SHA-256 of the expanded [`SPREADSHEET`].
pub const SPREADSHEET_SHA256: [u8; 32] = SPREADSHEET.sha256;

/// The class the spreadsheet component declares.
pub const SPREADSHEET_CLASS: &str = r"Novis\Spreadsheet\Engine";

/// Every built-in component, in the order the loader reads them.
pub static STORED: [&Stored; 4] = [&IMAGE, &INTL, &PDF, &SPREADSHEET];

/// The built-in component `name` among [`STORED`].
pub fn stored(name: &str) -> Option<&'static Stored> {
    STORED.iter().copied().find(|stored| stored.name == name)
}

#[cfg(debug_assertions)]
thread_local! {
    static EXPANSIONS: Cell<u64> = const { Cell::new(0) };
}

/// How many times this thread has expanded a built-in component. Debug builds only, so the
/// release binary carries no counter. A test reads it to show a cache hit expands nothing.
#[cfg(debug_assertions)]
pub fn expansions() -> u64 {
    EXPANSIONS.with(Cell::get)
}

/// The `.nvsx` the built-in component `name` expands to, checked against its pin.
///
/// The only reader of a component's wasm: a compile that misses the cache calls it, and frees
/// the result once the component is compiled. Memory: the expanded size, about 16 MB at most,
/// and Brotli's 4 MiB window, for the call.
///
/// # Errors
///
/// A name no built-in component has, bytes that do not expand, or an expanded `.nvsx` whose
/// SHA-256 is not the pin. Each one is a broken build.
pub fn expanded(name: &str) -> Result<Vec<u8>, Malformed> {
    let stored =
        stored(name).ok_or_else(|| malformed(format!("no built-in component `{name}`")))?;
    #[cfg(debug_assertions)]
    EXPANSIONS.with(|count| count.set(count.get() + 1));
    let mut out = Vec::new();
    brotli::Decompressor::new(stored.compressed, 4096)
        .read_to_end(&mut out)
        .map_err(|err| {
            malformed(format!(
                "the built-in component `{name}` does not expand: {err}"
            ))
        })?;
    if Sha256::digest(&out).as_slice() != stored.sha256 {
        return Err(malformed(format!(
            "the built-in component `{name}` does not expand to the bytes its pin names"
        )));
    }
    Ok(out)
}

/// `rule:core-classes/image-pixel-cap` on the host's side of a call: every encoded source in the
/// arguments of a `Novis\Image\Codec::run` or `::variants` call — the source itself and each
/// overlay's — gets `maxPixels` set to the smaller of its own and `in_force`, the `[image]
/// max_pixels` of the calling request's snapshot. A source with none gets `in_force`, so a call can
/// lower the cap and never raise it. Any other call is left as it is.
///
/// An encoded source is the `source` case whose required key is `data`; a value that is not one,
/// or a `maxPixels` that is not a whole number, is left for the manifest's conversion to refuse.
pub fn cap_pixels(class: &str, method: &str, args: &mut [Value], in_force: u64) {
    if class != IMAGE_CLASS || !matches!(method, "run" | "variants") {
        return;
    }
    let [source, plans, ..] = args else {
        return;
    };
    cap_source(source, in_force);
    if method == "run" {
        cap_overlays(plans, in_force);
    } else if let Value::Array(plans) = plans {
        for (_, plan) in plans {
            cap_overlays(plan, in_force);
        }
    }
}

fn field<'a>(fields: &'a mut [(Key, Value)], name: &str) -> Option<&'a mut Value> {
    fields
        .iter_mut()
        .find(|(key, _)| matches!(key, Key::String(key) if key == name))
        .map(|(_, value)| value)
}

/// The configured decompression bound, `[limits] max_decompressed` and
/// `[limits] max_decompression_ratio`, as [`cap_decompression`] clamps a call to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decompression {
    /// The most octets a part held whole decompresses to.
    pub bytes: u64,
    /// The most octets of output per octet of input.
    pub ratio: u64,
}

/// `rule:core-classes/decompression-bound` on the host's side of a call: the bound of a
/// `Novis\Spreadsheet\Engine::readerOpen` or `::templateOpen` call, or a
/// `Novis\Pdf\Engine::merge`, `::read` or `::readText` call, its second argument, gets
/// `maxBytes` and `maxRatio` set to the smaller of its own and `in_force`'s, the calling request's
/// `[limits]`. A bound with neither gets `in_force`'s, so a call can lower the bound and never
/// raise it. A `Novis\Pdf\Engine::render` or `::renderFirst` call gets the same in its options'
/// `bound`, which every placed PDF asset decodes under, and a missing or `null` one becomes
/// `in_force`'s. Any other call is left as it is, and a bound that is not a shape is left for the
/// manifest's conversion to refuse.
pub fn cap_decompression(class: &str, method: &str, args: &mut [Value], in_force: Decompression) {
    let bound = match (class, method) {
        (SPREADSHEET_CLASS, "readerOpen" | "templateOpen")
        | (PDF_CLASS, "merge" | "read" | "readText") => args.get_mut(1),
        (PDF_CLASS, "render") => render_bound(args.get_mut(0)),
        (PDF_CLASS, "renderFirst") => match args.get_mut(0) {
            Some(Value::Array(fields)) => render_bound(field(fields, "document")),
            _ => None,
        },
        _ => None,
    };
    if let Some(Value::Array(fields)) = bound {
        clamp(fields, "maxBytes", in_force.bytes);
        clamp(fields, "maxRatio", in_force.ratio);
    }
}

/// The `bound` of a PDF `document`'s options, set to an empty shape first when it is missing or
/// `null`, so [`clamp`] fills both halves.
fn render_bound(document: Option<&mut Value>) -> Option<&mut Value> {
    let Some(Value::Array(fields)) = document else {
        return None;
    };
    let Some(Value::Array(options)) = field(fields, "options") else {
        return None;
    };
    if field(options, "bound").is_none() {
        options.push((Key::String("bound".to_owned()), Value::Null));
    }
    let bound = field(options, "bound")?;
    if matches!(bound, Value::Null) {
        *bound = Value::Array(Vec::new());
    }
    Some(bound)
}

fn cap_source(source: &mut Value, in_force: u64) {
    let Value::Array(fields) = source else {
        return;
    };
    if field(fields, "data").is_some() {
        clamp(fields, "maxPixels", in_force);
    }
}

/// Sets the whole number under `key` to the smaller of its own and `in_force`, or to `in_force`
/// when it is missing or `null`.
fn clamp(fields: &mut Vec<(Key, Value)>, key: &str, in_force: u64) {
    match field(fields, key) {
        None => fields.push((Key::String(key.to_owned()), Value::Uint(in_force))),
        Some(cap @ Value::Null) => *cap = Value::Uint(in_force),
        Some(Value::Uint(cap)) => *cap = (*cap).min(in_force),
        Some(cap @ Value::Int(_)) => {
            if let Value::Int(asked) = *cap
                && let Ok(asked) = u64::try_from(asked)
            {
                *cap = Value::Uint(asked.min(in_force));
            }
        }
        Some(_) => {}
    }
}

fn cap_overlays(plan: &mut Value, in_force: u64) {
    let Value::Array(fields) = plan else {
        return;
    };
    let Some(Value::Array(overlays)) = field(fields, "overlays") else {
        return;
    };
    for (_, overlay) in overlays {
        if let Value::Array(fields) = overlay
            && let Some(source) = field(fields, "source")
        {
            cap_source(source, in_force);
        }
    }
}

/// Records every built-in component's digest with [`nvs_config::cache::set_built_in`], so each
/// `env_hash` this process computes afterwards folds them in. The binary calls it once, first.
pub fn fold_into_env_hash() -> bool {
    nvs_config::cache::set_built_in(&[IMAGE_SHA256, INTL_SHA256, PDF_SHA256, SPREADSHEET_SHA256])
}
