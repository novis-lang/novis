Returns the name of one method of a class.

`name()` returns the name that the class declares, such as `send` or `total`. The name has no class
name and no parentheses. The constructor of a class is named `constructor`. A class also lists the
methods it inherits from its parent class.

You can pass the name to `Core\Reflect\ClassInfo::hasMethod` to check that a method exists. You can
pass it to `Core\Reflect\ClassInfo::call` to call the method. Names are case-sensitive, so `Send`
does not find `send`.

**The examples below** print the methods of a class, show the methods a class inherits, and run a
command whose name a user typed.
