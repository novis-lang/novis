A property may declare `get` and `set` hooks, scoped to that one property, and a `readonly`
property declares none. A hooked property's read is a call
to its `get` and its write a call to its `set`, at every access spelling alike — including one inside
a string interpolation.

Each hook body compiles to its own function, with the receiver in the ordinary parameter-0 slot, so
a hooked access costs no new instruction, no new calling convention and no dispatch-table entry.
Inside a hook the property names its own backing slot, which is what makes a `set` hook that
transforms the value it stores terminate rather than recurse.

Novis keeps the backing slot for every hooked property, so there is no split between virtual and
backed properties. That spends one slot on a property whose `get` computes its answer, and buys one
storage model instead of two — and a `set` hook that commits a value discharges that property's
initialization obligation (`rule:classes/definite-property-initialization`) exactly as a plain
assignment does.
