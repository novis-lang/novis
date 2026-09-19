A class can build on another class. `extends` names the parent, and the new class starts with
everything the parent has.

The subclass keeps every property and method of the parent, and may write its own version of a
method. `parent::` reaches the version the parent wrote. When the parent has a constructor, the
subclass constructor must call `parent::constructor(...)`.

Three words shape a hierarchy. `abstract` marks a class that is not finished: it has no objects, and
a subclass must write the methods it left open. `final` marks a class or a method that nobody
extends or replaces. `is` tests what an object really is.

**Good to know:** a class extends one parent only. To share behaviour between classes that are not
related, use an interface.

**The examples below** show a subclass that adds to its parent, then an abstract class every
subclass fills in, then a class the program picks while it runs.
