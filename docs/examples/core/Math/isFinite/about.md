`Core\Math::isFinite` returns `true` when a `float` is an ordinary number. A `float` can also be
one of three special values: `INFINITY`, `-INFINITY` and `NaN` (a value that means "not a
number"). For those three, the result is `false`.

A special value usually comes from a calculation that went out of range. A number too big for a
`float` becomes `INFINITY`. Zero divided by zero with `Core\Math::fdiv` gives `NaN`. When a
special value is added to a total, the total becomes special too. So test a result with
`Core\Math::isFinite` before you save it or show it.

To tell an infinity from `NaN`, use `Core\Math::isNan` as well. An infinity is not finite and is
not `NaN`.

**The examples below** show which values are finite, then how to tell the three special values
apart, then a savings calculator that checks its result before it shows it.
