Reads an array as the type you write at the call site. Values from a form, a query string or a
decoded document arrive as text.

You write the type between angle brackets. It can be a shape, such as `{ name: string, age: int }`, or
a class that carries `#[Core\Json\Derive]`. A class inside a shape or an array needs the attribute
too, however deep it is. Each field the type names is converted with `as`. A key
the type does not name is left out of the result. Write `array<T>` to read a whole list of them in
one call.

A field can also be a `Core\Time\Date`, `TimeOfDay`, `DateTime`, `Instant`, `Duration`, `Zone` or
`Core\Uuid`. Its value is read from text. A date or a time is its ISO text, such as `2026-05-14`. A
`Duration` is text such as `1h30m`, and a `Zone` is its name, such as `Europe/Vienna`.

**Good to know:** a field that is missing, or that holds a value its declared type cannot take,
throws a `ParseError`. That error lists every field that failed, each at its own path, so a form can
show all of its problems at once.

**The examples below** read a posted form into a shape and read a list of order lines. They read
the query parameters of a search page into a class, and a form with a date and a time.
