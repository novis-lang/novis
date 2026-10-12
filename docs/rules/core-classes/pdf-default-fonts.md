The PDF component carries **Liberation Sans, Serif and Mono in regular, bold, italic and bold
italic**, so a plain document renders with no font in its asset map. `sans-serif`, Arial and Helvetica
map to Sans, `serif`, Times and Times New Roman to Serif, and `monospace` and Courier to Mono. The same
twelve files stand in for the standard-14 fonts when the image component rasterises a PDF
(`rule:core-classes/pdf-page-is-an-image-source`), and live once in the tree under `extensions/fonts/`.

**The font chain** of a run of text is the `@font-face` fonts its `font-family` names, from the asset
map, then the default of its generic family. A character no font in the chain has is drawn as the
font's empty box and listed in `Rendered::missingGlyphs` with the chain it tried. It never throws: the
rest of the document is right, and a program that must have every glyph asserts the list is empty. A
font file that does not parse throws `ParseError` naming it. Every font is subset into the output, so
a document carries only the glyphs it draws.

There is no system font lookup, because the guest has no filesystem, and a render on a laptop and on a
server gives the same bytes.

The Liberation fonts are under **OFL-1.1**, admitted for font data only: the binary's attribution
carries their licence and copyright (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`),
and no code under OFL-1.1 is admitted by it.

- **Memory:** the twelve fonts are part of each component's bytes, a few megabytes of the binary. A
  render holds the faces it uses inside its request's cap.

**Partly on disk.** The twelve files are under `extensions/fonts/` with their release recorded in
`SOURCE.json`, and the image and PDF components embed them. `extensions/pdf/src/fonts.rs` maps the
family names above to them, registers each `@font-face` font from the asset map, and builds the
chain; `extensions/pdf/src/layout.rs` lists the missing glyphs.

**Declined:**
- **Noto or another font with wide script coverage as a default.** Its size grows the binary by tens of
  megabytes for scripts most documents never use; a document in another script passes its font as an
  asset.
- **Throwing on a missing glyph.** One unknown character in a customer's name would lose the whole
  invoice.
