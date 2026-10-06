Rounds a number up to the next whole number. `4.1` becomes `5.0`, and a number that is already
whole stays the same.

For a negative number, up means toward zero: `-4.1` becomes `-4.0`. A number between `-1.0` and
`0.0` becomes `-0.0`, which prints as `-0`. The result is always a `float`. `NaN` (a value that
means "not a number") and the infinities stay the same.

Use it when a part still needs a whole one: a page that is only half full is still a page, and a
box with one item in it is still a box. `Core\Math::floor` rounds down, and `Core\Math::round`
rounds to the nearest number.

**The examples below** show a few numbers rounded up, then negative numbers, then how a list finds
how many pages it needs.
