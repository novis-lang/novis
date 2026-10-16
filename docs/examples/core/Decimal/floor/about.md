Cuts a number down, to a whole number or to a chosen number of digits after the point.

`Core\Decimal::floor` always moves towards the smaller number. `2.9` becomes `2`, and `-2.1` becomes
`-3`. The second argument says how many digits after the point to keep, and leaving it out gives a
whole number. The answer is exact, because it is a `decimal` and not a floating-point number.

Reach for it when the part you cut off is worth nothing to the person on the other side: how many
whole items a budget buys, how many full hours a job is billed for, or a commission cut to cents so
a shop never pays out more than the rate allows.

**Good to know:** `Core\Decimal::ceil` moves towards the larger number, and
`Core\Decimal::truncate` drops the digits after the point. All three agree on a positive number. On
a negative one `floor` is the only one that can reach a smaller number, so `floor(-0.4)` is `-1`
while the other two answer `0`.
