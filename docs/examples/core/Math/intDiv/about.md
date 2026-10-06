Divides one whole number by another and returns a whole number. The remainder is dropped, so the
result moves toward zero: `7` divided by `2` is `3`, and `-7` divided by `2` is `-3`. The `%`
operator gives the remainder that this function drops.

Dividing by `0` throws an `ArithmeticError`. Dividing `Core\Math::INT_MIN` by `-1` also throws an
`ArithmeticError`, because the result is one more than `Core\Math::INT_MAX`.

Use it when only complete groups count: full pages, full boxes, full minutes.
`Core\Math::floor` rounds a `float` down instead.

**The examples below** show a few divisions, then negative numbers and a division by zero, then
how to turn a number of seconds into minutes and seconds.
