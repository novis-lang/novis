An interface is a list of methods a class promises to provide. A class writes `implements` and then
writes those methods.

A class may implement many interfaces, and an interface may extend another one. A variable typed as
an interface accepts any class that implements it. A class that leaves out a required method does
not compile.

An interface may also have a method with a body. Every class that implements the interface gets
that method and may replace it. An interface may also have constants and `static` methods.

Two interfaces may both give a class a method with the same name. Then the class must declare that
method itself, or it does not compile.

**Good to know:** Novis has no `trait`. To share methods, write them in an interface. To pass the
work to another object, write `implements Greets by $voice`. Every method of `Greets` then calls the
same method on the object in `$voice`.

**The examples below** show a class that implements an interface, an interface with its own method,
and a class that passes its work to another object. Examples 04 and 05 show more of an interface,
and the others do not compile.
