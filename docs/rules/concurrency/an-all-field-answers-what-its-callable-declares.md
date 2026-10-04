Each field of a `Core\Task::all` argument answers **its own callable's declared return type**. A
field whose callable declares none — bare `callable`, the top of the lattice — answers `mixed`, and
only that field: every other one still carries the type it declared.

The field's type is read off the *argument's type*, not off the expression written at the call, so
where the callable came from stopped mattering. An anonymous function, a method reference, a parameter and a
variable holding the whole shape are all equally good, and a shape assembled somewhere else and
passed in is too. What the parameter still refuses is a value that is not a shape of callables at
all: an array, a scalar or a field holding something that cannot be called, each reported as the
ordinary type mismatch.

`rule:types/callable-signature` is what made this possible, and the restriction it replaces is worth
naming: until a `callable` carried a signature, the type existed only at the written literal, so a
framework storing callables in a variable could not use `all` at all. It can now, and it pays only
for the signatures it declines to write.
