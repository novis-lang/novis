Finishes the document and returns it as one string. This is the only method of the writer that
returns something. You can print the string, send it as a response, or save it to a file.

Before it returns, `endDocument` checks the document. If an element is still open, it throws a
`LogicError` that names the element. It also throws a `LogicError` when no root element was
written. So the string you get is always a complete document that an XML parser can read.

After `endDocument`, the writer is finished. Every other call on it throws a `LogicError`.

**The examples below** show the returned string, the errors, and a function that builds a sitemap
and returns it.
