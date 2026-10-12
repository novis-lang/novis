The CSS subset is **data**: `extensions/pdf/subset.json` lists every property, value function,
at-rule, selector and PDF-specific HTML attribute the engine implements, each with a one-line plain
description, and nothing restates it item by item. A test reads the file: every property it lists
parses, and every property it does not list is dropped and reported
(`rule:core-classes/pdf-css-subset`). The editor's completion and hover data are generated from it, so
the editor offers exactly what renders.

What the subset holds, by family — the file is the list, this is the decision:

| Family | In the subset |
|---|---|
| Cascade | type, class, id, attribute, descendant, child and sibling selectors, the structural pseudo-classes, `::before` and `::after`, specificity, inheritance, `!important`, `var()`, `calc()`, `@media print` and `all` (never `screen`), `@font-face` |
| Text | fonts and the font chain, alignment with `justify`, `line-height`, indent, decoration, transform, spacing, `white-space`, `word-break`, `overflow-wrap`, `text-overflow: ellipsis`, `vertical-align`, `font-feature-settings`, bidi, CJK line breaking, `hyphens: auto` |
| Boxes | margin, padding, borders (solid, dashed, dotted, double), `border-radius`, sizes with min and max, `box-sizing`, background colours, images and gradients, `outline`, `opacity`, `inline-block`, `display: none` |
| Lists | markers (decimal, roman, alpha, disc), `counter-reset`, `counter-increment`, `content` |
| Images | JPEG, PNG, GIF, WebP, SVG and a PDF page in `<img>`, inline `<svg>`, `object-fit` |
| Layout | flex and grid, floats and `clear`, `relative`, `absolute` and `fixed` positioning with `z-index`, 2D transforms |
| Pages | `@page` with size, margins, `:first`, `:left`, `:right`, `:blank` and named pages; the 16 margin boxes; `counter(page)`, `counter(pages)`, `string-set`, `string()`, `position: running()`, `element()`, `target-counter()`, `leader()`; `break-before`, `break-after`, `break-inside`, the `page-break-*` aliases, `orphans`, `widows` |
| Tables | `rule:core-classes/pdf-tables-across-pages`, with `box-decoration-break: slice` |
| HTML | `data-repeat`, `data-mark`, `<slot name>`, `<a href>` to `http(s):`, `mailto:` and `#anchor`, `<title>`, `<meta>`, `lang` |

**Out of the subset, dropped and reported:** footnotes, multi-column, `box-shadow`, `text-shadow`,
`filter`, masks, `clip-path`, blend modes, 3D transforms, animation, transitions, `position: sticky`,
vertical writing modes, ruby, subgrid, `box-decoration-break: clone`, form fields, `<iframe>`,
`<video>` and `<canvas>`. `<script>` never runs. Splitting one flex line or grid row across pages is
out too: a row moves to the next page whole.

The line between in and out is one test: **every common case is in, an edge case is in when it is
easy, and a hard edge case almost nobody needs in a document stays out.** Everything on the out list
either draws nothing on paper (animation, transitions, `sticky`), needs a second layout model for a
rare document (footnotes, multi-column, vertical writing, ruby, subgrid) or is a raster effect a
vector file draws badly (shadows, filters, masks, blend modes).

- **Memory:** none at run time beyond the cascade's computed styles, which are part of the layout tree.

**Partly on disk.** `extensions/pdf/subset.json` is the list, and `extensions/pdf/tests/css.rs`
holds it equal to the properties `extensions/pdf/src/css/properties.rs` parses. The editor's data is
not generated from it yet.

**Declined:**
- **The subset as prose only.** A list nothing reads drifts from what the engine parses, and the
  editor could not complete from it.
