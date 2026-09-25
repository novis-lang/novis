Returns the angle whose sine is the number you give it. This is the arc sine, and it undoes
`Core\Math::sin`.

The number must be from `-1.0` to `1.0`, because a sine is always in that range. The angle is in
radians, from `-PI / 2` to `PI / 2`. `Core\Math::toDegrees` converts it to degrees. A number outside
the range gives `NaN` (a value that means "not a number"), and `Core\Math::isNan` tests for it. This
replaces PHP's `asin`.

**Good to know:** a sine you calculated can end up a tiny bit past `1.0` because of rounding, and
then the result is `NaN`. Use `Core\Math::clamp` to keep the number in range first.

**The examples below** show the angles for a few sines, then what happens outside the range, then
how steep a ramp is from its length and its height.
