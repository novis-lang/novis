A position in an image, written as a direction from the center. Image programs call this position the
gravity.

`Image::Gravity` is declared in the class `Novis\Image\Image`.

`composite` and `text` use an `Image::Gravity` as `gravity` to place the overlay or the text. `resize`
uses it to place the image when `fit` is `Image::Fit::Contain`.

When `fit` is `Image::Fit::Cover`, the image fills the box and some of it is cut off. The gravity
chooses which part `resize` keeps. `Center` keeps the middle and is the default. `North` keeps the
top, `South` the bottom, `West` the left and `East` the right. The other four keep a corner.
`NorthEast` keeps the top right and `SouthEast` the bottom right. `SouthWest` keeps the bottom left
and `NorthWest` the top left.

For example, a thumbnail of a portrait photo often looks better with `North`, because the face is
usually near the top.
