Writes a document type declaration, such as `<!DOCTYPE html>`, before the root element.

Some programs that read XML look at this line to learn what kind of document they get. An XHTML
page is a common example: a browser reads `<!DOCTYPE html>` and shows the page in standards mode.
The declaration names the document type, and usually that name is the name of the root element.

`doctype` writes only the name. It does not write a link to a DTD file or any entity definitions,
so the document does not depend on any other file. A document has at most one declaration, and it
must come before the root element. In other cases, `doctype` throws a `LogicError` and writes
nothing.

The declaration is for programs outside Novis. `Core\Xml::parse` and `Core\Xml::reader` throw an
error for any document that contains `<!DOCTYPE`.

**The examples below** write a declaration, show the errors, and write an XHTML page.
