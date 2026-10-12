Beyond the pipeline, each remaining job is exactly one entry point.

**Comparison** decodes both inputs, refuses a size mismatch, and answers whether they are identical,
how many pixels differ, the maximum delta, a structural similarity score, and optionally a rendered
diff image for a test report. There is no assertion member: a composite assertion is an ordinary
method, so a test asserts on the comparison's own numbers and the failure ledger records it.
**Perceptual, difference and average hashing** plus a distance function serve near-duplicate
detection. **Placeholders** and **palette extraction** serve the front end.

**Text** renders over a font supplied as bytes, with no system font lookup, since the guest has no
filesystem. It is shaped by `harfrust` in runs the Unicode bidirectional algorithm orders, so
ligatures, Arabic joining, Indic conjuncts and right-to-left lines are drawn as the font defines
them. **QR codes** are one entry point, because a QR code is an image a web application
produces daily. **SVG** is an input format whose declared canvas counts against the pixel cap and
whose scripts and foreign objects are ignored. An image inside it is read only from a `data:` URI,
and **an external reference throws** `RuntimeError` naming it, because there is nothing in the guest
to resolve one with. Rasterising is how an uploaded SVG is displayed safely; sanitising one would be
the repair that is refused everywhere else.

Comparison is the component's `compare` export, whose module doc
`extensions/image/src/compare.rs` says what a delta is and how the SSIM and the rendered diff are
made, and `Image::compare` returning a `Diff` in `extensions/image/nvs/`, which compares an `Image`
with no `format` step as PNG so no lossy re-encode moves a pixel (`crates/nvs-ext/tests/image_analysis.rs`,
`tests/conformance/novis/`). Hashing is the component's `hash` export, whose module
doc `extensions/image/src/hash.rs` defines the three kinds, each its own fixed length so two kinds
never compare, and is written in the component with no hashing crate linked; `Image::hash` and
`Image::HashKind`; and `Image::hashDistance` as Novis source, counting differing bits and throwing a
`LogicError` for two lengths. Placeholders and the palette are the component's
`placeholder` and `palette` exports, whose module doc `extensions/image/src/summary.rs` defines
both placeholder formats and how the palette clusters `color_quant`'s colours, and
`Image::placeholder`, `Image::PlaceholderKind` and `Image::palette`. QR codes are the
component's `qr` export over the `qrcode` crate, whose module doc `extensions/image/src/qr.rs`
says how `size`, `margin` and the level make the image and what each default is, and
`QrCode::render` and `QrCode::Level`; the host checks read each code back with `rqrr`. Text is
the `text` step and the `text` source, shaped by `harfrust` and drawn by `ab_glyph`, whose
module doc `extensions/image/src/text.rs` says how a size, a line, shaping, `maxWidth` and `align` set the
text and what the box is, and `Image::text`, `Image::measureText` and `Font::fromBytes`, which
checks only the file's first four bytes and calls no export. SVG decodes through `resvg` with an
image resolver that reads only `data:` URIs (`extensions/image/src/svg.rs`), and
`crates/nvs-ext/tests/image_decode.rs` and
`tests/conformance/novis/image-an-svg-opens-as-an-image.nvst` throw for a path and an address.
