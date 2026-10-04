Every field value inside a `#[...]` payload is a compile-time constant: a literal, a class constant, an
enum case, or `Foo::class`. A variable, a function or method call, a `new`, `$this` and an interpolated
string are each refused where they are written (`E0725`) — one diagnostic per field, in source order,
so a payload with two computed values reads as two mistakes rather than one wrong attribute.

The admitted list is **closed**. An expression kind the grammar grows is refused until someone decides
it belongs in a constant pool, which is why this is its own walk over every attach site rather than a
row added to the pass that folds an enum case's value (`rule:enums/closed-integer-type`).

Two things follow with no rule of their own. The payload resolves once, at compile time, so there is no
"evaluate this attribute's arguments" step at declaration time. And no `tainted` value can reach a
payload, because a tainted value has no compile-time-constant source at all; a `secret` class constant
can, and is refused there as a sink (`E0727`).
