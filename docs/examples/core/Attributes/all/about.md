Returns every attribute of one kind attached to a class, a method, a property or a parameter.

You write the attribute's name between `<` and `>` at the call. This is the name of an `attribute`
declaration, such as `Tag` for `attribute Tag on class { name: string }`. Only attributes written as
`#[Tag(…)]` match. The result is an array, in the order the attributes were written. It is empty when
nothing matches.

Name the declaration with a method reference, which is the method's name without a call.
`User::constructor` names the class `User` itself, and `User::save` names its method `save`. For a
property or a parameter, add its name as a second argument.

**Good to know:** Novis reads the attributes while it compiles your program, so this costs nothing
while the program runs. Use `Core\Attributes::get` when a declaration has at most one attribute of
this kind.

**The examples below** read several labels from one class, then the rules on one property, then the
permissions a method lists.
