Sets the PDF standard that the file follows: `PdfA2b`, `PdfA3b` or `PdfUa1`.

Every PDF file that a render makes is tagged. A tagged file has its headings, tables and image text
marked, so a screen reader can read it. A standard adds strict checks on top of that.

- `Document::Standard::PdfA2b` and `PdfA3b` are PDF/A, for files kept in an archive for many years.
  A PDF/A file needs a creation time, which you set with `created`. It cannot have a password.
- `Document::Standard::PdfA3b` is also the standard for an electronic invoice, with `invoiceXml`.
- `Document::Standard::PdfUa1` is PDF/UA, for accessible files. It needs a `<title>`, and every
  image needs `alt` text.

A document that breaks its standard makes `render` throw a `LogicError` that says what is wrong.
A document has one standard at most.

**The examples below** make a certificate for an archive, make an accessible report, and show the
errors for a document that breaks its standard.
