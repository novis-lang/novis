An enum value is its backing integer and nothing more — zero additional bytes over what `int` or
`uint` already costs, no allocation, no refcount, no descriptor. Where the static type is known,
which the type system makes the common case, codegen already knows which enum it is and there is
nothing further to carry.

A runtime tag of its own is **reserved for an enum but not spent**. Materialized into a tagged
value, a case takes the tag of the type backing it, `Int` or `Uint`. A tag only has to answer "which
type is this?" where the static type does not — the `mixed` case, whose representation is still
open. The consequence to obey today: a value that reaches `mixed` is not distinguishable there from
its backing integer, nor one enum from another enum with the same backing.

Crossing an isolate boundary copies a plain scalar, with no object identity to preserve or discard.
