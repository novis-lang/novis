Opens an element, such as `<order>`. Everything you write after this call goes inside the element,
until you close it with `endElement`. You can open an element inside another one, and so build a
document of any shape.

Right after `startElement`, you can add attributes to the element with `attribute`. The name must be a
valid XML name, such as `order`, `_id` or `atom:link`. The writer does not escape a name, so a name
with a space or a `<` throws a `LogicError`.

A document has exactly one root element. When the root element is closed, a second `startElement`
throws a `LogicError`. At most 1024 elements can be open inside each other. Opening one more also
throws a `LogicError`.

**The examples below** build a nested document, show the errors, and write a sitemap for a website.
