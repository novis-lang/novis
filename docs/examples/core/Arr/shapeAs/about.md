Reads an array as the type you write at the call site. Values from a form, a query string or a
decoded document arrive as text.

You write the type between angle brackets. It can be a shape, such as `{name: string, age: int}`, or
a class that carries `#[Core\Json\Derive]`. Each field the type names is converted with `as`. A key
the type does not name is left out of the result. Write `array<T>` to read a whole list of them in
one call.

**Good to know:** a field that is missing, or that holds a value its declared type cannot take,
throws a `ParseError`. That error lists every field that failed, each at its own path, so a form can
show all of its problems at once.

**The examples below** show a posted form read into a shape, a list of order lines, and a search
page reading its query parameters into a class.
