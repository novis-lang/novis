Chooses pages of an existing PDF file, so `Pages::merge` can put them into a new file.

`Pages::of` takes the bytes of a PDF file and a list of page numbers. Pages count from 1. The new
file has the pages in the order of the list. A page listed twice is in the new file twice, and a page
not in the list is left out. Without a list, every page is chosen, in its own order.

**Good to know:** `Pages::of` does not read the file. `Pages::merge` reads it, and that is where a
page number past the end, or a file that is not a PDF, throws an error.

**The examples below** reorder the pages of a file, take every page of one file and one page of
another, and take the first page of each invoice for a summary.
