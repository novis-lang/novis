A closure is a small function you write where you need it, keep in a variable, pass to another call
or store in a list.

`fn` is the only way to write one. After the arrow comes either one expression, whose value the
closure gives back, or a block in braces that runs several lines and returns. Every parameter says
what type it takes; the one-expression form may leave the return type out and a block always
declares it. A closure reads the variables around it without being told to: each one it uses is
copied in when the closure is written, so changing that variable afterwards does not change what the
closure sees. Inside a method the object is copied in too, so the closure reads its properties.

`$fn->bindTo($obj)` returns a copy of the closure that uses `$obj` as `$this`. `$fn->bind($obj)` is
the same method. `$fn->call($obj, ...)` makes that copy and calls it with the other arguments. The
object must be of the class the closure was written in, or of a subclass. Any other object, and
`null`, throws a `LogicError`. A closure that does not use `$this` is returned unchanged.

**Good to know:** there is no `use (...)` list to write and no other kind of function literal. A call
through a closure gives back `mixed`, so say which type you are reading with `as`.
