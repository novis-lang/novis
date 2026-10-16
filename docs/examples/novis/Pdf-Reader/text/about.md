Returns the text of one page of a PDF file.

`text` takes a page number. Pages count from 1. The result is the text of that page, in the order
the file stores it, with one line of the page on each line of the result. Tables and columns are
not rebuilt, so the text of two columns can be mixed.

A page number past the end of the file throws a `LogicError`. A page whose content needs more
memory than the PDF engine may use stops the program.

**The examples below** print the text of each page, show the error for a page past the end, and
find the line with the total on an invoice.
