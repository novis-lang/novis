An enum value is its backing integer and nothing more — zero additional bytes over what `int` or
`uint` already costs, no allocation, no refcount, no descriptor. Where the static type is known,
which the type system makes the common case, codegen already knows which enum it is and there is
nothing further to carry.

Materialized into a tagged value — a `mixed`, an `array<mixed>` element, an erased parameter — a
case takes an **enum tag of its own**, one per backing type: `EnumInt` over an `int` payload and
`EnumUint` over a `uint` one. A condition, a type test, an anonymous function parameter's check and a declared
slot's write check read that tag, so a case in a `mixed` is always truthy (`rule:enums/truthiness`),
`is int` is `false` for it and `is` an enum is `false` for a plain integer. Every reader that only
decodes the value — printing, conversion, comparison, `===`, serialization — reads it as its backing
integer. The tag names no enum, so two enums with the same backing are not told apart in a `mixed`.

One tag per backing keeps the payload's signedness with the value, so nothing reads a `uint` case above
`i64::MAX` as a negative number, and no padding byte is written. The tag is written only where a case
is materialized into a tagged value, so a statically typed enum still costs nothing. An anonymous
function's enum parameter accepts a case of an enum with its backing and throws `LogicError` for an
`int`, and an `int` or `float` parameter throws for a case. An `array<T>` element-tag walk reads the
enum tag too. `===` and the hash beside it read the backing integer, so a case in a `mixed` is
identical there to an integer of the same value, and the two agree. `$any as Rank` still converts an
integer to a case, because the conversion reads the payload, and `Core\Reflect`'s type kind reports a
case in a `mixed` as `int` or `uint`.

Crossing an isolate boundary copies a plain scalar, with no object identity to preserve or discard.

- **Memory:** an enum value costs exactly what its `int` or `uint` costs, in the same payload. Nothing
  is held per enum, per request or per isolate: no case table, no singleton and no type descriptor.

**Declined:**
- **A case keeping the `Int` or `Uint` tag in a `mixed`.** A case backed by `0` would then be falsy
  there and truthy where its type is known, `is int` would be `true` for it, and `is Rank` would be
  `true` for any integer equal to one of its cases.
- **One enum tag with the signedness in a padding byte.** Every tag write would have to write the pad,
  and every reader would need a second byte to read the payload.
- **The enum's identity in the padding bytes.** It tells two enums apart in a `mixed`, which no rule
  asks for, and it costs a store on every materialization.
- **The payload-reading tag accessor returning the enum tags.** Every reader that decodes a payload
  would need a new arm, a few hundred of them, for a value they all read as its integer.
