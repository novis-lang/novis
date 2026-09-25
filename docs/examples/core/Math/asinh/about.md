Returns the inverse hyperbolic sine of the number you give it. It undoes `Core\Math::sinh`, so
`Core\Math::asinh(Core\Math::sinh(2.0))` gives `2.0` back.

Every number has a result. The result has the same sign as the number, and `0.0` gives `0.0`. Near
zero, the result is close to the number itself. For big numbers the result grows slowly, like a
logarithm. This replaces PHP's `asinh`.

**Good to know:** `Core\Math::log` has no usable result for zero or for a negative number.
`Core\Math::asinh` works for every number. That makes it a common way to draw a scale for values
that can be negative, such as profit and loss.

**The examples below** show a few results, then negative and very big numbers, then the bar
heights of a profit and loss chart.
