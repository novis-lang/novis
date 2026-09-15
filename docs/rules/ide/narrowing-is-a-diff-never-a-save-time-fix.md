Narrowing a declaration is not a local edit, because `rule:types/arrays`' element type is enforced on
every **write**: a binding that was an `array<mixed>` and is now an `array<array<int>>` refuses the
`$a[] = "x"` and the `$a = $wider;` that the wider annotation admitted, each of them a line the action
did not touch. Reading it is what stays safe — the narrowed array still satisfies every wider parameter
it reached before, that being the covariant direction — so what the action can break is exactly the
writes, and it cannot see them from the declaration. Two rules follow.

**The action is never registered under `source.fixAll.nvs`.** The casing fix and the legacy-cast fix
compose with format-on-save because their edit cannot break another line. This one can, so it is invoked,
previewed and approved like the generators beside it. A client test asserts it is absent from the
`source.fixAll.nvs` set, so a format-on-save never applies it.

**It does not chase the call sites it affects.** Repairing them means choosing between an `as` at the
call, a wider parameter and a narrower one — a refactoring with a decision in it, which
`rule:ide/a-code-action-writes-only-what-is-already-determined` refuses to let a light bulb make.

The request path gets faster where the action is used, which is the point: a narrowed array leaves the
generic helper path for the typed one, and its element writes become compile errors instead of runtime
throws. Nothing is spent per request, and no memory beyond one more interned descriptor where the program
did not already have that type.
