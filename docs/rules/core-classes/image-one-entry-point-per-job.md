Beyond the pipeline, each remaining job is exactly one entry point.

**Comparison** decodes both inputs, refuses a size mismatch, and answers whether they are identical,
how many pixels differ, the maximum delta, a structural similarity score, and optionally a rendered
diff image for a test report. There is no assertion member: a composite assertion is an ordinary
method, so a test asserts on the comparison's own numbers and the failure ledger records it.
**Perceptual, difference and average hashing** plus a distance function serve near-duplicate
detection. **Placeholders** and **palette extraction** serve the front end.

**Text** renders over a font supplied as bytes, with no system font lookup, since the guest has no
filesystem. **QR codes** are one entry point, because a QR code is an image a web application
produces daily. **SVG** is an input format whose declared canvas counts against the pixel cap, whose
scripts and foreign objects are ignored, and where **no external reference is ever resolved** —
there is nothing in the guest to resolve it with. Rasterising is how an uploaded SVG is displayed
safely; sanitising one would be the repair that is refused everywhere else.

**Not shipped.** Comparison is on disk: the component's `compare` export, whose module doc
`extensions/image/src/compare.rs` says what a delta is and how the SSIM and the rendered diff are
made, and `Image::compare` returning a `Diff` in `extensions/image/nvs/`, which compares an `Image`
with no `format` step as PNG so no lossy re-encode moves a pixel (`crates/nvs-ext/tests/image_analysis.rs`,
`tests/conformance/novis/`). Hashing is on disk too: the component's `hash` export, whose module
doc `extensions/image/src/hash.rs` defines the three kinds, each its own fixed length so two kinds
never compare, and is written in the component with no hashing crate linked; `Image::hash` and
`HashKind`; and `Image::hashDistance` as Novis source, counting differing bits and throwing a
`LogicError` for two lengths. Placeholders and the palette are on disk: the component's
`placeholder` and `palette` exports, whose module doc `extensions/image/src/summary.rs` defines
both placeholder formats and how the palette clusters `color_quant`'s colours, and
`Image::placeholder`, `PlaceholderKind` and `Image::palette`. QR codes are on disk: the
component's `qr` export over the `qrcode` crate, whose module doc `extensions/image/src/qr.rs`
says how `size`, `margin` and the level make the image and what each default is, and
`QrCode::render` and `QrLevel`; the host checks read each code back with `rqrr`. Text and SVG
are not.
