`Core\Math::round` rounds a `float` to a number of decimal places. This replaces PHP's `round`.

The `precision` option says how many decimal places to keep. The default is `0`, which gives a
whole number. A negative `precision` rounds to tens, hundreds and so on, so `-2` turns `1234.5`
into `1200`. The result is always a `float`.

The `mode` option decides what happens to a value exactly halfway between two results, such as
`2.5`. The default, `Core\RoundMode::HalfUp`, rounds it away from zero, so `2.5` becomes `3`.
`HalfDown` rounds it toward zero, so `2.5` becomes `2`. `HalfEven` gives the even neighbour, and
`HalfOdd` gives the odd one. `Up` and `Down` round every value away from or toward zero.

A `float` cannot store most decimal fractions exactly. `1.005` is stored as a number slightly
below it, so with two decimal places it rounds down to `1`. Use a `decimal` when the digits must
be exact, such as for money.

**The examples below** show `precision` and `mode`, then tens and hundreds, then an average
rating shown with one decimal place.

related: Core\Math::ceil, Core\Math::floor, Core\Math::truncate
