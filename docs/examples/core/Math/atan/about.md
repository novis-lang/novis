Returns the angle whose tangent is the number you give it. This is the arc tangent, and it undoes
`Core\Math::tan`.

Any number works, because every number is the tangent of some angle. The angle is in radians,
between `-PI / 2` and `PI / 2`. An infinite number gives exactly `PI / 2` or `-PI / 2`.
`Core\Math::toDegrees` converts the angle to degrees.

**Good to know:** when you divide one coordinate by the other to get the tangent, the signs of the
two coordinates are lost. The points (1, 1) and (-1, -1) then give the same angle. Use
`Core\Math::atan2` for a point, because it takes both coordinates.

**The examples below** show the angles for a few tangents, then very big tangents and the case where
`Core\Math::atan2` is the right choice, then the angle of a road from its gradient.
