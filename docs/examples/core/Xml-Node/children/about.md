Returns the children of an XML node as an array, in the order of the document. A child is a node
directly inside another node.

For the document node that `Core\Xml::parse` returns, the children are the root element and any
comment or processing instruction beside it. For an element, the children are everything between
its start tag and its end tag: elements, text, comments and processing instructions. A text, a
comment and a processing instruction have no children, so for them the array is empty.

The spaces and line breaks between two elements are text nodes, so they are children too. Use
`kind()` when you only want the elements.

**The examples below** visit every element of a document, show the text between elements, and read
the lines of an order.
