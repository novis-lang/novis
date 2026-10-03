An enum value is its backing integer and nothing more — zero additional bytes over what `int` or
`uint` already costs, no allocation, no refcount, no descriptor. Where the static type is known,
which the type system makes the common case, codegen already knows which enum it is and there is
nothing further to carry.

Materialized into a tagged value — a `mixed`, an `array<mixed>` element, an erased parameter — a
case takes an **enum tag of its own**, one per backing type: `EnumInt` over an `int` payload and
`EnumUint` over a `uint` one. A condition, a type test, a closure parameter's check and a declared
slot's write check read that tag, so a case in a `mixed` is always truthy (`rule:enums/truthiness`),
`is int` is `false` for it and `is` an enum is `false` for a plain integer. Every reader that only
decodes the value — printing, conversion, comparison, `===`, serialization — reads it as its backing
integer. The tag names no enum, so two enums with the same backing are not told apart in a `mixed`.

Crossing an isolate boundary copies a plain scalar, with no object identity to preserve or discard.
