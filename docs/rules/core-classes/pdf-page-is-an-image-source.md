A PDF is a **decode-only format of the image component**, not a member of the PDF generation package:
a writer and an interpreter share no code, and a raster produced there would have to recross the
boundary to enter the pipeline that is the whole point of loading one. No new entry point is added;
the format enum simply gains a case.

`open` gains two options meaningful for a paged source: a 1-based **page** and a **dpi**. One call
rasterises exactly that page, so a document's pages become images by iterating and cost stays
proportional to what was asked for. The pixel price is the page box scaled to the requested
resolution, counted against the pixel cap before any buffer is allocated
(`rule:core-classes/image-pixel-cap`), and a page beyond the document throws naming the page count.
The header reader parses the cross-reference table and page tree and never a content stream, so
counting pages allocates no pixel.

Rasterising is a pure function of the bytes and the options — no clock, no I/O, no system fonts — so
equal input gives byte-equal output. The interpreter is pure Rust, compiled into the sandbox beside
the codecs, with substitutes for the standard fonts embedded so an unembedded-font document renders.

**Not shipped.** There is no image component in the tree, so there is nothing for this to be a format
of.
