`foreach` is the loop that walks a collection of values and gives you one value at a time.

It accepts three kinds of subject: an array, an object whose class implements `Iterable`, and an
object whose class implements `Iterator`. A method that contains `yield` returns an `Iterator`, so it
is a subject too. Write `var` for the loop variable, and it gets the type of the subject's values. Or
write the type, and the compiler checks it. Over an array the loop may get the key as well as the
value. The other two subjects have no keys, so a loop over one gets the value alone.

**Good to know:** a method that takes `Iterable<T>` as a parameter type works with every class that
can be walked. One function can then read a list, a cart or a database result the same way.

**The examples below** show an array with and without its keys, then an object and a generator, then
a checkout that adds up a shopping cart. Each one uses `var` first and a written type after it.
