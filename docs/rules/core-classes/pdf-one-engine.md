The sole input language is **HTML plus the CSS subset plus inline SVG**. The top-down way of building
a document — append a heading, a table, a page break, mix computed parts with template parts — is a
Novis-side builder that **emits that same HTML**, and document parts compose by concatenation before
a single render call.

There is no second, imperative engine with its own coordinate model. That would be two spellings for
one job, and a program that owns both a DOM-based renderer and a coordinate-based one maintains two
layouts of every page. Drawings are inline SVG, the same answer the image component gives for charts.

The HTML is parsed by the same crate the language's own parser uses
(`rule:core-classes/html-parsing`), compiled into the sandbox rather than linked natively, so the
language and its PDF component parse HTML identically.

**The interface is the contract; the engine is a choice the program states explicitly.** A document
the subset will never reach — heavy JavaScript-era CSS, a pixel-perfect brand PDF signed off against
a browser — is served by an **opt-in** second backend behind the same call shape: an external
headless browser driven under the process capability (`rule:core-classes/process-is-argv-only`),
never shipped in the default path, and documented as trading the no-I/O guarantee for fidelity. It is
a second *engine*, not a second API, which is the whole reason a single interface was worth fixing
first.

No HTML paged-media engine exists to link as a library, so the engine is assembled from production
Rust crates — `html5ever`, `taffy` for layout, `rustybuzz` for shaping, `resvg` for SVG and
`krilla`/`pdf-writer` for the file — plus the pagination layer nobody ships, which is ours to write.
The rules on this component are written against the interface, not the crates: a production-grade Rust
paged-media engine may replace the assembled one wholesale, and it must still meet them.

- **Memory:** per request that renders, the HTML, the asset map, the layout tree and the output
  document, all inside the extension's memory cap, none of it outliving the request. Wall-clock is
  milliseconds for an invoice and seconds for a long report, so a bulk or slow render belongs in the
  queue.

**Not shipped.** There is no PDF package in the tree.

**Declined:**
- **An engine written in Novis.** It would be a second, unfuzzed implementation of shaping, bidi, line
  breaking, cascade and layout, buying nothing the wasm build does not already give. Novis code
  belongs in the builder and the templates.
- **Headless Chromium as the engine.** It is a browser process per render, hundreds of megabytes
  outside any request's attribution, with its own sandbox and lifecycle to operate. It is the opt-in
  fidelity backend above, not the foundation.
- **A first-party imperative PDF API.** The first-party answer to drawing at a coordinate is HTML, CSS
  and SVG; a third-party package may carry one.
