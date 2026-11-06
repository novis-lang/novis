Chooses pages of an existing PDF file, so `Pages::merge` can put them into a new file.

`Pages::of` takes a PDF file and a list of page numbers. The file is `bytes`, an open
`Core\IO\File` or an uploaded `Core\Request\Part`. Pages count from 1. The new file has the pages in
the order of the list. A page listed twice is in the new file twice, and a page not in the list is
left out. Without a list, every page is chosen, in its own order.

**Good to know:** `Pages::of` does not read the file. `Pages::merge` reads it, so the file must
still be open then. A page number past the end, or a file that is not a PDF, throws an error in
`merge`.

**The examples below** reorder the pages of a file, take every page of one file and one page of
another, and take the first page of each invoice for a summary.
