//! Links the prebuilt libwebp into the wasm build (`rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci`).
//!
//! `libwebp/libwebp.a` is compiled for `wasm32-wasip2` by `bun nv webp-lib`, so only a wasm build
//! links it. A host build of this crate, which runs the codec core's tests, links nothing, and its
//! lossy WebP arm returns `Runtime` (`src/encode.rs`).

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=libwebp/libwebp.a");
    if env::var("CARGO_CFG_TARGET_FAMILY").as_deref() != Ok("wasm") {
        return;
    }
    let dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it")).join("libwebp");
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=static=webp");
}
