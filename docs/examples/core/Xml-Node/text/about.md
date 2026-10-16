Returns the text of one XML node, as a string. For a text node, this is its characters. For a
comment, it is the comment's content. For a processing instruction such as `<?print fast?>`, it is
the part after the name, here `fast`.

An element returns an empty string. The text of `<name>Mug</name>` is in the text node inside the
element, so you read it from the element's `children`. The document node also returns an empty
string.

Entities such as `&amp;` are already expanded, so you get `&`. A CDATA section gives the text exactly
as it is written inside it. The result is `tainted`, because it came from outside the program.

**The examples below** show where an element's text is, read a comment and a CDATA section, and read
the values of a small settings file.
