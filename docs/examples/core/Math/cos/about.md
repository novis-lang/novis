Returns the cosine of an angle. The angle is in radians, and `Core\Math::toRadians` converts
degrees to radians. The result is always from `-1.0` to `1.0`. An infinity or `NaN` (a value that
means "not a number") gives `NaN`. This replaces PHP's `cos`.

**In plain words:** draw a line of length 1 from the centre of a circle, at the angle you give. The
cosine is how far the end of that line is to the right of the centre.

**Good to know:** the result is not always exact. The cosine of 90 degrees is zero, but the result
is a tiny number close to zero. Round the result when you show it.

**The examples below** show the cosines of a few angles, then angles in degrees and rounding, then
how far a sloped path goes on a map.
