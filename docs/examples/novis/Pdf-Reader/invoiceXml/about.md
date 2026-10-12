Returns the XML of an electronic invoice that is attached to a PDF file.

An electronic invoice in the Factur-X, ZUGFeRD or XRechnung format is a PDF file with its data
attached as XML. The attached file has the name `factur-x.xml`, `zugferd-invoice.xml` or
`xrechnung.xml`. `invoiceXml` returns that XML as a string, with the same text the sender attached.
If the file is not an electronic invoice, the result is `null`.

An invoice XML that is not UTF-8 text makes `Reader::open` throw a `ParseError`. Apart from that,
the XML is not checked. Use `Core\Xml::parse` to read it. `Core\Xml::parse` throws a `ParseError`
for XML that is not well-formed.

**The examples below** read the XML of an uploaded invoice, show the `null` result for a letter, and
sort uploads into invoices and other files.
