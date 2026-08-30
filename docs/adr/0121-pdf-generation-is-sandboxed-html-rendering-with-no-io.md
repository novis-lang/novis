# ADR 0121 — PDF generation is sandboxed HTML rendering with no I/O

- **Status:** Accepted
- **Date:** 2026-08-30
- **Scope:** where PDF generation is placed — Tier 1, first-party, the roster amendment that admits it —
  and the contract any implementation must meet: the no-I/O render, the one-engine rule, how the CSS
  subset is bounded and reported, what the output file may never contain, and the fidelity escape hatch.
  Not in scope: the API of the builder and the component's entry points, which the milestone that builds
  it designs the way [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) did
  for images; the sandbox contract itself ([0003](0003-extension-system.md)); how the package is named,
  pinned and granted ([0081](0081-packages-are-digests-resolution-is-a-maximum.md)); scheduling — no
  milestone owns this, deliberately.
- **Depends on:** [0003](0003-extension-system.md) — the sandbox this ADR spends its whole argument on;
  [0051](0051-standard-library-tiers.md) — the tests that decide the placement.
- **Amends:** [0051](0051-standard-library-tiers.md) § 3 — the first-party Ext roster gains a third
  component, unscheduled; the "two is the whole roster" sentence and its Consequences bullet now read
  three, with M9's build unchanged at two.
- **Amended by:** 0123

> **In short:** Novis will own one spelling for HTML-to-PDF, as a first-party Tier 1 `.nvsx` — package
> `nvs/pdf`, namespace `Novis\Pdf` — because in PHP this job is both fragmented across incompatible
> userland libraries and a standing CVE class whose every published exploit is "the renderer fetched what
> the HTML named". The component therefore has **no I/O at all**: every image, stylesheet and font is
> bytes the program passes in, an unresolved reference throws rather than rendering a gap, and SSRF and
> file disclosure are structurally impossible rather than audited away. There is **one engine** — HTML
> plus a documented CSS subset plus inline SVG — and the top-down programmatic path is a Novis-side
> builder that emits the same HTML, never a second imperative engine. Unsupported CSS is dropped by CSS's
> own error-recovery rules but **reported in the render result**, so a test can assert none was. The
> output is inert: no JavaScript, no launch or embedded-file actions, no timestamps, so equal input gives
> byte-equal output. No off-the-shelf engine exists to link — headless Chromium is a browser process, not
> a library; WeasyPrint is Python; Prince is closed; Blitz is experimental and unpaginated — so the
> engine is assembled from the production Rust pieces (html5ever, Servo's parser; `taffy`; `rustybuzz`;
> `krilla`/`pdf-writer`, Typst's PDF backend; `resvg`) plus the one layer nobody ships, pagination, which
> is the real cost and the reason this ADR decides placement and contract now but schedules nothing.

## Context

- **What PHP demonstrates, twice.** Userland grew Dompdf, mPDF, TCPDF, FPDF, Snappy and Browsershot —
  incompatible surfaces, disjoint CSS subsets, all maintained at arm's length — which is the
  fragmentation [0051](0051-standard-library-tiers.md) already names as what "leave it to the ecosystem"
  produces. And the category is a CVE class, not just a mess: Dompdf's remote-font RCE, its
  CVE-2022-2400 file disclosure, wkhtmltopdf's SSRF — one shape every time, user-influenced HTML making
  the renderer *fetch* something. That shape is a priority-1 reason to own the spelling, and to own it in
  a form where the fetch cannot happen.
- **The audience produces these documents constantly.** Invoices, tickets, statements, reports and
  packing slips are near-universal in the web applications
  [0080](0080-the-audience-nvs-is-built-for.md) describes, which is why every PHP shop ends up holding
  one of the libraries above.
- **[0051](0051-standard-library-tiers.md) § 2's tests, applied in order.** Test 1: no — a render is
  pure `bytes → bytes`, and a font cache is an optimisation, not cross-request state. Test 2: no,
  *because* § 2 below removes the I/O — a renderer that cannot fetch is a transform, not a sink, and
  needing no launderer is exactly what keeps it out of Core. Test 3: no, by the same design. Test 4: no —
  a render costs milliseconds to seconds, so the boundary crossing is noise. Test 5: **emphatically** —
  HTML, CSS, TrueType/OpenType (FreeType's CVE history is the genre's), embedded images and SVG make this
  a bigger hostile-bytes case than the image component the ADR calls the tier's headline. Test 6: yes —
  one spelling is most of the point.
- **There is no engine to link; that is the finding that shapes everything else.** Headless Chromium is
  the industry's de-facto answer and is a 150 MB browser process holding its own sandbox and its own
  lifecycle — an operational dependency, not a library. wkhtmltopdf was archived in 2023. Prince,
  PDFreactor and Antenna House are closed and commercial. WeasyPrint, the best open paged-media
  implementation, is Python. Typst produces excellent PDFs from its own markup, not HTML — but its
  internals are exactly the building blocks. Blitz (Stylo + Taffy + Parley) renders HTML in Rust and is
  explicitly experimental, with no pagination and a wasm-hostile CSS engine size.
- **The building blocks exist, production-grade, pure Rust, wasm-clean:** `html5ever` (Servo's HTML
  parser), `cssparser`/`lightningcss`, `taffy` (block, flex and grid layout), `rustybuzz` +
  `ttf-parser` + `subsetter` (shaping and fonts), `unicode-bidi`, `hypher`, `resvg`/`svg2pdf`,
  `image`, and `krilla`/`pdf-writer` (Typst's PDF backend). The missing piece is **pagination** — page
  boxes, fragmentation, `page-break-*`, running headers and footers, page counters — and writing that
  layer over these crates is the project's actual size: a milestone, not a slice.

## Decision

### 1. Placement: Ext, first-party, third on the roster, unscheduled

PDF generation is a **Tier 1 `.nvsx`**, first-party, distributed as the package **`nvs/pdf`** with
everything a program names under **`Novis\Pdf`** — the split, the two-payload package shape and the
one-registered-class rule all follow [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md)
§ 1. It is not Core: a layout engine, a font parser and image codecs unsandboxed in every binary would
contradict the reason the image component is Tier 1. It is not Native: nothing in it holds state or
privilege that test 1 would protect. [0051](0051-standard-library-tiers.md) § 3's first-party Ext roster
gains this entry — that section carries the current roster and its count — **unscheduled**: no milestone
owns it, M9 still builds two, and this ADR exists so that when the work is scheduled the placement and
the contract are already decided rather than improvised.

### 2. The render has no I/O, and an unresolved reference throws

The component performs **no I/O of any kind**. Every asset a document uses — images, stylesheets beyond
the inline ones, fonts — crosses the boundary as bytes in a caller-supplied map, and a reference in the
HTML (`<img src>`, CSS `url()`) resolves against that map and nothing else. A scheme, an absolute path or
a name the map lacks **throws** rather than rendering a broken-image gap: a generated document must be
deterministic, and [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s rule — refuse, never
repair — applies to a missing asset exactly. A program that wants a remote image fetches it itself with
`Core\Http\Client` under [0058](0058-outbound-request-policy.md)'s policy and passes the bytes, so the
authority question is answered where every other outbound request answers it. The class of exploit that
defines this category in PHP is thereby absent by construction: there is nothing to trick into fetching,
because there is no fetching.

Fonts follow the same rule — supplied as bytes, no system font lookup, since the guest has no filesystem —
with a small embedded default set in the component's data section so a plain document renders out of the
box, the precedent [0051](0051-standard-library-tiers.md) § 3 set when the intl component swallowed CLDR.
The guest needs no clock: a rendered date is text the program already put in its HTML.

### 3. One engine — HTML in, and the programmatic path emits the same HTML

The sole input language is **HTML plus the CSS subset plus inline SVG**. The "top-down" way of building a
document — append a heading, a table, a page break, mix computed parts with template parts — is a
Novis-side builder in `nvs/pdf`'s source payload that **emits that same HTML**, and document parts
compose by concatenation before a single render call. There is no second, FPDF-shaped imperative engine
with its own coordinate model: that would be two spellings for one job, test 6's exact target, and every
PHP shop that owns both Dompdf and TCPDF is living in the alternative. Drawings are inline SVG, the same
answer [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) § 4 gives for
charts.

### 4. The CSS subset is documented, and what was dropped is reported

The engine implements a **documented subset** of CSS — the building milestone fixes its exact contents;
block and inline flow, tables, flex, fonts, images, inline SVG and the `@page` family are the working
floor — and handles everything outside it by CSS's own forward-compatible parsing: an unknown or
unsupported declaration is dropped, never guessed at. What this ADR adds to that standard behaviour is
**visibility**: the render result carries the list of dropped declarations, so a test asserts the list is
empty and a document that silently depends on unsupported CSS cannot survive CI — the "renders fine in
Chrome, wrong in the library" failure every PHP library ships as a support forum instead of an API.

### 5. The output is inert, and equal input gives byte-equal output

The writer never emits JavaScript, launch actions, embedded files or any construct that makes a reader
fetch or execute on open; links exist only where the HTML wrote an `<a href>` with an `http(s):` or
`mailto:` target. Rendering `tainted` input is safe by construction — the render is a transform, not a
sink, and a program embedding untrusted markup that wants its *tags* constrained uses `Core\Html`'s
sanitizer first, the tool [0024](0024-taint-tracking-for-injection-sinks.md) already provides. The writer
also embeds **no timestamps and no generator entropy**, so two renders of equal input are byte-equal —
which turns document tests into `Core\Test` byte comparisons and removes the class of flaky fixture every
PDF test suite otherwise grows.

### 6. The fidelity escape hatch shares the interface

A document the subset will never reach — heavy JavaScript-era CSS, pixel-perfect brand PDFs signed off
against Chrome — is served by an **opt-in** second backend behind the same call shape: an external
headless browser driven under `process.exec` via [0044](0044-core-process-argv-only-no-shell.md), never
shipped in the default path and documented as trading § 2's no-I/O guarantee for fidelity. The interface
is the contract; the engine is a choice the program states explicitly.

**What it spends**, per request that renders: the HTML, the asset map, the layout tree and the output
document, all inside the extension's memory cap, none of it outliving the request; wall-clock is
milliseconds for an invoice and seconds for a long report, and a bulk or slow render belongs in
`Core\Queue` — the same line [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md)
§ 5 draws for AVIF encoding.

## Consequences

- **[0051](0051-standard-library-tiers.md)'s roster consequence changes shape**: the first-party Tier 1
  roster is no longer closed at two — this entry joins it unscheduled, § 3 there carrying the current
  count, so M9's scope and its two-component verification are untouched. The tier's keep is still earned by the third-party channel; this entry is admitted over that
  bar because the category is a security class and a fragmentation magnet at once, the same pairing that
  pulled the HTTP client into Core.
- **The genre's defining vulnerability class is absent, not mitigated.** No fetch exists to exploit, and
  the sandbox's memory cap and epoch deadline contain the hostile-bytes surface that remains.
- **A cost this ADR does not hide:** the pagination layer is ours to write, and the CSS subset is a
  permanent compatibility surface — "works in the browser, dropped by `Novis\Pdf`" is a standing ticket
  category that § 4's report converts from silent wrongness into a named, testable diagnostic, but does
  not eliminate.
- **No first-party imperative PDF API, ever.** A third-party `.nvsx` may carry one; the first-party
  answer to "draw at x,y" is HTML, CSS and SVG.

## Alternatives rejected

- **Core.** Puts a layout engine, font parser and image decoders — test 5's exact material — unsandboxed
  in every binary and every in-flight request. Rejected on priority 1, by the same argument that placed
  the image component.
- **Native (Tier 2).** Nothing here needs what Native protects: no connection lifetime, no runtime
  privilege. Rejected because the sandbox is not a cost to route around but the feature itself.
- **An engine written in Novis.** Dompdf's path: a second, unfuzzed implementation of shaping, bidi,
  line breaking, cascade and layout, losing on priorities 1, 2 and 3 to buy nothing the wasm build does
  not already give. Novis code belongs in the builder and the templates, not the engine.
- **Headless Chromium as *the* engine.** The fidelity ceiling, and an operational floor: a browser
  process per render, hundreds of megabytes outside any request's attribution, its own sandbox and
  lifecycle to operate. It is § 6's escape hatch, not the foundation.
- **Leave the category to the third-party channel.** The default under
  [0051](0051-standard-library-tiers.md)'s "two is the whole roster" stance, and the honest competitor.
  Rejected because both halves of the PHP outcome — five incompatible subsets, and a CVE class baked into
  the popular implementations' architecture — are what first-party ownership of one no-I/O spelling
  exists to prevent.

## Revisiting

If a production-grade Rust HTML paged-media engine emerges — Blitz with pagination is the plausible
candidate — the assembled engine is replaced wholesale and this ADR is not reopened: §§ 2–5 are written
against the interface, not the crates, and a swapped engine must still meet them.

## Verification

Owed by the milestone that eventually builds the component, recorded here so scheduling inherits them:

- A render with **no capability grants** succeeds — the no-I/O claim demonstrated, not asserted.
- Fixtures with `<img src="file:///…">`, an `http:` URL and a name absent from the asset map each throw
  naming § 2, and no network or filesystem access is observable from the guest.
- A scanner over the output of the fixture corpus finds no `/JavaScript`, `/Launch`, `/OpenAction` or
  `/EmbeddedFile` objects (§ 5).
- The same input rendered twice is byte-equal (§ 5), and a fixture using an unsupported property renders
  with that property in the dropped-declarations report while a clean fixture reports none (§ 4).
- [0051](0051-standard-library-tiers.md) § 5's existing test already covers that the component cannot
  register a `Core\` name.
