The interface for a class that is built from a piece of text and written back as text.

A class that implements `Parses` declares two methods. The static method
`parse(tainted string $s): static` checks the text and returns an object of the class. The text is
`tainted`. That means it came from outside your program, so you must check it before you trust it.
When the text is not valid, `parse` throws a `ParseError`.

The method `format()` returns the text that `parse` reads. Its return type is `tainted string`, or
`string` when the text is always safe.

A route capture, a `#[Core\Query]` value, a command argument or a command option can have the type
of your class. Novis then calls `parse`, and your code gets an object that is already checked.
`Core\Router::url` and `Core\Uri::buildQuery` call `format` to write the object into a link.

**Good to know:** `Parses` needs no `use`. Every program can use it, like `Comparable` and
`Stringable`.
