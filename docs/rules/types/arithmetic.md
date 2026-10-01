| operation | result | on overflow / edge |
|---|---|---|
| `int ⊕ int`, `uint ⊕ uint` for `+ - * ** %` | the same type | **throws `ArithmeticError`.** No wrap, no promotion to `float` |
| `int ⊕ uint` arithmetic | **compile error** | there is no representable common type; convert one side explicitly |
| `int / int`, `uint / uint` | `int\|float`, `uint\|float` — PHP-exact: `6/3` is an integer, `7/2` is a float | `/ 0` throws `ArithmeticError` |
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
case is not a number, and `as int` (`as ?int` for a nullable one) is how its backing integer joins a
computation. `-$size as int` already reads that way, because `as` binds tighter than a prefix operator.

Division is the one row that returns a union, and in practice the target's declared type absorbs it
through the `int → float` widening (`rule:types/implicit-widening`): `float $avg = $sum / $n;` works,
`int $n = 7 / 2;` is a diagnostic, and `Core\Math::intDiv` is there when integer division was meant.

An operand whose static type names no row — `mixed`, a union, the `int|float` a division returns — is
answered from its runtime **tag**: the rows above where the tags name one, and the same refusal as a
*catchable throw* where they do not, carrying the diagnostic's own wording. A union that can hold an
enum case is the exception and is refused where it is written, because a case carries its backing
integer's tag (`rule:enums/representation`) and no tag test can tell it from a number.

Overflow throwing is the divergence this table is least willing to trade. A silent promotion to
`float` changes a binding's type behind its declaration, and a silent wrap is the classic
size-computation bug. Code that wants unbounded magnitude declares `float`, or converts.
