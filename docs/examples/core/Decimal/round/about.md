Rounds a number to as many digits as you ask for, in the direction you name.

`Core\Decimal::round` takes the value, a `Core\RoundMode` case, and how many digits to keep after the
point. Leaving the last one out gives a whole number. The mode decides where a value that sits
exactly between two neighbours goes, so `0.125` at two digits is `0.13` under `Core\RoundMode::HalfUp`
and `0.12` under `Core\RoundMode::HalfEven`. The answer is exact, because it is a `decimal` and not a
floating-point number. A scale above 28 throws an `ArithmeticError`.

Use it where the rounding rule belongs to your program: a tax line, a price a shop prints, a figure a
report states at one digit.

**Good to know:** `Core\Decimal::floor`, `Core\Decimal::ceil` and `Core\Decimal::truncate` each move
in one fixed direction and take no mode. Name a mode here when the direction is part of your rule.
