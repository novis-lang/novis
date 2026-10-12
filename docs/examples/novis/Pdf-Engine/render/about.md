Renders an HTML document to a PDF file.

`Engine::render` takes one shape, and the shape decides what the call does. A shape with `html`,
`assets` and `options` renders the whole document at once. A shape with `document`, `headerHeight`
and `footerHeight` lays out the body and returns each page with its named strings and marks. A
shape with `pages` then draws the header, the footer and the slots of each page. It needs the call
with `document` earlier in the same request, and without it the call throws a `LogicError`.

The result has `data`, the PDF file, and `pageCount`. `pages` lists each page with its strings and
marks. `dropped`, `missingGlyphs` and `overflows` list what the render could not draw.

`Engine::render` throws a `LogicError` for an asset the HTML names but the shape does not have, and
for options the chosen PDF standard does not allow. An invoice XML that is not well-formed throws a
`ParseError`.

**Good to know:** most programs do not call `Engine::render` directly. `Document::render` builds
the shapes for you, and calls `Engine::render` once, or twice when the document has a header, a
footer or slots.
