Reads the next node of an XML document and returns it. At the end of the document, it returns
`null`.

You get a reader from `Core\Xml::reader`, and you call `read` in a loop. Each call returns one node:
an element, a text, a comment or a processing instruction. An element is returned when its opening
tag is read. It has its name and its attributes, but no children. The nodes inside it are returned
by the next calls. A closing tag is not a node.

The reader does not build a tree, so it is a good choice for a large document. If the document is
broken, `read` first returns every node before the error, and then throws a `ParseError`.

**The examples below** read every node in order, show a document that is broken in the middle, and
add up the stock in a product feed.
