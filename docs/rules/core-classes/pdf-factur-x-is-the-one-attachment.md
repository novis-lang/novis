`Document::invoiceXml` embeds **exactly one Factur-X (ZUGFeRD) XML file**, under PDF/A-3b only, and it
is the only attachment the writer ever emits (`rule:core-classes/pdf-output-is-inert`). The file name,
the `AFRelationship` and the XMP fields are the ones the standard fixes, so the program passes the XML
and nothing else. The XML is checked well-formed, inside the request's memory cap and with no entity
expansion, and is otherwise the program's to build. The component reads one thing from it: the
guideline id of its `GuidelineSpecifiedDocumentContextParameter`, which gives the profile
`fx:ConformanceLevel` names, `EN 16931` when the id is not one the standard lists. `MINIMUM` and
`BASIC WL` are attached as `Data`, because neither is a whole invoice, and every other profile as
`Alternative`.

An invoice XML without `Document::Standard::PdfA3b` throws `LogicError`, because Factur-X is defined
only on PDF/A-3. An XML that is not well-formed throws `ParseError`, and so does one that nests
elements more than 256 levels deep. A real invoice nests about ten, and the bound keeps a hostile one
from costing more than its node tree. `Reader::invoiceXml` returns the XML of a received invoice
(`rule:core-classes/pdf-reader-and-merge`), and throws `ParseError` for one that parses and nests
deeper than 256 levels.

An e-invoice is the one attachment businesses are required to send, and a hybrid PDF is how most of
Europe sends it. Any other attachment is a file a reader may be asked to open, which is the
delivery channel `pdf-output-is-inert` closes.

- **Memory:** the XML once, inside the request's cap, and the bytes it adds to the output.

**On disk.** `Document::invoiceXml` is in `extensions/pdf/nvs/Document.nvs`. An invoice XML without
PDF/A-3b, and one that is not well-formed, is refused before anything is laid out
(`extensions/pdf/src/lib.rs`'s `first`). `extensions/pdf/src/factur_x.rs` checks, embeds and writes
the XMP fields, and its module doc says how.

**Declined:**
- **General attachments.** A file a reader opens from inside a PDF is a delivery channel for anything,
  and a program sends other files beside the PDF.
- **Building or validating the invoice's content.** The XML schema and the invoice's rules belong to
  the program's invoicing code, which already knows the invoice.
