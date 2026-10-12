Reading a property that has never been written throws a catchable `LogicError` — "`Post`'s property
`$author` is read before it is initialized." — propagated by checked return (`rule:errors/propagation`)
rather than routed through `rule:errors/escalation-ladder`'s fatal ladder, because it is a recoverable
condition a caller can handle. The read never succeeds, so nothing is handed back that a caller could
inspect or compare against. Writing first makes every later read ordinary. A class implementing
`LateInit` calls `initProperty` before it throws, and throws only if that leaves the property
uninitialized (`rule:classes/late-init-initializes-on-the-first-read`).

A `lateinit` property is the one declaration that can reach the state. Every other non-nullable
property is discharged at its constructor (`rule:classes/definite-property-initialization`), and
reflection builds no object without running its constructor
(`rule:security/reflection-enforces-visibility`). The message names the object's own class, so a
compiled read, a read through `mixed` and `Core\Reflect\ClassInfo::get` throw the same error.

Internally the slot carries the tag `Unset`, distinct from every legal value including `null`. It is
not an entry in the type system and no expression produces one. The compiled read of a `lateinit`
property loads that tag byte and compares it before it uses the value. The payload cannot answer,
because `lateinit` takes every type (`rule:classes/lateinit-restrictions`) and a written `0`, `false`
or `null` has the same zero payload as the unset state. A read of any other property carries no test.

- **Memory:** zero additional bytes per property. The marker is a spare discriminant of the tagged
  value every property already has.
