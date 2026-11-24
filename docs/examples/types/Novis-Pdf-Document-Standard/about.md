The set of rules that a PDF file follows. `Document::withStandard` returns a copy of the document with
this standard set.

`Document::Standard` is declared in the class `Novis\Pdf\Document`.

- `PdfA2b` is PDF/A-2b, for files kept in an archive for many years.
- `PdfA3b` is PDF/A-3b. It is also for archives. It is the only standard that can contain an
  electronic invoice. An electronic invoice is an invoice as an XML file that a program can read.
  `Document::withInvoiceXml` returns a copy of the document with this XML file attached.
- `PdfUa1` is PDF/UA-1, for files that a screen reader (a program that reads text aloud) can read.
  The HTML of the document needs a `<title>` element, and every image needs `alt` text.

A PDF/A file needs a creation time, which you set with `Document::withCreationTime`. It cannot have a
password. A document that breaks its standard makes `render` throw a `LogicError` that says what is
wrong.

A document has one standard at most. Every file has tags, with a standard or without one. Tags tell a
screen reader which parts of a page are headings, lists, tables or images.
