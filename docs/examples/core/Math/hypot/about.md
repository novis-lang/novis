`Core\Math::hypot` returns the length of the longest side of a right triangle. You give it the
lengths of the two other sides. This is also the straight-line distance between two points on a
flat map: give it the difference in x and the difference in y. This replaces PHP's `hypot`.

The result is the square root of `$a * $a + $b * $b`. You could write that with
`Core\Math::sqrt`, but it fails for very big numbers. `$a * $a` can be too big for a `float` and
become `INFINITY`, even when the length itself fits. `Core\Math::hypot` gives the correct length
there too.

The sign of each side does not change the result, and the result is never negative.

**The examples below** show two triangles, then the difference from the `sqrt` form with a very
big number, then a program that finds the nearest store on a map.
