Returns the attributes of an XML element as an array. An attribute is a name and a value written
inside a start tag, such as `href` in `<link href="page.html"/>`.

The array is keyed by the attribute name, and the order is the order in the document. The name
keeps its prefix, such as `xml:lang`. Entities such as `&amp;` are already expanded in the value.
A tab or a line break written inside a value becomes a space, because XML requires this.

For a node that is not an element, the array is empty. This is not an error, so you can call the
method on every child in a loop. Every value is tainted, because it came from the document.

**The examples below** list all attributes of an element, use a default value when an attribute is
missing, and find the web page link in a news feed.
