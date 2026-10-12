PDF is on the image component's format roster **decode only**, beside SVG, as `Format`'s `Pdf` case
with no entry point of its own: a page opens through the same plan/terminal shape and the same closed
set of exports as every other format (`rule:core-classes/image-format-roster`,
`rule:core-classes/pdf-page-is-an-image-source`). The image component compiles the PDF interpreter,
`hayro`, into its sandbox.

Rasterising is not a member of `Novis\Pdf`. A raster produced in the PDF component would have to
recross the boundary to enter the image pipeline that is the whole point of loading one. Everything a
program does with a PDF that ends in text or bytes — reading its text, metadata and attachments,
merging and splitting pages, placing a page in a document — is `Novis\Pdf`'s
(`rule:core-classes/pdf-reader-and-merge`), which compiles its own copy of `hayro` into its own
sandbox.

**On disk.** `extensions/image/src/pdf.rs` decodes a page, and a page opens as an image
(`tests/conformance/novis/image-a-pdf-page-opens-as-an-image.nvst`,
`crates/nvs-ext/tests/image_decode.rs`).

The usual alternative is ImageMagick delegating to Ghostscript — an installed, unsandboxed
interpreter with an RCE history long enough that ImageMagick's stock policy ships with the PDF coder
disabled. A PDF interpreter is a strictly larger hostile-bytes case than any format already on the
roster, and the sandbox is where a parser that size belongs.

- **Memory:** the interpreter is in the image component's bytes, and the PDF component's reader
  carries a second copy, so the binary holds it twice; each request instantiates only the component
  it calls.

**Declined:**
- **Leaving PDF rasterising to the third-party channel.** Thumbnailing and counting the pages of an
  uploaded PDF is a universal job, and owning one sandboxed way to do it removes the host-interpreter
  shell-out as a class.
- **One component for both jobs.** A program thumbnailing an upload would instantiate a layout engine
  it never calls, and a render would carry the codecs; the second copy of `hayro` costs binary size,
  which is the cheaper account.
