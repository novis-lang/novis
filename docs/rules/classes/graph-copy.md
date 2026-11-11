The graph copy is a recursive, cycle-safe traversal of a value's reachable structure producing a
result that shares no mutable heap state with its source. Shared substructure stays shared — two
properties pointing at one nested object still point at one object on the other side — and a cycle
terminates instead of recursing. It never runs a constructor, and no hook fires: the shape that
crosses is the class's own declared properties, every time.

It refuses what has no meaning on the other side, naming the offending value and its path. An `inout`
binding aliases a specific frame, and the checker refuses it at the copy site. Five refusals are facts
about an object's class:

- a callable's class, which captures a heap and a scope;
- a generator's state class, which is a suspended frame;
- a class holding a host handle, whose key indexes the sending side's table;
- every other class the compiler made, known by a `$` in its name, except an anonymous object's
  `$shape{…}` class, which is data;
- a class with a `secret` property, named by its first one (`rule:security/secret-crosses-no-boundary`).

One function answers those five, and both directions call it: the walk for every object it copies or
encodes, and the decoder for every class a payload names. Declared types make most of that a
compile-time refusal at the copy site; the run-time check is what a `mixed` carrying one of them needs.
An object whose class the receiving side cannot resolve is refused by name, never stubbed.

One walk serves both carriers — arena-to-arena at the isolate boundary, and bytes through
`Core\Serialize` — so a rule added to it reaches both or neither. Two implementations that agree today
is the failure that costs.
