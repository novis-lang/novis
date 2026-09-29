Writes a processing instruction, such as `<?xml-stylesheet href="feed.css" type="text/css"?>`.

A processing instruction is a note for the program that reads the document. It has a target, which
names the program or the purpose, and data, which is plain text. The writer writes the data
unchanged, and it does not escape anything. When the data is `""`, only the target is written. You
can write an instruction before the root element, inside an element or after it.

The target must be a valid XML name, and it cannot be `xml` in any casing. `startDocument` already
writes that one. The data cannot contain `?>`, because that ends the instruction. In each of these
cases, `instruction` throws a `LogicError` and writes nothing.

**The examples below** read an instruction back with the parser, show the three errors, and link
a style sheet to a feed.
