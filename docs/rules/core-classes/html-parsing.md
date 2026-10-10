HTML parses through `Core\Html`, by the WHATWG parsing algorithm, and the parse **never fails**:
implied tags, error recovery and foster parenting are the specified output every conforming parser
produces, so recovering here does not violate the refuse-never-repair rule — nothing is guessed,
because the specification fixes the answer. `Core\Xml` keeps the opposite contract: malformed XML
throws. One API flipping between refuse-hard and recover-always under a flag is the ambiguity being
retired.

Both parsers materialise **the same node family**. Queries, traversal and the tree's memory story are
written once, and which door parsed a document does not change what a program can do with it.
Serialization follows the door: WHATWG rules through one, XML rules through the other. The engine is
`html5ever` driving a tree builder we own, so it builds request-attributed nodes directly rather than
through its sample DOM, and the same crate is compiled into the PDF component — the language and its
PDF renderer parse HTML identically: one behaviour to document, one parser to fuzz.

**Browser equivalence is the point, and so it is a compatibility surface.** A parser that builds a
different tree from the one a browser builds is a security defect, not a nuisance: mutation XSS is
exactly a sanitizer approving one document while the browser renders another. Parsing by the browser's
own algorithm leaves no second parse to disagree with, which is what `rule:core-classes/html-sanitize`
stands on. What `html5ever` parses is what Novis parses, so a divergence from WHATWG is an upstream bug
to fix, never a dialect to document. HTML parsing is request-path work over hostile input, and the
sanitizer in Core must parse to filter, so the parser is Core and not a component.

**Memory:** each parse holds the materialised tree, proportional to the document, attributed to the
request and freed with it. It is the same account `Core\Xml`'s tree opens, spent once because the tree
is one implementation.

**The stack of open elements holds at most 512 elements**, the depth Chromium, Firefox and WebKit all
cap a tree at. A start tag that arrives with 512 elements open first closes the current element, as
WebKit does, and the new element becomes its sibling: nesting past the cap flattens, and every
element, attribute and character of the document still arrives. The close is an end tag for the
current element's own name, a token the algorithm already has a rule for, so the parse still never
fails. The cap is a constant and not a setting, because it is what bounds the work one token costs:
every scope check the algorithm makes walks that stack, and without the cap a document costs time in
the square of its depth. It is part of the parse, so the PDF renderer holds it too. The two other
checks that grow with a document — a tag's attributes against each other for a duplicate, and the
active formatting elements against a new one for an equal entry — cost time in proportion to what
they check, and neither changes what the parse produces.

**The tree came first and this parse is written against it**, rather than beside it: the tree and the
builder interface are one implementation, so whichever was built second would otherwise have been
shaped by the first. `Core\Html::parse` drives `html5ever` through a tree builder of ours straight
into those nodes, so the boundary between the crate and the tree is where the one-family rule is
enforced — a construct one door could produce that the other could not would have to be a sixth kind
of node, and there is no way to write one.

Two placements follow from the family being closed at five, and both are refusals to invent a node
rather than gaps. A `<!DOCTYPE …>` leaves nothing behind, which is this door's answer to what the
other door refuses outright. A `<template>`'s contents stay on the element, because the DOM's
separate fragment has no kind here and a program looks for them under the element it wrote.

**Declined:**
- **An HTML mode on `Core\Xml`.** Two opposite failure contracts under one API, switched by a flag, is
  the ambiguity `rule:core-api/one-paradigm-per-operation` and `rule:errors/ambiguous-input-refused`
  exist to prevent, and it is the design behind two decades of HTML parsed differently from every
  browser.
- **A separate HTML node type.** It doubles every query and serializer for no gain in meaning.
- **`xml5ever` as `Core\Xml`'s parser.** It implements the error-recovering XML5 draft, the wrong
  contract for a door whose malformed input throws.
- **Writing the parser ourselves.** An unfuzzed reimplementation of one of the largest parsing
  specifications buys nothing `html5ever` does not already give, and loses on security and speed.
- **Leaving HTML parsing to a component or the ecosystem.** The sanitizer is Core and must parse to
  filter (`rule:core-classes/html-sanitize`).
