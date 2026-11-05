The engine implements a **documented subset** of CSS (`rule:core-classes/pdf-css-subset-table`) and
handles everything outside it by CSS's own forward-compatible parsing: an unknown declaration is
dropped, never guessed at.

What this rule adds to that standard behaviour is **visibility**. `Rendered::dropped` lists every
declaration the engine dropped with its property, value, selector and source position, so a test
asserts the list is empty and a document that silently depends on unsupported CSS cannot survive CI.
"Renders fine in the browser, wrong in the PDF" is the failure an invisible drop produces, and a visible
list is what turns it into a test. The same holds for content: an element the subset does not render
(`<iframe>`, `<video>`, `<canvas>`, a form field) is listed and drawn as nothing, and `<script>` never
runs and is not listed, because no program expects it to run in a document.

Three other things a render cannot do are **reported, never thrown**, because the document is still
correct apart from them and a throw would lose the whole file over one character:

- a character no font in the chain has is drawn as the font's empty box and listed in
  `Rendered::missingGlyphs` with the fonts it tried (`rule:core-classes/pdf-default-fonts`);
- markup in a margin, a header, a footer or a slot taller than its area is listed in
  `Rendered::overflows` with its page (`rule:core-classes/pdf-header-is-a-page-function`);
- a declaration the subset lists with a value it does not take is dropped and listed like an unknown
  one.

The component's fixtures owe a fixture using an unsupported property that renders with that property
in the dropped list, and a clean fixture whose three lists are empty.

**On disk.** `extensions/pdf/src/css/` is the cascade, and `Rendered::dropped` lists what it
drops with its selector and position, and each element the subset does not render as `<name>` at its
start tag, and `Rendered::missingGlyphs` lists each character no font has
(`extensions/pdf/tests/layout.rs`). A length, a number or a page `size` that is not finite, as
written or once computed, is dropped and listed like any value its property does not take, and a
`var()` chain resolves each custom property once per element (`extensions/pdf/tests/hostile_input.rs`).
`overflow` clips a box's content to its padding box, and a clipped box with a `height` is placed
whole, so its clipped content adds no page. `visibility: hidden` keeps the space and draws
nothing, `collapse` on a table row takes no space, negative margins collapse with the positive
ones, a span's `white-space` and `line-height` and a block's `text-align-last` are applied, and
`<ol type>` and `reversed` set the markers (`extensions/pdf/tests/boxes.rs`). `Rendered::overflows` lists margin, header, footer and slot
markup taller than its area (`crates/nvs-ext/tests/pdf.rs`). An `@page` selector with a
pseudo-class other than `:first`, `:left`, `:right` and `:blank`, an `@media` feature other than
`width`, `height` and `orientation` and their `min-` and `max-` forms, and a linked sheet that is not
UTF-8 are dropped and listed. `@media` tests its features against the first page's size.
`counter-reset: page` restarts the page numbers, a CSS margin box or running element taller than
its margin is listed in `Rendered::overflows` under its at-rule name, and the corner boxes align
toward the page by css-page-3's table (`extensions/pdf/tests/pages.rs`). A `data-mark`, a `string-set`, a
running element and an `id` inside a table cell, an inline-block, a float or a flex item are on the
page that box draws them on (`extensions/pdf/tests/paginate.rs`). A `:left` or `:right` page of
another width than the first page of its name lays out its lines at its own width
(`extensions/pdf/tests/paginate.rs`). A float in the middle of a paragraph is placed at the top of
its line, or on the next page when it does not fit (`extensions/pdf/tests/paginate.rs`). A line
the floats beside it leave no room starts below them, by CSS 2.1 §9.5, so a float wider than its
container sits alone on its line; a band with some room but narrower than the line's first word
keeps the line there, overflowing (`extensions/pdf/tests/edge_text.rs`). An
absolute box with `left` and `right` and no `width` fills the room between them, and a box with a
`transform` is the containing block of the absolute boxes inside it
(`extensions/pdf/tests/layout.rs`). A flex container's `::before` and `::after` and a `<br>` in it
are flex items, `align-items: baseline` aligns the items' first lines, and an `auto` margin on an
item takes the free space. An inline box in a right-to-left line has its start side at its right,
and `hyphenate-character` sets the text drawn at a break inside a word
(`extensions/pdf/tests/layout.rs`). What the layout does not do is stated in the module docs of
`extensions/pdf/src/layout.rs` and the files under `extensions/pdf/src/layout/`.

**Declined:**
- **A strict mode that throws on the first dropped declaration.** A test asserting `dropped` is empty
  is the strict mode, and it shows every drop at once instead of the first.
