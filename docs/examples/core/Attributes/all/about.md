Returns every attribute attached to a class, a method, a property or a parameter that has the shape
you ask for.

You write the shape between `<` and `>` at the call. An attribute matches when it has all the fields
that shape names, even when it has more. The result is an array, in the order the attributes were
written, and it is empty when nothing matches.

Name the declaration by writing it with `(...)` in place of its arguments. `User::constructor(...)`
names the class `User` itself, and `User::save(...)` names its method `save`. For a property or a
parameter, add its name as a second argument.

**Good to know:** Novis reads the attributes while it compiles your program, so this costs nothing
while the program runs. Use `Core\Attributes::get` when a declaration carries at most one matching
attribute.

**The examples below** read several labels from one class, then the rules on one property, then the
permissions a method lists.
