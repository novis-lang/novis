Cuts a number down, to a whole number or to a chosen number of digits after the point.

`Core\Decimal::floor` always moves towards the smaller number. `2.9` becomes `2`, and `-2.1` becomes
`-3`. The second argument says how many digits after the point to keep, and leaving it out gives a
whole number. The result is exact, because it is a `decimal` and not a floating-point number.

Use it when the part you cut off is worth nothing to the person on the other side. Examples are how
many whole items a budget buys, and how many full hours a job is billed for. Another is a commission
cut to cents, so a shop never pays out more than the rate allows.

**Good to know:** `Core\Decimal::ceil` moves towards the larger number, and
`Core\Decimal::truncate` removes the digits after the point. All three give the same result for a
positive number. They differ for a negative number: `floor(-0.4)` is `-1`, and the other two return `0`.
