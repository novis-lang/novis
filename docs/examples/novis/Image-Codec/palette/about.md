Returns the main colours of an encoded image, the most common first.

`Codec::palette` takes an encoded image and a `count`. It returns at most `count` colours. Each colour
is a shape with the keys `r`, `g` and `b`, from 0 to 255, and `alpha`, from 0 to 1. Transparent pixels
are not counted, so a fully transparent image returns an empty list.

`Codec::palette` throws a `LogicError` when `count` is above 256, and a `ParseError` when the bytes
are not an image in a format Novis reads.

**Good to know:** most programs do not call `Codec::palette` directly. `Image::palette` calls it, uses
5 when you give no `count`, and returns each colour as a `Color`.
