Returns a reader that goes through an XML document one node at a time. Each call to `read`
returns the next node, and `null` at the end of the document.

The reader does not build a tree. It keeps the current node and the names of the elements that
are open around it. This is useful for a large document, such as an export with thousands of
records, when you only need a few values from it.

An element is returned when its opening tag is read. Its children follow as the next nodes. A
closing tag is not a node. `depth` tells you how many elements are open around the node you just
read.

The reader finds an error only when it reaches it. The nodes before the error are returned
normally, and then `read` throws a `ParseError`. The rules are the same as for `Core\Xml::parse`.

**The examples below** list the elements of a document, show an error in the middle of one, and
count records in a large export.
