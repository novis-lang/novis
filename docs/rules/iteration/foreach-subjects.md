`foreach` accepts five subjects and refuses any other where it is written:

- an **`array<T>`**, iterated directly by the IR with no interface call and no allocation;
- an **`Iterable<T>`**, whose `iterate()` is called once and whose returned cursor the loop drives;
- an **`Iterator<T>`**, driven directly;
- a **`KeyedIterable<K, V>`**, whose `iterateKeyed()` is called once and whose returned keyed cursor the
  loop drives;
- a **`KeyedIterator<K, V>`**, driven directly (`rule:iteration/keyed-interfaces`).

Anything else is a compile error at the subject, `mixed` and `iterable` included: neither says which
of the loops to run, so the program converts the value where it is written, as
`foreach (($m as array<mixed>) as ...)`, and the conversion throws where the value is not an array.

**The binding form picks the pair** where a subject reaches both: a loop with a key binding drives the
keyed pair, and a loop with a value binding alone drives `Iterable`/`Iterator`
(`rule:iteration/keyed-interfaces`). So a value-only loop over a `Core\ObjectMap` still gives its keys.

An array and the keyed pair bind a key. A cursor of the value pair has no key to give
(`rule:iteration/two-interfaces`), so a key binding over one is refused rather than filled with a
counter. An array's key binds as a string. Where the keys can come from outside the program — the
array's element type carries `tainted` or `mixed` — the key binds as `tainted string`, and a key
written as a plain `string` there is refused (`rule:security/taint-propagation`). The keyed pair's key
binds as its `K`, qualifier included.

Each subject gives its value binding the element type `T`, or `V` for the keyed pair, which a written
binding type is checked against and a `var` binding takes as its own (`rule:types/var-inference`).

An object becomes iterable by declaring one of the four interfaces at a concrete type
(`rule:iteration/concrete-generic-implements`). There is no other way in: no property walk, and no
interface that turns a subscript or a count into a method call
(`rule:iteration/no-magic-collection-interfaces`).
