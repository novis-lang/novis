Sets a function that returns the footer of each page of a document.

`footer` works like `header`, in the bottom margin. `render` calls the function once for each page,
with that page as a `Page`. The function returns an `html` template. `height`, such as
`{ height: "2cm" }`, sets the bottom margin.

A footer often needs numbers from the page. `$page->isFirst()` and `$page->isLast()` test where the
page is. `$page->marksUpTo("amount")` returns every `data-mark="amount:..."` value up to this page,
so the footer can show the total so far.

The footer never moves the body. A footer taller than its margin is cut off, and
`Rendered::overflows` lists it with its page.

**The examples below** carry a total forward on an invoice, give the first page its own footer,
and find a footer that does not fit.
