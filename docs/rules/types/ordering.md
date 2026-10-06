`< <= > >= <=>` are defined on a **closed** list of operand pairs, and refused everywhere else:

| operands | result |
|---|---|
| two numerics, `int` against `uint` included | `bool` (`int` for `<=>`), mathematically exact over the full range of both |
| `decimal` against `int`, `uint` or `float` | `bool`, exact — a comparison is computable where a common arithmetic type is not |
| two `bool`s | `false < true`, the ordering of the one bit they already are |
| two objects whose static type is provably the same class implementing `Comparable` | `bool`/`int`, via `compareTo`; a throwing `compareTo` propagates as a checked status like any other call |
| any other operands | **compile error** |

There is no fallback. Ordering any other pair would mean converting an operand first, and there is no
implicit conversion for that to be. So two strings order through `Core\Str::compare`, an enum
case orders through its backing `as int` (`rule:enums/closed-integer-type`), and an `array<T>`, a
`callable` and `null` do not order at all. Two objects with no `Comparable` between them are a
compile error whose diagnostic names `Comparable` as the fix, however the receiver was spelled — an
erased `object` and a shape type included.

An operand whose static type names no row is answered from its runtime tag, and refuses as a
*catchable throw* where the tags name none. Two consequences follow from the tag being all there is:
two objects behind two `mixed`s throw, because `compareTo` is dispatched from the class the *site*
named, and an enum case orders as the integer it is even though the written spelling is still refused
— which is where the author is told to say `as int`.

`==` and `!=` are a different question and are unaffected by any of this.
