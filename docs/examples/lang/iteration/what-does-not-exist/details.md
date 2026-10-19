`Countable`, `ArrayAccess`, `IteratorAggregate`, `Traversable` and `Generator` are not declared. A
class that implements one, or a parameter of one of these types, is the error `E0303`. The message
names no replacement.

`iterator_to_array`, `iterator_count`, `count`, `current`, `next`, `reset` and `array_walk` are not
functions. A call to one is the error `E0320`.

`$obj[0]` on an object is the error `E0482`, for a read and for a write. Only an array can be read
with `[ ]`.

A `foreach` loop over an object that implements neither `Iterable<T>` nor `Iterator<T>` is the error
`E0443`. A method named `getIterator` does not change this. A loop never reads an object's
properties.
