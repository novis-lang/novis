Sets a function that returns the header of each page of a document.

`render` calls the function once for each page, with that page as a `Page`. The `Page` has the page
number, the number of pages, and values that the HTML sets for that page, such as the current
chapter. The function returns an `html` template, and `render` draws it in the top margin of that
page.

The header never moves the body of the document. `height`, such as `{ height: "3cm" }`, sets the top
margin before the body is placed on the pages. Without it, the margin is the one that CSS `@page`
gives. A header taller than its margin is cut off at the margin. `Rendered::overflows` lists it with
its page number.

**Good to know:** a header that needs no logic can be plain CSS. `@page { @top-right { content:
counter(page) } }` writes the page number. When both are there, the function's header is drawn.

**The examples below** number the pages of a report, show the chapter of each page, and make room
for a taller header.
