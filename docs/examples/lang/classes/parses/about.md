`Parses` is the interface for a class that is built out of a piece of text.

A class that implements it writes one method,
`public static function parse(tainted string $s): static`. The method returns an object of the class,
or throws an error when the text is not valid. `tainted` says the text came from outside your
program, such as a web request or a command line.

Novis needs this interface everywhere text from outside becomes a value: a piece of a route path, a
`#[Core\Query]` parameter, a command argument and a command option. You declare the parameter at your
own class, and Novis calls `parse` for you. A parameter typed at a class that does not implement
`Parses` does not compile, and the error says so.

**Good to know:** the return type `static` means the class the call named, so a subclass that
inherits `parse` gets an object of the subclass back.

**The examples below** show a product code built from text, what a subclass gets back, and a list of
values from a form checked one by one.
