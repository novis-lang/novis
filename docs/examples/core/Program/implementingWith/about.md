Returns every class that implements an interface, each next to the one attribute you ask for.

`Core\Program::implementing` finds the classes. This member finds the same classes and reads one
attribute off each of them, so a framework can find its pages and their routes in one call. You write
the interface and the attribute's shape between `<` and `>`, and the name of the member that carries the
attribute as the argument. An empty name reads the attributes on the class itself. Each row has two
fields: `instance` is the object, and `attribute` is the matching attribute, or `null` when the class
has none.

**Good to know:** Novis resolves this while it compiles your program, so nothing is scanned while the
program runs. Two matching attributes on one class do not compile.

**The examples below** build a dispatch table from pages and their routes, then order plugins by an
attribute on the class, then read a setting off a property.
