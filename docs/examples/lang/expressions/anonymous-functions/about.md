An anonymous function is a small function you write where you need it. You can keep it in a
variable, pass it to another call or store it in a list.

`fn` is the only way to write one. After the arrow comes one expression, whose value the function
returns, or a block in braces. Every parameter has a type. A block always declares its return type.
An anonymous function captures each outside variable it uses. This means it copies the value when
the function is written. Changing that variable later does not change what the function sees.
Inside a method, the object is captured too.

`$fn->bindTo($obj)` and `$fn->bind($obj)` return a copy of the function that uses `$obj` as `$this`.
`$fn->call($obj, ...)` makes that copy and calls it with the other arguments. The object must be of
the class the function was written in, or of a subclass. Any other object, or `null`, throws a
`LogicError`.

**Good to know:** there is no `use (...)` list. A call through a `callable` returns `mixed`, so use
`as` to say which type you read.
