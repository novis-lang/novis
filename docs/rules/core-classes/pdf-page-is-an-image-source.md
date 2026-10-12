Rasterising a PDF page is a **decode-only format of the image component**, not a member of
`Novis\Pdf`: a raster produced there would have to recross the boundary to enter the pipeline that is
the whole point of loading one. Reading a PDF's text, metadata and attachments, and merging pages, are
`Novis\Pdf`'s (`rule:core-classes/pdf-reader-and-merge`), because they return text and bytes rather
than pixels. No new entry point is added;
the format enum simply gains a case.

`open` gains two options meaningful for a paged source: a 1-based **page**, default `1`, and a
**dpi**, default `150`. One call rasterises exactly that page, so a document's pages become images by
iterating and cost stays proportional to what was asked for. The pixel price is the page box scaled
to the requested resolution, `ceil(width_pt × dpi / 72) × ceil(height_pt × dpi / 72)`, counted
against the pixel cap before any buffer is allocated (`rule:core-classes/image-pixel-cap`), so the
same bytes refused at one dpi pass at a lower one. A page beyond the document throws a `RuntimeError`
naming the page count.

`info` parses the cross-reference table and page tree and never a content stream: `format` is
`Format::Pdf`, `frames` is the **page count**, and `width` and `height` are the first page's box in
whole points. Counting pages is one header-only call that allocates no pixel.

Rasterising is a pure function of the bytes and the options — no clock, no I/O, no system fonts — so
equal input gives byte-equal output. The interpreter is **hayro**, pure Rust, compiled into the
sandbox beside the codecs, with the Liberation fonts (`rule:core-classes/pdf-default-fonts`) standing
in for the standard-14 fonts so an unembedded-font document renders. It is linked only once it builds
for the component's `wasm32` target and its dependency tree passes `cargo deny` over the component's
own graph. There is no fallback interpreter. The sandbox is where a parser this large
belongs: the hostile-bytes surface it leaves is contained by the sandbox's memory cap and epoch
deadline like every other decoder's, and the build gains no C.

The decoder's fixtures owe, asserted inside the guest: a 100-page document whose `info` gives
`frames: 100` and `Format::Pdf` with no pixel buffer allocated and one host-to-guest call; a page box
over the cap at one `dpi` that passes at a lower one; `{page: 7}` of a three-page document throwing
with the count 3; the same page rasterised twice byte-equal; and a benchmark of one text page and one
image-heavy page, guest against the same crate native.

- **Memory:** a request that opens a PDF holds the encoded document and one page's raster of at most
  the pixel cap, 96 MiB at the default cap in RGBA8, inside the extension's existing store limits, and
  nothing outlives the request. The interpreter and the font substitutes grow the binary by
  single-digit megabytes. CPU is the request's epoch deadline, and bulk rasterising is queue work.

A page is drawn on white, so every pixel is opaque, and a side may be at most 65535 pixels, which is
`hayro`'s renderer's own bound. Symbol and ZapfDingbats are drawn in Liberation Serif and Sans, which
have few of their glyphs. No predefined CMap is carried, so CJK text in an encoding that names one is
not drawn; carrying them is `hayro`'s `embed-cmaps` feature. The decoder is
`extensions/image/src/pdf.rs`, and its fixtures are `crates/nvs-ext/tests/image_decode.rs`.

The benchmark is two jobs of `crates/nvs-ext/benches/image_guest.rs`: an A4 page of text in two
unembedded standard fonts, and an A4 page drawing an 800x600 JPEG six times, each rasterised at
150 dpi. Its figures are in `benches/results/image-guest.json`. Measured on x86_64 Windows, with only
the 16-byte size crossing back, the text page is 13 ms in the guest against 9 ms native, and the
image page 53 ms against 31 ms. Returning the page's raw RGBA8 adds about 390 ms to either, because
a `list<u8>` crosses as one value per byte (`rule:packaging/a-guest-runs-under-the-requests-budget`).
The text page's pixels agree with native to within one level per channel, at the anti-aliased
edges of its glyphs, and the image page's byte for byte (`crates/nvs-ext/tests/image_guest.rs`).

**Declined:**
- **A `rasterize` member in `Novis\Pdf`.** Every raster would recross the boundary as bytes to reach
  `resize`.
- **pdfium or MuPDF compiled to wasm, as the interpreter or as a fallback.** pdfium in wasm needs
  Emscripten and has no WASI build, and MuPDF is AGPL and outside `deny.toml`, while a pure-Rust
  interpreter exists.
