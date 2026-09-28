`Core\Test::partial()` builds a test double that replaces only some methods of a real object. You
write the interface in angle brackets, then give the real object and one closure for each method you
want to replace. The result has the interface type, so you can pass it to any code that takes one.

A call to a replaced method runs your closure. A call to any other method runs the method of the
real object, with the same arguments. Inside the real object, `$this` is still the real object. So
when a real method calls another method on `$this`, it calls the real one and not your closure.
Like `Core\Test::double`, a partial keeps a list of its calls for `Core\Test::assertCalled`.

**The examples below** show how to replace one method, how the real object keeps its own state, and
a test of a disk that is full.
