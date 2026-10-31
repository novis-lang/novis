//! The built-in components: `.nvsx` bytes embedded in the binary
//! (`rule:packaging/the-first-party-components-are-built-in`).
//!
//! `build.rs` compiles each component's crate under `extensions/` for `wasm32-wasip2` and packs it
//! with [`crate::pack::pack`], compiled into the build script with `#[path]`, so the bytes here are
//! the ones `nvs ext build` would write from the same inputs. It also writes each one's SHA-256,
//! which is the component's pin: an `[[extension]]` entry carries a `sha256`, and a built-in
//! component has none because the binary is its pin.
//!
//! The bytes are only data here. Nothing reads or compiles them until a component is loaded.
//!
//! [`cap_pixels`] is the host's half of the image component's pixel cap, and
//! [`cap_decompression`] of the spreadsheet component's decompression bound: each clamps a call's
//! arguments to what the request's snapshot gives, before they cross.

use crate::convert::{Key, Value};

/// The image component, `Novis\Image\Codec`, as a packed `.nvsx`.
pub static IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/image.nvsx"));

/// The SHA-256 of [`IMAGE`], taken when the binary was built.
pub const IMAGE_SHA256: [u8; 32] = *include_bytes!(concat!(env!("OUT_DIR"), "/image.sha256"));

/// The class the image component declares.
pub const IMAGE_CLASS: &str = "Novis\\Image\\Codec";

/// The intl component, `Novis\Intl\Icu`, as a packed `.nvsx`.
pub static INTL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/intl.nvsx"));

/// The SHA-256 of [`INTL`], taken when the binary was built.
pub const INTL_SHA256: [u8; 32] = *include_bytes!(concat!(env!("OUT_DIR"), "/intl.sha256"));

/// The class the intl component declares.
pub const INTL_CLASS: &str = "Novis\\Intl\\Icu";

/// The PDF component, `Novis\Pdf\Engine`, as a packed `.nvsx`.
pub static PDF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pdf.nvsx"));

/// The SHA-256 of [`PDF`], taken when the binary was built.
pub const PDF_SHA256: [u8; 32] = *include_bytes!(concat!(env!("OUT_DIR"), "/pdf.sha256"));

/// The class the PDF component declares.
pub const PDF_CLASS: &str = "Novis\\Pdf\\Engine";

/// The spreadsheet component, `Novis\Spreadsheet\Engine`, as a packed `.nvsx`.
pub static SPREADSHEET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/spreadsheet.nvsx"));

/// The SHA-256 of [`SPREADSHEET`], taken when the binary was built.
pub const SPREADSHEET_SHA256: [u8; 32] =
    *include_bytes!(concat!(env!("OUT_DIR"), "/spreadsheet.sha256"));

/// The class the spreadsheet component declares.
pub const SPREADSHEET_CLASS: &str = "Novis\\Spreadsheet\\Engine";

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
/// raise it. Any other call is left as it is, and a bound that is not a shape is left for the
/// manifest's conversion to refuse.
pub fn cap_decompression(class: &str, method: &str, args: &mut [Value], in_force: Decompression) {
    let bounded = match class {
        SPREADSHEET_CLASS => matches!(method, "readerOpen" | "templateOpen"),
        PDF_CLASS => matches!(method, "merge" | "read" | "readText"),
        _ => false,
    };
    if !bounded {
        return;
    }
    if let Some(Value::Array(fields)) = args.get_mut(1) {
        clamp(fields, "maxBytes", in_force.bytes);
        clamp(fields, "maxRatio", in_force.ratio);
    }
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
