Starts a PDF document from HTML.

The HTML can be a string or the result of an `html` template. Use a template when the page has text
from a user, such as a customer name. The template escapes that text, so it shows as text and never
as HTML. A document has only the HTML it was given. An image, a stylesheet or a font that the HTML
names by a file name must be added with `asset`.

`fromHtml` returns a new `Document`. Each method after it, such as `header` or `password`, returns
a new `Document` too. Call `render` at the end to make the PDF file.

**Good to know:** the PDF follows the CSS for printed pages. `@page` sets the paper size and the
margins, and `break-before: page` starts a new page.

**The examples below** make a receipt from a string, build a page with an `html` template, and make
a long report with a table of contents.
