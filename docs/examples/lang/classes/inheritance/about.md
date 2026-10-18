A class can build on another class. `extends` names the parent, and the new class starts with
everything the parent has.

The subclass keeps every property and method of the parent, and may write its own version of a
method. `parent::` reaches the version the parent wrote. When the parent has a constructor, the
subclass constructor must call `parent::constructor(...)`.

An `abstract` class is not finished. You cannot create an object of it, and a subclass must write
each method it declares without a body. A `final` class cannot be extended, and a `final` method
cannot be replaced. `$x is T` tests whether an object is a `T`.

**Good to know:** a class extends one parent only. To share behaviour between classes that are not
related, use an interface.

**The examples below** show a subclass that adds to its parent, then an abstract class every
subclass fills in, then a class the program picks while it runs. Examples 06, 09 and 10 show an
`abstract static` method, `is` and a static call through a class value. The others show what does
not compile.
