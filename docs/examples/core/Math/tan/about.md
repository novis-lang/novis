Returns the tangent of an angle. The angle is in radians, and `Core\Math::toRadians` converts
degrees to radians. The tangent is the sine divided by the cosine, so it can be any number. An
infinity or `NaN` (a value that means "not a number") gives `NaN`. This replaces PHP's `tan`.

**In plain words:** the tangent says how steep a slope is. It is how far the slope goes up for
each step of 1 forward.

**Good to know:** the result gets very large as the angle gets close to 90 degrees. A `float`
cannot store 90 degrees in radians exactly, so the tangent of 90 degrees is a very large number.
There is no error.

**The examples below** show the tangents of a few angles, then angles close to 90 degrees, then
how a map app finds the grade of a road.
