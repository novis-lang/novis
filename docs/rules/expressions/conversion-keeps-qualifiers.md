Nullability and the security qualifiers are orthogonal axes. `as ?T` runs the **same** qualifier rule
the checked form runs — one code path decides both spellings — so writing `?T` instead of `T` changes
nothing about `tainted` or `secret`.

Where the target is a qualifiable type, the operand's `tainted` and `secret` are carried into the
result: `tainted string as ?string` is `?tainted string`. Where a checked conversion into the target
already launders, `as ?T` launders exactly as much and no more.

**`as ?T` is never a launderer in its own right.** It introduces no qualifier hole the checked form
does not already have, and laundering stays what it is elsewhere: the narrow, sink-named `Core`
members that declare it.
