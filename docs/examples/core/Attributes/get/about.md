Returns the one attribute of one kind attached to a class, a method, a property or a parameter. The
result is `null` when there is none.

You write the attribute's name between `<` and `>` at the call. This is the name of an `attribute`
declaration, such as `Cache` for `attribute Cache on method { seconds: int }`. Only attributes written
as `#[Cache(…)]` match. Test the result against `null` before you read a field out of it.

Name the declaration with a method reference, which is the method's name without a call.
`User::constructor` names the class `User` itself, and `User::save` names its method `save`. For a
property or a parameter, add its name as a second argument.

**Good to know:** two matching attributes on one declaration do not compile. The error names
`Core\Attributes::all` as the method that returns both. Novis reads the attributes while it compiles
your program, so this costs nothing while the program runs.

**The examples below** read a length limit off a parameter, then a page size with a default for the
class that carries none, then a cache setting a handler may leave out.
