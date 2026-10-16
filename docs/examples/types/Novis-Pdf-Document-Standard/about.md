A PDF standard that the file follows. `Document::standard` sets it.

`Document::Standard` is declared in the class `Novis\Pdf\Document`.

- `PdfA2b` is PDF/A-2b, for files kept in an archive for many years.
- `PdfA3b` is PDF/A-3b. It is also for archives, and it is the standard for an electronic invoice
  with `invoiceXml`.
- `PdfUa1` is PDF/UA-1, for files that a screen reader can read. It needs a `<title>`, and every
  image needs `alt` text.

A PDF/A file needs a creation time, which you set with `Document::created`. It cannot have a
password. A document that breaks its standard makes `render` throw a `LogicError` that says what is
wrong.

A document has one standard at most. Without one, the file is still tagged for screen readers.
