Closes the element that `startElement` opened last. You do not pass the element's name, because the
writer remembers which elements are open and in which order. So every end tag matches its start tag,
and a document can never close `<a>` with `</b>`.

An element with nothing inside is written in the short form `<a/>`. An element with text or other
elements inside is closed with `</a>`.

When no element is open, `endElement` throws a `LogicError`. If you forget to close an element,
`endDocument` throws a `LogicError` that names the element that is still open.

**The examples below** show the two forms of an end tag, the errors, and a function that writes a
tree of categories by calling itself.
