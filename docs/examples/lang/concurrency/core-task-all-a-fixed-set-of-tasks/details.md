The type of a field in the result is the return type of its function. For `fn(): int => …` it is
`int`. A `fn` with an expression body and no return type gives the type of that expression. A
variable of type `callable`, with no signature, gives `mixed`. A variable of type `callable(): int`
gives `int`.

A field of type `mixed` converted with `as string` is a `tainted string`.

A function that returns `void` gives a `void` field. `echo $r->a` on it is `E0707`.

Reading a field that the shape does not have compiles and throws `RuntimeError` when it runs.

`Core\Task::all({})` has no tasks and returns at once.

Each field must be a function. A field with another value, such as `{ a: 1 }`, is `E0401`. An array
of functions is `E0401` too.
