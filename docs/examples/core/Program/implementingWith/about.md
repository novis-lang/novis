Returns every class that implements an interface or extends a class, each next to the one attribute you ask for.

`Core\Program::implementing` finds the classes. This method finds the same classes and reads one
attribute of each of them, so a framework can find its pages and their routes in one call. You write
the interface or the class, and the attribute's shape, between `<` and `>`. The argument is the name of
the method, property or constructor parameter that has the attribute. An empty name reads the
attributes of the class itself. Each row has two fields: `instance` is the object, and `attribute` is
the matching attribute, or `null` when the class has none.

**Good to know:** Novis resolves this while it compiles your program, so nothing is scanned while the
program runs. Two matching attributes on one class do not compile.

**The examples below** build a dispatch table from pages and their routes, order plugins by an
attribute on the class, read a setting from a property, and find scheduled jobs that extend a base
class.
