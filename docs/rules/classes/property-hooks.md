A property may declare one `get` and one `set` hook, scoped to that one property. A second hook of
one kind is `E0304` at the second hook, and the first is the one that runs. A `readonly` property
declares none (`E0843`), and neither does a `static` one (`E0872`): a hook runs at a read or a write
through an instance, and a static property is never reached through one. A hooked property's read is a call
to its `get` and its write a call to its `set`, at every access spelling alike — including one inside
a string interpolation.

Each hook body compiles to its own function, with the receiver in the ordinary parameter-0 slot, so
a hooked access costs no new instruction, no new calling convention and no dispatch-table entry.
Inside a hook the property names its own backing slot, which is what makes a `set` hook that
transforms the value it stores terminate rather than recurse.

A `set` hook's parameter may leave its type out, which gives it the property's type. A written type
accepts the property's type without a conversion, or the hook does not compile (`E0401`): every write
is checked against the property's type and handed to the hook unchanged, so `set (float $v)` on an
`int` property would read the `int` as a `float`. A wider type such as `?int` or `mixed` compiles.

Novis keeps the backing slot for every hooked property, so there is no split between virtual and
backed properties. That spends one slot on a property whose `get` computes its answer, and buys one
storage model instead of two — and a `set` hook that commits a value discharges that property's
initialization obligation (`rule:classes/definite-property-initialization`) exactly as a plain
assignment does.
