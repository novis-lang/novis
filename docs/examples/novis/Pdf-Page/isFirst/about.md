Checks whether a page is the first page of the document.

`isFirst` returns `true` when the page number is 1, and `false` on every other page. Use it in a
header or footer function to give the first page a different header. A letter often has the full
company address on the first page and only a short line on the pages after it.

**Good to know:** the CSS rule `@page :first` sets the margins and margin boxes of the first page.
`isFirst` is for a header or footer that a function writes.

**The examples below** leave out the header on the first page, put a letterhead on the first page
only, and print a page number on every page except a title page.
