Returns the name an attribute was written with.

An attribute can be written in two forms. The named form starts with the name of a type, such as
`#[Route(path: "/orders")]`, and `name` returns `Route`. The short form, such as `#[{cache: 60}]`,
has no name, and `name` returns an empty string.

`Core\Reflect` gives you one `Core\Reflect\AttributeInfo` for each attribute of a class. Compare
`name` with the name you are looking for to keep one kind of attribute and skip all the others.

**Good to know:** the result is the name exactly as it is written in the source file. No namespace
is added to it.
