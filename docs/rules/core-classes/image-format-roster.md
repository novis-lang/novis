The roster is a closed table, each row naming what decodes it, what encodes it, and what implements
both. JPEG, PNG with APNG, WebP, GIF and AVIF decode and encode. JPEG XL decodes only, because no
Rust encoder exists and the C library is not admitted. SVG and PDF decode only, by rasterisation, in
a second wave. TIFF, BMP, TGA, ICO, PNM, HEIC and gd's own formats do neither.

Each absence is a decision rather than an omission. HEIC is out because the codec is
patent-encumbered and has no Rust decoder, and iOS browsers upload JPEG by default. gd's own formats
are out because nothing outside gd reads them.

Every crate is pure Rust and on the licence allowlist, with exactly one C library — the lossy WebP
encoder — admitted because it runs only inside the sandbox. AVIF encoding costs seconds of CPU per
image and is documented as queue work rather than a request-path call.

**Not shipped.** The image component's `run` decodes JPEG, PNG, WebP, GIF, AVIF and JPEG XL inside
the guest and refuses to encode JPEG XL (`crates/nvs-ext/tests/image_decode.rs`). AVIF decodes
through `rav1d` on one thread (`extensions/image/src/avif.rs`). JPEG, PNG, WebP, GIF and AVIF
encode (`extensions/image/src/encode.rs`, `crates/nvs-ext/tests/image_pipeline.rs`): lossy WebP
through the libwebp that `extensions/image/build.rs` links, and AVIF through `ravif` on one thread.
APNG is neither decoded as an animation nor encoded.
