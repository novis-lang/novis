`foreach` is the loop that walks a collection of values and gives you one value at a time.

It accepts three kinds of subject: an array, an object whose class implements `Iterable`, and an
object whose class implements `Iterator`. A method that contains `yield` returns an `Iterator`, so it
is a subject too. The loop declares the type of the variable it binds, and the compiler checks that
the subject really produces that type. Over an array the loop may bind the key as well as the value.
The other two subjects have no keys, so a loop over one binds the value alone.

**Good to know:** a method that takes `Iterable<T>` as a parameter type works with every class that
can be walked. This is how one function reads a list, a cart or a database result without knowing how
any of them stores its values.

**The examples below** show an array walked with and without its keys, then an object and a generator
walked by the same kind of loop, then a checkout that adds up a shopping cart.
