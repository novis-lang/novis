`#[...]` written in front of a declaration attaches extra information to it. A class, an interface, an
enum, a method, a property, a class constant and a parameter can each carry one.

Inside the brackets you write named values, as in `#[Route(path: "/users")]`. The name in front of the
brackets must be a `type` alias for a shape, and Novis checks your values against that shape. An
attribute is not a class. Novis creates nothing for it, and none of your code runs for it.

Every value must be known while the program is compiled: a number, a string, `true`, `null`, a list, a
class constant or an enum case. A variable or a function call there does not compile.

**Good to know:** `#` on its own starts a comment. Only `#[` starts an attribute.
