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

What it spends, per request that renders: the HTML, the asset map, the layout tree and the output
document, all inside the extension's memory cap, none of it outliving the request. Wall-clock is
milliseconds for an invoice and seconds for a long report, so a bulk or slow render belongs in the
queue.

**Not shipped.** There is no PDF package in the tree.
