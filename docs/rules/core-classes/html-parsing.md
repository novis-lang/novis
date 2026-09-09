HTML parses through `Core\Html`, by the WHATWG parsing algorithm, and the parse **never fails**:
implied tags, error recovery and foster parenting are the specified output every conforming parser
produces, so recovering here does not violate the refuse-never-repair rule — nothing is guessed,
because the specification fixes the answer. `Core\Xml` keeps the opposite contract: malformed XML
throws. One API flipping between refuse-hard and recover-always under a flag is the ambiguity being
retired, and it is what PHP's libxml2 surface is.

Both parsers materialise **the same node family**. Queries, traversal and the tree's memory story are
written once, and which door parsed a document does not change what a program can do with it.
Serialization follows the door: WHATWG rules through one, XML rules through the other. The engine is
`html5ever` driving a tree builder we own, so it builds request-attributed nodes directly rather than
through its sample DOM, and the same crate is compiled into the PDF component — the language and its
PDF renderer parse HTML identically: one behaviour to document, one parser to fuzz.

What it spends, per parse: the materialised tree, proportional to the document, attributed to the
request and gone with it.

**Half shipped, and the halves do not separate.** `Core\Xml`'s door and the node family both parsers
produce are on disk; the WHATWG parse over them is not, and the goal that landed the tree builds it.
The tree and the builder interface are one implementation, so whichever was built second would
otherwise have been shaped by the first — which is why the tree came first and this parse is written
against it rather than beside it.
