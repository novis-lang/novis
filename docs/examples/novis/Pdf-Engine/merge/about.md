Puts the chosen pages of several PDF files into one new PDF file.

`Engine::merge` takes a list of parts. Each part is a shape with `data`, the PDF file, and `pages`,
the page numbers to take from it. Pages count from 1. When `pages` is `null`, every page of the
file is taken. The new file has the pages of each part, in the order of the list.

A page number past the end of its file throws a `LogicError`, and so does a list that chooses no
page at all. A file that is encrypted, or that is not a PDF file, throws a `ParseError`. A merge
that needs more memory than the PDF engine may use stops the program.

**Good to know:** most programs do not call `Engine::merge` directly. `Pages::merge` calls it, and
`Pages::of` builds each part for you.
