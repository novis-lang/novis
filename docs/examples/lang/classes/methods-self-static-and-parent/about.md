A method is a function that belongs to a class.

An instance method runs on one object, and `$this` is that object. A `static` method belongs to the
class itself. It has no `$this`, and you call it as `Class::method()`.

Three keywords call a method without writing the class name. `self` is the
class the code is written in. `static` is the class the call was made on, which may be a subclass.
`parent` is the class this one extends, so `parent::format()` calls the parent's version.

**Good to know:** a method with the return type `static` returns the object it was called on, or a
new object of the same class. In a `final` class, `static` is that class, so the method may return
any object of it, such as a stored one.

**The examples below** show the two kinds of method, `self` beside `static` in a subclass, and a
subclass that calls its parent's version. Then come a chain of calls with an anonymous function that
keeps `$this`, and a return type `static` that does not compile. The last one shows a `final` class
that returns a stored object.
