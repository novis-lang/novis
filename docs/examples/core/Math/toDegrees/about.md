Converts an angle from radians to degrees. `Core\Math::atan2`, `Core\Math::asin` and
`Core\Math::acos` return angles in radians. People usually read angles in degrees, so you use this
before you show an angle. `PI` radians is 180 degrees. `Core\Math::toRadians` converts the other
way. An infinity gives an infinity, and `NaN` (a value that means "not a number") gives `NaN`.

**The examples below** show a few angles, then angles that other functions return, then how a
delivery app finds the compass direction to a customer.
