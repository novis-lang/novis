Returns the name of an XML node, as a string.

For an element such as `<price>`, the name is `price`. For a processing instruction such as
`<?print fast?>`, the name is `print`. A text node, a comment and the document have no name, so the
result is an empty string.

The name is exactly as the document writes it. If the tag has a prefix, the prefix is part of the
name: for `<media:title>` the result is `media:title`. Use `namespaceUri` to find out what the prefix
means. The result is `tainted`, because it came from outside the program.

**The examples below** print the name of every node, show a name with a prefix, and add up the
prices in an order by looking for elements by name.
