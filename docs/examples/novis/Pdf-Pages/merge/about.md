Puts the chosen pages of several PDF files into one new PDF file.

`Pages::merge` takes a list of parts. Each part is a `Pages::of` with a file and the pages to take
from it. The new file has the pages of the first part, then the pages of the second part, and so on.
You can add terms to a contract, split a file, or take pages out of it.

A page number past the end of its file throws a `LogicError`. A file that is encrypted, or that is
not a PDF file, throws a `ParseError`. A part of a file that unpacks to more than the server's
`[limits]` allow also throws a `ParseError`. An empty list of parts also throws an error.

**Good to know:** two merges of the same parts give the same bytes. A test can compare the new file
with a saved copy.

**The examples below** add the terms to a contract on a letterhead, show the error for a page past
the end, and show the error for an encrypted file.
