Returns the namespace URI of an XML element, or `null` if the element is in no namespace.

A namespace is a URI that says which vocabulary a name belongs to. A document declares one with
`xmlns:x="…"` and then writes names such as `<x:title>`. The declaration covers the element that
writes it and every element inside it. A name without a prefix uses the nearest `xmlns="…"`. An
inner declaration replaces an outer one.

The result is `null` for a node that is not an element, and for an element that no declaration
covers. The name does not change: `name` still returns `x:title`. The result is `tainted`,
because it came from outside the program.

**The examples below** print the namespace of each element, show an inner declaration that
replaces an outer one, and find elements by their namespace when two documents use different
prefixes.
