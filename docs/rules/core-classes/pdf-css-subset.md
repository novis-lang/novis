The engine implements a **documented subset** of CSS — block and inline flow, tables, flex, fonts,
images, inline SVG and the page family are the working floor — and handles everything outside it by
CSS's own forward-compatible parsing: an unknown declaration is dropped, never guessed at.

What this rule adds to that standard behaviour is **visibility**. The render result carries the list
of dropped declarations, so a test asserts the list is empty and a document that silently depends on
unsupported CSS cannot survive CI. "Renders fine in the browser, wrong in the library" is the failure
an invisible drop produces, and a visible list is what turns it into a test.

**Not shipped.** There is no PDF package in the tree.
