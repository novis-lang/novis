Returns what kind of node this is, as a `Core\Xml\NodeKind` case. There are five kinds: an element, a
text, a comment, a processing instruction and the document.

The node that `Core\Xml::parse` returns is always the document. Every node below it is one of the
other four. The spaces and line breaks between two elements are text nodes too.

A program usually checks the kind before it reads anything else from a node. For example, it skips
everything that is not an element, or it collects only the comments. You compare the result with a
case such as `Core\Xml\NodeKind::Element`, or you use `match` to handle all five kinds.

**The examples below** name the kind of every node in a document, find the comments, and get the plain
text of a description written as XHTML.
