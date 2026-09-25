Returns the cube root of the number you give it. The cube root of `27.0` is `3.0`, because
3 × 3 × 3 is 27.

A negative number works too. The cube root of `-8.0` is `-2.0`, so the result always has the same
sign as the number. PHP has no function for this. PHP code often writes `pow($n, 1/3)`, which gives
`NaN` (a value that means "not a number") for a negative number.

**In plain words:** a cube has the same length on every side. When you know how much space a cube
fills, the cube root tells you how long each side is.

**The examples below** show a few cube roots, then negative numbers, then how a shop finds the size
of a cube-shaped box that holds a given volume.
