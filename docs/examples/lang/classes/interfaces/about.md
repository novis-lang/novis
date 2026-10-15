An interface is a list of methods a class promises to provide. A class writes `implements` and then
writes those methods.

A class may implement any number of interfaces, and an interface may extend another one. Wherever a
value is declared at the interface, any class that implements it fits. A class that leaves out a
required method does not compile, and the error names the method and the interface.

An interface may also carry work of its own: a method with a body, which every class gets for free
and may replace, a constant, and a `static` method.

Two interfaces may both give a class a method with the same name. Then the class must declare that
method itself, or it does not compile. Inside it, `Greets::hello()` calls the body from `Greets`.

**Good to know:** Novis has no `trait`. To share methods, write them in an interface. To share an
object that already does the work, write `implements Greets by $voice` and every method of `Greets`
is passed on to the object in that property.

**The examples below** show a class that implements an interface, then an interface that brings its
own method, then one class passing the work on to another.
