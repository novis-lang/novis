Attaches the XML of an electronic invoice to the PDF file, in the Factur-X format.

A Factur-X invoice, also called ZUGFeRD, is one file for two readers. A person reads the PDF pages,
and accounting software reads the attached XML. Your program builds the XML and passes it as a
string. The render attaches it under the name `factur-x.xml`, with the fields the standard needs.

The XML needs the standard `Document::Standard::PdfA3b`, and PDF/A needs a creation time from
`created`. Without PDF/A-3b, `render` throws a `LogicError`. XML that is not well-formed makes
`render` throw a `ParseError`. XML with elements nested more than 256 levels deep also throws a
`ParseError`. Apart from that, the render does not check the XML. The invoice
fields in it are your program's work.

**The examples below** make an electronic invoice, show the error without PDF/A-3b, and show the
error for XML that is not well-formed.
