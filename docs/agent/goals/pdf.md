---
milestone: M17
position: last
---
# Loop goal 199 — `Novis\Pdf` is built into every binary: HTML and CSS to paginated PDF, tables that survive a page break, headers from a Novis function, and PDF and SVG as image sources

The third component built into every `nvs` binary, and the image component's second wave beside it.
`Novis\Pdf` turns HTML, a documented CSS subset and inline SVG into a paginated, tagged, byte-equal PDF
with no I/O. It places pages of existing PDFs, merges them, reads their text, metadata and attachments,
and writes PDF/A, PDF/UA-1, Factur-X and password-protected files. `Novis\Image` gains SVG and PDF
decoding, real text shaping and barcodes.

```nvs
use Novis\Pdf\{Document, Page, Pages, Reader, Standard};   // names are the record's; this is a sketch

$result = Document::fromHtml(html`<h1>Invoice <?= $number ?></h1><table>...</table>`)
    ->asset('logo.png', $logo)                      // every image, font, stylesheet and PDF is bytes by name
    ->header(fn(Page $p) => $p->number === 1 ? html`` : html`<img src="logo.png"> <?= $p->string('chapter') ?>`)
    ->footer(fn(Page $p) => html`Carried forward: <?= Money::sum($p->marksUpTo('amount')) ?>
                                 Page <?= $p->number ?> of <?= $p->count ?>`)
    ->standard(Standard::PdfA3b)
    ->invoiceXml($facturX)                          // Factur-X: one XML attachment, PDF/A-3 only
    ->render();                                     // ->bytes, ->pageCount, ->dropped, ->missingGlyphs, ->overflows

$all  = Pages::merge([Pages::of($result->bytes), Pages::of($terms, [1, 2])]);
$text = Reader::open($upload)->text(1);
$png  = Image::open($upload, {page: 1, dpi: 150})->resize(...)->encode();
```

Its scope is § *Standing decisions*, which the user settled question by question when the goal was
written, under one rule of theirs: **every common case is in, an edge case is in when it is easy, and a
hard edge case almost nobody needs stays out.** Spreadsheets (M17 Part 3) are not in it.

## Why here

It stands on finished work only: the extension system, the build script and embedding path goals
`ext-image` and `ext-intl` built for two components, `html5ever` with its 512-element cap
(`rule:core-classes/html-parsing`), and ``html`…` `` templates (`rule:core-classes/html-template`). Its
files are `extensions/image`, a new `extensions/pdf`, `wit/` and `nvs-ext`'s build and load tables, and
it shares none of them with goal `ldap` in front of it. Nothing waits for it, so it goes at the end.
The user put it there.

It carries `position: last` because every goal on the chain is pinned there.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands what
makes it untrue, and not before:

- **By Stage 2's record**, because they state decisions this goal reverses:
  `docs/rules/core-classes/pdf-one-engine.md` (the builder paragraph, the headless-browser backend,
  the crate list); `pdf-output-is-inert.md` ("never … embedded files", the fidelity-backend
  paragraph); `pdf-render-has-no-io.md` ("a scheme … throws", which `data:` now passes);
  `pdf-decode-refusals.md` and `docs/rules/packaging/pdf-decoding-ships-in-the-image-package.md`
  (text extraction, merging and splitting are no longer third-party; the pdfium fallback is gone);
  `docs/plan/m17.md` Parts 1 and 2 (now this goal's, through a `## milestone: M17` wrap section; Part 3
  stays unscheduled).
- **By the stage that lands the behaviour:** every "Not shipped" paragraph of the seven `pdf-*`
  fragments; `docs/rules/core-classes/image-format-roster.md`'s "Not shipped" list and
  `image-one-entry-point-per-job.md:35` ("SVG is not"), at Stage 3;
  `docs/rules/packaging/the-first-party-components-are-built-in.md`'s count of two, at Stage 5.

The search that closes the stage, run after Stage 16:
`grep -rn -i "pdf\|svg\|headless\|fidelity\|two first-party" docs/rules docs/reference docs/plan/m17.md extensions/image/src`,
read line by line. Every hit is true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and
are regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goals `ext-image`,
`ext-image-analysis`, `ext-intl` and `ldap` turned green stays green; every image fixture decodes as it
does today, and `Image::text` on Latin text gives the same pixels until Stage 4 changes the shaper on
purpose, with the fixture's expected output rewritten in that slice and the reason in its commit.

## Stage 2 — the record, its fragments, `wit/pdf.wit` and the font licence

**Does:** Writes the `Novis\Pdf` record from § *Standing decisions*, the fragments it creates and
rewrites, the WIT interface the component exports, and admits the default fonts' licence.

One file set: `docs/decisions/`, `docs/rules/core-classes/`, `docs/rules/packaging/`, `wit/pdf.wit`
(new), `wit/image.wit`, `tools/nv/cmd/gen-attribution.ts:66`, `deny.toml:38`.

- **The record**, this goal's one slot, written first. Every class, member and options shape under
  `Novis\Pdf`, the new `Novis\Image` members (barcodes, SVG output, `open`'s `page` and `dpi`), the
  documented CSS subset as a table, the `Page` value and its marks, the table rules, the standards,
  the reader and `merge`, encryption, the error kinds, the tradeoffs, and the user's calls as decided.
  It amends ADRs 0121, 0128 and 0247 in its front matter, and records the headless-browser backend,
  form filling, signatures, PDF417 and Aztec as declined.
- **The fragments.** It rewrites the seven `pdf-*` fragments to the new decisions, and creates at
  least these, under the ids the Stage 2 check reads: `core-classes/pdf-css-subset-table` (the
  subset as data a test reads), `core-classes/pdf-tables-across-pages`,
  `core-classes/pdf-header-is-a-page-function`, `core-classes/pdf-factur-x-is-the-one-attachment`,
  `core-classes/pdf-reader-and-merge`, `core-classes/pdf-default-fonts`,
  `core-classes/image-barcode-roster`.
- **`wit/pdf.wit`**, batch-shaped like `wit/intl.wit`: `render` (HTML, the asset map, options, and
  the per-page header and footer markup when a second pass carries it), `merge`, `read`, and the
  types they share. The two-pass header design (§ *Standing decisions*) is in it from the first write.
- **The proofs checks the record makes possible**: one `bun nv proofs --verify --group '<class>'`
  check per class it names, in both components, added to Stage 16 of `data/goals/pdf.json`. They
  are written here and not with the goal, because a `--group` check behind a goal whose list runs
  the whole-roster gate holds that goal (`gatesInFrontOfGroups` in `tools/nv/lib/chain.ts`).
- **The font licence.** OFL-1.1 is admitted for **font data only**: `PREFERENCE` in
  `gen-attribution.ts` and the attribution the binary ships carry the Liberation fonts' licence and
  copyright. `deny.toml` gains it only if a crate in a component's graph declares it.
- **A `cargo deny` over each component's own graph.** None exists today: `deny.toml`'s `[graph]
  targets` omit `wasm32-wasip2` and `extensions/` is outside the workspace. The slice that adds it
  adds its check to this record.
- **Pinned by** the Stage 2 check.

## Stage 3 — SVG and PDF open as images

**Does:** `Novis\Image` decodes SVG through `resvg` and rasterises a PDF page through `hayro`, with
`page` and `dpi` passed from `Image::open` to the guest.

One file set: `extensions/image/src/decode.rs:60` (`sniff`), `extensions/image/src/info.rs:35`,
`extensions/image/src/lib.rs:130` (`Input::Encoded`) and `:446` (the WIT mapping that drops `page` and
`dpi`), `extensions/image/nvs/Image.nvs:61` (`open`), `extensions/fonts/` (new: Liberation, shared with
Stage 5), `crates/nvs-ext/tests/image_decode.rs`.

- **The two prerequisites first** (ADR 0128 § 4): `hayro` builds for `wasm32-wasip2` and its graph
  passes the Stage 2 deny check. If either fails, the session records it and stops the PDF half with
  `BLOCKED`. There is no pdfium fallback (§ *Standing decisions*).
- **SVG** decodes with a `usvg` image resolver that only reads the asset bytes it is given; the
  default resolver reads files from disk and is never used. An external `href` throws.
- **PDF** follows `rule:core-classes/pdf-page-is-an-image-source` and
  `rule:core-classes/pdf-decode-refusals`: one page per `open`, priced by the pixel cap before a
  buffer exists, `info` counts pages without a pixel, a page past the end throws naming the count,
  an encrypted file throws `ParseError`, and the Liberation fonts stand in for the 14 standard fonts.
- **The bomb corpus** of ADR 0128's *Verification*: recursive XObjects, an xref loop, a stream-length
  lie and a deflate bomb each end in a throw or a contained trap.
- **Pinned by** the Stage 3 checks.

## Stage 4 — shaping and barcodes in `Novis\Image`

**Does:** `Image::text` shapes with `harfrust`, and `Novis\Image` renders 1D barcodes, DataMatrix,
and QR codes as SVG.

One file set: `extensions/image/src/text.rs`, `extensions/image/src/qr.rs`,
`extensions/image/src/barcode.rs` (new), `extensions/image/nvs/QrCode.nvs`,
`extensions/image/nvs/Barcode.nvs` (new), `crates/nvs-ext/tests/image_analysis.rs`,
`crates/nvs-ext/tests/image_barcode.rs` (new).

- **Shaping** replaces the first wave's Latin-only kerning (`text.rs:1`) with `harfrust`, with
  `unicode-bidi` for direction, so Arabic, Hebrew and Devanagari draw correctly. ADR 0120 § 9 and M17
  Part 1 placed it here.
- **Barcodes** (`core-classes/image-barcode-roster`): Code 128 and GS1-128, EAN-13, EAN-8,
  UPC-A, Code 39 and ITF-14 through `barcoders`, DataMatrix through `datamatrix`. Each renders as an
  SVG string or as any raster format the component encodes. A wrong check digit or a character the
  symbology cannot carry throws `LogicError` naming it.
- **QR codes gain SVG output**, and an EPC payload helper (GiroCode) builds the SEPA payment text in
  Novis and passes it to the QR code.
- **The tests read every code back**: QR with `rqrr`, as today; 1D and DataMatrix with a reader that
  is a dev-dependency of the test only.
- **Pinned by** the Stage 4 checks.

## Stage 5 — the PDF component is built in

**Does:** Builds `extensions/pdf` into every binary as the third component, and renders a paragraph of
HTML to a byte-equal PDF with the default fonts and no grant.

One file set: `extensions/pdf/` (new), `crates/nvs-ext/build.rs:115` (`COMPONENTS`),
`crates/nvs-ext/src/builtin.rs:18`, `crates/nvs-ext/src/load.rs:667`, `crates/nvs-ext/tests/pdf.rs`
(new), `docs/rules/packaging/the-first-party-components-are-built-in.md`.

- **The crate** in `extensions/image`'s shape: its own `[workspace]`, `cdylib` and `rlib`, release
  with LTO and `panic = "abort"`. It points `html5ever` at `vendor/html5ever` (`Cargo.toml:1151`), so
  the PDF component parses with the same patched copy and the same 512-element cap as `Core\Html`.
- **The asset map** (`rule:core-classes/pdf-render-has-no-io`): a name resolves against the map and
  nothing else; `data:` URIs decode in place; `file:`, `http(s):`, an absolute path or a name the map
  lacks throws naming it. Nothing in the guest can fetch.
- **The writer** is `krilla` over `pdf-writer`. No timestamp unless the program passes one, a fixed
  document id derived from the content, and no hash-map order or float formatting that varies between
  runs. A scanner over the output finds no `/JavaScript`, `/Launch` or `/OpenAction`.
- **The fonts** are `extensions/fonts/`'s Liberation set; `sans-serif`, `serif`, `monospace`, Arial,
  Helvetica, Times, Times New Roman and Courier map to them.
- **Pinned by** the Stage 5 checks.

## Stage 6 — the cascade

**Does:** Computes every element's style from `<style>` blocks and `style=""` attributes, and reports
every declaration it dropped.

One file set: `extensions/pdf/src/css/` (new), `extensions/pdf/tests/css.rs` (new).

- **Our own cascade** over `cssparser` and `selectors`: the selectors the subset lists, specificity,
  inheritance, `!important`, `var()`, `calc()`, `@media print` and `all` applied and `screen` not.
- **The subset is data**: `extensions/pdf/subset.json` (new), which
  `core-classes/pdf-css-subset-table` points to and nothing restates. It lists every property,
  value function, at-rule, selector and PDF-specific HTML attribute (`data-repeat`, `data-mark`,
  `<slot>`) with a one-line plain description. A test reads it: every property it lists parses, and
  every property it does not list is dropped and reported with its selector and source position
  (`rule:core-classes/pdf-css-subset`). Stage 15 generates the editor's data from the same file.
- **Pinned by** the Stage 6 checks.

## Stage 7 — text, blocks, lists and images

**Does:** Lays out block and inline content with `parley`, with every script, the font chain,
hyphenation, lists, images and inline SVG.

One file set: `extensions/pdf/src/layout/` (new: block, inline, text), `extensions/pdf/tests/layout.rs`
(new).

- **Text** through `parley` with `harfrust`, `skrifa` and ICU4X, its font discovery off: alignment
  including justify, `line-height`, indent, decoration, transform, spacing, `white-space`,
  `word-break`, `overflow-wrap`, `text-overflow: ellipsis`, `vertical-align`,
  `font-feature-settings`, bidi, CJK line breaking, and `hyphens: auto` through `hypher`.
- **The font chain**: `@font-face` from the asset map, then the defaults. A character no font has is
  drawn as the font's empty box and listed in `missingGlyphs` with the chain it tried; it never throws.
- **Boxes**: margin, padding, borders (solid, dashed, dotted, double), `border-radius`, sizes with
  min and max, `box-sizing`, background colours, images and gradients, `outline`, `opacity`,
  `inline-block`, `display: none`.
- **Lists and counters**: markers (decimal, roman, alpha, disc), `counter-reset`,
  `counter-increment`, `::before` and `::after` with `content`.
- **Images**: JPEG, PNG, GIF and WebP, `object-fit`, inline `<svg>` and an SVG `<img>` as vector
  through `krilla-svg`. A raster image is priced by the request's memory cap.
- **Pinned by** the Stage 7 checks.

## Stage 8 — flex, grid, floats and positioning

**Does:** Adds flex and grid through `taffy`, floats and `clear`, `relative`, `absolute` and `fixed`
positioning with `z-index`, and 2D transforms.

One file set: `extensions/pdf/src/layout/` (flex, grid, float, position), `extensions/pdf/tests/layout.rs`.

- **Flex and grid** break across pages only between items and rows; a single flex line or grid row is
  never split.
- **`position: fixed`** repeats the element on every page, which is how a watermark is written.
- **Pinned by** the Stage 8 checks.

## Stage 9 — pagination

**Does:** Breaks the laid-out document into pages under `@page`, with margin boxes, running elements,
counters, page breaks, links, bookmarks and metadata.

One file set: `extensions/pdf/src/paginate/` (new), `extensions/pdf/tests/paginate.rs` (new).

- **`@page`**: size (named sizes, landscape, custom), margins, `:first`, `:left`, `:right`, `:blank`,
  named pages through `page`.
- **The 16 margin boxes**, `counter(page)` and `counter(pages)`, `string-set` and `string()`, running
  elements through `position: running()` and `element()`.
- **Breaks**: `break-before`, `break-after`, `break-inside: avoid`, the `page-break-*` aliases,
  `orphans` and `widows`.
- **A table of contents**: `target-counter()` and `leader()`, laid out again until the page numbers
  stop changing, which `counter(pages)` needs anyway. A document whose numbers never settle throws
  after a fixed number of passes the record names.
- **Links and structure**: `<a href>` to `http(s):`, `mailto:` and `#anchor`; bookmarks from `h1` to
  `h6`; `<title>`, `<meta>` and `lang` become metadata.
- **Pinned by** the Stage 9 checks.

## Stage 10 — header and footer functions

**Does:** `Document::header`, `::footer` and `::slots` take a Novis function that is called once per
page with a `Page` value, and a second call places what it returns.

One file set: `extensions/pdf/nvs/` (new: `Document.nvs`, `Page.nvs`, `Rendered.nvs`),
`extensions/pdf/src/paginate/`, `crates/nvs-ext/tests/pdf.rs`.

- **`core-classes/pdf-header-is-a-page-function`**: the component lays out the body once and
  returns each page's facts; Novis calls the function once per page with no crossing; one more call
  places the header and footer markup in the page margins. The instance keeps the laid-out body
  between the two calls (`rule:packaging/a-fresh-instance-per-request`, `crates/nvs-ext/src/call.rs:1`).
- **`Page`** carries the number, the count, first and last, the named page, the `string-set` values
  and the marks: `data-mark="amount:12.50"` on any element, several separated by `;`, read with
  `marks(name)` for that page and `marksUpTo(name)` for every page up to it, as strings for Novis to
  add up exactly. One fixed attribute name, not `data-mark-<name>`, because the editor's custom data
  completes fixed names only (Stage 15).
- **The margin is fixed** by `@page margin` or the function's `height`, so a header never moves the
  body. Markup taller than its area is listed in `overflows`. Where a margin box and a function both
  fill one margin, the function's markup is drawn.
- **Pinned by** the Stage 10 checks.

## Stage 11 — tables across pages

**Does:** A table keeps its columns, repeats its header and footer, never cuts a row through a line,
closes its borders at every break and fills its running values.

One file set: `extensions/pdf/src/layout/table.rs` (new), `extensions/pdf/src/paginate/`,
`extensions/pdf/tests/tables.rs` (new).

The user's eight rules (`core-classes/pdf-tables-across-pages`), each a default with no
configuration:

1. **Column widths are computed once for the whole table**, so every page has the same columns.
2. **`thead` repeats at the top and `tfoot` at the bottom of every page the table is on.** A row
   inside either says when it shows with `data-repeat`: `every` (the default), `first`, `last`, or
   `continued` (only where the table goes on).
3. **A row that fits on a page is never split.** A row taller than a page splits between lines of
   text, never through a line, an image or a glyph, and each cell continues on the next page.
4. **Borders are closed at every break**: the table's outer border and collapsed cell borders are
   drawn on both sides. `box-decoration-break: slice` on the table restores CSS's open edge.
5. **A header is never alone.** `orphans` and `widows` on a table count rows, default 2, and a table
   never starts at a page bottom with only its `thead`.
6. **A `<tbody>` and a `rowspan` group stay together** when they fit on a page, and split between
   rows when they do not.
7. **The caption shows on the first page only**, and `:nth-child` striping follows source order.
8. **`<slot name="…">` in a repeated row** is filled by `Document::slots(fn(Page $p) => [...])`.
   The slot keeps its row's height, so filling it never moves anything; text that does not fit is
   listed in `overflows`.

- **Pinned by** the Stage 11 checks.

## Stage 12 — standards, Factur-X and passwords

**Does:** Output is always tagged; `standard` selects PDF/A-2b, PDF/A-3b or PDF/UA-1; `invoiceXml`
embeds one Factur-X XML; `password` encrypts with AES-256.

One file set: `extensions/pdf/src/write.rs`, `extensions/pdf/src/encrypt.rs` (new),
`extensions/pdf/nvs/Document.nvs`, `crates/nvs-ext/tests/pdf_standards.rs` (new).

- **Tagging** from the HTML: headings, paragraphs, lists, tables with header cells, figures with
  `alt`, and the document language.
- **A document that breaks its chosen standard throws** naming what broke, through `krilla`'s
  validation: an image with no `alt` under PDF/UA-1, a missing font embedding under PDF/A.
- **Factur-X** (`core-classes/pdf-factur-x-is-the-one-attachment`): exactly one XML file,
  checked well-formed and inside the memory cap, under PDF/A-3b only, with the fixed file name, the
  `AFRelationship` and the XMP fields the standard requires. Any other attachment, and this one
  outside PDF/A-3b, throws. The XML is the program's to build.
- **A password** encrypts with AES-256 (revision 6) and nothing weaker. Its salts and file key are
  derived from a hash of the document and the password, so equal input stays byte-equal. A password
  with a PDF/A standard throws, because PDF/A forbids encryption.
- **Pinned by** the Stage 12 checks.

## Stage 13 — existing PDFs: placing, merging and reading

**Does:** A page of an existing PDF is placed like an image or as a page background, `Pages::merge`
combines and reorders pages, and `Reader` returns a PDF's page count, metadata, attachments and text.

One file set: `extensions/pdf/src/read.rs` (new), `extensions/pdf/src/merge.rs` (new),
`extensions/pdf/nvs/` (`Pages.nvs`, `Reader.nvs`), `crates/nvs-ext/tests/pdf_read.rs` (new).

- **Placing** uses `krilla`'s `pdf` feature over `hayro`: `<img src="terms.pdf#page=2">` and
  `@page { background-image: url(letterhead.pdf) }` draw the page as vector content. Only the page's
  appearance is kept; its links, form fields and bookmarks are not.
- **`merge`** takes PDFs with optional page lists and returns one PDF: merging, splitting,
  reordering and removing pages are that one call.
- **`Reader`** returns the page count, the metadata, the attachments (the Factur-X XML of a received
  invoice) and the text of a page in the order the file stores it, through a text output over
  `hayro`'s interpreter and the fonts' Unicode maps. It does not rebuild tables or columns.
- **An encrypted source throws** in all three, as on the image side.
- **Pinned by** the Stage 13 checks.

## Stage 14 — the growth, the benches and a bundle

**Does:** Measures what the PDF component and the image second wave add to the binary, commits the
render benchmarks, and proves a bundle calls all three components.

One file set: `extensions/pdf/src/lib.rs` (its module doc), `crates/nvs-ext/tests/builtin.rs`,
`crates/nvs-cli/tests/bundle.rs`, `benches/members/novis/`.

- **The growth** is recorded in the component's module doc and held there by
  `the_recorded_growth_matches_the_embedded_components`. The goal writer's estimate was 6 to 9 MB, not
  checked.
- **Two render benchmarks**: one invoice and one 200-page report with a long table, guest time
  recorded beside the image component's figures. ADR 0128's text-page and image-page rasterising
  benchmark goes beside them.
- **If the growth looks too large**, the session records the figure and puts the question in the
  handoff's `## Backlog`. Nothing becomes optional: that is the user's call.
- **Pinned by** the Stage 14 checks.

## Stage 15 — completion and hover in the editor

**Does:** VS Code completes and explains the PDF-specific HTML attributes and CSS inside every HTML
region and ``html`…` `` template, from data generated out of `extensions/pdf/subset.json`.

One file set: `editors/vscode/package.json:29` (`contributes`), `editors/vscode/data/` (new),
`editors/vscode/test/contributions/contributions.test.ts`, a generator under `tools/nv/` (new).

- **The editor's own HTML and CSS services already answer inside markup**
  (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`), and the Novis server
  answers nothing there (`crates/nvs-lsp/src/completion.rs:1267`). So the work is custom data, not
  server code: `contributes.html.customData` and `contributes.css.customData` point at two generated
  files.
- **HTML**: `data-repeat` with its four values, `data-mark`, and `<slot>`'s use in a repeated row,
  each with a hover. **CSS**: what VS Code's own data lacks, among them `string-set`, `running()`,
  `element()`, `target-counter()`, `leader()` and the 16 margin-box at-rules, so they complete and the
  editor stops marking them unknown.
- **One home**: the generator writes both files from `subset.json`, and a headless test fails when
  they differ from what it would write. The hovers are text an end user reads (AGENTS.md § *Text an
  end user reads*) and link the reference page Stage 16 writes.
- **They show in every HTML region**, because the editor cannot know which template becomes a PDF.
  The attributes do nothing in a browser.
- **Pinned by** the Stage 15 check.

## Stage 16 — the feature proofs

**Does:** Writes the feature proofs for every member the record named, in both components.

One file set: `docs/examples/novis/`, `tests/hostile/novis/`, `benches/members/novis/`,
`docs/reference/novis/` (new pages for `Novis\Pdf`), `data/proofs/policy.json`.

- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench, one attack and its help in the binary. `bun nv proofs --id '<member>'`
  prints what is owed.
- **The manifest class** follows `Novis\Image\Codec`'s rule: tests, attack and help in full;
  examples and bench in `data/proofs/policy.json`'s `skip`, one reason each.
- **The attacks**: `<img src="file:///etc/passwd">` and `http://169.254.169.254/` in HTML, CSS and
  SVG; HTML nested past the element cap; a table of 100,000 rows under a small memory cap; a
  `target-counter` loop that never settles; a font file that lies about its tables; a PDF bomb passed
  to `merge` and `Reader`; a Factur-X XML of a billion laughs.
- **The examples** are the cases users meet: an invoice with a carried-forward total, a long report
  with a table of contents, a shipping label with a barcode, a certificate in landscape, a contract on
  a letterhead PDF with terms appended, and an uploaded invoice's XML read back.
- **Pinned by** the Stage 15 checks.

## Standing decisions

- **The user's calls, as instructions.** One goal for the image second wave and `Novis\Pdf`.
  `Novis\Pdf` is the third built-in component, always on, under `Novis\`. The engine is our own
  cascade, layout and pagination over `krilla`, `parley`, `harfrust` and `taffy`. The CSS subset is the
  list in Stages 6 to 9 and 11. Out of it, dropped and reported: footnotes, multi-column,
  `box-shadow`, `text-shadow`, `filter`, masks, `clip-path`, blend modes, 3D transforms, animation,
  transitions, `position: sticky`, vertical writing modes, ruby, subgrid, splitting one flex or grid
  row, `box-decoration-break: clone`, form fields, `<iframe>`, `<video>`, `<canvas>`, and `<script>`,
  which never runs. `data:` URIs are allowed. The default fonts are Liberation Sans, Serif and Mono in
  four styles, under OFL-1.1 admitted for font data. A missing glyph is reported, never thrown.
  Output is always tagged; PDF/A-2b, PDF/A-3b and PDF/UA-1 are options; Factur-X is the one
  attachment. Existing PDFs are placed in HTML and merged. Password protection, reading attachments and
  metadata, and text extraction are in; form filling and signatures are out. Barcodes are 1D,
  DataMatrix, SVG output and the GiroCode helper; PDF417 and Aztec are out. Input is HTML: 0121 § 3's
  builder is dropped. The headless-browser backend is removed from the rules for good. Headers and
  footers are CSS margin boxes and running elements plus the per-page function. The eight table rules
  of Stage 11 are all in. Marks are one attribute, `data-mark="name:value"`. VS Code completes the
  PDF-specific HTML and CSS through custom data generated from the subset table.
- **The goal writer's calls, not confirmed by the user, also standing.** Reading, merging and text
  extraction live in `Novis\Pdf`; only rasterising a page is `Novis\Image`'s. There is no pdfium
  fallback, because pdfium in wasm needs Emscripten and has no WASI path. **No new limit**: a render
  runs under its request's memory cap and deadline, the HTML under the parser's element cap, a raster
  image priced by the memory cap, and a bulk render belongs in `Core\Queue`. Class names in the sketch
  and the checks (`Document`, `Page`, `Rendered`, `Pages`, `Reader`, `Standard`, `Engine` for the
  manifest class, `Novis\Image\Barcode`) are the record's to change; a rename rewrites the checks in
  the same slice.
- **One record slot**: one new record and no other number, checked against `docs/decisions/` right
  before it is written, because another agent may take a number first. It is Stage 2's first slice.
- **The crates**, each pinned: `krilla` and `krilla-svg` (with the `pdf` feature for placing pages),
  `parley` with `fontique`'s system discovery off, `harfrust`, `skrifa`, `taffy`, `cssparser`,
  `selectors`, `hypher`, `html5ever` through `vendor/html5ever`, `hayro`, `resvg` and `usvg`,
  `barcoders`, `datamatrix`. `cssparser` and `selectors` are MPL-2.0, already on `deny.toml`'s list.
  `fulgur` (the same design, young) and WeasyPrint are references and test comparisons, never
  dependencies. A crate that fails to build for `wasm32-wasip2` is replaced by its nearest pure-Rust
  equivalent, and the record says so.
- **How a session judges layout.** Box geometry is asserted in the crate's host tests
  (`extensions/pdf/tests/`). A visual result is checked by rasterising the page with the image
  component's PDF decoding from Stage 3 and reading the PNG; a golden PNG is compared with
  `Image::compare` at a tolerance. A golden is never rewritten to make a check pass.
- **Determinism is part of every stage**: two renders of equal input are byte-equal, including
  encrypted output.
- **The tradeoffs**, stated here and in the record because AGENTS.md asks. Performance: one call per
  render, two with a header function; milliseconds for an invoice and seconds for a long report.
  Memory: nothing until first use; then the compiled module, shared by every core, and each rendering
  request's layout tree and output inside its own cap, freed when the request ends. The binary grows
  by the measured size of Stage 14. Usability: invoices, reports, labels and letters with no
  installation, headers and tables that survive page breaks, and e-invoices. Simplicity: one input
  language, one engine, no builder, and a CSS subset that is written down and tested.
- **Neutral names only** in every test, example and record: `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ext` or a
  `--test` filter.
