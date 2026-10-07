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

/// The image component, `Novis\Image\Codec`, as a packed `.nvsx`.
pub static IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/image.nvsx"));

/// The SHA-256 of [`IMAGE`], taken when the binary was built.
pub const IMAGE_SHA256: [u8; 32] = *include_bytes!(concat!(env!("OUT_DIR"), "/image.sha256"));

/// Records every built-in component's digest with [`nvs_config::cache::set_built_in`], so each
/// `env_hash` this process computes afterwards folds them in. The binary calls it once, first.
pub fn fold_into_env_hash() -> bool {
    nvs_config::cache::set_built_in(&[IMAGE_SHA256])
}
