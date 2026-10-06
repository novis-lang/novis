| operation | result | on overflow / edge |
|---|---|---|
| `int ⊕ int`, `uint ⊕ uint` for `+ - * ** %` | the same type | **throws `ArithmeticError`.** No wrap, no promotion to `float` |
| `int ⊕ uint` arithmetic | **compile error** | there is no representable common type; convert one side explicitly |
| `int / int`, `uint / uint` | `int\|float`, `uint\|float` — an exact quotient is an integer and any other is a float: `6/3` is `2`, `7/2` is `3.5` | `/ 0` throws `ArithmeticError` |
| either operand a `float` | `float` for `+ - * ** /`; **`%` is a compile error** | `/ 0` throws here too — the zero divisor is refused before the operand types are consulted. IEEE division is `Core\Math::fdiv` |
| `decimal ⊕ decimal`, `decimal ⊕ int`, `decimal ⊕ uint` for `+ - * %` | `decimal` | throws when the mantissa exceeds 96 bits **or** the scale would exceed 28 |
| `decimal / decimal` | `decimal`, half-even at the maximum scale the result admits | `/ 0` throws |
| `decimal ⊕ float`; `**` with a `decimal` base | **compile error** | no representable common type; `Core\Decimal::pow` for the power |
| `>>` | arithmetic on `int`, **logical on `uint`** | — |
| `& \| ^ ~ <<` | the operand type, preserved | — |

The arithmetic rows are a **closed** list. Their operands are `int`, `uint`, `float` and `decimal`, so
a `bool`, a `string`, a `bytes`, an `array<T>`, a `callable`, `null`, an object and an enum value have
no `+` at all and are refused where they are written. `%` is narrower than its own float row: a
`float` operand is refused rather than given one of two plausible answers, and `Core\Math::mod` is the
member that says the floating-point remainder out loud.

An enum value is refused under every arithmetic and bitwise operator, prefix `-`, `+`, `~` and
`++`/`--` included, and so is any type that can hold one: a case-subset type, `?Size`, `int|Size`. A
case is not a number, and `as int` is how its backing integer joins a computation. A nullable one
converts with `as ?int` and then needs a default, `($size as ?int) ?? 0`, because an operand that can
hold `null` is refused too (below). `-$size as int` already reads that way, because `as` binds tighter
than a prefix operator.

Division is the one row that returns a union, and in practice the target's declared type absorbs it
through the `int → float` widening (`rule:types/implicit-widening`): `float $avg = $sum / $n;` works,
`int $n = 7 / 2;` is a diagnostic, and `Core\Math::intDiv` is there when integer division was meant.

An operand whose static type names no row — `mixed`, a union, the `int|float` a division returns — is
answered from its runtime **tag**: the rows above where the tags name one, and the same refusal as a
*catchable throw* where they do not, carrying the diagnostic's own wording. A union that can hold an
enum case is the exception and is refused where it is written: a case reaches arithmetic only
through `as int` or `as uint`, and a tagged operand's arithmetic reads a case as its backing integer
(`rule:enums/representation`).

A union that can hold `null` — `?float`, `int|float|null` — is the second exception, refused under
the same operators. `null` has no row, so its tag would throw on exactly the path a test is least
likely to take; the fix is a `!= null` test, which narrows (`rule:types/narrowing`), or a default
through `??`. `mixed` is not a union and is still answered from its tag.

Overflow throwing is the divergence this table is least willing to trade. A silent promotion to
`float` changes a binding's type behind its declaration, and a silent wrap is the classic
size-computation bug. Code that wants unbounded magnitude declares `float`, or converts.
