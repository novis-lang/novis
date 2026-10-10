Reading a property that has never been written throws an ordinary, catchable `Throwable`, propagated
by checked return (`rule:errors/propagation`) rather than routed through
`rule:errors/escalation-ladder`'s fatal ladder — it is a recoverable condition a caller can handle.
The read never succeeds, so nothing is handed back that a caller could inspect or compare against.
Writing first makes every later read ordinary.

A `lateinit` property is the one declaration that can reach the state. Every other non-nullable
property is discharged at its constructor (`rule:classes/definite-property-initialization`), and
reflection builds no object without running its constructor
(`rule:security/reflection-enforces-visibility`).

Internally the slot carries one non-user-observable "never written" marker, distinct from every legal
value including `null`. It is not an entry in the type system and no expression produces one. It costs
zero additional bytes per property: the tagged value representation already had a spare discriminant,
and the compiled read goes by the payload — null in this state and in no other, because `lateinit` is
restricted to a non-nullable class or interface type.
