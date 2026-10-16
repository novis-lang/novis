Returns the one attribute attached to a class, a method, a property or a parameter that has the shape
you ask for, or `null` when there is none.

You write the shape between `<` and `>` at the call. An attribute matches when it has all the fields
that shape names, even when it has more. Test the result against `null` before you read a field out
of it.

Name the declaration by writing it with `(...)` in place of its arguments. `User::constructor(...)`
names the class `User` itself, and `User::save(...)` names its method `save`. For a property or a
parameter, add its name as a second argument.

**Good to know:** two matching attributes on one declaration do not compile, and the error names
`Core\Attributes::all` as the member that returns both. Novis reads the attributes while it compiles
your program, so this costs nothing while the program runs.

**The examples below** read a length limit off a parameter, then a page size with a default for the
class that carries none, then a cache setting a handler may leave out.
