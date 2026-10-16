Returns the name of one property of a class.

`name()` returns the name that the class declares, such as `email` or `total`. The name has no `$`
and no class name. A class also lists the properties it inherits from its parent class, and those
come first.

You can pass the name to `Core\Reflect\ClassInfo::hasProperty` to check that a property exists. You
can pass it to `Core\Reflect\ClassInfo::get` and `Core\Reflect\ClassInfo::set` to read and write the
property. Names are case-sensitive, so `Email` does not find `email`.

**The examples below** print the properties of a class, show the properties a class inherits, and
write the header line of a CSV file from the properties of a class.
