Writes an XML comment, such as `<!-- Generated file -->`, at the place the writer has reached.

You can write a comment before the root element, inside an element, or after the root element is
closed. A program that reads the document skips comments, so they are notes for a person.

XML has no way to escape anything inside a comment. The writer writes your text as it is. For this
reason, the text cannot contain two dashes in a row (`--`), and it cannot end with a dash. If it
does, `comment` throws a `LogicError` and writes nothing. A text with `<` or `&` is allowed, and it
is written unchanged.

**The examples below** add comments in three places, show the two errors, and put a header comment
at the top of an exported file.
