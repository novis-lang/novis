The roster is a closed table, each row naming what decodes it, what encodes it, and what implements
both. JPEG, PNG with APNG, WebP, GIF and AVIF decode and encode. JPEG XL decodes only, because no
Rust encoder exists and the C library is not admitted. SVG and PDF decode only, by rasterisation.
TIFF, BMP, TGA, ICO, PNM, HEIC and gd's own formats do neither.

Each absence is a decision rather than an omission. HEIC is out because the codec is
patent-encumbered and has no Rust decoder, and iOS browsers upload JPEG by default. TIFF is a large
reader with a long CVE history, for a format web browsers do not display. gd's own formats are out
because nothing outside gd reads them. A third-party extension may carry any of them.

Every crate is pure Rust and on the licence allowlist, with exactly one C library — the lossy WebP
encoder — admitted because it runs only inside the sandbox. It is a build cost carried for one
feature, and it is dropped once a pure-Rust lossy WebP encoder exists. Resampling is
`fast_image_resize`, for its SIMD paths, and the guest is built with `simd128`
(`extensions/image/.cargo/config.toml`). AVIF encoding costs seconds of CPU per image and is
documented as queue work rather than a request-path call.

**Partly on disk.** The image component's `run` decodes JPEG, PNG, WebP, GIF, AVIF and JPEG XL inside
the guest and refuses to encode JPEG XL (`crates/nvs-ext/tests/image_decode.rs`). AVIF decodes
through `rav1d` on one thread (`extensions/image/src/avif.rs`). JPEG, PNG, WebP, GIF and AVIF
encode (`extensions/image/src/encode.rs`, `crates/nvs-ext/tests/image_pipeline.rs`): lossy WebP
through the libwebp that `extensions/image/build.rs` links, and AVIF through `ravif` on one thread.
`Image::info`'s `frames` is an animated input's real count, read off its header
(`extensions/image/src/info.rs`). Every frame of a GIF, an APNG, an animated WebP, an AVIF image
sequence and a JPEG XL animation decodes with its delay (`extensions/image/src/decode.rs`,
`extensions/image/src/avif.rs`). APNG is not encoded. SVG decodes through `resvg` with an image
resolver that reads only `data:` URIs and throws `RuntimeError` naming any other `href`
(`extensions/image/src/svg.rs`); its `<text>` is drawn with the twelve Liberation faces and never a
system font, in `Novis\Image` and in a `Novis\Pdf` render (`extensions/pdf/src/fonts.rs`). A
PDF page rasterises through `hayro` (`extensions/image/src/pdf.rs`).
