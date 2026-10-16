`foreach` accepts three subjects and refuses a fourth where it is written:

- an **`array<T>`**, iterated directly by the IR with no interface call and no allocation;
- an **`Iterable<T>`**, whose `iterate()` is called once and whose returned cursor the loop drives;
- an **`Iterator<T>`**, driven directly.

Anything else is a compile error at the subject. Only the array form binds a key — a cursor has no
key to give (`rule:iteration/two-interfaces`), so a key binding over one is refused rather than
filled with a counter, and an array's key binds as a string. Where the keys can come from outside the
program — the array's element type carries `tainted` or `mixed`, or the subject is `mixed` — the key
binds as `tainted string`, and a key written as a plain `string` there is refused
(`rule:security/taint-propagation`).

Each of the three gives its value binding the element type `T`, which a written binding type is
checked against and a `var` binding takes as its own (`rule:types/var-inference`).

An object becomes iterable by declaring one of the two interfaces at a concrete type
(`rule:iteration/concrete-generic-implements`). There is no other way in: no property walk, and no
interface that turns a subscript or a count into a method call
(`rule:iteration/no-magic-collection-interfaces`).
