Describes the class of an object you already have.

`Core\Reflect::forObject` takes any value and returns a `Core\Reflect\ClassInfo` for the class of
that object. The description has the class's name, its properties, its methods, its constants and
its attributes. It describes the object's own class, so a subclass gives the subclass's name.

The parameter is `mixed`, so the compiler does not check that the value is an object. When it is
not an object, `forObject` throws a `RuntimeError`. Check the value with `Core\Reflect::typeOf`
first if it may be something else.

**The examples below** show the class name of an object, the properties of a subclass, and a log line
that names the class of any value a function receives.
