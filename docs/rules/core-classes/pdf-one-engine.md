The sole input language is **HTML plus the CSS subset plus inline SVG**, and there is one engine. A
program builds a document the way it builds a page: with ``html`…` `` templates
(`rule:core-classes/html-template`) that compose by concatenation before a single render call. There is
no Novis-side builder of headings and tables, because a template already is one.

There is no second, imperative engine with its own coordinate model. That would be two ways to write
one job, and a program that owns both a DOM-based renderer and a coordinate-based one maintains two
layouts of every page. Drawings are inline SVG, the same answer the image component gives for charts,
and an existing PDF's page is placed with `<img>` (`rule:core-classes/pdf-reader-and-merge`).

The HTML is parsed by the same crate the language's own parser uses
(`rule:core-classes/html-parsing`), the patched copy under `vendor/html5ever`, compiled into the
sandbox rather than linked natively, so the language and its PDF component parse HTML identically and
under the same element cap.

No HTML paged-media engine exists to link as a library, so the engine is **our own cascade, layout and
pagination** over production Rust crates: `cssparser` and `selectors` for the cascade, `parley` over
`harfrust` and `skrifa` for text, with `fontique`'s system font discovery off, `hypher` for
hyphenation, `taffy` for flex and grid, `usvg` and `krilla-svg` for SVG, `hayro` through `krilla`'s
`pdf` feature for placed pages, and `krilla` for the file. The pagination layer nobody ships is ours to
write. A crate that does not build for `wasm32-wasip2` is replaced by its nearest pure-Rust equivalent,
and this paragraph names the replacement. The rules on this component are written against the
interface, not the crates, so a production-grade paged-media crate may replace the assembled engine
wholesale and must still meet them.

- **Memory:** per request that renders, the HTML, the asset map, the layout tree and the output
  document, all inside the request's memory cap, none of it outliving the request. Wall-clock is
  milliseconds for an invoice and seconds for a long report, so a bulk or slow render belongs in the
  queue.
- **Depth.** Table cells and flex and grid items are each laid out by a call inside the one around
  them, so their nesting is bounded at 32 levels (`MAX_DEPTH` in `extensions/pdf/src/layout/text.rs`).
  A document nested deeper throws `LogicError` naming the bound. A level costs about 21 KiB of the
  component's stack in a release build, and the stack is 2 MiB (`extensions/pdf/.cargo/config.toml`),
  twice the target's default, so the deepest document leaves more than half free. The larger stack
  spends 1 MiB more linear memory per render. Plain blocks and inline-blocks are laid out without
  recursing and need no bound, and the parser's element cap already flattens anything past 512 open
  elements.
- **Limits.** Three inputs grow the work with their size and end at the request's memory limit (128
  MiB by default), which throws as every guest that reaches it does (`rule:packaging/a-guest-crash-throws`):
  one word of two million characters, 100,000 nested `div`s (flattened into siblings; 50,000 render
  under the default, and time grows linearly), and a page whose content stream saves its state a
  million times when its text is read. No bound below the memory limit is fairer, because each costs
  what it asks for. The attacks under `tests/hostile/novis/Pdf-Document/fromHtml/` and
  `tests/hostile/novis/Pdf-Reader/text/02-a-page-that-saves-its-state.nvs` pin them with
  `ends-early`.

**On disk.** `extensions/pdf/` parses with the patched `html5ever` under the same cap,
computes styles with its cascade over `cssparser` and `selectors`, shapes with `parley`, hyphenates
with `hypher`, lays out flex and grid with `taffy`, draws inline SVG with `usvg` and `krilla-svg`,
places existing pages through `krilla`'s `pdf` feature and reads them with `hayro-interpret`, and
writes with `krilla` (`extensions/pdf/Cargo.toml`). Pagination is `extensions/pdf/src/paginate/`.

**Declined:**
- **A Novis-side document builder** that appends headings, tables and page breaks and emits HTML. It
  is a second way to write what a template already writes, and every element it knows is one more
  name to learn and keep in step with the subset.
- **A headless-browser backend**, opt-in or default. It is a browser process per render, hundreds of
  megabytes outside any request's attribution, with its own sandbox and lifecycle to operate, and it
  fetches whatever the HTML names. A document the subset cannot reach is written in the subset, or
  rendered by a program outside Novis.
- **An engine written in Novis.** It would be a second, unfuzzed implementation of shaping, bidi, line
  breaking, cascade and layout, buying nothing the wasm build does not already give.
- **A first-party imperative PDF API.** The first-party answer to drawing at a coordinate is HTML, CSS
  and SVG; a third-party package may carry one.
- **`fulgur` or WeasyPrint as a dependency.** `fulgur` has the same design and is young, and WeasyPrint
  is Python; both are references and test comparisons only.
