Makes a colour from its red, green, blue and alpha values.

`Color::rgba($r, $g, $b, $alpha)` returns a `Color`. Red, green and blue are whole numbers from 0 to
255. Alpha is how opaque the colour is: `0.0` is fully transparent and `1.0` is fully opaque. If you
leave alpha out, it is `1.0`.

The four values are on the `Color` as `r`, `g`, `b` and `alpha`, and you cannot change them. A
channel above 255 throws a `LogicError`. An alpha below `0.0`, above `1.0` or `NAN` also throws a
`LogicError`.

You give a `Color` to the image methods that need one: `Image::create` fills a new image with it,
`flatten` puts it behind transparent pixels, and `tint` colours an image with it.

**Good to know:** `Color::hex` makes the same colour from text such as `"#ff8800"`. Use it when the
colour comes from a setting or a style sheet.
