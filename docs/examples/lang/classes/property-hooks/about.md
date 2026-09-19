A property hook is a small piece of code that runs every time a property is read or written.

You write `get` to decide what a read gives back, and `set` to decide what a write stores. Either one
may stand alone. A property with only a `get` hook is a computed property: it works out its value
from the other properties, and only its own class writes it.

The hooks run for every read and every write, including the ones the constructor makes, the ones
inside a string, and the ones a subclass makes. Inside the hooks, `$this->name` is the stored value
itself, so a `set` hook stores with `$this->name = ...` and does not call itself. A `set` hook that
throws an error stops the write, and the property keeps the value it had.

**The examples below** show a `set` hook that checks a value, a `get` hook that works one out, and a
class that keeps a name and a display form in step.
