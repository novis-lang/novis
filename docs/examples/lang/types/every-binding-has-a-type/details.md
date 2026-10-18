A variable is declared once. Every later `$count = …` is an assignment and has no type. A value
of another type does not compile: `$count = "two";` after `var $count = 1;` is the error `E0401`.
The message points to the line that gave `$count` its type.

`int $i = null;` and `uint $u = -1;` are the error `E0401` too. The only conversion without `as` is
widening, such as an `int` value in a `float` variable.

Assigning to a variable that was never declared is the error `E0301`. Reading a variable before
every path has given it a value is also the error `E0301`.

`var` needs a value whose type it can see. `var $xs = [];` does not compile, because an empty array
has no element type. `var $xs = [1, "a"];` does not compile either, because the elements have two
types. Both are the error `E0414`, and the message shows the type to write, such as
`array<int|string> $xs = [1, "a"];`.

The variable of a `catch` clause is a declaration too. Two `catch` clauses in one function may use
the same name when they catch the same type.
