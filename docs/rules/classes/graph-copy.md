The graph copy is a recursive, cycle-safe traversal of a value's reachable structure producing a
result that shares no mutable heap state with its source. Shared substructure stays shared — two
properties pointing at one nested object still point at one object on the other side — and a cycle
terminates instead of recursing. It never runs a constructor, and no hook fires: the shape that
crosses is the class's own declared properties, every time.

It refuses what has no meaning on the other side, naming the offending value and its path: a callable,
which captures a heap and a scope; an `inout` binding, which aliases a specific frame; and an object
holding a host handle. Declared types make most of that a compile-time refusal at the copy site; the
run-time check is what a `mixed` carrying one of them needs. An object whose class the receiving side
cannot resolve is refused by name, never stubbed.

One walk serves both carriers — arena-to-arena at the isolate boundary, and bytes through
`Core\Serialize` — so a rule added to it reaches both or neither. Two implementations that agree today
is the failure that costs.
