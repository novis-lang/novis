A method is a function that belongs to a class.

An instance method runs on one object, and `$this` is that object. A `static` method belongs to the
class itself. It has no `$this`, and you call it as `Class::method()`.

Inside a class, three names reach a method without writing the class name again. `self` is the class
the code is written in. `static` is the class the call was made on, which may be a class that extends
it. `parent` is the class this one extends, so `parent::format()` calls the version the parent wrote
and `parent::constructor(...)` runs its constructor.

**In plain words:** `self` is where the code was written. `static` is who called it.

**Good to know:** a method that says it returns `static` must return the object it was called on, or
a new object of the same class. That is what makes it safe to call from a subclass.

**The examples below** show the two kinds of method, then `self` beside `static` in a subclass, then
a subclass that calls the version its parent wrote.
