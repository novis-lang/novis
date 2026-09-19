Says that a class can be built from a piece of text, so anywhere text arrives can ask for the class
instead.

A class implementing it declares one member, `parse`. It takes the text, checks it, and answers an
instance of the class — or throws when the text is not one. That is all it takes: a path segment, a
query value, a command argument or a command option can each be declared at your class, and what
your code receives is the class, already checked. Because the checking happens in one place, nothing
downstream has to wonder whether it happened.

The text `parse` receives is marked as having come from outside your program, and that is deliberate:
this is where such text is meant to be looked at.

**Good to know:** `Parses` needs no `use`. Every program already has the name, beside `Comparable`
and `Stringable`.
