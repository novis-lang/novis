`Parses` is the interface for a class that is built out of a piece of text.

A class that implements it writes two methods. The static method `parse(tainted string $s): static`
returns an object of the class, or throws an error when the text is not valid. `tainted` says the
text came from outside your program. The method `format()` returns the text that `parse` reads.
`Core\Router::url` calls `format` to write the value into a link.

Novis calls `parse` for you when a route capture, a `#[Core\Query]` parameter or a command argument
has the type of your class. In a route, `parse` runs before `Core\Router\Gate::allows`.

**The examples below** show a product code built from text, what a subclass gets back, values from a
form, a class used in a route, a route that does not compile, and a `final` class that loads a
stored order.
