`foreach` is the loop that walks a collection of values and gives you one value at a time.

It can loop over an array, an object whose class implements `Iterable`, and an object whose class
implements `Iterator`. A method that contains `yield` is a generator. It returns an `Iterator`, so
`foreach` can loop over its result too. Write `var` for the loop variable, and it gets the type of
the values. You can also write the type, and the compiler checks it.

Over an array the loop may get the key as well as the value. An `Iterable` or an `Iterator` has no
keys, so a loop over one gets the value alone. A class that implements `KeyedIterable` or
`KeyedIterator` gives a key with each value, as `Core\ObjectMap` does.

**Good to know:** a method that takes `Iterable<T>` as a parameter type works with every class that
can be walked. One function can then read a list, a cart or a database result the same way.

**The examples below** show an array with and without its keys, an object and a generator, a
checkout that adds up a shopping cart, and a second loop over one iterator.
