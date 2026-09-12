---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "PDF and spreadsheets"
description: "Two first-party extensions that perform no I/O and write nothing a reader would execute, byte-identical for equal input."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/uris-and-images/
  label: "URIs and images"
next:
  link: /docs/rules/observability/
  label: "Observability"
---

<p class="nv-section-lead">Two first-party extensions that perform no I/O and write nothing a reader would execute, byte-identical for equal input.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">14</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">1</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">13</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#pdf-render-has-no-io">PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#pdf-one-engine">The only input language is HTML plus a CSS subset, and every backend answers that one interface</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#pdf-css-subset">The CSS subset is documented, and the render result lists every declaration it dropped</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#pdf-output-is-inert">The writer emits nothing a reader would execute, and equal input gives byte-equal output</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#pdf-page-is-an-image-source">A PDF page is a decode-only format of the image component: one page per <code>open</code>, priced by the pixel cap</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#pdf-decode-refusals">An encrypted document and a detectable omission both throw, and everything past rasterising is somebody else's job</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#spreadsheet-has-no-io">The spreadsheet component is a first-party extension that fetches nothing and executes nothing it reads</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#spreadsheet-formula-is-a-value">Only an explicit <code>Formula</code> value writes a formula cell, so an export built from user data is inert</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#spreadsheet-bulk-boundary">A workbook crosses the boundary in whole blocks, and a bomb is refused before a sheet buffer exists</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#spreadsheet-evaluation-and-roster">The roster is read-wide and write-narrow, a macro is never written, and evaluation is an explicit call</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#spreadsheet-output-is-inert">The writer embeds no executable content and no entropy, so equal input gives byte-equal output</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#net-one-api-three-transports"><code>Core\Net</code> is one class over TCP, UDP and Unix sockets, reached through five entry points that park on the runtime's own reactor</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#net-a-socket-does-not-outlive-its-request">A <code>Core\Net</code> socket closes with the request that opened it, so there is no persistent connection and no pool across requests</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#net-udp-carries-no-reliability-layer">A datagram socket sends and receives messages and offers no ordering, retransmission or acknowledgement above them</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="pdf-render-has-no-io">

## PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#pdf-render-has-no-io"><code>core-classes/pdf-render-has-no-io</code></a>
</div>

PDF generation is a first-party Tier 1 extension package, sandboxed, with everything a program names
under one namespace and one registered class. It is not `Core`: a layout engine, a font parser and
image codecs unsandboxed in every binary would contradict the reason the image component is an
extension at all. It is not native: nothing in it holds state or privilege.

**The component performs no I/O of any kind.** Every asset a document uses — images, stylesheets
beyond the inline ones, fonts — crosses the boundary as bytes in a caller-supplied map, and a
reference in the HTML resolves against that map and nothing else. A scheme, an absolute path, or a
name the map lacks **throws** rather than rendering a broken-image gap: a generated document must be
deterministic, and refusing beats repairing. A program that wants a remote image fetches it itself,
under the outbound policy, and passes the bytes.

The class of exploit that defines this category in PHP — server-side request forgery and local file
read through a `<img src>` — is thereby absent by construction: there is nothing to trick into
fetching, because there is no fetching. Fonts follow the same rule, with a small embedded default set
so a plain document renders out of the box, and the guest needs no clock.

**Not shipped.** There is no PDF package in the tree; M17 builds it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The SSRF and local-file-read class that defines PHP's HTML-to-PDF libraries is absent by construction — there is nothing in the sandbox to fetch with</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-one-engine" title="The only input language is HTML plus a CSS subset, and every backend answers that one interface"><code>core-classes/pdf-one-engine</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-output-is-inert" title="The writer emits nothing a reader would execute, and equal input gives byte-equal output"><code>core-classes/pdf-output-is-inert</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-has-no-io" title="The spreadsheet component is a first-party extension that fetches nothing and executes nothing it reads"><code>core-classes/spreadsheet-has-no-io</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0120.md">record 0120</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a></dd></div></dl>

</div>

<div class="nv-rule" id="pdf-one-engine">

## The only input language is HTML plus a CSS subset, and every backend answers that one interface

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#pdf-one-engine"><code>core-classes/pdf-one-engine</code></a>
</div>

The sole input language is **HTML plus the CSS subset plus inline SVG**. The top-down way of building
a document — append a heading, a table, a page break, mix computed parts with template parts — is a
Novis-side builder that **emits that same HTML**, and document parts compose by concatenation before
a single render call.

There is no second, imperative engine with its own coordinate model. That would be two spellings for
one job, and every PHP shop that owns both a DOM-based renderer and a coordinate-based one is living
in the alternative. Drawings are inline SVG, the same answer the image component gives for charts.

The HTML is parsed by the same crate the language's own parser uses
([`core-classes/html-parsing`](/docs/rules/core-classes/regex-html-and-introspection/#html-parsing "HTML parses by the WHATWG algorithm onto Core\Xml's own tree, and that parse never fails")), compiled into the sandbox rather than linked natively, so the
language and its PDF component parse HTML identically.

**The interface is the contract; the engine is a choice the program states explicitly.** A document
the subset will never reach — heavy JavaScript-era CSS, a pixel-perfect brand PDF signed off against
a browser — is served by an **opt-in** second backend behind the same call shape: an external
headless browser driven under the process capability ([`core-classes/process-is-argv-only`](/docs/rules/core-classes/processes-and-files/#process-is-argv-only "Core\Process is the one way to run another program, and there is no shell string anywhere in it")),
never shipped in the default path, and documented as trading the no-I/O guarantee for fidelity. It is
a second *engine*, not a second API, which is the whole reason a single interface was worth fixing
first.

What it spends, per request that renders: the HTML, the asset map, the layout tree and the output
document, all inside the extension's memory cap, none of it outliving the request. Wall-clock is
milliseconds for an invoice and seconds for a long report, so a bulk or slow render belongs in the
queue.

**Not shipped.** There is no PDF package in the tree.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no second FPDF-shaped imperative engine with its own coordinate model — drawings are inline SVG</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-render-has-no-io" title="PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws"><code>core-classes/pdf-render-has-no-io</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-css-subset" title="The CSS subset is documented, and the render result lists every declaration it dropped"><code>core-classes/pdf-css-subset</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-parsing" title="HTML parses by the WHATWG algorithm onto Core\Xml's own tree, and that parse never fails"><code>core-classes/html-parsing</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0120.md">record 0120</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0122.md">record 0122</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a></dd></div></dl>

</div>

<div class="nv-rule" id="pdf-css-subset">

## The CSS subset is documented, and the render result lists every declaration it dropped

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#pdf-css-subset"><code>core-classes/pdf-css-subset</code></a>
</div>

The engine implements a **documented subset** of CSS — block and inline flow, tables, flex, fonts,
images, inline SVG and the page family are the working floor — and handles everything outside it by
CSS's own forward-compatible parsing: an unknown declaration is dropped, never guessed at.

What this rule adds to that standard behaviour is **visibility**. The render result carries the list
of dropped declarations, so a test asserts the list is empty and a document that silently depends on
unsupported CSS cannot survive CI. "Renders fine in the browser, wrong in the library" is the failure
every PHP PDF library ships as a support forum instead of an API, and it exists because the drop is
invisible.

**Not shipped.** There is no PDF package in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-one-engine" title="The only input language is HTML plus a CSS subset, and every backend answers that one interface"><code>core-classes/pdf-one-engine</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-evaluation-and-roster" title="The roster is read-wide and write-narrow, a macro is never written, and evaluation is an explicit call"><code>core-classes/spreadsheet-evaluation-and-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a></dd></div></dl>

</div>

<div class="nv-rule" id="pdf-output-is-inert">

## The writer emits nothing a reader would execute, and equal input gives byte-equal output

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#pdf-output-is-inert"><code>core-classes/pdf-output-is-inert</code></a>
</div>

The writer never emits JavaScript, launch actions, embedded files, or any construct that makes a
reader fetch or execute on open. Links exist only where the source wrote one with an `http(s):` or
`mailto:` target. Rendering `tainted` input is safe by construction — the render is a transform, not
a sink — and a program embedding untrusted markup that wants its *tags* constrained sanitizes first.

The writer also embeds **no timestamps and no generator entropy**, so two renders of equal input are
byte-equal. That turns a document test into a byte comparison and removes the class of flaky fixture
every PDF test suite otherwise grows.

The second, fidelity backend ([`core-classes/pdf-one-engine`](/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-one-engine "The only input language is HTML plus a CSS subset, and every backend answers that one interface")) makes no exception to any of this:
it trades the no-I/O guarantee, not the inertness of what it writes.

**Not shipped.** There is no PDF package in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-render-has-no-io" title="PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws"><code>core-classes/pdf-render-has-no-io</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-output-is-inert" title="The writer embeds no executable content and no entropy, so equal input gives byte-equal output"><code>core-classes/spreadsheet-output-is-inert</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-page-is-an-image-source" title="A PDF page is a decode-only format of the image component: one page per open, priced by the pixel cap"><code>core-classes/pdf-page-is-an-image-source</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a></dd></div></dl>

</div>

<div class="nv-rule" id="pdf-page-is-an-image-source">

## A PDF page is a decode-only format of the image component: one page per `open`, priced by the pixel cap

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#pdf-page-is-an-image-source"><code>core-classes/pdf-page-is-an-image-source</code></a>
</div>

A PDF is a **decode-only format of the image component**, not a member of the PDF generation package:
a writer and an interpreter share no code, and a raster produced there would have to recross the
boundary to enter the pipeline that is the whole point of loading one. No new entry point is added;
the format enum simply gains a case.

`open` gains two options meaningful for a paged source: a 1-based **page** and a **dpi**. One call
rasterises exactly that page, so a document's pages become images by iterating and cost stays
proportional to what was asked for. The pixel price is the page box scaled to the requested
resolution, counted against the pixel cap before any buffer is allocated
([`core-classes/image-pixel-cap`](/docs/rules/core-classes/uris-and-images/#image-pixel-cap "The pixel cap is read off the header before a buffer is allocated, and a call may only lower it")), and a page beyond the document throws naming the page count.
The header reader parses the cross-reference table and page tree and never a content stream, so
counting pages allocates no pixel.

Rasterising is a pure function of the bytes and the options — no clock, no I/O, no system fonts — so
equal input gives byte-equal output. The interpreter is pure Rust, compiled into the sandbox beside
the codecs, with substitutes for the standard fonts embedded so an unembedded-font document renders.

**Not shipped.** There is no image component in the tree, so there is nothing for this to be a format
of.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Rasterising a page needs no Imagick, no Ghostscript and no shell — it is <code>Image::open</code> with a <code>page</code> and a <code>dpi</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/uris-and-images/#image-pixel-cap" title="The pixel cap is read off the header before a buffer is allocated, and a call may only lower it"><code>core-classes/image-pixel-cap</code></a> <a href="/docs/rules/core-classes/uris-and-images/#image-format-roster" title="The format roster is a closed table of decoders and encoders, each naming what implements it"><code>core-classes/image-format-roster</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-decode-refusals" title="An encrypted document and a detectable omission both throw, and everything past rasterising is somebody else's job"><code>core-classes/pdf-decode-refusals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0128.md">record 0128</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0120.md">record 0120</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a></dd></div></dl>

</div>

<div class="nv-rule" id="pdf-decode-refusals">

## An encrypted document and a detectable omission both throw, and everything past rasterising is somebody else's job

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#pdf-decode-refusals"><code>core-classes/pdf-decode-refusals</code></a>
</div>

An **encrypted document throws** naming the encryption. There is no password option, and decrypting
stays in the third-party channel. An **embedded object the engine recognises but cannot decode
throws** naming the object, rather than rendering a blank region: a generated image must be
deterministic, not approximately right. An external reference never resolves, because nothing in the
guest can resolve one, and JavaScript, launch actions and embedded files are inert by construction,
since rasterising reads a page's appearance and never executes an action.

Text extraction, page splitting and merging, form filling, signing and password decryption are
**different jobs over the same bytes**, and belong to the third-party channel rather than this
roster. Generation is a separate package entirely. The fidelity escape hatch for a document the
interpreter renders wrong is an installed external tool under the process capability, and is never
made first-party.

The surface this leaves open is an *approximated* construct — one the interpreter supports but
renders imperfectly — which is a silent fidelity miss rather than a detectable omission, and is owned
as a known consequence rather than closed by a rule.

**Not shipped.** There is no image component in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-page-is-an-image-source" title="A PDF page is a decode-only format of the image component: one page per open, priced by the pixel cap"><code>core-classes/pdf-page-is-an-image-source</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-render-has-no-io" title="PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws"><code>core-classes/pdf-render-has-no-io</code></a> <a href="/docs/rules/errors/ambiguous-input/#ambiguous-input-refused" title="Ambiguous input is refused whole, never repaired"><code>errors/ambiguous-input-refused</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0128.md">record 0128</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a></dd></div></dl>

</div>

<div class="nv-rule" id="spreadsheet-has-no-io">

## The spreadsheet component is a first-party extension that fetches nothing and executes nothing it reads

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#spreadsheet-has-no-io"><code>core-classes/spreadsheet-has-no-io</code></a>
</div>

Spreadsheet reading and generation are one first-party Tier 1 extension package, sandboxed, with one
registered class and everything else Novis source that calls it. It is not `Core`: a
PhpSpreadsheet-sized API is a permanent simplicity cost, and its parsers unsandboxed in every
in-flight request is exactly the outcome the tier system exists to prevent.

A workbook crosses the boundary as bytes and the result returns as bytes; **the component fetches
nothing**. An external-workbook reference, a linked image or a remote data connection in a read file
comes back as **data** — the reference itself — and is never resolved. An image placed on write is
bytes the caller supplies. A workbook is self-contained, so this needs no asset map.

**Reading evaluates nothing.** A cell reads as its stored scalar, or as formula text plus the cached
result the file's author wrote beside it, and the two are distinguishable at the API, because a
cached value is a claim by the author rather than a computation by us. VBA and embedded OLE objects
are inert payload the reader may enumerate but can never execute. Cells read from `tainted` bytes are
`tainted`.

**Not shipped.** There is no spreadsheet package in the tree; M17 builds it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An external workbook reference, a linked image and a remote data connection come back as data, and VBA is enumerable payload that never runs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-formula-is-a-value" title="Only an explicit Formula value writes a formula cell, so an export built from user data is inert"><code>core-classes/spreadsheet-formula-is-a-value</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-bulk-boundary" title="A workbook crosses the boundary in whole blocks, and a bomb is refused before a sheet buffer exists"><code>core-classes/spreadsheet-bulk-boundary</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-render-has-no-io" title="PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws"><code>core-classes/pdf-render-has-no-io</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0123.md">record 0123</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0120.md">record 0120</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0055.md">record 0055</a></dd></div></dl>

</div>

<div class="nv-rule" id="spreadsheet-formula-is-a-value">

## Only an explicit `Formula` value writes a formula cell, so an export built from user data is inert

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#spreadsheet-formula-is-a-value"><code>core-classes/spreadsheet-formula-is-a-value</code></a>
</div>

Only an explicit `Formula` value produces a formula cell. Every string writes as a **text cell** —
including one beginning `=`, `+`, `-`, `@` or a control character, which is the whole trigger set for
the formula-injection class.

An export built from user data is therefore inert by construction rather than by a caller remembering
to prefix a quote. This is the same move taint tracking makes everywhere else — code and data
separated by type, not by inspection — applied at a boundary PHP userland leaves as a footnote in a
security advisory.

What it costs is one wrapper at the call sites that genuinely mean a formula, which is the smaller
half of any real export.

**Not shipped.** There is no spreadsheet package in the tree.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A string beginning <code>=</code>, <code>+</code>, <code>-</code> or <code>@</code> writes as text, so formula injection is closed by type rather than by a remembered quote prefix</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-has-no-io" title="The spreadsheet component is a first-party extension that fetches nothing and executes nothing it reads"><code>core-classes/spreadsheet-has-no-io</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-evaluation-and-roster" title="The roster is read-wide and write-narrow, a macro is never written, and evaluation is an explicit call"><code>core-classes/spreadsheet-evaluation-and-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0123.md">record 0123</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div></dl>

</div>

<div class="nv-rule" id="spreadsheet-bulk-boundary">

## A workbook crosses the boundary in whole blocks, and a bomb is refused before a sheet buffer exists

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#spreadsheet-bulk-boundary"><code>core-classes/spreadsheet-bulk-boundary</code></a>
</div>

The builder assembles the whole document — sheets, cells, styles, charts — as Novis values, and
**one call serializes it**. Reading opens a workbook once and pulls whole sheets or declared ranges
as bulk blocks. **There is no per-cell accessor, and none is added later**: a member that would be
called in a loop over the file's own contents costs the boundary, not the work — the same refusal the
image component makes.

Before any sheet buffer is allocated, the component reads the declared dimensions and the container's
declared uncompressed sizes, and refuses a workbook over a cell cap with a throw naming the cap and
the declared size. The cap is policy and the sandbox's memory cap is the backstop, which is the shape
the image component's pixel cap already has.

What it spends, per request that calls it: the document model and its serialized form, inside the
extension's memory cap, none of it outliving the request. A million-row export belongs in the queue,
and the write path must offer a bounded-memory streaming mode, so a large export costs rows in
flight rather than rows total.

**Not shipped.** There is no spreadsheet package in the tree.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no per-cell accessor to loop over, which is the shape most PhpSpreadsheet code takes</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/uris-and-images/#image-pipeline" title="An image is an immutable value carrying a plan, and nothing decodes until a terminal runs it"><code>core-classes/image-pipeline</code></a> <a href="/docs/rules/core-classes/uris-and-images/#image-pixel-cap" title="The pixel cap is read off the header before a buffer is allocated, and a call may only lower it"><code>core-classes/image-pixel-cap</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-has-no-io" title="The spreadsheet component is a first-party extension that fetches nothing and executes nothing it reads"><code>core-classes/spreadsheet-has-no-io</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0123.md">record 0123</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0120.md">record 0120</a></dd></div></dl>

</div>

<div class="nv-rule" id="spreadsheet-evaluation-and-roster">

## The roster is read-wide and write-narrow, a macro is never written, and evaluation is an explicit call

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#spreadsheet-evaluation-and-roster"><code>core-classes/spreadsheet-evaluation-and-roster</code></a>
</div>

The roster is **read-wide and write-narrow**. The modern XML format reads and writes, and is the
parity target: styles, merged cells, charts, conditional formats, tables, autofilters, images and
defined names. The macro-enabled variant reads data only, with its macros enumerable and inert, and
**is never written** — a filled template emits the macro-free format. The binary and legacy formats
read and do not write, because a writer for a 1997 binary format is legacy nothing should produce.
The open format reads, and writes in a second wave. Template fill is named explicitly, because it is
the workflow behind most real exports: open a styled workbook, set values, save — styling preserved,
macros dropped.

**Nothing evaluates implicitly** — not on read, and not on write, where a formula cell is written for
the opening application to compute. An explicit evaluate call is second-wave work, and its result
carries the list of functions and references it could not compute, so a test asserts that list is
empty. That is the same visibility rule the PDF component applies to dropped CSS: silent wrongness
becomes a named, testable diagnostic.

**Not shipped.** There is no spreadsheet package in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-formula-is-a-value" title="Only an explicit Formula value writes a formula cell, so an export built from user data is inert"><code>core-classes/spreadsheet-formula-is-a-value</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-output-is-inert" title="The writer embeds no executable content and no entropy, so equal input gives byte-equal output"><code>core-classes/spreadsheet-output-is-inert</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-css-subset" title="The CSS subset is documented, and the render result lists every declaration it dropped"><code>core-classes/pdf-css-subset</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0123.md">record 0123</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a></dd></div></dl>

</div>

<div class="nv-rule" id="spreadsheet-output-is-inert">

## The writer embeds no executable content and no entropy, so equal input gives byte-equal output

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#spreadsheet-output-is-inert"><code>core-classes/spreadsheet-output-is-inert</code></a>
</div>

The writer emits no macros, no embedded executables and no launch-shaped content; hyperlinks exist
only where the document wrote them.

It embeds **no timestamps and no generator entropy** — document properties carry fixed dates unless
the caller sets them, and archive entries a fixed time — so equal input gives byte-equal output and a
document test is a byte comparison. This is the same rule the PDF writer holds
([`core-classes/pdf-output-is-inert`](/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-output-is-inert "The writer emits nothing a reader would execute, and equal input gives byte-equal output")), and it exists for the same reason: a test that cannot
compare bytes ends up comparing nothing.

**Not shipped.** There is no spreadsheet package in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-output-is-inert" title="The writer emits nothing a reader would execute, and equal input gives byte-equal output"><code>core-classes/pdf-output-is-inert</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#spreadsheet-evaluation-and-roster" title="The roster is read-wide and write-narrow, a macro is never written, and evaluation is an explicit call"><code>core-classes/spreadsheet-evaluation-and-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0123.md">record 0123</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a></dd></div></dl>

</div>

<div class="nv-rule" id="net-one-api-three-transports">

## `Core\Net` is one class over TCP, UDP and Unix sockets, reached through five entry points that park on the runtime's own reactor

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#net-one-api-three-transports"><code>core-classes/net-one-api-three-transports</code></a>
</div>

`Core\Net` is the whole socket surface, replacing PHP's `socket_*`, `stream_socket_*` and `fsockopen`
with one class over three transports: TCP, UDP and Unix-domain sockets. A program reaches them through
five entry points — an outbound TCP connection, a listening TCP socket, a bound UDP socket, an
outbound Unix-domain connection and a listening Unix-domain socket — and accepting is a member on the
listener rather than a sixth way in.

**No entry point decides between transports by reading its argument.** A host and a socket path are
separate members taking separately-typed arguments, so [`security/a-path-is-not-a-url`](/docs/rules/security/closed-doors/#a-path-is-not-a-url "A path is a filesystem path: no member dispatches on a scheme prefix, and nothing may register one")'s refusal —
no member dispatches on the textual content of a path — holds by construction rather than by a check.
That is why the surface is five members where PHP has two: `stream_socket_client("unix://…")` and
`stream_socket_client("tcp://…")` are one function distinguished by a prefix, and the second member is
the security property.

Every one of those sockets parks on the runtime's own reactor. `crates/nvs-host/src/net.rs` is the
contract they share — a `Read` and a `Write` that hand the core back instead of blocking it — and
`crates/nvs-host/src/reactor.rs` is its readiness half. A second event loop is never the answer: it
would be a second poll structure whose fairness, shutdown, deadline and drain semantics must be made
to agree with the reactor's by hand, and whose disagreements appear only under load, which is what
[`concurrency/one-scheduler`](/docs/rules/concurrency/tasks/#one-scheduler "Concurrency is the Core\Task roster over the runtime's own single scheduler") refuses for the scheduler on the same ground. A shape that cannot be
expressed over the reactor is cut, not given a loop of its own.

The connected transports answer a `Read` and a `Write` shaped like every other stream in the language.
A datagram socket does not, because it has no stream to read: it sends and receives whole messages,
addressed one at a time.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#net-listen-is-a-separate-grant-from-net-connect" title="The grant follows what the program is doing rather than the transport: reaching out is net.connect, binding is net.listen, and neither widens the other"><code>security/net-listen-is-a-separate-grant-from-net-connect</code></a> <a href="/docs/rules/security/closed-doors/#a-path-is-not-a-url" title="A path is a filesystem path: no member dispatches on a scheme prefix, and nothing may register one"><code>security/a-path-is-not-a-url</code></a> <a href="/docs/rules/concurrency/tasks/#one-scheduler" title="Concurrency is the Core\Task roster over the runtime's own single scheduler"><code>concurrency/one-scheduler</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#net-udp-carries-no-reliability-layer" title="A datagram socket sends and receives messages and offers no ordering, retransmission or acknowledgement above them"><code>core-classes/net-udp-carries-no-reliability-layer</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0162.md">record 0162</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-tcp-echo-round-trips-over-the-reactor.nvst"><code>tests/conformance/core/net-a-tcp-echo-round-trips-over-the-reactor.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-closed-socket-refuses-every-member-that-needs-it.nvst"><code>tests/conformance/core/net-a-closed-socket-refuses-every-member-that-needs-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-read-is-bounded-by-its-max-and-leaves-the-rest.nvst"><code>tests/conformance/core/net-a-read-is-bounded-by-its-max-and-leaves-the-rest.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-no-door-reads-a-transport-out-of-its-argument.nvst"><code>tests/conformance/core/net-no-door-reads-a-transport-out-of-its-argument.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="net-a-socket-does-not-outlive-its-request">

## A `Core\Net` socket closes with the request that opened it, so there is no persistent connection and no pool across requests

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#net-a-socket-does-not-outlive-its-request"><code>core-classes/net-a-socket-does-not-outlive-its-request</code></a>
</div>

A socket opened through `Core\Net` closes with the request that opened it. There is no persistent
connection, no pool held across requests and no handle a later request can find, which is
[`security/no-cross-request-state`](/docs/rules/security/closed-doors/#no-cross-request-state "Nothing a request does is observable by another request except through an explicit, capability-gated store") applied to the one subsystem whose PHP ancestor offered the
opposite: `pfsockopen`'s whole purpose was a connection that outlived the script, and its row in the
migration table is a member whose persistent half is dropped.

What this spends, per [`programs/memory-priority`](/docs/rules/programs/claims-and-priorities/#memory-priority "Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"): one reactor registration per open socket,
attributable to the request that opened it and released with its arena. The process therefore holds
one registration per socket **in flight** and nothing per socket served, which is the O(in-flight)
bound that separates a cost from a leak.

A program that wants a connection to survive a request wants a store, and the stores are the ones
already granted by name — [`config/cache-shared-is-the-grant-over-the-configured-store`](/docs/rules/config/stores-and-caches/#cache-shared-is-the-grant-over-the-configured-store "A store an operator configured is authorized by the configuring — cache.shared is the grant, unscoped, and asks no address")'s shared
cache and [`core-classes/db-one-api`](/docs/rules/core-classes/connecting-to-a-database/#db-one-api "Core\Db is the only database API, and every statement it runs is prepared")'s database — where the endpoint is an operator's and the
lifetime is the runtime's.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#net-one-api-three-transports" title="Core\Net is one class over TCP, UDP and Unix sockets, reached through five entry points that park on the runtime's own reactor"><code>core-classes/net-one-api-three-transports</code></a> <a href="/docs/rules/security/closed-doors/#no-cross-request-state" title="Nothing a request does is observable by another request except through an explicit, capability-gated store"><code>security/no-cross-request-state</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0162.md">record 0162</a></dd></div></dl>

</div>

<div class="nv-rule" id="net-udp-carries-no-reliability-layer">

## A datagram socket sends and receives messages and offers no ordering, retransmission or acknowledgement above them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#net-udp-carries-no-reliability-layer"><code>core-classes/net-udp-carries-no-reliability-layer</code></a>
</div>

A `Core\Net` datagram socket sends and receives messages, reports what it sent and what arrived, and
offers nothing above that: no ordering, no retransmission, no fragmentation and no acknowledgement. A
datagram that is lost is lost, and a program that cannot tolerate that wants TCP.

The refusal is written down because the pressure to add "just a retry" is constant and its result is
always the same — a reliability layer nobody specified, whose failure modes belong to the library
while the timing budget it spends belongs to the program. A protocol that genuinely provides reliable
datagrams is a protocol, placed by [`core-api/tier-placement`](/docs/rules/core-api/what-belongs-in-core/#tier-placement "A library candidate is placed by six ordered tests, not by PHP's extension list") like any other, and Tier 0 is not
where it lands.

A program that builds ordering or retries over this surface is making that choice explicitly, which is
the point: the trade is visible in the program rather than hidden in a member's contract.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#net-one-api-three-transports" title="Core\Net is one class over TCP, UDP and Unix sockets, reached through five entry points that park on the runtime's own reactor"><code>core-classes/net-one-api-three-transports</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#tier-placement" title="A library candidate is placed by six ordered tests, not by PHP's extension list"><code>core-api/tier-placement</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0162.md">record 0162</a></dd></div></dl>

</div>
