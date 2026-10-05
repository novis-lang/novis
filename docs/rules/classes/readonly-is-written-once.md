A `readonly` property is written once, by its own class's `constructor` through `$this`, and nothing
writes it after construction ends.

The checker refuses every other write it can see, as E0782: a write from any other method or class, a
write through another instance even inside the constructor, a second write on one path through the
constructor (a promoted parameter is the first), a write inside a loop, and an element write once the
property is written, because that writes the property again. A `readonly` property declares no `get`
or `set` hook (E0843), because a hook runs code at every access and the value read would no longer be
the one written.

A write the checker cannot see — through an `object` or `mixed` receiver, or
`Core\Reflect\ClassInfo::set` — throws a `RuntimeError` naming the property, and the value stays as
the constructor wrote it.

Because the value cannot change once the object is built, a test of a `readonly` property still holds
at the next read, which is what lets `rule:types/narrowing` narrow one.
