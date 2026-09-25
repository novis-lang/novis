Returns the sine of an angle. The angle is in radians, and `Core\Math::toRadians` converts
degrees to radians. The result is always from `-1.0` to `1.0`. An infinity or `NaN` (a value that
means "not a number") gives `NaN`. This replaces PHP's `sin`.

**In plain words:** draw a line of length 1 from the centre of a circle, at the angle you give. The
sine is how far the end of that line is above the centre.

**Good to know:** the result is not always exact. The sine of 180 degrees is zero, but the result
is a tiny number close to zero. Round the result when you show it.

**The examples below** show the sines of a few angles, then angles in degrees and rounding, then
how high a ramp rises.
