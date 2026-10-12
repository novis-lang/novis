Returns the text of chosen pages of a PDF file.

`Engine::readText` takes the bytes of a PDF file and a list of page numbers. Pages count from 1. It
returns a list with the text of each page, in the order of the page numbers. A page can be in the
list more than once. An empty list of page numbers gives an empty result.

A page number past the end of the file throws a `LogicError`, and so does page 0. A file that is
encrypted, or that is not a PDF file, throws a `ParseError`.

**Good to know:** most programs do not call `Engine::readText` directly. `Reader::text` calls it
for one page.
