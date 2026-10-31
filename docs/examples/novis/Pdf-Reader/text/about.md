Returns the text of one page of a PDF file.

`text` takes a page number. Pages count from 1. The result is the text of that page, in the order
the file stores it, with one line of the page on each line of the result. Tables and columns are
not rebuilt, so the text of two columns can be mixed.

`Reader::open` reads the file once and keeps it until the request ends. Each call to `text` reads
that copy. You can call `text` for many pages, and the file is not copied or read again.

A page number past the end of the file throws a `LogicError`. A page whose content unpacks to
more than 64 MB, or to more than 1000 times its packed size, throws a `ParseError`. The server's
`[limits]` set these two numbers, as for `Reader::open`.

**The examples below** print the text of each page, show the error for a page past the end, and
find the line with the total on an invoice.
