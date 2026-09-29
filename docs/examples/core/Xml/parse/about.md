Reads a whole XML document and returns it as a tree. XML is a text format for structured data,
such as a product feed or an invoice.

The tree is made of nodes of the class `Core\Xml\Node`. The document node is at the top, and its
child is the root element. An element has a name, attributes and children. The text inside an
element is a node of its own.

The document must be well-formed. A tag that is never closed, two root elements or an unknown
entity throws a `ParseError`. The method never repairs a document, and it never loads anything
from a file or from the network.

Every text you read from the tree is tainted. This means it came from outside the program.

**The examples below** read a small document, show what happens with a broken one, and read a
stock list from a supplier.
