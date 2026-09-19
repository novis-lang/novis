What a class implements so a `foreach` loop can walk it, the way it walks an array.

An `Iterable` promises one method, `iterate`, which hands the loop something that steps through the
values one at a time. You fix the value type when you declare the class — `implements
Iterable<string>` for a sequence of strings — and every loop over it sees that type. The simplest
`iterate` is one that `yield`s each value in turn; the language builds the stepper from it.

Each loop asks for a fresh stepper, so the same object can be walked more than once, and two loops
over it can nest. A loop over an `Iterable` binds the value alone: there is no key.

**Good to know:** a class is walkable only because it declares this interface. Nothing else gets a
`foreach` in — not a property walk, and not a count turned into a method call. It replaces
PHP's `IteratorAggregate`.
