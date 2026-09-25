Converts an angle from degrees to radians. `Core\Math::sin`, `Core\Math::cos` and `Core\Math::tan`
need the angle in radians, so you use this first when your angle is in degrees. A full circle is
360 degrees or `2 * PI` radians, so 180 degrees is `PI` radians. `Core\Math::toDegrees` converts
the other way. An infinity gives an infinity, and `NaN` (a value that means "not a number") gives
`NaN`. This replaces PHP's `deg2rad`.

**Good to know:** the result is computed the same way as in PHP. When you convert an angle to
radians and back with `Core\Math::toDegrees`, you get the same number as in PHP.

**The examples below** show a few angles, then a conversion to radians and back, then how a map
app finds the distance between two cities.
