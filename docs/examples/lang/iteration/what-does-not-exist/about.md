PHP's iteration helpers do not exist in Novis. Naming one is a compile error, and the message names
what to write instead.

There is no `ArrayAccess`, `Countable`, `IteratorAggregate`, `Traversable` or `Generator`, and no
`iterator_to_array`, `count`, `current`, `next` or `reset`. Each one has a replacement. A class
becomes walkable by implementing `Iterable<T>` or `Iterator<T>`. A generator returns an
`Iterator<T>`, so it needs no class of its own. `Core\Arr` holds what the free functions did:
`Core\Arr::count` counts, and `Core\Arr::from` reads a sequence into an array. Reading an element of
an object, `$basket[0]`, has no interface behind it. Give the class a method and name it yourself.

**Good to know:** you meet all of these when you compile, never while the program is running.

**The examples below** show the three replacements: `Iterable<T>` for `IteratorAggregate`, `Core\Arr`
for the free functions, and methods of your own for `ArrayAccess` and `Countable`.
