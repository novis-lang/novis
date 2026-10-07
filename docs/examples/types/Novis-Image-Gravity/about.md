A position in an image, written as a direction from the center.

`resize` uses a `Gravity` as `gravity` when `fit` is `Fit::Cover`. The image fills the box and some
of it is cut off, and the gravity says which part is kept. `Center` keeps the middle and is the
default. `North` keeps the top, `South` the bottom, `West` the left and `East` the right.
`NorthEast`, `SouthEast`, `SouthWest` and `NorthWest` keep a corner.

For example, a thumbnail of a portrait photo often looks better with `North`, because the face is
usually near the top.
