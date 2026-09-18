How deep a chain of spawned scripts may nest before the next one is refused.

A script can run another in its own isolate, and that one can do the same. Nothing about such a chain
says where it stops, so a program that spawns itself builds isolates until the tree runs out of heap,
and the out-of-memory that arrives says nothing about the recursion behind it.

The ceiling turns that into a clear answer: a spawn past it is refused before the child exists, and
the report names the depth. It holds a value when nobody writes one — sixty-four, far above anything
written on purpose and well below where the heap would notice. It is the operator's to set: a script
able to raise its own could exhaust the tree before any depth stopped it. Written as false, the
ceiling comes off.

The example prints the ceiling this checkout runs under, and which numbers beside it a program may
move.
