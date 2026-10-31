Existing PDFs are **placed, merged and read in `Novis\Pdf`**; only rasterising a page is the image
component's (`rule:core-classes/pdf-page-is-an-image-source`).

- **Placing.** `<img src="terms.pdf#page=2">` draws that page of an asset as vector content, and
  `@page { background-image: url(letterhead.pdf) }` draws it behind every page. Only the page's
  appearance is kept: its links, form fields and bookmarks are not.
- **Merging.** `Pages::merge` takes PDFs, each with an optional list of page numbers, and returns one
  PDF. Merging, splitting, reordering and removing pages are that one call: a list that repeats a
  page repeats it, and a list that leaves one out removes it.
- **Reading.** `Reader::open` returns the page count, the metadata (title, author, subject, keywords,
  creator, producer), the attachments by name, and `invoiceXml()`, the Factur-X XML of a received
  invoice or `null`. `text($page)` returns a page's text in the order the file stores it, through
  `hayro`'s interpreter and each font's Unicode map. It does not rebuild tables or columns.

**An encrypted source throws `ParseError`** in all three, as the image side does
(`rule:core-classes/pdf-decode-refusals`), and so does a source that does not parse. A page number
outside its document throws `LogicError` naming the page count. **A stream that decompresses past
`rule:core-classes/decompression-bound` throws `ParseError`** in all three, naming the half of the
bound that stopped it and the `[limits]` key that raises it. Each job is one call: `Reader::open`
reads the count, the metadata and the attachments together, and `text` takes the bytes again with only
the pages it asks for.

These are the jobs a program does with the PDFs it receives and sends — attach the terms, put a
letterhead under a letter, read back an uploaded invoice — and each was a reason to install a second
tool. They live beside the writer because they return text and bytes, never pixels.

- **Memory:** per call, the source bytes and what the call returns, inside the request's cap; a merge
  holds its sources and the merged file at once.

**On disk.** The component places, merges and reads (`extensions/pdf/src/read.rs`,
`extensions/pdf/src/merge.rs`, `extensions/pdf/src/layout/image.rs`), pinned by
`crates/nvs-ext/tests/pdf_read.rs`, and `extensions/pdf/nvs/Pages.nvs` and `Reader.nvs` are the
classes a program calls, pinned by `tests/conformance/novis/pdf-merge-takes-chosen-pages.nvst` and
`pdf-reader-returns-an-invoice-xml-and-text.nvst`. A written page list such as `[3, 1]` crosses
because the crossing takes a non-negative `int` as a `uint` (`crates/nvs-ext/src/convert.rs`).

**Declined:**
- **Rebuilding tables and columns from extracted text.** That is a layout guess, right for some files
  and wrong for others; the file's own order is the honest answer.
- **Decrypting a source with a password.** A received encrypted PDF is opened by the person it was
  sent to, and the password is theirs.
- **Editing a page's content in place.** It is a second, imperative model of the file
  (`rule:core-classes/pdf-one-engine`); a new page is HTML.
