A PDF is a **decode-only format of the image component**, not a member of the PDF generation package:
a writer and an interpreter share no code, and a raster produced there would have to recross the
boundary to enter the pipeline that is the whole point of loading one. No new entry point is added;
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
sandbox beside the codecs, with substitutes for the standard-14 fonts embedded in the component's data
section so an unembedded-font document renders. It is linked only once it builds for the component's
`wasm32` target and its dependency tree passes `deny.toml`. The sandbox is where a parser this large
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

**Not shipped.** There is no PDF decoder in the image component. `Format::Pdf` exists, and `run`
refuses to encode it (`extensions/image/src/encode.rs`).

**Declined:**
- **A `rasterize` entry on the PDF generation package.** The writer and an interpreter share nothing,
  and every raster would recross the boundary as bytes to reach `resize`.
- **pdfium or MuPDF compiled to wasm.** pdfium is a large C++ build no other component needs a
  toolchain for, and MuPDF is AGPL and outside `deny.toml`, while a pure-Rust interpreter exists.
