Returns the angle of a point, measured from the positive x-axis. You give it the point's two
coordinates, `$y` first and then `$x`.

The angle is in radians, from `-PI` to `PI`. Points above the x-axis give a positive angle, and
points below it give a negative angle. `Core\Math::toDegrees` converts the angle to degrees. The
point (0, 0) gives `0.0`. This replaces PHP's `atan2`.

**Good to know:** `Core\Math::atan` of `$y / $x` loses the signs of the two coordinates, so it
cannot tell (1, 1) from (-1, -1). `Core\Math::atan2` takes both coordinates, so it always gives the
right direction. The order is easy to get wrong: `$y` comes first.

**The examples below** show the angles of four points, then points on the axes, then a compass
direction from a map.
