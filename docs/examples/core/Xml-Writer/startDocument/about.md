Starts a new XML document. It writes the XML declaration, the first line of every document:
`<?xml version="1.0" encoding="UTF-8"?>`. You call it once, before any other method of the writer.

A writer writes exactly one document. If you call `startDocument` a second time, it throws a
`LogicError`. This is also true after `endDocument` has finished the document. To write a second
document, create a new writer with `Core\Xml::writer()`.

Every other method of the writer needs a started document. If you call `startElement` before
`startDocument`, it throws a `LogicError`.

**The examples below** show the declaration, the errors, and a loop that writes one document for
each customer.
