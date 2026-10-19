A method is a function that belongs to a class.

An instance method runs on one object, and `$this` is that object. A `static` method belongs to the
class itself. It has no `$this`, and you call it as `Class::method()`.

Three keywords call a method without writing the class name. `self` is the
class the code is written in. `static` is the class the call was made on, which may be a subclass.
`parent` is the class this one extends, so `parent::format()` calls the parent's version.

**Good to know:** a method that says it returns `static` must return the object it was called on, or
a new object of the same class.

**The examples below** show the two kinds of method, `self` beside `static` in a subclass, and a
subclass that calls its parent's version. The last two show a chain of calls with an anonymous
function that keeps `$this`, and a return type `static` that does not compile.
