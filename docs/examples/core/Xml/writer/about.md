Returns a writer that builds a new XML document one node at a time. You call `startDocument`,
then open elements, add attributes and text, and close the elements again. `endDocument` returns
the finished document as a string.

The writer escapes every text and attribute value for you. A `<` or `&` in your data stays text
and never becomes markup.

The writer also remembers which elements are open, so `endElement` needs no name. A document
that is not finished is an error. `endDocument` throws a `LogicError` while an element is still
open, and so does any call in the wrong order.

The option `indent` puts each element on its own line and indents it. It must contain only
whitespace, such as two spaces.

**The examples below** write a small document, show how text is escaped, and export a list of
orders.
