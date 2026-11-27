The PDF component carries **Liberation Sans, Serif and Mono in regular, bold, italic and bold
italic**, so a plain document renders with no font in its asset map. `sans-serif`, Arial and Helvetica
map to Sans, `serif`, Times and Times New Roman to Serif, and `monospace` and Courier to Mono. An
SVG `<text>`'s `font-family` maps the same way, and a list where no name maps is Sans in both. The same
twelve files stand in for the standard-14 fonts when the image component rasterises a PDF
(`rule:core-classes/pdf-page-is-an-image-source`), and live once in the tree under `extensions/fonts/`.

**The font chain** of a run of text is the `@font-face` fonts its `font-family` names, from the asset
map, then the default of its generic family. A name that is neither, a misspelt family or one a
browser would find installed, is skipped and listed in `Rendered::dropped` with `font-family` as its
property and the name as its value: with no system lookup it matches no font on any machine, and the
text is drawn in another font. Every such name in a declaration is listed, after a name that matched
too, and each once. A character no font in the chain has is drawn as the
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

The twelve files are under `extensions/fonts/` with their release recorded in `SOURCE.json`, and the
image and PDF components embed them. `extensions/pdf/src/fonts.rs` maps the family names above to
them, registers each `@font-face` font from the asset map, and builds the chain;
`extensions/pdf/src/css/mod.rs` lists a name the chain skips, `extensions/pdf/src/layout.rs` lists
the missing glyphs, and `extensions/pdf/tests/layout.rs` holds the chain, the subsetting, the
skipped names and the missing-glyph list.

**Declined:**
- **Noto or another font with wide script coverage as a default.** Its size grows the binary by tens of
  megabytes for scripts most documents never use; a document in another script passes its font as an
  asset.
- **Throwing on a missing glyph.** One unknown character in a customer's name would lose the whole
  invoice.
- **A separate list of missing fonts on `Rendered`.** It would add a constructor parameter and a
  fourth list every check reads, for a fact `dropped` already carries with the declaration's line and
  column. A test that asserts `dropped` is empty catches a misspelt family with no change.
- **Listing only a name with no fallback after it.** A misspelt family before `sans-serif` is the
  common mistake, and it would pass unseen.
