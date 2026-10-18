A class is a type you write yourself. It holds some data and the functions that work on that data. You
create one with `new`.

Every member of a class writes `public`, `protected` or `private`. A `public` member is reachable from
anywhere. A `protected` member is reachable from the class and from the classes that extend it. A
`private` member is reachable only from the class that declares it. There is no default, so a member
written without one of those three words does not compile.

The constructor is a method named `constructor`. It runs once, when `new` creates the object. It must
give a value to every property that has no default. A class where every property already has a default
needs no constructor.

**Good to know:** a visibility keyword in front of a constructor parameter declares a property of that
name and fills it with the argument.

**The examples below** show a class with data and methods, a constructor that declares its
properties, and a class that keeps its data private. The last two show a property with no visibility,
which does not compile, and a class with no constructor.
