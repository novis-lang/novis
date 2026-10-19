An anonymous function is a small function you write where you need it. You can keep it in a
variable, pass it to another call or store it in a list.

`fn` is the only way to write one. After the arrow comes one expression, whose value the function
returns, or a block in braces. Every parameter has a type. A block declares its return type.
The function copies each outside variable it uses when the function is written.

`$fn->bindTo($obj)` and `$fn->bind($obj)` return a copy that uses `$obj` as `$this`.
`$fn->call($obj, ...)` makes that copy and calls it. The object must be of the class the function
was written in, or a subclass. Any other object, or `null`, throws a `LogicError`.

**Good to know:** there is no `use (...)` list. A call through a `callable` returns `mixed`.

**The examples below** write functions, show captures and check a form.
Example 04 reads `$this`, 05 calls itself, and 06 does not compile.
