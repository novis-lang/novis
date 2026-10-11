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

**Not shipped.** There is no PDF component in the tree.

**Declined:**
- **A strict mode that throws on the first dropped declaration.** A test asserting `dropped` is empty
  is the strict mode, and it shows every drop at once instead of the first.
