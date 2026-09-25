Returns the angle whose cosine is the number you give it. This is the arc cosine.

The number must be from `-1.0` to `1.0`, because a cosine is always in that range. The angle is in
radians, from `0.0` to `PI`. `Core\Math::toDegrees` converts it to degrees. A number outside the
range gives `NaN` (a value that means "not a number"), and `Core\Math::isNan` tests for it. This
replaces PHP's `acos`.

**Good to know:** a cosine you calculated can end up a tiny bit past `1.0` because of rounding, and
then the result is `NaN`. Use `Core\Math::clamp` to keep the number in range first.

**The examples below** show the angles for a few cosines, then what happens outside the range, then
the angle between two directions.
