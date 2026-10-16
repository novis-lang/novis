Rounds a number up, to a whole number or to a chosen number of digits after the point.

`Core\Decimal::ceil` always moves towards the larger number. `2.1` becomes `3`, and `-2.9` becomes
`-2`. The second argument says how many digits after the point to keep, and leaving it out gives a
whole number. The answer is exact, because it is a `decimal` and not a floating-point number.

Reach for it when a part of something still costs a whole one. The pages a list needs, the boxes an
order fills and a fee that is always rounded in the seller's favour are all this.

**Good to know:** `Core\Decimal::floor` is the same cut in the other direction, and
`Core\Decimal::truncate` cuts towards zero. The three give the same answer for a positive number and
part on a negative one.
