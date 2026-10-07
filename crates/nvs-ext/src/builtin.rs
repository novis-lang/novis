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
//! [`cap_pixels`] is the host's half of the image component's pixel cap: it clamps a call's
//! arguments to the cap the request's snapshot gives, before they cross.

use crate::convert::{Key, Value};

/// The image component, `Novis\Image\Codec`, as a packed `.nvsx`.
pub static IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/image.nvsx"));

/// The SHA-256 of [`IMAGE`], taken when the binary was built.
pub const IMAGE_SHA256: [u8; 32] = *include_bytes!(concat!(env!("OUT_DIR"), "/image.sha256"));

/// The class the image component declares.
pub const IMAGE_CLASS: &str = "Novis\\Image\\Codec";

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

fn cap_source(source: &mut Value, in_force: u64) {
    let Value::Array(fields) = source else {
        return;
    };
    if field(fields, "data").is_none() {
        return;
    }
    match field(fields, "maxPixels") {
        None => fields.push((Key::String("maxPixels".to_owned()), Value::Uint(in_force))),
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
    nvs_config::cache::set_built_in(&[IMAGE_SHA256])
}
